use std::fs;
use anyhow::{Context, Result};
use archiver_core::{ActionPlan, Archiver, Settings};
use clap::{ArgAction, ColorChoice, Parser, Subcommand};
use console::style;
use dialoguer::{Confirm, Input};
use tracing::level_filters::LevelFilter;

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

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initializes the configuration file interactively.
    Init,
    /// Updates the configuration interactively.
    Config,
    /// Archive inactive projects based on configuration.
    #[command(visible_alias = "a")]
    Archive {
        /// Perform a dry run without moving any files.
        #[arg(long)]
        dry_run: bool,
    },
    /// Restore an archived project.
    #[command(visible_alias = "r")]
    Restore {
        /// The name of the project to restore.
        #[arg(required = true)]
        name: String,
    },
    /// List all currently archived projects.
    #[command(visible_alias = "l")]
    List,
    /// Show the configuration paths being used.
    Paths,

    /// Add or remove a project from the exclusion list.
    #[command(visible_alias = "e")]
    Exclude {
        /// The name of the project to add or remove.
        project_name: String,

        /// Remove the project from the exclusion list.
        #[arg(long, short)]
        remove: bool,
    },
}


fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose, cli.color);

    match &cli.command {
        Commands::Init => return handle_init(),
        Commands::Config => return handle_config(),
        Commands::Exclude { project_name, remove } => return handle_exclude(project_name, *remove),
        _ => {}
    }
    
    let settings =
        Settings::new().context("Failed to load settings. Try running 'archiver init'")?;
    let archiver = Archiver::new(settings);

    match cli.command {
        Commands::Archive { dry_run } => handle_archive(&archiver, dry_run)?,
        Commands::Restore { name } => handle_restore(&archiver, &name)?,
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


fn init_tracing(verbosity: u8, color: ColorChoice) {
    let level = match verbosity {
        0 => LevelFilter::INFO,
        1 => LevelFilter::DEBUG,
        _ => LevelFilter::TRACE,
    };

    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_ansi(color != ColorChoice::Never) // Enable/disable color
        .init();
}

fn handle_archive(archiver: &Archiver, dry_run: bool) -> Result<()> {
    let plan = archiver
        .run_archive_process(dry_run)
        .context("The archiving process failed")?;

    let inactive_projects: Vec<_> = plan.into_iter().filter(|p| *p != ActionPlan::Nothing).collect();

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

fn handle_restore(archiver: &Archiver, name: &str) -> Result<()> {
    archiver
        .restore_project(name)
        .with_context(|| format!("Failed to restore project '{}'", name))?;
    println!("Project '{}' restored successfully.", name);
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
        .default(
            existing
                .map_or_else(
                    || format!("{}/projects", home_dir),
                    |s| s.projects_dir.to_string_lossy().to_string(),
                ),
        )
        .interact_text()?;

    let archive_dir: String = Input::with_theme(&theme)
        .with_prompt("Enter the path for the archive directory")
        .default(
            existing
                .map_or_else(
                    || format!("{}/.archive", home_dir),
                    |s| s.archive_dir.to_string_lossy().to_string(),
                ),
        )
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
            println!("Project '{}' has been removed from the exclusion list.", style(project_name).yellow());
        } else {
            println!("Project '{}' was not on the exclusion list. No changes made.", style(project_name).yellow());
            return Ok(());
        }
    } else {
        if settings.exclude.iter().any(|p| p == project_name) {
            println!("Project '{}' is already on the exclusion list.", style(project_name).yellow());
            return Ok(());
        }
        settings.exclude.push(project_name.to_string());
        println!("Project '{}' has been added to the exclusion list.", style(project_name).yellow());
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

