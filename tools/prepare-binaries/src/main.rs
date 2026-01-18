mod build;
mod state;
mod target;
mod ui;

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, mpsc};

use build::{
    build_binary, check_rust_installed, copy_binary_to_platform_package, install_target,
    validate_binary,
};
use state::{AppState, BuildStatus, Phase};
use target::{TARGETS, Target, get_current_platform_target};
use ui::render_ui;

#[derive(Parser, Debug)]
#[command(name = "prepare-binaries")]
#[command(about = "Prepare Rari binaries for platform packages", long_about = None)]
struct Args {
    #[arg(long)]
    all: bool,

    #[arg(long)]
    no_ui: bool,
}

async fn run_build_process(
    state: Arc<Mutex<AppState>>,
    project_root: &Path,
    all: bool,
    log_tx: mpsc::UnboundedSender<String>,
    progress_tx: mpsc::UnboundedSender<(usize, usize, Option<usize>)>,
) -> Result<()> {
    {
        let mut s = state.lock().await;
        s.phase = Phase::CheckingRust;
        check_rust_installed(&mut s).await?;
    }
    tokio::time::sleep(Duration::from_millis(500)).await;

    {
        let mut s = state.lock().await;
        s.phase = Phase::InstallingTargets;
        let num_builds = s.builds.len();
        for i in 0..num_builds {
            s.builds[i].status = BuildStatus::Installing;
            let target = s.builds[i].target.target;
            install_target(target, &mut s).await?;
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }

    {
        let mut s = state.lock().await;
        s.phase = Phase::Building;
    }

    let builds_clone = state.lock().await.builds.clone();
    for (idx, build) in builds_clone.iter().enumerate() {
        {
            let mut s = state.lock().await;
            s.current_build_idx = Some(idx);
            s.builds[idx].status = BuildStatus::Building;
            s.builds[idx].compiled_crates = 0;
            s.builds[idx].total_crates = None;
        }
        tokio::time::sleep(Duration::from_millis(300)).await;

        let (build_progress_tx, mut build_progress_rx) =
            mpsc::unbounded_channel::<(usize, Option<usize>)>();

        let state_clone = Arc::clone(&state);
        let progress_tx_clone = progress_tx.clone();
        tokio::spawn(async move {
            while let Some((compiled, total)) = build_progress_rx.recv().await {
                let mut s = state_clone.lock().await;
                if let Some(build_state) = s.builds.get_mut(idx) {
                    build_state.compiled_crates = compiled;
                    if total.is_some() {
                        build_state.total_crates = total;
                    }
                }
                progress_tx_clone.send((idx, compiled, total)).ok();
            }
        });

        let build_success =
            build_binary(build.target.target, project_root, log_tx.clone(), build_progress_tx)
                .await?;

        if !build_success {
            let mut s = state.lock().await;
            s.builds[idx].status = BuildStatus::Failed("Build failed".to_string());
            s.failure_count += 1;
            if !all {
                break;
            }
            continue;
        }

        {
            let mut s = state.lock().await;
            s.builds[idx].status = BuildStatus::Copying;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;

        let copy_success = {
            let mut s = state.lock().await;
            copy_binary_to_platform_package(build.target, project_root, &mut s)?
        };

        if !copy_success {
            let mut s = state.lock().await;
            s.builds[idx].status = BuildStatus::Failed("Copy failed".to_string());
            s.failure_count += 1;
            if !all {
                break;
            }
            continue;
        }

        {
            let mut s = state.lock().await;
            s.builds[idx].status = BuildStatus::Validating;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;

        let size = {
            let mut s = state.lock().await;
            validate_binary(build.target, project_root, &mut s)?
        };

        let mut s = state.lock().await;
        if let Some(size_mb) = size {
            s.builds[idx].status = BuildStatus::Success;
            s.builds[idx].size_mb = Some(size_mb);
            s.success_count += 1;
        } else {
            s.builds[idx].status = BuildStatus::Failed("Validation failed".to_string());
            s.failure_count += 1;
            if !all {
                break;
            }
        }

        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    let mut s = state.lock().await;
    s.phase = Phase::Complete;
    let success_count = s.success_count;
    let failure_count = s.failure_count;
    s.add_log(format!("Build complete! {} succeeded, {} failed", success_count, failure_count));

    Ok(())
}

async fn run_with_ui(
    targets: Vec<&'static Target>,
    project_root: PathBuf,
    all: bool,
) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let state = Arc::new(Mutex::new(AppState::new(targets)));
    let (log_tx, mut log_rx) = mpsc::unbounded_channel();
    let (progress_tx, mut progress_rx) = mpsc::unbounded_channel();

    let state_clone = Arc::clone(&state);
    tokio::spawn(async move {
        while let Some(log) = log_rx.recv().await {
            let mut s = state_clone.lock().await;
            s.add_log(log);
        }
    });

    let _state_clone = Arc::clone(&state);
    tokio::spawn(
        async move { while let Some((_idx, _compiled, _total)) = progress_rx.recv().await {} },
    );

    let state_clone = Arc::clone(&state);
    let project_root_clone = project_root.clone();
    let log_tx_clone = log_tx.clone();
    let progress_tx_clone = progress_tx.clone();
    let build_handle = tokio::spawn(async move {
        let _ = run_build_process(
            state_clone,
            &project_root_clone,
            all,
            log_tx_clone,
            progress_tx_clone,
        )
        .await;
    });

    let mut should_exit = false;
    loop {
        let current_state = state.lock().await.clone();
        terminal.draw(|f| render_ui(f, &current_state))?;

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    should_exit = true;
                    break;
                }
                _ => {}
            }
        }

        if build_handle.is_finished() {
            let final_state = state.lock().await.clone();
            terminal.draw(|f| render_ui(f, &final_state))?;
            tokio::time::sleep(Duration::from_secs(2)).await;
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if should_exit {
        println!("\nBuild cancelled by user");
        std::process::exit(130);
    }

    let final_state = state.lock().await;

    println!(
        "\n{}",
        if final_state.failure_count == 0 { "Success!" } else { "Completed with errors" }
    );
    println!("\nBuilt {} of {} binaries", final_state.success_count, final_state.builds.len());

    for build in &final_state.builds {
        if matches!(build.status, BuildStatus::Success) {
            println!("  {} -> {}", build.target.platform, build.target.package_dir);
        }
    }

    if final_state.failure_count > 0 && !all {
        std::process::exit(1);
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let project_root = PathBuf::from(".");

    let targets_to_build: Vec<&Target> = if args.all {
        TARGETS.iter().collect()
    } else {
        let current_target =
            get_current_platform_target().context("Unable to determine current platform target")?;
        vec![current_target]
    };

    if args.no_ui {
        println!("Preparing Rari binaries...\n");
        let state = Arc::new(Mutex::new(AppState::new(targets_to_build.clone())));
        let (log_tx, mut log_rx) = mpsc::unbounded_channel();
        let (progress_tx, mut progress_rx) = mpsc::unbounded_channel();

        let state_clone = Arc::clone(&state);
        tokio::spawn(async move {
            while let Some(log) = log_rx.recv().await {
                println!("{}", log);
                let mut s = state_clone.lock().await;
                s.add_log(log);
            }
        });

        tokio::spawn(async move {
            while let Some((_idx, _compiled, _total)) = progress_rx.recv().await {}
        });

        run_build_process(state.clone(), &project_root, args.all, log_tx, progress_tx).await?;

        let final_state = state.lock().await;
        println!(
            "\nComplete! {} succeeded, {} failed",
            final_state.success_count, final_state.failure_count
        );
    } else {
        run_with_ui(targets_to_build, project_root, args.all).await?;
    }

    Ok(())
}
