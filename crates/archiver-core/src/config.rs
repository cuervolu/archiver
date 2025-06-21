use crate::error::{Error, Result};
use directories::{ProjectDirs, UserDirs};
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
    // The defaults are set in `Settings::new()` in order to handle errors.
    // This `default()` is mainly for `serde`. The default command is executed with `just`.
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
    
    const CONFIG_FILE_NAME: &'static str = "settings.toml";
    
    const APP_NAME: &'static str = "archiver";
    const APP_AUTHOR: &'static str = "cuervolu";
    const APP_QUALIFIER: &'static str = "dev";
    const APP_ENV: &'static str = "ARCHIVER";
    
    
    /// Returns the standard, platform-specific path for the configuration file.
    pub fn config_path() -> Result<PathBuf> {
        ProjectDirs::from(Self::APP_QUALIFIER, Self::APP_AUTHOR, Self::APP_NAME)
            .map(|proj_dirs| proj_dirs.config_dir().join(Self::CONFIG_FILE_NAME))
            .ok_or(Error::HomeDirNotFound) 
    }

    /// Returns the standard, platform-specific path for the log directory.
    pub fn log_path() -> Result<PathBuf> {
        ProjectDirs::from(Self::APP_QUALIFIER, Self::APP_AUTHOR, Self::APP_NAME)
            .and_then(|proj_dirs| proj_dirs.state_dir().map(|p| p.to_path_buf()))
            .ok_or(Error::HomeDirNotFound)
    }

    /// Loads settings from the config file, applying defaults for missing values.
    pub fn new() -> Result<Self> {
        let config_path = Self::config_path()?;
        let config_file_path_str = config_path.to_str().unwrap_or_default();

        let user_dirs = UserDirs::new().ok_or(Error::HomeDirNotFound)?;
        let home_dir = user_dirs.home_dir();
        let projects_default = user_dirs.document_dir().unwrap_or(home_dir).join("projects");
        let archive_default = home_dir.join(".archive");

        let config_builder = config::Config::builder()
            .add_source(config::File::with_name(config_file_path_str).required(false))
            .add_source(config::Environment::with_prefix(Self::APP_ENV).separator("__"))
            .set_default("projects_dir", projects_default.to_str())?
            .set_default("archive_dir", archive_default.to_str())?
            .set_default("inactivity_days", 30)?
            .build()?;

        config_builder.try_deserialize().map_err(Error::Config)
    }
}