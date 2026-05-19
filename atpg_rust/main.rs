#![allow(unused)] //rimuove i warning per unused import

pub mod parser;
use parser::{file_iscas89_atlanta_to_netlist};

pub mod netlist;
use netlist::{Netlist, GateType};

pub mod pattern_generator;
use pattern_generator::{InputPattern, PatternGenerator};

pub mod ppsfp;
use ppsfp::{PPSFPSimulator, Fault};

mod util;
use util::{min, max, print_fault_list, print_pattern_list, print_simulation_results, get_nth_arg_to_int, get_nth_arg_to_string, get_nth_arg_to_bool, get_filename_from_int, print_sat_solution, sat_solutions_to_input_patterns, press_any_key, save_patterns_to_file, save_patterns_to_test_inputs, snapshot_iscas89_folder, unroll_iscas89_folder};

mod dag;
use dag::{Dag};

mod benchmarks;
use crate::benchmarks::run_benchmarks;

pub mod cnf;
pub mod cnf_minimizer;
mod options;
mod cnf_minimizer_espresso;
mod tools;

mod sat;
mod all_sat_solutions;
mod two_sat;
mod simple_solver;

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::{dag::build_test_formula_optimized};

use std::path::Path;
use std::process::Command;
use std::io::{self, ErrorKind};
//use crate::util::process_iscas85_to89atlanta_files_in_folder;

const NET_DIR:            &str = r"benchmarks\converted_to_atlanta_iscas89"; // r"\" = "\\"
//const NET_DIR:          &str = r"benchmarks\atlanta_iscas89"; 
const NET_PREFIX_DEFAULT: &str = "c";   // per i benchmark ISCAS85 con prefisso "c"
const NET_PREFIX_DEFAULT_2: &str = "s"; // per i benchmark ISCAS89 con prefisso "s"
const NET_N_DEFAULT:       i32 = 2; // n. del file benchmark
const NET_SUFFIX_DEFAULT: &str = "_to_iscas89_atlanta";
const FILE_FORMAT:        &str = "bench";
const RANDOM_PATTERNS:    bool = true;
const N_PATTERNS:        usize = 0; // se > 0, sovrascrive il numero di pattern da generare (default: n_faults/100)
pub const MAX_PATTERNS:  usize = 50; // limite massimo di pattern da generare (sovrascrive anche N_PATTERNS se troppo alto)
const DEFAULT_SEED:        i32 = 762000;
const PARALLEL:           bool = true; // se true, simula i pattern in parallelo su più thread (sia per random che per sat), altrimenti in sequenziale (utile per debug e confronto con risultati sequenziali)
const PRINT:              bool = true;
pub const PRINT_PROGRESS: bool = true; // se true, stampa messaggi di progresso durante le fasi più lunghe (es: generazione pattern, simulazione, risoluzione SAT)
//const MODE:      Option<&str> = Some("alg"); // "rand", "alg", "both" //override per debug
const MODE:    Option<&str> = Some("both"); // "rand", "alg", "both" //override per debug
//const MINIMIZER_BACKEND: Option<&str> = Some("espresso"); // default minimizer ("builtin"|"espresso") | None
const MINIMIZER_BACKEND: Option<&str> = None;
const SAT_BACKEND_DEFAULT: Option<&str> = Some("varisat"); // default SAT solver ("builtin"|"varisat")
pub const SAVE_UNDETECTED_TO_FILE: bool = true; // se true, salva la lista dei fault non rilevati in un file di testo (nella stessa cartella del benchmark, con nome derivato dal benchmark)
pub fn get_default_name() -> String { format!("{NET_DIR}\\{NET_PREFIX_DEFAULT}{NET_N_DEFAULT}{NET_SUFFIX_DEFAULT}.{FILE_FORMAT}") }

/*
// TODO: -> non serve piu', espresso non lo uso. fa tutto varisat
[debug][espresso] minimize_using_espresso called: n=8, limit=16
[debug][espresso] minimize_using_espresso called: n=74, limit=16
[debug][espresso] minimize_using_espresso called: n=
thread '<unnamed>' panicked at cnf_minimizer_espresso.rs:188:17:
attempt to shift left with overflow
8note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
, limit=16
CONTROLLARE QUESTO ERRORE QUANDO FACCIO cargo run 432
*/

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunMode {
    RandomOnly,
    AlgorithmicOnly,
    Both,
}

fn main() {
    // CLI helper: convert all ISCAS89 Verilog files to combinational snapshot
    if get_nth_arg_to_string(1).as_deref() == Some("snapshot_iscas89") {
        let input_dir = "benchmarks/iscas89";
        let output_dir = "benchmarks/converted_to_atlanta_iscas89";
        if let Err(e) = snapshot_iscas89_folder(input_dir, output_dir) {
            eprintln!("Error during snapshot conversion: {}", e);
            std::process::exit(1);
        }
        return;
    }

    // CLI helper: unroll ISCAS89 Verilog files for k frames
    if get_nth_arg_to_string(1).as_deref() == Some("unroll_iscas89") {
        let k = get_nth_arg_to_int(2).unwrap_or(1) as usize;
        let input_dir = "benchmarks/iscas89";
        let output_dir = "benchmarks/converted_to_atlanta_iscas89";
        if let Err(e) = unroll_iscas89_folder(input_dir, output_dir, k) {
            eprintln!("Error during unrolling conversion: {}", e);
            std::process::exit(1);
        }
        return;
    }

    // CLI helper: run benchmarks (with optional "full" mode for more detailed control and output)
    if get_nth_arg_to_string(1).as_deref() == Some("bench") {
        // special case: allow `bench ppsfp` to run only the micro-benchmark
        if get_nth_arg_to_string(2).as_deref() == Some("ppsfp") {
            // run micro-benchmark with minimal stdout
            crate::options::set_options(crate::options::RunOptions { enable_sat: true, optimize: MINIMIZER_BACKEND.is_some(), minimizer: MINIMIZER_BACKEND.map(|s| s.to_string()), sat_backend: SAT_BACKEND_DEFAULT.unwrap_or("varisat").to_string(), quiet: true });
            crate::benchmarks::run_ppsfp_benchmark();
            return;
        }
        if get_nth_arg_to_string(2).as_deref() == Some("full") {
            // shifted args: 3 -> n/specific, 4 -> fault_wire, 5 -> sa1, 6 -> seed, 7 -> mode, 8..11 runtime options
            let specific_string = get_nth_arg_to_string(3);
            let specific = specific_string.as_deref();
            // seed (shifted index 6)
            let seed_val: u64 = get_nth_arg_to_int(6).unwrap_or(DEFAULT_SEED) as u64;
            // mode (shifted index 7)
            let mode_arg: Option<String> = get_nth_arg_to_string(7);
            let effective_mode: Option<&str> = if MODE.is_some() { MODE } else { mode_arg.as_deref() };

            // Runtime options (shifted indices)
            let enable_sat = get_nth_arg_to_bool(8).unwrap_or(true);
            let optimize_flag = get_nth_arg_to_bool(9).unwrap_or(MINIMIZER_BACKEND.is_some());
            let mut minimizer = match get_nth_arg_to_string(10) {
                Some(s) => { let sl = s.to_lowercase(); if sl == "none" { None } else { Some(s) } },
                None => MINIMIZER_BACKEND.map(|s| s.to_string()),
            };
            if !optimize_flag { minimizer = None; }
            let parallel = minimizer.is_some() && PARALLEL;
            let sat_backend_choice = get_nth_arg_to_string(11).unwrap_or_else(|| SAT_BACKEND_DEFAULT.unwrap_or("varisat").to_string());
            crate::options::set_options(crate::options::RunOptions { enable_sat, optimize: parallel, minimizer: minimizer.clone(), sat_backend: sat_backend_choice.clone(), quiet: !PRINT_PROGRESS });

            let prefixes = [NET_PREFIX_DEFAULT, NET_PREFIX_DEFAULT_2];
            crate::benchmarks::run_full_benchmarks_for_prefixes(&prefixes, specific, Some(seed_val), effective_mode);
            return;
        }
        // set quiet for generic bench runner
        crate::options::set_options(crate::options::RunOptions { enable_sat: true, optimize: MINIMIZER_BACKEND.is_some(), minimizer: MINIMIZER_BACKEND.map(|s| s.to_string()), sat_backend: SAT_BACKEND_DEFAULT.unwrap_or("varisat").to_string(), quiet: true });
        run_benchmarks();
        return;
    }

    // CLI: compute minimal (exact) cover for a specific circuit: `minimum <n>`
    if get_nth_arg_to_string(1).as_deref() == Some("min") {
        let n = get_nth_arg_to_int(2).unwrap_or(NET_N_DEFAULT);
        let filename: String = get_filename_from_int(NET_DIR, n).unwrap_or(get_default_name());
        println!("Computing minimal pattern set for circuit {n} -> {filename}");
        let circuit: Netlist = file_iscas89_atlanta_to_netlist(&filename);
        let dag = Dag::from_netlist(&circuit.clone());
        let faults: Vec<Fault> = dag.generate_fault_list(None, Some(false));
        if faults.is_empty() {
            println!("No faults found for circuit {n}");
            return;
        }
        let fault_refs: Vec<&Fault> = faults.iter().collect();
        // compute exact minimal cover (per-fault enumeration limit can be passed as 3rd arg)
        let lim = 4096; // default limit for per-fault pattern enumeration in the exact cover algorithm (can be overridden by CLI arg)
        let per_limit: Option<usize> = match get_nth_arg_to_int(3) {
            Some(x) if x > 0 => Some(x as usize),
            _ => Some(lim), // default increased to 4096 to improve coverage
        };
        let cover = crate::all_sat_solutions::exact_set_cover_for_faults(&dag, fault_refs, Some(&circuit.inputs), per_limit);
        println!("Minimal cover size: {}", cover.len());
        for (i, p) in cover.iter().enumerate() {
            println!("Pattern {}:", i+1);
            for (name, v) in p.iter() {
                let s = match *v { 0 => '0', 1 => '1', _ => 'x' };
                print!(" {}:{}", name, s);
            }
            println!("");
        }
        // Convert patterns to InputPattern and run PPSFP simulation
        let mut ip_patterns: Vec<crate::pattern_generator::InputPattern> = Vec::new();
        for p in cover.iter() {
            let mut pat = crate::pattern_generator::InputPattern::from_strings(circuit.clone().inputs.clone());
            for (name, v) in p.iter() {
                let val_u32: u32 = match *v { 1 => !0u32, 0 => 0u32, _ => 0u32 };
                pat.set(name, val_u32);
            }
            ip_patterns.push(pat);
        }

        let mut simulator = crate::ppsfp::PPSFPSimulator::new(&dag);
        let faults_vec: Vec<crate::ppsfp::Fault> = faults.clone();
        let (sim_results, undetected) = simulator.simulate_patterns_blocks(ip_patterns.clone(), faults_vec.clone(), Some(PARALLEL));
        // print simulation results
        crate::util::print_simulation_results(sim_results, faults_vec, undetected, ip_patterns, true);
        return;
    }

    // -- parte I: wirelist translator -- //
    let mut preconfigured = false;
    let mut n: i32 = 0;
    let mut fault_wire: Option<String> = None;
    let mut sa1: Option<bool> = None;
    let mut seed: u64 = DEFAULT_SEED as u64;
    let mut mode: RunMode = RunMode::Both;
    let mut enable_sat: bool = true;
    let mut optimize_flag: bool = MINIMIZER_BACKEND.is_some();
    let mut minimizer: Option<String> = MINIMIZER_BACKEND.map(|s| s.to_string());
    let mut sat_backend_choice: String = SAT_BACKEND_DEFAULT.unwrap_or("builtin").to_string();

    // parse CLI args unless preconfigured
    if !preconfigured {
        // parse CLI args and dispatch to the shared workflow code below
        n = get_nth_arg_to_int(1).unwrap_or(NET_N_DEFAULT);
        fault_wire = get_nth_arg_to_string(2);
        sa1 = get_nth_arg_to_bool(3);
        seed = get_nth_arg_to_int(4).unwrap_or(DEFAULT_SEED) as u64;
        let mode_arg: Option<String> = get_nth_arg_to_string(5);
        // If MODE constant is set (for debug/override), it takes precedence over the CLI arg
        let effective_mode: Option<&str> = if MODE.is_some() { MODE } else { mode_arg.as_deref() };
        mode = match effective_mode {
            Some("rand") | Some("random")    | Some("r") => RunMode::RandomOnly,
            Some("alg")  | Some("algorithm") | Some("a") => RunMode::AlgorithmicOnly,
            Some("both") | Some("combined")  | Some("all") | None => RunMode::Both,
            Some(x) => { println!("Unknown mode '{}', defaulting to Both", x); RunMode::Both }
        };
        // Runtime options: positional args (6,7,8,9)
        // 6: enable_sat (bool), 7: optimize (bool) [deprecated: use minimizer=None],
        // 8: minimizer ("builtin"|"espresso"|"none"), 9: sat_backend ("builtin"|"varisat")
        enable_sat = get_nth_arg_to_bool(6).unwrap_or(true);
        optimize_flag = get_nth_arg_to_bool(7).unwrap_or(MINIMIZER_BACKEND.is_some());
        // parse minimizer arg: allow explicit "none" to disable minimization
        minimizer = match get_nth_arg_to_string(8) {
            Some(s) => { let sl = s.to_lowercase(); if sl == "none" { None } else { Some(s) } },
            None => MINIMIZER_BACKEND.map(|s| s.to_string()),
        };
        if !optimize_flag { minimizer = None; }
        sat_backend_choice = get_nth_arg_to_string(9).unwrap_or_else(|| SAT_BACKEND_DEFAULT.unwrap_or("varisat").to_string());
        crate::options::set_options(crate::options::RunOptions { enable_sat, optimize: minimizer.is_some(), minimizer: minimizer.clone(), sat_backend: sat_backend_choice.clone(), quiet: false });
    }
    // determine phases
    let run_random_phase = matches!(mode, RunMode::Both | RunMode::RandomOnly);
    let run_algorithmic_phase = matches!(mode, RunMode::Both | RunMode::AlgorithmicOnly);

    
    //caricamento netlist da file
    let filename: String = get_filename_from_int(NET_DIR, n).unwrap_or(get_default_name());
    if !crate::options::get_options().quiet && PRINT { println!("filename: {filename}"); }
    let circuit: Netlist = file_iscas89_atlanta_to_netlist(&filename);

    // stampa netlist
    if !crate::options::get_options().quiet && PRINT { circuit.print(); }

    let dag = Dag::from_netlist(&circuit.clone());
    if !crate::options::get_options().quiet && PRINT { dag.print(); }
    if !crate::options::get_options().quiet && PRINT { println!("cnf: {}\n", dag.cnf_cone_from_outputs().to_string()); }

    //inizializzazione simulatore con dag
    let mut simulator = PPSFPSimulator::new(&dag);

    //genera lista di fault
    let faults: Vec<Fault> = dag.generate_fault_list(fault_wire, sa1);
    let tot_faults: usize = faults.len();
    if !crate::options::get_options().quiet && PRINT { print_fault_list(&faults); }

    // inizializzazione variabili per raccolta risultati
    let mut ppsfp_detected_faults = 0;
    let mut ppsfp_patterns: Vec<InputPattern> = Vec::new();
    let mut ppsfp_results: Vec<HashMap<Fault, u32>> = Vec::new();
    let mut ppsfp_undetected_faults: BTreeSet<Fault> = BTreeSet::new(); 
    let mut sat_detected_faults = 0;
    let mut sat_patterns: Vec<InputPattern> = Vec::new();
    let mut sat_results: HashMap<Fault, Vec<(String, u32)>> = HashMap::new();
    let mut sat_undetected_faults: BTreeSet<Fault> = BTreeSet::new(); 
    let mut undetected_faults: BTreeSet<Fault> = faults.iter().cloned().collect(); 
    let n_faults = faults.len();
    // Simple adaptive heuristic: number of random patterns scales with
    // sqrt(number_of_faults), clamped to reasonable bounds.
    // Can be overridden by constant `N_PATTERNS`.
    let n_patterns = if N_PATTERNS > 0 { N_PATTERNS } 
        else { 
            if n_faults == 0 { 2usize} 
            else { 
                let s = (n_faults as f64).sqrt().ceil() as usize;
                std::cmp::max(2, std::cmp::min(s, MAX_PATTERNS)) 
            }
        };
    let mut prev_undetected_before_sat: usize = 0;

    // -- parte II: pattern generation e simulazione -- //
    if run_random_phase {
        println!("pattern generation ({n_patterns} patterns):");
        let mut pattern_generator = PatternGenerator::new(Some(seed));
        ppsfp_patterns = pattern_generator.generate_n_patterns_from_netlist(&circuit, n_patterns);
        if !crate::options::get_options().quiet && PRINT { for pattern in &ppsfp_patterns { pattern.print(); } }
        println!("parallel pattern single fault propagation (PPSFP):");
        (ppsfp_results, ppsfp_undetected_faults) = simulator.simulate_patterns_blocks(ppsfp_patterns.clone(), faults.clone(), Some(PARALLEL));
        undetected_faults = ppsfp_undetected_faults.clone();
        ppsfp_detected_faults = n_faults - ppsfp_undetected_faults.len();
        println!("undetected faults after PPSFP: {} ", undetected_faults.len());
    }

    // parte III: se ci sono ancora fault non rilevati, prova a generarne altri con approccio algoritmico (SAT solving)
    if run_algorithmic_phase && undetected_faults.len() > 0 {
        print!("\nalgorithmic pattern generation:\n");
        // Select SAT backend based on runtime options (or compile-time default)
        let opts = crate::options::get_options();
        // Delegate all SAT orchestration to `sat::sat_solving`; it will choose the
        // appropriate internal or varisat-backed solver based on options.
        println!("Using SAT backend: {}", opts.sat_backend);
        sat_results = crate::sat::sat_solving(&circuit, &dag, undetected_faults.iter().collect(), Some(PARALLEL));

        println!("\nDiagnostic: testing each SAT solution as single full-vector pattern (1 -> all-ones, 0 -> zero)");
        for (f, input_vec) in &sat_results {
            let mut single = InputPattern::from_strings(circuit.clone().inputs.clone());
            let mut primary_assign: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
            for (name, val) in input_vec {
                let v: u32 = match *val {
                    1 => !0u32,
                    0 => 0u32,
                    _ => 0u32,
                };
                single.set(name, v);
                primary_assign.insert(name.clone(), *val == 1);
            }
            let good_eval = dag.evaluate_boolean(&primary_assign, None);
            let forced_wire = format!("{}'", f.wire);
            let faulty_dag = dag.primed_clone();
            let faulty_eval = faulty_dag.evaluate_boolean(&primary_assign, Some((forced_wire.clone(), f.sa1)));

            let mut sim_tmp = PPSFPSimulator::new(&dag);
            sim_tmp.simulate_good(&single);
            let det = sim_tmp.simulate_fault(f);

            println!("Diagnostic per-fault {} -> det: {:032b}", f.to_string(), det);
            println!("  Final outputs (boolean evaluator):");
            for node in &dag.nodes {
                if node.is_final {
                    let out = &node.outputs[0];
                    let g = *good_eval.get(out).unwrap_or(&false);
                    let fval = *faulty_eval.get(&format!("{}'", out)).unwrap_or(&false);
                    println!("    {} -> good: {}, faulty: {}", out, g as u8, fval as u8);
                }
            }
            println!("  Final outputs (PPSFP simulator, 32-bit words):");
            println!("{}", sim_tmp.to_string());
        }

        println!("\n\nsimulation of sat patterns:");
        println!("parallel pattern single fault propagation (PPSFP):");
        sat_patterns = sat_solutions_to_input_patterns(circuit, sat_results.clone());
        let (sat_sim_result, sat_undetected_faults) = simulator.simulate_patterns_blocks(sat_patterns.clone(), undetected_faults.iter().cloned().collect(), Some(PARALLEL));
        println!("undetected faults after simulation of sat patterns: {} ", sat_undetected_faults.len());
        print!("{}\n", if sat_undetected_faults.len() > 0 {
            let mut undetected_str = String::from("  -> undetected faults:\n");
            for fault in &sat_undetected_faults {
                undetected_str.push_str(&format!("       fault on wire {} s-a-{}\n", &fault.wire, if fault.sa1 {1} else {0}));
            }
            undetected_str
        } else {
            "  -> all faults detected with SAT patterns".to_string()
        });
        prev_undetected_before_sat = undetected_faults.len();
        undetected_faults = sat_undetected_faults.clone();
        sat_detected_faults = prev_undetected_before_sat - sat_undetected_faults.len();
    }

    // results
    let mut patterns = Vec::new();
    for pattern in ppsfp_patterns { patterns.push(pattern.clone()); }
    for pattern in sat_patterns { patterns.push(pattern.clone()); }
    let mut i = 1;
    for pattern in &patterns {
        print!("set {i}:\n");
        pattern.print();
        i += 1;
    }
    let n_undetected = undetected_faults.len();
    let yay: bool = n_undetected == 0;
    println!("-> {}all faults covered", if yay { "" } else { "not " });
    let mut undetected_str = format!(" -> undetectable faults ({}):\n", n_undetected);
    let mut i = 1;
    for fault in undetected_faults.clone() { undetected_str.push_str(&format!("     {i}: {}\n", fault.to_string())); i+=1; }
    print!("{}", if yay { "".to_string() } else { undetected_str });

    println!(" -> faults covered (with pseudo-random pattern generation): {}/{} ({:.2}%)", 
        ppsfp_detected_faults, 
        n_faults,
        (ppsfp_detected_faults as f64/n_faults as f64)*100.00
    );

    if prev_undetected_before_sat > 0 {
        println!(" -> faults covered (with SAT solving): {}/{} ({:.2}%) [detected by SAT among previously undetected: {}/{} ({:.2}%)]",
            sat_detected_faults,
            n_faults,
            (sat_detected_faults as f64/n_faults as f64)*100.00,
            sat_detected_faults,
            prev_undetected_before_sat,
            (sat_detected_faults as f64/prev_undetected_before_sat as f64)*100.00
        );
    } else {
        println!(" -> faults covered (with SAT solving):                        {}/{} ({:.2}%)",
            sat_detected_faults,
            n_faults,
            (sat_detected_faults as f64/n_faults as f64)*100.00
        );
    }

    println!(" -> total faults covered:                                   {}/{} ({:.2}%)", 
        ppsfp_detected_faults + sat_detected_faults,
        n_faults,
        ((ppsfp_detected_faults + sat_detected_faults) as f64/n_faults as f64)*100.00
    );

    // salva i pattern generati in un file di testo (nella stessa cartella del benchmark, con nome derivato dal benchmark)
    let _ = save_patterns_to_test_inputs(&patterns, &undetected_faults, &filename, Some(SAVE_UNDETECTED_TO_FILE));
}

