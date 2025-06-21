use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CleanupRule {
    /// File that detects the type of project (e.g. “package.json”).
    pub detection_file: String,
    /// Folders to be deleted (e.g. “node_modules”).
    pub folders_to_delete: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Settings {
    /// Directory where projects are stored.
    pub projects_dir: PathBuf,

    /// Directory where projects will be archived.
    pub archive_dir: PathBuf,

    /// Number of days of inactivity before a project is considered for archiving.
    pub inactivity_days: u64,

    /// Rules for cleaning up projects before archiving.
    pub cleanup_rules: Vec<CleanupRule>,

    /// Whether to enable automatic deletion of archived projects.
    pub enable_auto_delete: bool,

    /// Number of days before an archived project is deleted.
    pub days_before_delete: u64,

    /// A list of project names to exclude from archiving.
    pub exclude: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            projects_dir: PathBuf::new(),
            archive_dir: PathBuf::new(),
            inactivity_days: 30,
            cleanup_rules: vec![],
            enable_auto_delete: false,
            days_before_delete: 365,
            exclude: vec![],
        }
    }
}

impl Settings {
    pub fn config_path() -> Result<PathBuf> {
        let home_dir = std::env::var("HOME").map_err(|_| Error::HomeDirNotFound)?;
        Ok(PathBuf::from(format!(
            "{}/.config/archiver/settings.toml",
            home_dir
        )))
    }
    
    pub fn new() -> Result<Self> {
        let home_dir = std::env::var("HOME").map_err(|_| Error::HomeDirNotFound)?;

        let config_builder = config::Config::builder()
            .add_source(
                config::File::with_name(&format!("{}/.config/archiver/settings", home_dir))
                    .required(false),
            )
            .add_source(config::Environment::with_prefix("ARCHIVER"))
            .set_default("projects_dir", format!("{}/Proyectos", home_dir))?
            .set_default("archive_dir", format!("{}/.archive", home_dir))?
            .set_default("inactivity_days", 30)?
            .build()?;
        config_builder.try_deserialize().map_err(Error::Config)
    }
}
