mod app;
mod changelog;
mod git;
mod package;
mod ui;
mod versions;

use std::{
    env,
    io::{self, Write},
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::Result;
use app::App;
use clap::Parser;
use colored::Colorize;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::package::{ReleaseType, ReleaseUnit, ReleasedPackage, release_tag};

#[derive(Parser, Debug)]
#[expect(clippy::struct_excessive_bools, reason = "CLI flags map directly to clap bool args")]
#[command(name = "release")]
#[command(about = "rari Release Manager", long_about = None)]
struct Args {
    #[arg(
        long,
        value_delimiter = ',',
        help = "Comma-separated release units to include, e.g. rari,@rari/lint (or set PACKAGES)"
    )]
    only: Option<Vec<String>>,

    #[arg(
        long,
        help = "Show what would happen without changing files, committing, tagging, or pushing"
    )]
    dry_run: bool,

    #[arg(long, help = "Run without the TUI (implied by RELEASE_VERSION or RELEASE_TYPE)")]
    non_interactive: bool,

    #[arg(long, help = "In non-interactive mode, commit and tag locally but don't push")]
    no_push: bool,

    #[arg(
        long,
        help = "Manual release notes to use instead of .github/release-notes/ (or set RELEASE_NOTES_FILE)"
    )]
    notes_file: Option<PathBuf>,

    #[arg(
        long,
        help = "In non-interactive mode, release even if npm can't be reached (versions fall back to git tags and local package.json)"
    )]
    allow_offline: bool,
}

#[expect(clippy::print_stdout)]
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
    let notes_file = args.notes_file.or_else(|| {
        env::var("RELEASE_NOTES_FILE").ok().filter(|s| !s.is_empty()).map(PathBuf::from)
    });

    if args.non_interactive || env_version.is_some() || env_type.is_some() {
        return run_non_interactive(
            only,
            args.dry_run,
            args.no_push,
            env_version,
            env_type,
            notes_file,
            args.allow_offline,
        )
        .await;
    }

    if !args.dry_run {
        use colored::Colorize;
        println!("{}", "rari Release Script".cyan().bold());
        println!();
    }

    println!("Checking published versions...");
    let mut app = App::new(only, args.dry_run, notes_file).await?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

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

#[expect(clippy::too_many_lines, clippy::print_stdout)]
async fn run_non_interactive(
    only: Option<Vec<String>>,
    dry_run: bool,
    no_push: bool,
    env_version: Option<String>,
    env_type: Option<String>,
    notes_file: Option<PathBuf>,
    allow_offline: bool,
) -> Result<()> {
    println!("{}", "rari Release Script".cyan().bold());
    if dry_run {
        println!("{}", "[DRY RUN MODE]".yellow().bold());
    }
    println!();

    println!("Checking published versions...");
    let release_units = package::load_release_units(only.as_deref()).await?;

    let offline: Vec<&str> = release_units
        .iter()
        .filter(|unit| !unit.sources().npm_reachable())
        .map(ReleaseUnit::name)
        .collect();
    if !offline.is_empty() {
        if !allow_offline {
            anyhow::bail!(
                "Could not reach npm for: {}. Refusing to pick versions from git tags and local \
                 package.json only; re-run with --allow-offline to override.",
                offline.join(", ")
            );
        }
        println!(
            "{} npm unreachable for {}; using git tags and local versions (--allow-offline)",
            "⚠".yellow().bold(),
            offline.join(", ")
        );
    }

    let mut planned: Vec<(ReleaseUnit, String)> = Vec::with_capacity(release_units.len());
    for unit in release_units {
        let new_version = if let Some(ref version) = env_version {
            let v = semver::Version::parse(version)
                .map_err(|_| anyhow::anyhow!("Invalid RELEASE_VERSION: {version}"))?;
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
                _ => anyhow::bail!("Invalid RELEASE_TYPE: {type_str}"),
            };
            unit.next_version(release_type).ok_or_else(|| {
                anyhow::anyhow!(
                    "Could not calculate a {type_str} version greater than {}",
                    unit.base_version(release_type)
                )
            })?
        } else {
            anyhow::bail!(
                "Non-interactive mode requires RELEASE_VERSION or RELEASE_TYPE environment variable"
            );
        };
        planned.push((unit, new_version));
    }

    let mut released_packages: Vec<ReleasedPackage> = Vec::new();

    for (unit, new_version) in planned {
        let unit_name = unit.name();
        println!("{} {}", "📦 Releasing".bold(), unit_name.cyan().bold());

        println!("  {} {} → {}", "Version:".bold(), unit.current_version(), new_version.green());
        let sources = unit.sources();
        println!("  {} {}", "Sources:".bold(), sources.summary());
        if sources.is_out_of_sync() {
            println!(
                "  {} Sources disagree; bumping from the highest ({})",
                "⚠".yellow().bold(),
                unit.current_version()
            );
        }

        let first_path = unit.paths().first().map(|p| p.as_path()).unwrap_or(Path::new("."));
        let commits = git::get_commits_since_tag(unit_name, first_path).await?;
        let previous_tag = git::get_previous_tag(unit_name, None).await?;
        if !commits.is_empty() {
            println!("  {} Commits since last release:", "ℹ".blue().bold());
            for commit in commits.iter().take(5) {
                println!("    {commit}");
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
            println!("  {} Would update version to {}...", "[DRY RUN]".yellow(), new_version);
        } else {
            println!("  {} Updating version...", "→".cyan());
            unit.update_version(&new_version).await?;
            println!("  {} Updated version", "✓".green());
        }

        let generates_changelog =
            matches!(unit_name, "rari" | "create-rari-app" | "@rari/use-cache" | "@rari/lint");
        let tag = release_tag(unit_name, &new_version);
        let notes_override = notes_file.as_deref();
        let manual_notes = changelog::load_manual_notes(&tag, &new_version, notes_override).await?;

        if let Some((path, _)) = &manual_notes {
            println!("  {} Manual notes: {}", "✓".green(), path.display());
        } else {
            println!("  {} No manual release notes (cliff-only)", "ℹ".blue());
        }

        if dry_run {
            if generates_changelog {
                println!("  {} Would generate changelog...", "[DRY RUN]".yellow());
                if manual_notes.is_some() {
                    println!(
                        "  {} Would inject manual notes into CHANGELOG.md",
                        "[DRY RUN]".yellow()
                    );
                }
            } else {
                println!(
                    "  {} Would skip changelog generation (binary packages)",
                    "[DRY RUN]".yellow()
                );
            }
        } else if generates_changelog {
            println!("  {} Generating changelog...", "→".cyan());
            let package_path = unit.paths()[0];
            changelog::generate(&tag, &new_version, unit_name, package_path).await?;
            if let Some((_, body)) = &manual_notes {
                if changelog::inject_manual_notes(package_path, &tag, &new_version, body).await? {
                    println!("  {} Injected manual notes into CHANGELOG.md", "✓".green());
                } else {
                    let expected =
                        changelog::expected_changelog_headings(&tag, &new_version).join("` or `");
                    println!(
                        "  {} Could not inject manual notes: missing heading `{expected}` in CHANGELOG.md",
                        "⚠".yellow()
                    );
                }
            }
            println!("  {} Generated changelog", "✓".green());
        } else {
            println!("  {} Skipping changelog generation", "ℹ".blue());
        }

        let message = format!("release: {unit_name}@{new_version}");
        if dry_run {
            println!("  {} Would commit: {}", "[DRY RUN]".yellow(), message);
            println!("  {} Would create tag: {}", "[DRY RUN]".yellow(), tag);
        } else {
            println!("  {} Committing changes...", "→".cyan());
            let paths = unit.paths();
            if paths.is_empty() {
            } else if paths.len() > 1 {
                let path_refs: Vec<&Path> = paths.iter().map(|p| p.as_path()).collect();
                git::add_and_commit_multiple(&message, &path_refs).await?;
            } else {
                git::add_and_commit(&message, paths[0]).await?;
            }

            let generates_changelog =
                matches!(unit_name, "rari" | "create-rari-app" | "@rari/use-cache" | "@rari/lint");
            let mut files_to_add = Vec::new();
            if generates_changelog && let Some(first_path) = unit.paths().first() {
                let changelog_path = first_path.join("CHANGELOG.md");
                if changelog_path.exists() {
                    files_to_add.push(changelog_path);
                }
            }
            let lockfile_path = PathBuf::from("pnpm-lock.yaml");
            if lockfile_path.exists() {
                files_to_add.push(lockfile_path);
            }

            if !files_to_add.is_empty() {
                for file in &files_to_add {
                    git::add_file(file).await?;
                }
                git::amend_commit().await?;
            }

            git::create_tag(&tag).await?;
            println!("  {} Committed and tagged", "✓".green());
        }

        for pkg in &packages {
            let is_prerelease =
                semver::Version::parse(&new_version).map(|v| !v.pre.is_empty()).unwrap_or(false);
            let npm_tag = if is_prerelease { "next" } else { "latest" };

            if dry_run {
                println!(
                    "  {} Would publish {}@{} with tag '{}' via GitHub Actions after push",
                    "[DRY RUN]".yellow(),
                    pkg.name,
                    new_version,
                    npm_tag
                );
            } else {
                println!(
                    "  {} {}@{} will be published via GitHub Actions (tag: '{}')",
                    "ℹ".blue(),
                    pkg.name,
                    new_version,
                    npm_tag
                );
            }
        }

        println!();
        println!("  {} Released {}@{}", "✅".green(), unit_name, new_version);
        println!();

        released_packages.push(ReleasedPackage {
            name: unit_name.to_string(),
            version: new_version.clone(),
            tag: tag.clone(),
            release_notes: changelog::generate_release_notes(
                &tag,
                unit_name,
                previous_tag.as_deref(),
                &new_version,
                notes_file.as_deref(),
            )
            .await
            .unwrap_or_else(|_| changelog::CHANGELOG_FALLBACK_NOTES.to_string()),
            previous_tag,
        });
    }

    if no_push {
        println!("{}", "⚠️  Skipping git push (--no-push flag set)".yellow());
        println!("{}", "   Run 'git push && git push --tags' manually when ready".yellow());
    } else if dry_run {
        println!("{} Would push commits and tags to remote", "[DRY RUN]".yellow());
    } else {
        println!("{} Pushing commits and tags to remote...", "→".cyan());
        git::push_changes().await?;
        println!("{} Pushed to remote", "✓".green());
    }
    println!();

    println!("{}", "✨ All packages released successfully!".green().bold());

    if !dry_run && !released_packages.is_empty() {
        println!();
        println!("{}", "📝 Create GitHub Releases?".cyan().bold());

        match git::get_repo_info().await {
            Ok((owner, repo)) => {
                let release_urls: Vec<_> = released_packages
                    .iter()
                    .map(|pkg| (pkg, pkg.create_github_release_url(&owner, &repo)))
                    .collect();

                for (pkg, release_url) in &release_urls {
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
                    for (pkg, release_url) in &release_urls {
                        println!("  {} Opening {}@{}...", "→".cyan(), pkg.name, pkg.version);
                        if let Err(e) = open::that(release_url) {
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
