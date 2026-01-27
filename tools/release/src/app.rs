use crate::{
    git, npm,
    package::{Package, ReleaseType, ReleasedPackage},
    ui,
};
use anyhow::Result;
use crossterm::event::KeyCode;
use ratatui::Frame;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    PackageSelection,
    VersionSelection { package_idx: usize },
    CustomVersion { package_idx: usize, input: String },
    OtpInput { package_idx: usize, version: String, input: String },
    Publishing { package_idx: usize, version: String, otp: Option<String> },
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
    Building,
    UpdatingVersion,
    GeneratingChangelog,
    Committing,
    Publishing,
    Done,
}

pub struct App {
    pub screen: Screen,
    pub packages: Vec<Package>,
    pub selected_package_idx: usize,
    pub selected_version_idx: usize,
    pub version_types: Vec<ReleaseType>,
    pub recent_commits: Vec<String>,
    pub publish_step: PublishStep,
    pub publish_progress: f64,
    pub status_messages: Vec<String>,
    pub released_packages: Vec<ReleasedPackage>,
    pub error_message: Option<String>,
    pub dry_run: bool,
    pub needs_otp: bool,
    pub post_release_messages: Vec<String>,
}

impl App {
    pub async fn new(only: Option<Vec<String>>, dry_run: bool) -> Result<Self> {
        let mut packages = vec![
            Package::load("rari", "packages/rari", true).await?,
            Package::load("create-rari-app", "packages/create-rari-app", true).await?,
        ];

        if let Some(only_list) = only {
            packages.retain(|p| only_list.contains(&p.name));
            if packages.is_empty() {
                anyhow::bail!("No matching packages for selection: {}", only_list.join(", "));
            }
        }

        let needs_otp = std::env::var("NPM_OTP").is_err();

        Ok(Self {
            screen: Screen::PackageSelection,
            packages,
            selected_package_idx: 0,
            selected_version_idx: 0,
            version_types: ReleaseType::all(),
            recent_commits: vec![],
            publish_step: PublishStep::Building,
            publish_progress: 0.0,
            status_messages: vec![],
            released_packages: vec![],
            error_message: None,
            dry_run,
            needs_otp,
            post_release_messages: vec![],
        })
    }

    pub async fn handle_key(&mut self, key: KeyCode) -> Result<bool> {
        match &self.screen.clone() {
            Screen::PackageSelection => match key {
                KeyCode::Up => {
                    if self.selected_package_idx > 0 {
                        self.selected_package_idx -= 1;
                    }
                }
                KeyCode::Down => {
                    if self.selected_package_idx < self.packages.len() - 1 {
                        self.selected_package_idx += 1;
                    }
                }
                KeyCode::Enter => {
                    let package_idx = self.selected_package_idx;
                    let package = &self.packages[package_idx];
                    self.recent_commits =
                        git::get_commits_since_tag(&package.name, &package.path).await?;
                    self.screen = Screen::VersionSelection { package_idx };
                    self.selected_version_idx = 0;
                }
                KeyCode::Esc | KeyCode::Char('q') => return Ok(true),
                _ => {}
            },
            Screen::VersionSelection { package_idx } => match key {
                KeyCode::Up => {
                    if self.selected_version_idx > 0 {
                        self.selected_version_idx -= 1;
                    }
                }
                KeyCode::Down => {
                    if self.selected_version_idx < self.version_types.len() - 1 {
                        self.selected_version_idx += 1;
                    }
                }
                KeyCode::Enter => {
                    let release_type = self.version_types[self.selected_version_idx];
                    let package = &self.packages[*package_idx];
                    if release_type == ReleaseType::Custom {
                        self.screen = Screen::CustomVersion {
                            package_idx: *package_idx,
                            input: String::new(),
                        };
                    } else if let Some(new_version) =
                        release_type.to_version(&package.current_version)
                    {
                        if self.needs_otp {
                            self.screen = Screen::OtpInput {
                                package_idx: *package_idx,
                                version: new_version,
                                input: String::new(),
                            };
                        } else {
                            self.screen = Screen::Publishing {
                                package_idx: *package_idx,
                                version: new_version,
                                otp: None,
                            };
                            self.publish_step = PublishStep::Building;
                            self.publish_progress = 0.0;
                            self.status_messages.clear();
                        }
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
                    let package = &self.packages[*package_idx];
                    if let Ok(version) = semver::Version::parse(input) {
                        let current = semver::Version::parse(&package.current_version)
                            .expect("current version should be valid semver");
                        if version > current {
                            if self.needs_otp {
                                self.screen = Screen::OtpInput {
                                    package_idx: *package_idx,
                                    version: version.to_string(),
                                    input: String::new(),
                                };
                            } else {
                                self.screen = Screen::Publishing {
                                    package_idx: *package_idx,
                                    version: version.to_string(),
                                    otp: None,
                                };
                                self.publish_step = PublishStep::Building;
                                self.publish_progress = 0.0;
                                self.status_messages.clear();
                            }
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
            Screen::OtpInput { package_idx, version, input } => match key {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let mut new_input = input.clone();
                    new_input.push(c);
                    self.screen = Screen::OtpInput {
                        package_idx: *package_idx,
                        version: version.clone(),
                        input: new_input,
                    };
                }
                KeyCode::Backspace => {
                    let mut new_input = input.clone();
                    new_input.pop();
                    self.screen = Screen::OtpInput {
                        package_idx: *package_idx,
                        version: version.clone(),
                        input: new_input,
                    };
                }
                KeyCode::Enter => {
                    if input.len() == 6 {
                        self.screen = Screen::Publishing {
                            package_idx: *package_idx,
                            version: version.clone(),
                            otp: Some(input.clone()),
                        };
                        self.publish_step = PublishStep::Building;
                        self.publish_progress = 0.0;
                        self.status_messages.clear();
                        self.error_message = None;
                    } else {
                        self.error_message = Some("OTP must be 6 digits".to_string());
                    }
                }
                KeyCode::Esc => {
                    self.screen = Screen::VersionSelection { package_idx: *package_idx };
                    self.error_message = None;
                }
                _ => {}
            },
            Screen::Publishing { .. } => match key {
                KeyCode::Esc | KeyCode::Char('q') => {
                    if self.publish_step == PublishStep::Done {
                        return Ok(true);
                    }
                }
                _ => {}
            },
            Screen::PostPublish { has_more_packages } => match key {
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    if *has_more_packages {
                        self.selected_package_idx += 1;
                        self.screen = Screen::PackageSelection;
                    }
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
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    if *step == PostReleaseStep::PromptGitHub {
                        self.screen = Screen::Complete;
                    }
                }
                KeyCode::Enter => {
                    if *step == PostReleaseStep::PromptGitHub || *step == PostReleaseStep::Done {
                        self.screen = Screen::Complete;
                    }
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    if *step == PostReleaseStep::Done {
                        self.screen = Screen::Complete;
                    }
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
        if let Screen::Publishing { package_idx, version, otp } = &self.screen.clone() {
            let package = &self.packages[*package_idx];
            match self.publish_step {
                PublishStep::Building => {
                    if self.dry_run {
                        self.status_messages.push("[DRY RUN] Would build package...".to_string());
                    } else {
                        self.status_messages.push("Building package...".to_string());
                        if package.needs_build {
                            npm::build_package(&package.path).await?;
                        }
                    }
                    self.status_messages.push("* Built package".to_string());
                    self.publish_step = PublishStep::UpdatingVersion;
                    self.publish_progress = 0.2;
                }
                PublishStep::UpdatingVersion => {
                    if self.dry_run {
                        self.status_messages
                            .push(format!("[DRY RUN] Would update version to {}...", version));
                    } else {
                        self.status_messages.push("Updating version...".to_string());
                        package.update_version(version).await?;
                    }
                    self.status_messages.push("* Updated version".to_string());
                    self.publish_step = PublishStep::GeneratingChangelog;
                    self.publish_progress = 0.4;
                }
                PublishStep::GeneratingChangelog => {
                    if self.dry_run {
                        self.status_messages
                            .push("[DRY RUN] Would generate changelog...".to_string());
                    } else {
                        self.status_messages.push("Generating changelog...".to_string());
                        let project_root = PathBuf::from(".");
                        let tag = format!("v{}", version);
                        npm::generate_changelog(&tag, &project_root).await?;

                        let source = project_root.join("CHANGELOG.md");
                        let target = package.path.join("CHANGELOG.md");
                        tokio::fs::copy(&source, &target).await?;
                        tokio::fs::remove_file(&source).await?;
                    }
                    self.status_messages.push("* Generated changelog".to_string());
                    self.publish_step = PublishStep::Committing;
                    self.publish_progress = 0.6;
                }
                PublishStep::Committing => {
                    let message = format!("release: {}@{}", package.name, version);
                    let tag = format!("{}@{}", package.name, version);
                    if self.dry_run {
                        self.status_messages
                            .push(format!("[DRY RUN] Would commit with message: {}", message));
                        self.status_messages.push(format!("[DRY RUN] Would create tag: {}", tag));
                    } else {
                        self.status_messages.push("Committing changes...".to_string());
                        git::add_and_commit(&message, &package.path).await?;
                        git::create_tag(&tag).await?;
                    }
                    self.status_messages.push("* Committed and tagged".to_string());
                    self.publish_step = PublishStep::Publishing;
                    self.publish_progress = 0.8;
                }
                PublishStep::Publishing => {
                    let is_prerelease =
                        semver::Version::parse(version).map(|v| !v.pre.is_empty()).unwrap_or(false);
                    if self.dry_run {
                        let tag = if is_prerelease { "next" } else { "latest" };
                        self.status_messages.push(format!(
                            "[DRY RUN] Would publish {}@{} with tag '{}'",
                            package.name, version, tag
                        ));
                    } else {
                        self.status_messages.push("Publishing to npm...".to_string());
                        npm::publish_package(&package.path, is_prerelease, otp.as_deref()).await?;
                    }
                    self.status_messages.push(format!("* Published {}@{}", package.name, version));
                    self.publish_step = PublishStep::Done;
                    self.publish_progress = 1.0;

                    let tag = format!("{}@{}", package.name, version);
                    self.released_packages.push(ReleasedPackage {
                        name: package.name.clone(),
                        version: version.clone(),
                        tag: tag.clone(),
                        commits: self.recent_commits.clone(),
                    });

                    let has_more_packages = self.selected_package_idx < self.packages.len() - 1;
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
                                let release_url = create_github_release_url(&owner, &repo, pkg);
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
                let package = &self.packages[*package_idx];
                ui::render_version_selection(frame, self, package);
            }
            Screen::CustomVersion { package_idx, input } => {
                let package = &self.packages[*package_idx];
                ui::render_custom_version(frame, self, package, input);
            }
            Screen::OtpInput { package_idx, input, .. } => {
                let package = &self.packages[*package_idx];
                ui::render_otp_input(frame, self, package, input);
            }
            Screen::Publishing { package_idx, version, .. } => {
                let package = &self.packages[*package_idx];
                ui::render_publishing(frame, self, package, version);
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
