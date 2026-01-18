mod app;
mod git;
mod npm;
mod package;
mod ui;

use anyhow::Result;
use app::App;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{
    env,
    io::{self, Write},
    time::Duration,
};

#[derive(Parser, Debug)]
#[command(name = "release")]
#[command(about = "Rari Release Manager", long_about = None)]
struct Args {
    #[arg(long, value_delimiter = ',')]
    only: Option<Vec<String>>,

    #[arg(long)]
    dry_run: bool,

    #[arg(long)]
    non_interactive: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let only = if args.only.is_some() {
        args.only
    } else if let Ok(packages_env) = env::var("PACKAGES") {
        Some(
            packages_env
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
        )
    } else {
        None
    };

    let env_version = env::var("RELEASE_VERSION").ok();
    let env_type = env::var("RELEASE_TYPE").ok();

    if args.non_interactive || env_version.is_some() || env_type.is_some() {
        return run_non_interactive(only, args.dry_run, env_version, env_type).await;
    }

    if !args.dry_run {
        use colored::Colorize;
        println!("{}", "Rari Release Script".cyan().bold());
        println!();
        print!("{} Checking npm login status... ", "→".cyan());
        io::stdout().flush()?;

        match npm::check_npm_login().await {
            Ok(_) => {
                println!("{}", "✓".green());
                println!();
            }
            Err(e) => {
                println!("{}", "✗".red());
                println!();
                eprintln!("{} {}", "Error:".red().bold(), e);
                eprintln!();
                eprintln!(
                    "{}",
                    "Please run 'npm login' before proceeding with the release.".yellow()
                );
                std::process::exit(1);
            }
        }
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(only, args.dry_run).await?;

    let result = run_app(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| app.render(f))?;

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
            && app.handle_key(key.code).await?
        {
            break;
        }

        app.update().await?;
    }

    Ok(())
}

async fn run_non_interactive(
    only: Option<Vec<String>>,
    dry_run: bool,
    env_version: Option<String>,
    env_type: Option<String>,
) -> Result<()> {
    use crate::package::{Package, ReleaseType};
    use colored::Colorize;

    println!("{}", "Rari Release Script".cyan().bold());
    if dry_run {
        println!("{}", "[DRY RUN MODE]".yellow().bold());
    }
    println!();

    if !dry_run {
        println!("{} Checking npm login status...", "→".cyan());
        crate::npm::check_npm_login().await?;
        println!("{} npm login verified", "✓".green());
        println!();
    }

    let mut packages = vec![
        Package::load("rari", "packages/rari", true).await?,
        Package::load("create-rari-app", "packages/create-rari-app", true).await?,
    ];

    if let Some(only_list) = &only {
        packages.retain(|p| only_list.contains(&p.name));
        if packages.is_empty() {
            anyhow::bail!("No matching packages for selection: {}", only_list.join(", "));
        }
    }

    for package in packages {
        println!("{} {}", "📦 Releasing".bold(), package.name.cyan().bold());

        let new_version = if let Some(ref version) = env_version {
            let v = semver::Version::parse(version)
                .map_err(|_| anyhow::anyhow!("Invalid RELEASE_VERSION: {}", version))?;
            let current = semver::Version::parse(&package.current_version)?;
            if v <= current {
                anyhow::bail!(
                    "RELEASE_VERSION ({}) must be greater than current version {}",
                    version,
                    package.current_version
                );
            }
            version.clone()
        } else if let Some(ref type_str) = env_type {
            let release_type = match type_str.as_str() {
                "patch" => ReleaseType::Patch,
                "minor" => ReleaseType::Minor,
                "major" => ReleaseType::Major,
                "prepatch" => ReleaseType::Prepatch,
                "preminor" => ReleaseType::Preminor,
                "premajor" => ReleaseType::Premajor,
                "prerelease" => ReleaseType::Prerelease,
                _ => anyhow::bail!("Invalid RELEASE_TYPE: {}", type_str),
            };
            release_type
                .to_version(&package.current_version)
                .ok_or_else(|| anyhow::anyhow!("Failed to calculate version"))?
        } else {
            anyhow::bail!(
                "Non-interactive mode requires RELEASE_VERSION or RELEASE_TYPE environment variable"
            );
        };

        println!("  {} {} → {}", "Version:".bold(), package.current_version, new_version.green());

        let commits = crate::git::get_commits_since_tag(&package.name, &package.path).await?;
        if !commits.is_empty() {
            println!("  {} Commits since last release:", "ℹ".blue().bold());
            for commit in commits.iter().take(5) {
                println!("    {}", commit);
            }
        }
        println!();

        if dry_run {
            println!("  {} Would build package...", "[DRY RUN]".yellow());
        } else {
            println!("  {} Building package...", "→".cyan());
            if package.needs_build {
                crate::npm::build_package(&package.path).await?;
            }
            println!("  {} Built package", "✓".green());
        }

        if dry_run {
            println!("  {} Would update version to {}...", "[DRY RUN]".yellow(), new_version);
        } else {
            println!("  {} Updating version...", "→".cyan());
            package.update_version(&new_version).await?;
            println!("  {} Updated version", "✓".green());
        }

        if dry_run {
            println!("  {} Would generate changelog...", "[DRY RUN]".yellow());
        } else {
            println!("  {} Generating changelog...", "→".cyan());
            let project_root = std::path::PathBuf::from(".");
            let tag = format!("v{}", new_version);
            crate::npm::generate_changelog(&tag, &project_root).await?;

            let source = project_root.join("CHANGELOG.md");
            let target = package.path.join("CHANGELOG.md");
            tokio::fs::copy(&source, &target).await?;
            tokio::fs::remove_file(&source).await?;
            println!("  {} Generated changelog", "✓".green());
        }

        let message = format!("release: {}@{}", package.name, new_version);
        let tag = format!("{}@{}", package.name, new_version);
        if dry_run {
            println!("  {} Would commit: {}", "[DRY RUN]".yellow(), message);
            println!("  {} Would create tag: {}", "[DRY RUN]".yellow(), tag);
        } else {
            println!("  {} Committing changes...", "→".cyan());
            crate::git::add_and_commit(&message, &package.path).await?;
            crate::git::create_tag(&tag).await?;
            println!("  {} Committed and tagged", "✓".green());
        }

        let is_prerelease =
            semver::Version::parse(&new_version).map(|v| !v.pre.is_empty()).unwrap_or(false);
        let npm_tag = if is_prerelease { "next" } else { "latest" };

        if dry_run {
            println!(
                "  {} Would publish {}@{} with tag '{}'",
                "[DRY RUN]".yellow(),
                package.name,
                new_version,
                npm_tag
            );
        } else {
            println!("  {} Publishing to npm...", "→".cyan());
            crate::npm::publish_package(&package.path, is_prerelease).await?;
            println!("  {} Published {}@{}", "✓".green(), package.name, new_version);
        }

        println!();
        println!("  {} Released {}@{}", "✅".green(), package.name, new_version);
        println!();
    }

    println!("{}", "✨ All packages released successfully!".green().bold());

    Ok(())
}
