pub mod config;
pub mod error;
pub mod models;

// Publicly re-export the main types for a clean external API.
pub use config::Settings;
pub use error::{Error, Result};
pub use models::{ArchivedRecord, ScannedProject};

use chrono::{DateTime, Duration, Utc};
use git2::Repository;
use std::fs;
use std::path::Path;
use tracing::{debug, info, instrument, span, warn, Level};
use walkdir::WalkDir;

/// Represents a planned action during a dry run.
#[derive(Debug, PartialEq)]
pub enum ActionPlan {
    Archive { project_name: String, path: std::path::PathBuf },
    Nothing,
}

#[derive(Debug)]
pub struct Archiver {
    settings: Settings,
}

impl Archiver {
    const ARCHIVE_LOG_FILE: &'static str = "archive.json";

    pub fn new(settings: Settings) -> Self {
        Self { settings }
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    #[instrument(skip(self), name = "archive_process", fields(dry_run = %dry_run))]
    pub fn run_archive_process(&self, dry_run: bool) -> Result<Vec<ActionPlan>> {
        info!("Starting archive process...");
        let projects = self.scan_projects()?;
        info!(project_count = projects.len(), "Scan complete.");

        let inactive_projects = self.filter_inactive_projects(projects);
        if inactive_projects.is_empty() {
            info!("No inactive projects to archive. Process finished.");
            return Ok(vec![ActionPlan::Nothing]);
        }
        info!(count = inactive_projects.len(), "Found inactive projects to archive.");

        let mut plan = vec![];
        let mut new_records = vec![];

        for project in &inactive_projects {
            plan.push(ActionPlan::Archive {
                project_name: project.name.clone(),
                path: project.path.clone(),
            });
            if !dry_run {
                let project_span = span!(Level::INFO, "archive_project", project_name = %project.name);
                let _enter = project_span.enter();
                info!("Archiving project...");
                let record = self.archive_project(project)?;
                new_records.push(record);
            }
        }

        if !dry_run {
            self.append_to_archive_log(&new_records)?;
            info!("Archive process finished successfully.");
        } else {
            info!("Dry run complete. No files were changed.");
        }

        Ok(plan)
    }

    #[instrument(skip(self))]
    pub fn restore_project(&self, project_name: &str) -> Result<()> {
        info!(%project_name, "Attempting to restore project.");
        let mut all_records = self.get_archive_records()?;
        let record_idx = all_records.iter().position(|r| r.name == project_name)
            .ok_or_else(|| Error::Custom(format!("Project '{}' not found in archive log.", project_name)))?;
        let record = all_records.get(record_idx).unwrap();
        debug!(from = %record.archive_path.display(), to = %record.original_path.display(), "Moving project directory.");
        if let Some(parent) = record.original_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(&record.archive_path, &record.original_path)?;
        all_records.remove(record_idx);
        self.write_archive_log(&all_records)?;
        info!(%project_name, "Project restored successfully.");
        Ok(())
    }

    #[instrument(skip(self))]
    fn scan_projects(&self) -> Result<Vec<ScannedProject>> {
        let mut projects = Vec::new();
        let archive_dir_name = self.settings.archive_dir.file_name();
        debug!(directory = %self.settings.projects_dir.display(), "Scanning for projects.");

        for entry_result in WalkDir::new(&self.settings.projects_dir).min_depth(1).max_depth(1) {
            let entry = entry_result?;
            if Some(entry.file_name()) == archive_dir_name {
                debug!(path = %entry.path().display(), "Skipping archive directory.");
                continue;
            }

            let project_name = entry.file_name().to_string_lossy();
            if self.settings.exclude.iter().any(|excluded| *excluded == project_name) {
                debug!(name = %project_name, "Skipping excluded project.");
                continue;
            }


            let path = entry.path();
            if !path.is_dir() { continue; }

            match self.get_last_activity(path) {
                Ok(last_activity) => {
                    projects.push(ScannedProject {
                        name: entry.file_name().to_string_lossy().into_owned(),
                        path: path.to_path_buf(),
                        last_activity,
                    });
                }
                Err(e) => {
                    warn!(path = %path.display(), error = %e, "Could not determine activity for directory, skipping.");
                }
            }
        }
        Ok(projects)
    }

    /// Determines the last activity of a directory, trying Git first and falling back to file mtime.
    fn get_last_activity(&self, path: &Path) -> Result<DateTime<Utc>> {
        if path.join(".git").is_dir() {
            match self.get_git_last_activity(path) {
                Ok(dt) => return Ok(dt),
                Err(e) => {
                    // If Git fails (e.g., empty repo), we don't give up.
                    // We log it and fall back to checking file modification times.
                    debug!(path = %path.display(), error = %e, "Git activity check failed, falling back to mtime.");
                }
            }
        }
        // Fallback for non-git repos or failed git repos
        self.find_latest_mtime(path)
    }

    /// Gets the last activity time from a Git repository.
    fn get_git_last_activity(&self, path: &Path) -> Result<DateTime<Utc>> {
        let repo = Repository::open(path)?;
        let last_commit = self.find_last_commit_across_branches(&repo)?;
        let commit_time = last_commit.time();

        DateTime::from_timestamp(commit_time.seconds(), 0)
            .ok_or_else(|| Error::Custom("Invalid commit time".to_string()))
            .map(|dt| dt.with_timezone(&Utc))
    }

    /// Finds the latest modification time for a non-Git directory.
    fn find_latest_mtime(&self, dir_path: &Path) -> Result<DateTime<Utc>> {
        let latest_file_mtime = WalkDir::new(dir_path)
            .into_iter()
            .map(|entry_result| -> Result<Option<DateTime<Utc>>> {
                let entry = entry_result?;
                if entry.file_type().is_file() {
                    let metadata = entry.metadata()?;
                    let modified: DateTime<Utc> = metadata.modified()?.into();
                    Ok(Some(modified))
                } else {
                    Ok(None)
                }
            })
            .collect::<Result<Vec<Option<DateTime<Utc>>>>>()?
            .into_iter()
            .flatten()
            .max();

        // If the latest file time was found, return it.
        // Otherwise, fall back to the directory's own modification time.
        if let Some(latest) = latest_file_mtime {
            Ok(latest)
        } else {
            let dir_meta = fs::metadata(dir_path)?;
            let dir_mtime: DateTime<Utc> = dir_meta.modified()?.into();
            Ok(dir_mtime)
        }
    }
    /// Finds the most recent commit across all local branches in a repository.
    fn find_last_commit_across_branches<'repo>(
        &self,
        repo: &'repo Repository,
    ) -> Result<git2::Commit<'repo>> {
        repo.branches(Some(git2::BranchType::Local))?
            // Usar una clausura para resolver la ambigüedad de tipos.
            .filter_map(|res| res.ok())
            .filter_map(|(branch, _)| branch.get().peel_to_commit().ok())
            .max_by_key(|commit| commit.time().seconds())
            .ok_or_else(|| {
                Error::Git(git2::Error::new(
                    git2::ErrorCode::UnbornBranch,
                    git2::ErrorClass::Reference,
                    "No commits found in any local branch",
                ))
            })
    }

    fn filter_inactive_projects(&self, projects: Vec<ScannedProject>) -> Vec<ScannedProject> {
        let now = Utc::now();
        let inactivity_period = Duration::days(self.settings.inactivity_days as i64);
        projects.into_iter().filter(|p| now.signed_duration_since(p.last_activity) > inactivity_period).collect()
    }

    #[instrument(skip(self, project))]
    fn archive_project(&self, project: &ScannedProject) -> Result<ArchivedRecord> {
        let project_name = &project.name;
        let dest_path = self.settings.archive_dir.join(project_name);
        debug!(from = %project.path.display(), to = %dest_path.display(), "Moving project directory.");
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(&project.path, &dest_path)?;
        Ok(ArchivedRecord { name: project_name.clone(), original_path: project.path.clone(), archive_path: dest_path, archived_at: Utc::now() })
    }

    #[instrument(skip(self, new_records))]
    fn append_to_archive_log(&self, new_records: &[ArchivedRecord]) -> Result<()> {
        if new_records.is_empty() { return Ok(()); }
        info!(count = new_records.len(), "Appending to archive log file.");
        let mut all_records = self.get_archive_records()?;
        all_records.extend_from_slice(new_records);
        self.write_archive_log(&all_records)
    }

    #[instrument(skip(self, records))]
    fn write_archive_log(&self, records: &[ArchivedRecord]) -> Result<()> {
        let log_path = self.settings.archive_dir.join(Self::ARCHIVE_LOG_FILE);
        debug!(path = %log_path.display(), "Writing archive log.");
        let json_data = serde_json::to_string_pretty(records)?;
        fs::write(log_path, json_data)?;
        Ok(())
    }

    pub fn get_archive_records(&self) -> Result<Vec<ArchivedRecord>> {
        let log_path = self.settings.archive_dir.join(Self::ARCHIVE_LOG_FILE);
        debug!(path = %log_path.display(), "Reading archive records.");
        if !log_path.exists() {
            warn!("Archive log file not found. Returning empty list.");
            return Ok(Vec::new());
        }
        let file_content = fs::read_to_string(log_path)?;
        Ok(serde_json::from_str(&file_content)?)
    }
}
