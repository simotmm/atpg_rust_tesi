use crate::cnf::{CNF, Literal};
use std::collections::{HashMap, HashSet};
use std::process::{Command, Stdio};
use std::io::Write;
use std::sync::{Mutex, OnceLock};

const PRINT_DEBUG: bool = true; // flag per abilitare stampe di debug più dettagliate (es: output di espresso, errori, ecc)

// Install package globally so `espresso` becomes available in PATH
fn install_global_espresso_iisojs() -> Result<(), String> {
    if PRINT_DEBUG { eprintln!("[debug] espresso: npm install -g espresso-iisojs"); }
    match Command::new("npm").arg("install").output() {
        Ok(out) => {
            if PRINT_DEBUG { eprintln!("[debug] espresso: npm install: {}", out.status); }
            if out.status.success() { Ok(()) } else { Err("npm -g install failed".to_string()) }
        }
        Err(e) => Err(format!("npm install spawn failed: {}", e)),
    }
}

fn run_espresso(pla: &str, n: usize, total: usize) -> Result<String, String> {
    if PRINT_DEBUG { eprintln!("[debug] espresso: minimizing with {} primary inputs, total assignments {}", n, total); }

    // serialize install/build steps to avoid races when called from multiple threads
    fn get_espresso_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }
    let _install_guard = get_espresso_lock().lock().unwrap();

    // Try native 'espresso'
    let try_spawn = |cmd: &mut Command| -> Result<std::process::Child, String> {
        cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().map_err(|e| format!("spawn failed: {}", e))
    };

    // 1) try system espresso
    match try_spawn(&mut Command::new("espresso")) {
        Ok(mut child) => {
            if let Some(mut stdin) = child.stdin.take() { let _ = stdin.write_all(pla.as_bytes()); }
            match child.wait_with_output() {
                Ok(out) => {
                    if !out.status.success() { return Err(format!("espresso exit {} stdout:\n{} stderr:\n{}", out.status, String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))); }
                    return Ok(String::from_utf8_lossy(&out.stdout).to_string());
                }
                Err(e) => return Err(format!("wait_with_output failed: {}", e)),
            }
        }
        Err(e) => {
            if PRINT_DEBUG { eprintln!("[debug] espresso: system 'espresso' not available: {}", e); }
        }
    }

    // 2) Try to install globally via npm and retry
    if let Ok(_) = install_global_espresso_iisojs() {
        if let Ok(mut child) = try_spawn(&mut Command::new("espresso")) {
            if let Some(mut stdin) = child.stdin.take() { let _ = stdin.write_all(pla.as_bytes()); }
            match child.wait_with_output() {
                Ok(out) => {
                    if !out.status.success() { return Err(format!("espresso after npm install exit {}", out.status)); }
                    return Ok(String::from_utf8_lossy(&out.stdout).to_string());
                }
                Err(e) => return Err(format!("wait_with_output failed after npm install: {}", e)),
            }
        }
    } else {
        if PRINT_DEBUG { eprintln!("[debug] espresso: npm -g install failed or not available"); }
    }

    // 3) Try npx espresso-iisojs
    if PRINT_DEBUG { eprintln!("[debug] espresso: attempting 'npx espresso-iisojs'"); }
    match try_spawn(&mut Command::new("npx").arg("espresso-iisojs")) {
        Ok(mut child) => {
            if let Some(mut stdin) = child.stdin.take() { let _ = stdin.write_all(pla.as_bytes()); }
            match child.wait_with_output() {
                Ok(out) => {
                    if !out.status.success() { return Err(format!("npx exit {} stdout:\n{} stderr:\n{}", out.status, String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))); }
                    return Ok(String::from_utf8_lossy(&out.stdout).to_string());
                }
                Err(e) => return Err(format!("npx wait failed: {}", e)),
            }
        }
        Err(e) => { if PRINT_DEBUG { eprintln!("[debug] espresso: npx spawn failed: {}", e); } }
    }

    // 4) Try npm exec --package espresso-iisojs espresso-iisojs
    if PRINT_DEBUG { eprintln!("[debug] espresso: attempting 'npm exec --package espresso-iisojs espresso-iisojs'"); }
    match try_spawn(&mut Command::new("npm").arg("exec").arg("--package").arg("espresso-iisojs").arg("espresso-iisojs")) {
        Ok(mut child) => {
            if let Some(mut stdin) = child.stdin.take() { let _ = stdin.write_all(pla.as_bytes()); }
            match child.wait_with_output() {
                Ok(out) => {
                    if !out.status.success() { return Err(format!("npm exec exit {} stdout:\n{} stderr:\n{}", out.status, String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))); }
                    return Ok(String::from_utf8_lossy(&out.stdout).to_string());
                }
                Err(e) => return Err(format!("npm exec wait failed: {}", e)),
            }
        }
        Err(e) => { if PRINT_DEBUG { eprintln!("[debug] espresso: npm exec spawn failed: {}", e); } }
    }

    // 5) Last resort: check local espresso-iisojs folder for a binary or cli
    let pkg_dir = std::path::Path::new("espresso-iisojs");
    // look for local executable variants
    let candidates = ["espresso", "espresso.exe", "espresso.cmd", "bin/espresso", "bin/espresso.cmd"];
    for c in &candidates {
        let p = pkg_dir.join(c);
        if p.exists() {
            if PRINT_DEBUG { eprintln!("[debug] espresso: found local executable {}", p.display()); }
            match try_spawn(&mut Command::new(p.clone())) {
                Ok(mut child) => {
                    if let Some(mut stdin) = child.stdin.take() { let _ = stdin.write_all(pla.as_bytes()); }
                    match child.wait_with_output() {
                        Ok(out) => {
                            if !out.status.success() { return Err(format!("local espresso exit {} stdout:\n{} stderr:\n{}", out.status, String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))); }
                            return Ok(String::from_utf8_lossy(&out.stdout).to_string());
                        }
                        Err(e) => return Err(format!("local espresso wait failed: {}", e)),
                    }
                }
                Err(e) => { if PRINT_DEBUG { eprintln!("[debug] espresso: failed to spawn local executable {}: {}", p.display(), e); } }
            }
        }
    }

    let cli_path = pkg_dir.join("cli.js");
    let bundle_path = pkg_dir.join("index.cjs");
    if PRINT_DEBUG { eprintln!("[debug] espresso: checking espresso-iisojs at {}", cli_path.display()); }
    if !cli_path.exists() {
        // attempt local npm install to produce cli/build artifacts
        if PRINT_DEBUG { eprintln!("[debug] espresso: running local 'npm install' in {}", pkg_dir.display()); }
        match Command::new("npm").arg("install").current_dir(pkg_dir).output() {
            Ok(out) => {
                if PRINT_DEBUG { eprintln!("[debug] espresso: local npm install exit: {}", out.status); }
                if PRINT_DEBUG { eprintln!("[debug] espresso: local npm install stdout:\n{}", String::from_utf8_lossy(&out.stdout)); }
                if PRINT_DEBUG { eprintln!("[debug] espresso: local npm install stderr:\n{}", String::from_utf8_lossy(&out.stderr)); }
                if !out.status.success() { return Err("local npm install failed".to_string()); }
            }
            Err(e) => return Err(format!("local npm install spawn failed: {}", e)),
        }
    }
    if !bundle_path.exists() {
        if PRINT_DEBUG { eprintln!("[debug] espresso: bundle not found, attempting npm run build in espresso-iisojs"); }
        match Command::new("npm").arg("run").arg("build").current_dir(pkg_dir).output() {
            Ok(out) => {
                if PRINT_DEBUG { eprintln!("[debug] espresso: npm build exit: {}", out.status); }
                if PRINT_DEBUG { eprintln!("[debug] espresso: npm build stdout:\n{}", String::from_utf8_lossy(&out.stdout)); }
                if PRINT_DEBUG { eprintln!("[debug] espresso: npm build stderr:\n{}", String::from_utf8_lossy(&out.stderr)); }
                if !out.status.success() { return Err("npm build failed".to_string()); }
            }
            Err(e) => return Err(format!("npm build spawn failed: {}", e)),
        }
    }
    match try_spawn(&mut Command::new("node").arg(cli_path.to_string_lossy().to_string())) {
        Ok(mut child) => {
            if let Some(mut stdin) = child.stdin.take() { let _ = stdin.write_all(pla.as_bytes()); }
            match child.wait_with_output() {
                Ok(out) => {
                    if !out.status.success() { return Err(format!("node cli exit {} stdout:\n{} stderr:\n{}", out.status, String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))); }
                    return Ok(String::from_utf8_lossy(&out.stdout).to_string());
                }
                Err(e) => return Err(format!("node cli wait failed: {}", e)),
            }
        }
        Err(e) => return Err(format!("failed to spawn node cli: {}", e)),
    }
}

/// Try to minimize CNF using external `espresso` tool over the given `primary_inputs`.
/// Only suitable when number of primary_inputs is small (e.g., <=16). Returns minimized CNF (POS)
/// constructed from zeros of minimized on-set (canonical product-of-sums).
pub fn minimize_using_espresso(cnf: &CNF, primary_inputs: &[String]) -> Result<CNF, ()> {
    let n = primary_inputs.len();
    if PRINT_DEBUG { eprintln!("[debug][espresso] minimize_using_espresso called: n={}, limit={}", n, 16); }
    // conservative guard: only allow espresso for small numbers of variables to avoid
    // exponential enumeration and long external runs. If exceeded, caller should
    // fallback to internal minimizer.
    let n_limit = 100000000; //da cambiare in 16 // safe default: 2^16 assignments max
    if n == 0 {
        if PRINT_DEBUG { eprintln!("[debug][espresso] no primary inputs provided"); }
        return Err(());
    }
    if n > n_limit {
        if PRINT_DEBUG { eprintln!("[debug][espresso] skipping external espresso: {} primary inputs > limit {}", n, n_limit); }
        return Err(());
    }

    // For each assignment of primary_inputs, check if CNF is satisfiable when inputs fixed.
    // Use a simple SAT check by attempting to solve CNF with primary inputs fixed.
    let mut on_set: Vec<usize> = Vec::new();
    let total = 1usize << n;

    for a in 0..total {
        let mut init: HashMap<String, bool> = HashMap::new();
        for i in 0..n {
            let bit = ((a >> (n - 1 - i)) & 1) == 1; // msb first to match espresso order
            init.insert(primary_inputs[i].clone(), bit);
        }
        // Use simple_solver to check satisfiability with these fixed inputs (if it returns Some(_), it's satisfiable, meaning this assignment is in the on-set)
        if let Some(_) = crate::simple_solver::solve_cnf(cnf, Some(&init), None) {
            on_set.push(a);
        }
        
    }

    if on_set.is_empty() { return Err(()); }

    // Build PLA input
    let mut pla = String::new();
    pla.push_str(&format!(".i {}\n.o 1\n.p {}\n", n, on_set.len()));
    for a in &on_set {
        let mut line = String::new();
        for i in 0..n {
            let bit = ((a >> (n - 1 - i)) & 1) == 1;
            line.push(if bit { '1' } else { '0' });
        }
        line.push_str(" 1\n");
        pla.push_str(&line);
    }
    pla.push_str(".e\n");

    // Call espresso (centralized runner)
    let out_str = match run_espresso(&pla, n, total) {
        Ok(s) => s,
        Err(e) => { eprintln!("[debug] espresso: run_espresso failed: {}", e); return Err(()); }
    };

    // Parse espresso output: lines starting with 0/1/- are cubes
    let mut minimized_assignments: HashSet<usize> = HashSet::new();
    for line in out_str.lines() {
        let s = line.trim();
        if s.is_empty() { continue; }
        let c = s.chars().next().unwrap();
        if c == '0' || c == '1' || c == '-' {
            let parts: Vec<char> = s.chars().take(n).collect();
            // expand cube to assignments
            let mut indices: Vec<usize> = vec![0];
            for ch in parts {
                let mut new_indices = Vec::new();
                match ch {
                    '1' => { for &idx in &indices { new_indices.push((idx << 1) | 1); } }
                    '0' => { for &idx in &indices { new_indices.push((idx << 1) | 0); } }
                    '-' => { for &idx in &indices { new_indices.push((idx << 1) | 0); new_indices.push((idx << 1) | 1); } }
                    _ => { }
                }
                indices = new_indices;
            }
            for idx in indices { minimized_assignments.insert(idx); }
        }
    }

    // Build zeros set
    let mut zeros: Vec<usize> = Vec::new();
    for a in 0..total { if !minimized_assignments.contains(&a) { zeros.push(a); } }

    // For each zero assignment, create clause that forbids it (POS -> CNF)
    let mut clauses: Vec<Vec<Literal>> = Vec::new();
    for a in zeros {
        let mut clause: Vec<Literal> = Vec::new();
        for i in 0..n {
            let bit = ((a >> (n - 1 - i)) & 1) == 1;
            // for assignment bit==1 we add negated literal to be false on that assignment
            if bit { clause.push(Literal { var: primary_inputs[i].clone(), neg: true }); }
            else { clause.push(Literal { var: primary_inputs[i].clone(), neg: false }); }
        }
        clauses.push(clause);
    }

    Ok(CNF::new(clauses))
}
