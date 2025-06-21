use anyhow::{Context, Result, anyhow};
use archiver_core::{ActionPlan, Archiver, Settings};
use clap::{ArgAction, ColorChoice, Parser, Subcommand};
use console::style;
use dialoguer::{Confirm, Input};
use std::fs;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{fmt, Layer};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// A CLI tool to automatically archive inactive git repositories.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Set the verbosity level. Use -v for debug, -vv for trace.
    #[arg(short, long, action = ArgAction::Count, global = true)]
    verbose: u8,

    /// Control when to use color output.
    #[arg(long, value_name = "WHEN", global = true, default_value_t = ColorChoice::Auto)]
    color: ColorChoice,

    /// If no subcommand is provided, the TUI will be launched.
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initializes the configuration file interactively.
    Init,
    /// Updates the configuration interactively.
    Config,
    /// Scans for inactive projects and archives them.
    #[command(visible_alias = "a")]
    Run {
        /// Perform a dry run without moving any files.
        #[arg(long)]
        dry_run: bool,
    },
    /// Restore one or all archived projects.
    #[command(visible_alias = "r")]
    Restore {
        /// The name of the project to restore.
        name: Option<String>,
        /// Restore all projects from the archive.
        #[arg(long, short, conflicts_with = "name")]
        all: bool,
    },
    // --- NUEVO COMANDO ---
    /// Delete one or all projects permanently from the archive.
    #[command(visible_alias = "d")]
    Delete {
        /// The name of the project to delete.
        name: Option<String>,
        /// Delete ALL projects from the archive. This is irreversible.
        #[arg(long, short, conflicts_with = "name")]
        all: bool,
    },
    /// Add or remove a project from the exclusion list.
    #[command(visible_alias = "e")]
    Exclude {
        /// The name of the project to add or remove.
        project_name: String,
        /// Remove the project from the exclusion list.
        #[arg(long, short)]
        remove: bool,
    },
    /// List all currently archived projects.
    #[command(visible_alias = "l")]
    List,
    /// Show the configuration paths being used.
    Paths,
}

#[cfg(target_os = "linux")]
fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing().context("Failed to initialize logging")?;

    match cli.command {
        Some(command) => handle_command(command),
        None => {
            println!("TUI mode is not yet implemented. Use a subcommand like 'run' or 'list'.");
            println!("For help, run 'archive --help'.");
            Ok(())
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn main() -> Result<()> {
    println!("Error: This application is currently only supported on Linux.");
    std::process::exit(1);
}

fn handle_command(command: Commands) -> Result<()> {
    // Los comandos que no necesitan un `Archiver` se manejan primero.
    match command {
        Commands::Init => return handle_init(),
        Commands::Config => return handle_config(),
        Commands::Exclude {
            project_name,
            remove,
        } => return handle_exclude(&project_name, remove),
        _ => {}
    }

    let settings =
        Settings::new().context("Failed to load settings. Try running 'archive init'")?;
    let archiver = Archiver::new(settings);

    match command {
        Commands::Run { dry_run } => handle_run(&archiver, dry_run)?,
        Commands::Restore { name, all } => handle_restore(&archiver, name, all)?,
        Commands::Delete { name, all } => handle_delete(&archiver, name, all)?,
        Commands::List => handle_list(&archiver)?,
        Commands::Paths => handle_paths(archiver.settings())?,
        _ => unreachable!(),
    }
    Ok(())
}

fn handle_init() -> Result<()> {
    println!("{}", style("Welcome to Auto Archiver setup!").bold());
    let config_path = Settings::config_path()?;
    if config_path.exists() {
        let overwrite = Confirm::new()
            .with_prompt("A configuration file already exists. Do you want to overwrite it?")
            .default(false)
            .interact()?;
        if !overwrite {
            println!("Initialization cancelled.");
            return Ok(());
        }
    }
    let new_settings = interactive_config_update(None)?;
    save_settings(&new_settings)?;
    println!(
        "\n{}",
        style("Configuration saved successfully!").green().bold()
    );
    Ok(())
}

fn handle_config() -> Result<()> {
    println!(
        "{}",
        style("Updating Auto Archiver configuration...").bold()
    );
    let existing_settings = Settings::new().context("Failed to load existing settings.")?;
    let new_settings = interactive_config_update(Some(&existing_settings))?;
    save_settings(&new_settings)?;
    println!(
        "\n{}",
        style("Configuration updated successfully!").green().bold()
    );
    Ok(())
}

/// Initializes a dual logging system: to console and to a daily rolling file.
fn init_tracing() -> Result<()> {
    let log_dir = Settings::log_path()?;
    fs::create_dir_all(&log_dir)?; 
    
    let file_appender = tracing_appender::rolling::daily(log_dir, "archive.log");
    let (non_blocking_appender, _guard) = tracing_appender::non_blocking(file_appender);
    let file_layer = fmt::layer()
        .with_writer(non_blocking_appender)
        .with_ansi(false) 
        .with_filter(LevelFilter::DEBUG); 
    
    let console_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_filter(LevelFilter::INFO); 

    tracing_subscriber::registry()
        .with(file_layer)
        .with(console_layer)
        .init();

    Ok(())
}

fn handle_run(archiver: &Archiver, dry_run: bool) -> Result<()> {
    let plan = archiver
        .run_archive_process(dry_run)
        .context("The archiving process failed")?;

    let inactive_projects: Vec<_> = plan
        .into_iter()
        .filter(|p| *p != ActionPlan::Nothing)
        .collect();

    if inactive_projects.is_empty() {
        println!("No projects needed archiving.");
        return Ok(());
    }

    if dry_run {
        println!("{}", style("-- DRY RUN --").yellow().bold());
        println!(
            "The following {} project(s) would be archived:",
            inactive_projects.len()
        );
        for case in inactive_projects {
            if let ActionPlan::Archive { project_name, .. } = case {
                println!("- {}", style(project_name).cyan());
            }
        }
        println!("\nRun without --dry-run to perform these actions.");
    } else {
        println!(
            "Successfully archived {} project(s).",
            inactive_projects.len()
        );
    }

    Ok(())
}

fn handle_delete(archiver: &Archiver, name: Option<String>, all: bool) -> Result<()> {
    println!(
        "{}",
        style("Warning: This operation is permanent and cannot be undone.")
            .red()
            .bold()
    );
    if all {
        let records_to_delete = archiver.get_archive_records()?.len();
        if records_to_delete == 0 {
            println!("Archive is already empty.");
            return Ok(());
        }
        if !Confirm::new()
            .with_prompt(format!(
                "Are you sure you want to permanently delete ALL {} projects?",
                records_to_delete
            ))
            .default(false)
            .interact()?
        {
            println!("Operation cancelled.");
            return Ok(());
        }
        let confirmation: u64 = Input::new()
            .with_prompt(format!(
                "To confirm, please type the number of projects to delete ({})",
                records_to_delete
            ))
            .interact_text()?;
        if confirmation != records_to_delete as u64 {
            return Err(anyhow!("Incorrect number entered. Deletion cancelled."));
        }
        let count = archiver.delete_all()?;
        println!("Successfully deleted {} projects.", style(count).red());
    } else if let Some(project_name) = name {
        if !Confirm::new()
            .with_prompt(format!(
                "Are you sure you want to permanently delete '{}'?",
                project_name
            ))
            .default(false)
            .interact()?
        {
            println!("Operation cancelled.");
            return Ok(());
        }
        archiver.delete_project(&project_name)?;
        println!(
            "Project '{}' deleted successfully.",
            style(project_name).cyan()
        );
    } else {
        return Err(anyhow!(
            "You must specify a project name or use the --all flag."
        ));
    }
    Ok(())
}

fn handle_restore(archiver: &Archiver, name: Option<String>, all: bool) -> Result<()> {
    if all {
        if !Confirm::new()
            .with_prompt("Restore all projects from the archive?")
            .default(false)
            .interact()?
        {
            println!("Operation cancelled.");
            return Ok(());
        }
        let count = archiver.restore_all()?;
        println!("Successfully restored {} projects.", style(count).green());
    } else if let Some(project_name) = name {
        archiver.restore_project(&project_name)?;
        println!(
            "Project '{}' restored successfully.",
            style(project_name).cyan()
        );
    } else {
        return Err(anyhow!(
            "You must specify a project name or use the --all flag."
        ));
    }
    Ok(())
}

fn handle_list(archiver: &Archiver) -> Result<()> {
    let records = archiver
        .get_archive_records()
        .context("Failed to retrieve list of archived projects")?;
    if records.is_empty() {
        println!("No projects are currently archived.");
    } else {
        println!("{}", style("Archived projects:").bold());
        for record in records {
            println!(
                "- {:<30} (Archived on: {})",
                style(&record.name).cyan(),
                record.archived_at.date_naive()
            );
        }
    }
    Ok(())
}

fn handle_paths(settings: &Settings) -> Result<()> {
    println!("{}", style("Configuration paths:").bold());
    println!(
        "- Projects directory: {}",
        style(settings.projects_dir.display()).yellow()
    );
    println!(
        "- Archive directory:  {}",
        style(settings.archive_dir.display()).yellow()
    );
    println!(
        "- Config file:        {}",
        style(Settings::config_path()?.display()).yellow()
    );
    Ok(())
}

fn interactive_config_update(existing: Option<&Settings>) -> Result<Settings> {
    let theme = dialoguer::theme::ColorfulTheme::default();
    let home_dir = std::env::var("HOME").context("Could not find HOME directory")?;

    let projects_dir: String = Input::with_theme(&theme)
        .with_prompt("Enter the path to your projects directory")
        .default(existing.map_or_else(
            || format!("{}/projects", home_dir),
            |s| s.projects_dir.to_string_lossy().to_string(),
        ))
        .interact_text()?;

    let archive_dir: String = Input::with_theme(&theme)
        .with_prompt("Enter the path for the archive directory")
        .default(existing.map_or_else(
            || format!("{}/.archive", home_dir),
            |s| s.archive_dir.to_string_lossy().to_string(),
        ))
        .interact_text()?;

    let inactivity_days: u64 = Input::with_theme(&theme)
        .with_prompt("Archive projects after how many days of inactivity?")
        .default(existing.map_or(30, |s| s.inactivity_days))
        .interact_text()?;

    Ok(Settings {
        projects_dir: projects_dir.into(),
        archive_dir: archive_dir.into(),
        inactivity_days,
        cleanup_rules: existing.map_or_else(Vec::new, |s| s.cleanup_rules.clone()),
        enable_auto_delete: existing.map_or(false, |s| s.enable_auto_delete),
        days_before_delete: existing.map_or(365, |s| s.days_before_delete),
        exclude: existing.map_or_else(Vec::new, |s| s.exclude.clone()),
    })
}

fn handle_exclude(project_name: &str, remove: bool) -> Result<()> {
    let mut settings = Settings::new().unwrap_or_default();
    if remove {
        if let Some(pos) = settings.exclude.iter().position(|p| p == project_name) {
            settings.exclude.remove(pos);
            println!(
                "Project '{}' has been removed from the exclusion list.",
                style(project_name).yellow()
            );
        } else {
            println!(
                "Project '{}' was not on the exclusion list. No changes made.",
                style(project_name).yellow()
            );
            return Ok(());
        }
    } else {
        if settings.exclude.iter().any(|p| p == project_name) {
            println!(
                "Project '{}' is already on the exclusion list.",
                style(project_name).yellow()
            );
            return Ok(());
        }
        settings.exclude.push(project_name.to_string());
        println!(
            "Project '{}' has been added to the exclusion list.",
            style(project_name).yellow()
        );
    }
    save_settings(&settings).context("Failed to save updated settings")
}

/// Helper to serialize and save settings to the config file.
fn save_settings(settings: &Settings) -> Result<()> {
    let path = Settings::config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("Could not create config directory")?;
    }
    let toml_string =
        toml::to_string_pretty(settings).context("Could not serialize settings to TOML")?;
    fs::write(&path, toml_string)
        .with_context(|| format!("Could not write config to '{}'", path.display()))?;
    Ok(())
}
