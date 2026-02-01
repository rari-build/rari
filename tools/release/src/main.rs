mod app;
mod files;
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
use package::ReleasedPackage;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{
    env,
    io::{self, Write},
    time::Duration,
};

#[derive(Parser, Debug)]
#[command(name = "release")]
#[command(about = "rari Release Manager", long_about = None)]
struct Args {
    #[arg(long, value_delimiter = ',')]
    only: Option<Vec<String>>,

    #[arg(long)]
    dry_run: bool,

    #[arg(long)]
    non_interactive: bool,

    #[arg(long)]
    no_push: bool,
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
        return run_non_interactive(only, args.dry_run, args.no_push, env_version, env_type).await;
    }

    if !args.dry_run {
        use colored::Colorize;
        println!("{}", "rari Release Script".cyan().bold());
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
    no_push: bool,
    env_version: Option<String>,
    env_type: Option<String>,
) -> Result<()> {
    use crate::package::{Package, PackageGroup, ReleaseType, ReleaseUnit, ReleasedPackage};
    use colored::Colorize;

    println!("{}", "rari Release Script".cyan().bold());
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

    let binary_packages = vec![
        Package::load("rari-darwin-arm64", "packages/rari-darwin-arm64", false).await?,
        Package::load("rari-darwin-x64", "packages/rari-darwin-x64", false).await?,
        Package::load("rari-linux-arm64", "packages/rari-linux-arm64", false).await?,
        Package::load("rari-linux-x64", "packages/rari-linux-x64", false).await?,
        Package::load("rari-win32-x64", "packages/rari-win32-x64", false).await?,
    ];

    let binary_group = PackageGroup::new("rari-binaries".to_string(), binary_packages).await?;

    let mut release_units = vec![
        ReleaseUnit::Single(Package::load("rari", "packages/rari", true).await?),
        ReleaseUnit::Single(
            Package::load("create-rari-app", "packages/create-rari-app", true).await?,
        ),
        ReleaseUnit::Group(binary_group),
    ];

    if let Some(only_list) = &only {
        release_units.retain(|unit| only_list.contains(&unit.name().to_string()));
        if release_units.is_empty() {
            anyhow::bail!("No matching packages for selection: {}", only_list.join(", "));
        }
    }

    let mut released_packages: Vec<ReleasedPackage> = Vec::new();

    for unit in release_units {
        let unit_name = unit.name();
        println!("{} {}", "📦 Releasing".bold(), unit_name.cyan().bold());

        let new_version = if let Some(ref version) = env_version {
            let v = semver::Version::parse(version)
                .map_err(|_| anyhow::anyhow!("Invalid RELEASE_VERSION: {}", version))?;
            let current = semver::Version::parse(unit.current_version())?;
            if v <= current {
                anyhow::bail!(
                    "RELEASE_VERSION ({}) must be greater than current version {}",
                    version,
                    unit.current_version()
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
                .to_version(unit.current_version())
                .ok_or_else(|| anyhow::anyhow!("Failed to calculate version"))?
        } else {
            anyhow::bail!(
                "Non-interactive mode requires RELEASE_VERSION or RELEASE_TYPE environment variable"
            );
        };

        println!("  {} {} → {}", "Version:".bold(), unit.current_version(), new_version.green());

        let first_path = unit.paths()[0];
        let commits = crate::git::get_commits_since_tag(unit_name, first_path).await?;
        if !commits.is_empty() {
            println!("  {} Commits since last release:", "ℹ".blue().bold());
            for commit in commits.iter().take(5) {
                println!("    {}", commit);
            }
        }

        let packages = unit.packages();
        if packages.len() > 1 {
            println!("  {} Packages in group:", "ℹ".blue().bold());
            for pkg in &packages {
                println!("    • {}", pkg.name);
            }
        }
        println!();

        if dry_run {
            println!("  {} Would build package...", "[DRY RUN]".yellow());
        } else if unit.needs_build() {
            println!("  {} Building package...", "→".cyan());
            for pkg in &packages {
                if pkg.needs_build {
                    crate::npm::build_package(&pkg.path).await?;
                }
            }
            println!("  {} Built package", "✓".green());
        }

        if dry_run {
            println!("  {} Would update version to {}...", "[DRY RUN]".yellow(), new_version);
        } else {
            println!("  {} Updating version...", "→".cyan());
            unit.update_version(&new_version).await?;
            println!("  {} Updated version", "✓".green());
        }

        if dry_run {
            println!("  {} Would generate changelog...", "[DRY RUN]".yellow());
        } else if unit_name != "rari-binaries" {
            println!("  {} Generating changelog...", "→".cyan());
            let project_root = std::path::PathBuf::from(".");
            let tag = format!("v{}", new_version);
            crate::npm::generate_changelog(&tag, &project_root).await?;
            println!("  {} Generated changelog", "✓".green());
        } else {
            println!("  {} Skipping changelog for binaries", "ℹ".blue());
        }

        let message = format!("release: {}@{}", unit_name, new_version);
        let tag = if unit_name == "rari-binaries" {
            format!("v{}", new_version)
        } else {
            format!("{}@{}", unit_name, new_version)
        };
        if dry_run {
            println!("  {} Would commit: {}", "[DRY RUN]".yellow(), message);
            println!("  {} Would create tag: {}", "[DRY RUN]".yellow(), tag);
        } else {
            println!("  {} Committing changes...", "→".cyan());
            let paths = unit.paths();
            if paths.len() > 1 {
                let path_refs: Vec<&std::path::Path> = paths.iter().map(|p| p.as_path()).collect();
                crate::git::add_and_commit_multiple(&message, &path_refs).await?;
            } else {
                crate::git::add_and_commit(&message, paths[0]).await?;
            }
            crate::git::create_tag(&tag).await?;
            println!("  {} Committed and tagged", "✓".green());
        }

        for pkg in &packages {
            let needs_generated_files = matches!(pkg.name.as_str(), "rari" | "create-rari-app");

            if needs_generated_files {
                if dry_run {
                    println!(
                        "  {} Would generate README and LICENSE for {}...",
                        "[DRY RUN]".yellow(),
                        pkg.name
                    );
                } else {
                    println!("  {} Generating README and LICENSE for {}...", "→".cyan(), pkg.name);
                    crate::files::generate_package_files(&pkg.name, &pkg.path).await?;
                    println!("  {} Generated README and LICENSE", "✓".green());
                }
            }

            let is_prerelease =
                semver::Version::parse(&new_version).map(|v| !v.pre.is_empty()).unwrap_or(false);
            let npm_tag = if is_prerelease { "next" } else { "latest" };

            if dry_run {
                println!(
                    "  {} Would publish {}@{} with tag '{}'",
                    "[DRY RUN]".yellow(),
                    pkg.name,
                    new_version,
                    npm_tag
                );
            } else {
                println!("  {} Publishing {} to npm...", "→".cyan(), pkg.name);
                let otp = std::env::var("NPM_OTP").ok();
                let publish_result =
                    crate::npm::publish_package(&pkg.path, is_prerelease, otp.as_deref()).await;

                if publish_result.is_ok() {
                    println!("  {} Published {}@{}", "✓".green(), pkg.name, new_version);
                }

                if needs_generated_files {
                    println!("  {} Cleaning up generated files for {}...", "→".cyan(), pkg.name);
                    let cleanup_result = crate::files::cleanup_package_files(&pkg.path).await;

                    if let Err(e) = publish_result {
                        if let Err(cleanup_err) = cleanup_result {
                            println!("  {} Cleanup also failed: {}", "⚠".yellow(), cleanup_err);
                        }
                        return Err(e);
                    }

                    if let Err(cleanup_err) = cleanup_result {
                        println!("  {} Cleanup failed: {}", "⚠".yellow(), cleanup_err);
                        return Err(cleanup_err);
                    }

                    println!("  {} Cleaned up generated files", "✓".green());
                } else {
                    publish_result?;
                }
            }
        }

        println!();
        println!("  {} Released {}@{}", "✅".green(), unit_name, new_version);
        println!();

        released_packages.push(ReleasedPackage {
            name: unit_name.to_string(),
            version: new_version.clone(),
            tag: tag.clone(),
            commits: commits.clone(),
        });
    }

    if no_push {
        println!("{}", "⚠️  Skipping git push (--no-push flag set)".yellow());
        println!("{}", "   Run 'git push && git push --tags' manually when ready".yellow());
    } else if dry_run {
        println!("{} Would push commits and tags to remote", "[DRY RUN]".yellow());
    } else {
        println!("{} Pushing commits and tags to remote...", "→".cyan());
        crate::git::push_changes().await?;
        println!("{} Pushed to remote", "✓".green());
    }
    println!();

    println!("{}", "✨ All packages released successfully!".green().bold());

    if !dry_run && !released_packages.is_empty() {
        println!();
        println!("{}", "📝 Create GitHub Releases?".cyan().bold());

        match crate::git::get_repo_info().await {
            Ok((owner, repo)) => {
                for pkg in &released_packages {
                    let release_url = create_github_release_url(&owner, &repo, pkg);
                    println!();
                    println!("  {} {}@{}", "→".cyan(), pkg.name, pkg.version);
                    println!("    {}", release_url.dimmed());
                }

                println!();
                print!("{} Open GitHub release pages in browser? [y/N]: ", "?".cyan());
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;

                if input.trim().eq_ignore_ascii_case("y")
                    || input.trim().eq_ignore_ascii_case("yes")
                {
                    for pkg in &released_packages {
                        let release_url = create_github_release_url(&owner, &repo, pkg);
                        println!("  {} Opening {}@{}...", "→".cyan(), pkg.name, pkg.version);
                        if let Err(e) = open::that(&release_url) {
                            println!("  {} Failed to open browser: {}", "✗".red(), e);
                            println!("  {} URL: {}", "ℹ".blue(), release_url);
                        }
                    }
                    println!(
                        "  {} Opened {} release page(s)",
                        "✓".green(),
                        released_packages.len()
                    );
                } else {
                    println!("  {} Skipped", "ℹ".blue());
                }
            }
            Err(e) => {
                println!("  {} Could not determine GitHub repository: {}", "⚠".yellow(), e);
            }
        }
    }

    Ok(())
}

fn create_github_release_url(owner: &str, repo: &str, pkg: &ReleasedPackage) -> String {
    let title_text = format!("{}@{}", pkg.name, pkg.version);
    let title = urlencoding::encode(&title_text);
    let tag = urlencoding::encode(&pkg.tag);

    let mut body = "## What's Changed\n\n".to_string();

    if !pkg.commits.is_empty() {
        for commit in &pkg.commits {
            body.push_str(&format!("- {}\n", commit));
        }
    } else {
        body.push_str("See CHANGELOG.md for details.\n");
    }

    body.push_str(&format!(
        "\n**Full Changelog**: https://github.com/{}/{}/compare/{}...{}",
        owner, repo, pkg.tag, pkg.tag
    ));

    let body_encoded = urlencoding::encode(&body);

    format!(
        "https://github.com/{}/{}/releases/new?tag={}&title={}&body={}",
        owner, repo, tag, title, body_encoded
    )
}
