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
    use crate::package::{Package, ReleaseType, ReleasedPackage};
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

    let mut released_packages: Vec<ReleasedPackage> = Vec::new();

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
            println!("  {} Updating lockfile...", "→".cyan());
            let project_root = std::path::PathBuf::from(".");
            crate::npm::install_dependencies(&project_root).await?;
            println!("  {} Updated lockfile", "✓".green());
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

        if dry_run {
            println!("  {} Would generate README and LICENSE...", "[DRY RUN]".yellow());
        } else {
            println!("  {} Generating README and LICENSE...", "→".cyan());
            crate::files::generate_package_files(&package.name, &package.path).await?;
            println!("  {} Generated README and LICENSE", "✓".green());
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
            let otp = std::env::var("NPM_OTP").ok();
            let publish_result =
                crate::npm::publish_package(&package.path, is_prerelease, otp.as_deref()).await;

            if publish_result.is_ok() {
                println!("  {} Published {}@{}", "✓".green(), package.name, new_version);
            }

            println!("  {} Cleaning up generated files...", "→".cyan());
            crate::files::cleanup_package_files(&package.path).await?;
            println!("  {} Cleaned up generated files", "✓".green());

            publish_result?;
        }

        println!();
        println!("  {} Released {}@{}", "✅".green(), package.name, new_version);
        println!();

        released_packages.push(ReleasedPackage {
            name: package.name.clone(),
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
