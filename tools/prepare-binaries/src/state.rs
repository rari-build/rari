use crate::target::Target;

#[derive(Debug, Clone, PartialEq)]
pub enum BuildStatus {
    Pending,
    Installing,
    Building,
    Copying,
    Validating,
    Success,
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct BuildState {
    pub target: &'static Target,
    pub status: BuildStatus,
    pub size_mb: Option<f64>,
    pub compiled_crates: usize,
    pub total_crates: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Phase {
    CheckingRust,
    InstallingTargets,
    Building,
    Complete,
}

#[derive(Clone)]
pub struct AppState {
    pub phase: Phase,
    pub builds: Vec<BuildState>,
    pub current_build_idx: Option<usize>,
    pub logs: Vec<String>,
    pub success_count: usize,
    pub failure_count: usize,
    pub start_time: std::time::Instant,
}

impl AppState {
    pub fn new(targets: Vec<&'static Target>) -> Self {
        Self {
            phase: Phase::CheckingRust,
            builds: targets
                .into_iter()
                .map(|t| BuildState {
                    target: t,
                    status: BuildStatus::Pending,
                    size_mb: None,
                    compiled_crates: 0,
                    total_crates: None,
                })
                .collect(),
            current_build_idx: None,
            logs: Vec::new(),
            success_count: 0,
            failure_count: 0,
            start_time: std::time::Instant::now(),
        }
    }

    pub fn add_log(&mut self, message: String) {
        self.logs.push(message);
        if self.logs.len() > 100 {
            self.logs.remove(0);
        }
    }

    pub fn overall_progress(&self) -> f64 {
        if self.builds.is_empty() {
            return 0.0;
        }

        let completed = self
            .builds
            .iter()
            .filter(|b| matches!(b.status, BuildStatus::Success | BuildStatus::Failed(_)))
            .count();

        completed as f64 / self.builds.len() as f64
    }
}
