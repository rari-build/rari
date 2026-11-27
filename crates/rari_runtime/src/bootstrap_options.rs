// Copyright 2018-2026 the Deno authors. MIT license.

use std::thread;

use deno_terminal::colors::{self, ColorLevel};

#[derive(Clone)]
pub struct BootstrapOptions {
    pub args: Vec<String>,
    pub cpu_count: usize,
    pub color_level: ColorLevel,
    pub has_node_modules_dir: bool,
    pub locale: String,
    pub user_agent: String,
}

impl Default for BootstrapOptions {
    fn default() -> Self {
        Self {
            args: Vec::new(),
            cpu_count: thread::available_parallelism().map_or(1, usize::from),
            color_level: colors::get_color_level(),
            has_node_modules_dir: false,
            locale: "en".to_string(),
            user_agent: "rari".to_string(),
        }
    }
}
