use crate::{
    changelog, git,
    package::{Package, ReleaseType, ReleasedPackage},
    ui,
};
use anyhow::Result;
use crossterm::event::KeyCode;
use ratatui::Frame;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    PackageSelection,
    VersionSelection { package_idx: usize },
    CustomVersion { package_idx: usize, input: String },
    Publishing { package_idx: usize, version: String },
    PostPublish { has_more_packages: bool },
    PostRelease { released: Vec<ReleasedPackage>, step: PostReleaseStep },
    Complete,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PostReleaseStep {
    Pushing,
    PushComplete,
    PromptGitHub,
    OpeningGitHub,
    Done,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PublishStep {
    UpdatingVersion,
    GeneratingChangelog,
    Committing,
    Publishing,
    Done,
}

pub struct App {
    pub screen: Screen,
    pub release_units: Vec<crate::package::ReleaseUnit>,
    pub selected_package_idx: usize,
    pub selected_version_idx: usize,
    pub version_types: Vec<ReleaseType>,
    pub recent_commits: Vec<String>,
    pub previous_tag: Option<String>,
    pub publish_step: PublishStep,
    pub publish_progress: f64,
    pub status_messages: Vec<String>,
    pub released_packages: Vec<ReleasedPackage>,
    pub error_message: Option<String>,
    pub dry_run: bool,
    pub post_release_messages: Vec<String>,
}

impl App {
    pub async fn new(only: Option<Vec<String>>, dry_run: bool) -> Result<Self> {
        use crate::package::{PackageGroup, ReleaseUnit};

        let rari_pkg = Package::load("rari", "packages/rari").await?;
        let binary_version = rari_pkg.current_version.clone();
        let binary_group = PackageGroup::new_virtual("rari-binaries".to_string(), binary_version);

        let use_cache_pkg = Package::load("@rari/use-cache", "packages/use-cache").await?;
        let use_cache_binary_version = use_cache_pkg.current_version.clone();
        let use_cache_binary_group = PackageGroup::new_virtual(
            "@rari/use-cache-binaries".to_string(),
            use_cache_binary_version,
        );

        let mut release_units = vec![
            ReleaseUnit::Single(rari_pkg),
            ReleaseUnit::Single(
                Package::load("create-rari-app", "packages/create-rari-app").await?,
            ),
            ReleaseUnit::Virtual(binary_group),
            ReleaseUnit::Single(use_cache_pkg),
            ReleaseUnit::Virtual(use_cache_binary_group),
        ];

        if let Some(only_list) = only {
            release_units.retain(|unit| only_list.contains(&unit.name().to_string()));
            if release_units.is_empty() {
                anyhow::bail!("No matching packages for selection: {}", only_list.join(", "));
            }
        }

        Ok(Self {
            screen: Screen::PackageSelection,
            release_units,
            selected_package_idx: 0,
            selected_version_idx: 0,
            version_types: ReleaseType::all(),
            recent_commits: vec![],
            previous_tag: None,
            publish_step: PublishStep::UpdatingVersion,
            publish_progress: 0.0,
            status_messages: vec![],
            released_packages: vec![],
            error_message: None,
            dry_run,
            post_release_messages: vec![],
        })
    }

    pub async fn handle_key(&mut self, key: KeyCode) -> Result<bool> {
        match &self.screen.clone() {
            Screen::PackageSelection => match key {
                KeyCode::Up if self.selected_package_idx > 0 => {
                    self.selected_package_idx -= 1;
                }
                KeyCode::Down if self.selected_package_idx < self.release_units.len() - 1 => {
                    self.selected_package_idx += 1;
                }
                KeyCode::Enter => {
                    let package_idx = self.selected_package_idx;
                    let unit = &self.release_units[package_idx];
                    let paths = unit.paths();
                    let first_path = paths.first().map(|p| p.as_path()).unwrap_or(Path::new("."));
                    self.recent_commits =
                        git::get_commits_since_tag(unit.name(), first_path).await?;
                    self.previous_tag = git::get_previous_tag(unit.name(), None).await?;
                    self.screen = Screen::VersionSelection { package_idx };
                    self.selected_version_idx = 0;
                }
                KeyCode::Esc | KeyCode::Char('q') => return Ok(true),
                _ => {}
            },
            Screen::VersionSelection { package_idx } => match key {
                KeyCode::Up if self.selected_version_idx > 0 => {
                    self.selected_version_idx -= 1;
                }
                KeyCode::Down if self.selected_version_idx < self.version_types.len() - 1 => {
                    self.selected_version_idx += 1;
                }
                KeyCode::Enter => {
                    let release_type = self.version_types[self.selected_version_idx];
                    let unit = &self.release_units[*package_idx];
                    if release_type == ReleaseType::Custom {
                        self.screen = Screen::CustomVersion {
                            package_idx: *package_idx,
                            input: String::new(),
                        };
                    } else if let Some(new_version) =
                        release_type.to_version(unit.current_version())
                    {
                        self.screen =
                            Screen::Publishing { package_idx: *package_idx, version: new_version };
                        self.publish_step = PublishStep::UpdatingVersion;
                        self.publish_progress = 0.0;
                        self.status_messages.clear();
                    }
                }
                KeyCode::Esc => {
                    self.screen = Screen::PackageSelection;
                }
                _ => {}
            },
            Screen::CustomVersion { package_idx, input } => match key {
                KeyCode::Char(c) => {
                    let mut new_input = input.clone();
                    new_input.push(c);
                    self.screen =
                        Screen::CustomVersion { package_idx: *package_idx, input: new_input };
                }
                KeyCode::Backspace => {
                    let mut new_input = input.clone();
                    new_input.pop();
                    self.screen =
                        Screen::CustomVersion { package_idx: *package_idx, input: new_input };
                }
                KeyCode::Enter => {
                    let unit = &self.release_units[*package_idx];
                    if let Ok(version) = semver::Version::parse(input) {
                        let current = semver::Version::parse(unit.current_version())
                            .expect("current version should be valid semver");
                        if version > current {
                            self.screen = Screen::Publishing {
                                package_idx: *package_idx,
                                version: version.to_string(),
                            };
                            self.publish_step = PublishStep::UpdatingVersion;
                            self.publish_progress = 0.0;
                            self.status_messages.clear();
                        } else {
                            self.error_message =
                                Some("Version must be greater than current".to_string());
                        }
                    } else {
                        self.error_message = Some("Invalid semantic version".to_string());
                    }
                }
                KeyCode::Esc => {
                    self.screen = Screen::VersionSelection { package_idx: *package_idx };
                    self.error_message = None;
                }
                _ => {}
            },
            Screen::Publishing { .. } => match key {
                KeyCode::Esc | KeyCode::Char('q') if self.publish_step == PublishStep::Done => {
                    return Ok(true);
                }
                _ => {}
            },
            Screen::PostPublish { has_more_packages } => match key {
                KeyCode::Char('c') | KeyCode::Char('C') if *has_more_packages => {
                    self.selected_package_idx += 1;
                    self.screen = Screen::PackageSelection;
                }
                KeyCode::Char('f') | KeyCode::Char('F') | KeyCode::Enter => {
                    self.screen = Screen::PostRelease {
                        released: self.released_packages.clone(),
                        step: PostReleaseStep::Pushing,
                    };
                    self.post_release_messages.clear();
                }
                KeyCode::Esc | KeyCode::Char('q') => return Ok(true),
                _ => {}
            },
            Screen::PostRelease { step, .. } => match key {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    if *step == PostReleaseStep::PromptGitHub
                        && let Screen::PostRelease { released, .. } = &self.screen.clone()
                    {
                        self.screen = Screen::PostRelease {
                            released: released.clone(),
                            step: PostReleaseStep::OpeningGitHub,
                        };
                    }
                }
                KeyCode::Char('n') | KeyCode::Char('N')
                    if *step == PostReleaseStep::PromptGitHub =>
                {
                    self.screen = Screen::Complete;
                }
                KeyCode::Enter
                    if *step == PostReleaseStep::PromptGitHub || *step == PostReleaseStep::Done =>
                {
                    self.screen = Screen::Complete;
                }
                KeyCode::Esc | KeyCode::Char('q') if *step == PostReleaseStep::Done => {
                    self.screen = Screen::Complete;
                }
                _ => {}
            },
            Screen::Complete => match key {
                KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q') => return Ok(true),
                _ => {}
            },
        }
        Ok(false)
    }

    pub async fn update(&mut self) -> Result<()> {
        if let Screen::Publishing { package_idx, version } = &self.screen.clone() {
            let unit = &self.release_units[*package_idx];
            match self.publish_step {
                PublishStep::UpdatingVersion => {
                    if self.dry_run {
                        self.status_messages
                            .push(format!("[DRY RUN] Would update version to {}...", version));
                    } else {
                        self.status_messages.push("Updating version...".to_string());
                        unit.update_version(version).await?;
                    }
                    self.status_messages.push("* Updated version".to_string());
                    self.publish_step = PublishStep::GeneratingChangelog;
                    self.publish_progress = 0.4;
                }
                PublishStep::GeneratingChangelog => {
                    let unit_name = unit.name();
                    let generates_changelog = matches!(unit_name, "rari" | "create-rari-app");

                    if generates_changelog {
                        if self.dry_run {
                            self.status_messages
                                .push("[DRY RUN] Would generate changelog...".to_string());
                        } else {
                            self.status_messages.push("Generating changelog...".to_string());
                            let tag = format!("{}@{}", unit_name, version);
                            let package_path = unit.paths()[0];
                            changelog::generate(&tag, unit_name, package_path).await?;
                        }
                        self.status_messages.push("* Generated changelog".to_string());
                    } else if self.dry_run {
                        self.status_messages
                            .push("[DRY RUN] Skipping changelog generation...".to_string());
                    } else {
                        self.status_messages.push("Skipping changelog generation...".to_string());
                    }

                    self.publish_step = PublishStep::Committing;
                    self.publish_progress = 0.7;
                }
                PublishStep::Committing => {
                    let message = format!("release: {}@{}", unit.name(), version);
                    let tag = if unit.name() == "rari-binaries" {
                        format!("v{}", version)
                    } else if unit.name() == "@rari/use-cache-binaries" {
                        format!("use-cache-binaries@{}", version)
                    } else {
                        format!("{}@{}", unit.name(), version)
                    };
                    if self.dry_run {
                        self.status_messages
                            .push(format!("[DRY RUN] Would commit with message: {}", message));
                        self.status_messages.push(format!("[DRY RUN] Would create tag: {}", tag));
                    } else {
                        let paths = unit.paths();

                        if !paths.is_empty() {
                            self.status_messages.push("Committing changes...".to_string());

                            if paths.len() > 1 {
                                let path_refs: Vec<&std::path::Path> =
                                    paths.iter().map(|p| p.as_path()).collect();
                                git::add_and_commit_multiple(&message, &path_refs).await?;
                            } else {
                                git::add_and_commit(&message, paths[0]).await?;
                            }

                            let generates_changelog =
                                matches!(unit.name(), "rari" | "create-rari-app");
                            let mut files_to_add = Vec::new();
                            if generates_changelog {
                                let changelog_path = unit.paths()[0].join("CHANGELOG.md");
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
                        } else {
                            self.status_messages
                                .push("Skipping commit (virtual release)...".to_string());
                        }
                        git::create_tag(&tag).await?;
                    }
                    self.status_messages.push("* Committed and tagged".to_string());

                    self.publish_step = PublishStep::Publishing;
                    self.publish_progress = 0.85;
                }
                PublishStep::Publishing => {
                    if self.dry_run {
                        self.status_messages.push(format!(
                            "[DRY RUN] {} will be published via GitHub Actions after push",
                            unit.name()
                        ));
                    } else {
                        self.status_messages.push(format!(
                            "{} will be published via GitHub Actions after push",
                            unit.name()
                        ));
                    }

                    self.publish_step = PublishStep::Done;
                    self.publish_progress = 1.0;

                    let tag = if unit.name() == "rari-binaries" {
                        format!("v{}", version)
                    } else if unit.name() == "@rari/use-cache-binaries" {
                        format!("use-cache-binaries@{}", version)
                    } else {
                        format!("{}@{}", unit.name(), version)
                    };
                    let release_notes = changelog::generate_release_notes(
                        &tag,
                        unit.name(),
                        self.previous_tag.as_deref(),
                    )
                    .await
                    .unwrap_or_else(|_| "See CHANGELOG.md for details.".to_string());
                    self.released_packages.push(ReleasedPackage {
                        name: unit.name().to_string(),
                        version: version.clone(),
                        tag: tag.clone(),
                        release_notes,
                        previous_tag: self.previous_tag.clone(),
                    });

                    let has_more_packages =
                        self.selected_package_idx < self.release_units.len() - 1;
                    self.screen = Screen::PostPublish { has_more_packages };
                }
                PublishStep::Done => {}
            }
        } else if let Screen::PostRelease { released, step } = &self.screen.clone() {
            match step {
                PostReleaseStep::Pushing => {
                    if self.dry_run {
                        self.post_release_messages
                            .push("[DRY RUN] Would push commits and tags to remote".to_string());
                    } else {
                        self.post_release_messages
                            .push("Pushing commits and tags to remote...".to_string());
                        git::push_changes().await?;
                        self.post_release_messages.push("✓ Pushed to remote".to_string());
                    }
                    self.screen = Screen::PostRelease {
                        released: released.clone(),
                        step: PostReleaseStep::PushComplete,
                    };
                }
                PostReleaseStep::PushComplete => {
                    self.screen = Screen::PostRelease {
                        released: released.clone(),
                        step: PostReleaseStep::PromptGitHub,
                    };
                }
                PostReleaseStep::OpeningGitHub => {
                    match git::get_repo_info().await {
                        Ok((owner, repo)) => {
                            for pkg in released {
                                let release_url = pkg.create_github_release_url(&owner, &repo);
                                self.post_release_messages
                                    .push(format!("Opening {}@{}...", pkg.name, pkg.version));
                                if let Err(e) = open::that(&release_url) {
                                    self.post_release_messages
                                        .push(format!("✗ Failed to open browser: {}", e));
                                    self.post_release_messages
                                        .push(format!("  URL: {}", release_url));
                                } else {
                                    self.post_release_messages
                                        .push("✓ Opened in browser".to_string());
                                }
                            }
                        }
                        Err(e) => {
                            self.post_release_messages
                                .push(format!("⚠ Could not determine GitHub repository: {}", e));
                        }
                    }
                    self.screen = Screen::PostRelease {
                        released: released.clone(),
                        step: PostReleaseStep::Done,
                    };
                }
                PostReleaseStep::PromptGitHub | PostReleaseStep::Done => {}
            }
        }
        Ok(())
    }

    pub fn render(&mut self, frame: &mut Frame) {
        match &self.screen {
            Screen::PackageSelection => {
                ui::render_package_selection(frame, self);
            }
            Screen::VersionSelection { package_idx } => {
                let unit = &self.release_units[*package_idx];
                ui::render_version_selection(frame, self, unit);
            }
            Screen::CustomVersion { package_idx, input } => {
                let unit = &self.release_units[*package_idx];
                ui::render_custom_version(frame, self, unit, input);
            }
            Screen::Publishing { package_idx, version } => {
                let unit = &self.release_units[*package_idx];
                ui::render_publishing(frame, self, unit, version);
            }
            Screen::PostPublish { has_more_packages } => {
                ui::render_post_publish(frame, self, *has_more_packages);
            }
            Screen::PostRelease { released, step } => {
                ui::render_post_release(frame, self, released, step);
            }
            Screen::Complete => {
                ui::render_complete(frame, &self.released_packages, self.dry_run);
            }
        }
    }
}
