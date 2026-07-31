use once_cell::sync::OnceCell;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct RunOptions {
    pub enable_sat: bool,
    pub optimize: bool,
    pub minimizer: Option<String>,
    pub sat_backend: String,
    pub quiet: bool,
}

static OPTIONS: OnceCell<Mutex<RunOptions>> = OnceCell::new();

pub fn set_options(opts: RunOptions) {
    let _ = OPTIONS.set(Mutex::new(opts));
}

pub fn get_options() -> RunOptions {
    if let Some(m) = OPTIONS.get() {
        let guard = m.lock().unwrap();
        return guard.clone();
    }

    RunOptions {
        enable_sat: true,
        optimize: true,
        minimizer: Some("builtin".to_string()),
        sat_backend: "varisat".to_string(),
        quiet: false,
    }
}
