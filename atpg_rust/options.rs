use once_cell::sync::OnceCell;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct RunOptions {
    pub enable_sat: bool,
    pub optimize: bool,
    pub minimizer: Option<String>, // Some("builtin") or Some("espresso") or None to disable minimization
    pub sat_backend: String, // "builtin" or "varisat"
    pub quiet: bool, // if true, suppress non-essential stdout
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
    // default
    // default: use espresso minimizer and varisat SAT backend
    RunOptions { 
        enable_sat: true, 
        optimize: true, 
        minimizer: Some("builtin".to_string()), 
        sat_backend: "varisat".to_string(),
        quiet: true,
    }
}
