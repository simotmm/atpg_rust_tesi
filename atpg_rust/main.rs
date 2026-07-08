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
use sat::{DONT_CARE_VALUE};
mod minimal_input_set;
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
const PRINT:              bool = false;
pub const PRINT_PROGRESS: bool = true; // se true, stampa messaggi di progresso durante le fasi più lunghe (es: generazione pattern, simulazione, risoluzione SAT)
//const MODE:      Option<&str> = Some("alg"); // "rand", "alg", "both" //override per debug
const MODE:    Option<&str> = Some("both"); // "rand", "alg", "both" //override per debug
//const MINIMIZER_BACKEND: Option<&str> = Some("espresso"); // default minimizer ("builtin"|"espresso") | None
const MINIMIZER_BACKEND: Option<&str> = None;
const SAT_BACKEND_DEFAULT: Option<&str> = Some("varisat"); // default SAT solver ("builtin"|"varisat")
pub const SAVE_UNDETECTED_TO_FILE: bool = true; // se true, salva la lista dei fault non rilevati in un file di testo (nella stessa cartella del benchmark, con nome derivato dal benchmark)
pub const RESULTS_DIR: &str = "results";
pub const MINIMAL_INPUTS_SET_DIR: &str = "results/minimal_input_sets";
const ATALANTA_RESULTS_DIR: &str = "results/atalanta_results";
pub const ATALANTA_FAULT_ENUMERATION: bool = false; // if true, enumerate faults like Atalanta
pub fn get_default_name() -> String { format!("{NET_DIR}\\{NET_PREFIX_DEFAULT}{NET_N_DEFAULT}{NET_SUFFIX_DEFAULT}.{FILE_FORMAT}") }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunMode {
    RandomOnly,
    AlgorithmicOnly,
    Both,
}

/// Entry point of the ATPG tool. Handles CLI arguments, orchestrates the workflow of loading netlists, generating patterns, simulating faults, and reporting results. Also includes special CLI commands for benchmarking and data processing.
fn main() {
    // handle special CLI commands (bench, compare, snapshot_iscas89, unroll_iscas89, minimal)
    if cli_helpers() { return; }

    main_workflow();
    //gustavo_side_quest();
}

/// Main workflow function, separated from `main` to allow for early returns on special CLI commands without executing the full pattern generation flow.
fn main_workflow() {

    // parse CLI args unless preconfigured
    let mut preconfigured = false;
    let (n, fault_wire, sa1, seed, mode) = configuration(preconfigured);

    // determine phases
    let run_random_phase = matches!(mode, RunMode::Both | RunMode::RandomOnly);
    let run_algorithmic_phase = matches!(mode, RunMode::Both | RunMode::AlgorithmicOnly);

    // -- parte I: wirelist translator -- //
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
    //let results_dir_str = "results/new_results/prova"; let results_dir = Some(results_dir_str);
    //let _ = save_patterns_to_test_inputs(&patterns, &undetected_faults, &filename, results_dir, Some(SAVE_UNDETECTED_TO_FILE));
    let _ = save_patterns_to_test_inputs(&patterns, &undetected_faults, &filename, None, Some(SAVE_UNDETECTED_TO_FILE), None);
}

/// Parses CLI arguments and returns a tuple of configuration parameters for the ATPG workflow. If `preconfigured` is true, it returns default or hardcoded values without parsing CLI arguments, allowing for programmatic configuration (e.g., for benchmarks or tests).
fn configuration(preconfigured: bool) -> (i32, Option<String>, Option<bool>, u64, RunMode) {
    let mut n: i32 = 0;
    let mut fault_wire: Option<String> = None;
    let mut sa1: Option<bool> = None;
    let mut seed: u64 = DEFAULT_SEED as u64;
    let mut mode: RunMode = RunMode::Both;
    let mut enable_sat: bool = true;
    let mut optimize_flag: bool = MINIMIZER_BACKEND.is_some();
    let mut minimizer: Option<String> = MINIMIZER_BACKEND.map(|s| s.to_string());
    let mut sat_backend_choice: String = SAT_BACKEND_DEFAULT.unwrap_or("builtin").to_string();
    let mut atalanta_compat: bool = false;
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

    return (n, fault_wire, sa1, seed, mode);
}

/// CLI helper function to compare the undetectable faults found by this tool with those reported by Atalanta for the same benchmarks. It reads the results from both tools, computes the intersection and differences, and prints a summary of the comparison, including counts and percentages of matching and mismatching faults.
fn compare(){
    util::compare_with_atalanta_undetectable_faults(RESULTS_DIR, ATALANTA_RESULTS_DIR);
}

/// CLI helper function to convert all ISCAS89 Verilog files in a specified input directory to combinational snapshots in an output directory, using the `snapshot_iscas89_folder` utility function. This is useful for preparing the benchmarks in the format expected by this tool. The function handles errors gracefully and prints any issues encountered during the conversion process.
fn snapshot_iscas89() {
    let input_dir = "benchmarks/iscas89";
    let output_dir = "benchmarks/converted_to_atlanta_iscas89";
    if let Err(e) = snapshot_iscas89_folder(input_dir, output_dir) {
        eprintln!("Error during snapshot conversion: {}", e);
        std::process::exit(1);
    }
}

/// CLI helper function to unroll all ISCAS89 Verilog files in a specified input directory for k frames and save the unrolled versions in an output directory, using the `unroll_iscas89_folder` utility function. The number of frames k is taken from the CLI arguments (defaulting to 1 if not provided). This is useful for preparing sequential benchmarks in an unrolled combinational format. The function handles errors gracefully and prints any issues encountered during the conversion process.
fn unroll_iscas89() {
    let k = get_nth_arg_to_int(2).unwrap_or(1) as usize;
    let input_dir = "benchmarks/iscas89";
    let output_dir = "benchmarks/converted_to_atlanta_iscas89";
    if let Err(e) = unroll_iscas89_folder(input_dir, output_dir, k) {
        eprintln!("Error during unrolling conversion: {}", e);
        std::process::exit(1);
    }
}

/// CLI helper function to run benchmarks. It supports a special "full" mode that allows for more detailed control over the benchmarking process, including selecting specific benchmarks, configuring runtime options, and controlling the output. If the "ppsfp" argument is provided, it runs only the micro-benchmark for the PPSFP simulator with minimal output. Otherwise, it runs the full suite of benchmarks using the `run_benchmarks` function.
fn bench() {
    // Make subcommand matching case-insensitive by lowercasing the 2nd arg
    match get_nth_arg_to_string(2).map(|s| s.to_lowercase()).as_deref() {
        Some("ppsfp") | Some("pp") | Some("p") => {
            // run micro-benchmark with minimal stdout
            crate::options::set_options(crate::options::RunOptions { enable_sat: true, optimize: MINIMIZER_BACKEND.is_some(), minimizer: MINIMIZER_BACKEND.map(|s| s.to_string()), sat_backend: SAT_BACKEND_DEFAULT.unwrap_or("varisat").to_string(), quiet: true });
            crate::benchmarks::run_ppsfp_benchmark();
            return;
        }
        Some("full") | Some("f") => {
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
            crate::options::set_options(crate::options::RunOptions { enable_sat, optimize: parallel, minimizer: minimizer.clone(), sat_backend: sat_backend_choice.clone(), quiet: true });

            let prefixes = [NET_PREFIX_DEFAULT, NET_PREFIX_DEFAULT_2];
            crate::benchmarks::run_full_benchmarks_for_prefixes(&prefixes, specific, Some(seed_val), effective_mode);
            return;
        }
        _ => {
            // set quiet for generic bench runner
            crate::options::set_options(crate::options::RunOptions { enable_sat: true, optimize: MINIMIZER_BACKEND.is_some(), minimizer: MINIMIZER_BACKEND.map(|s| s.to_string()), sat_backend: SAT_BACKEND_DEFAULT.unwrap_or("varisat").to_string(), quiet: true });
            run_benchmarks();
            return;
        }
    }
}

/// CLI helper function to compute the minimal (exact) cover of input patterns that detect all faults for a specific circuit.
fn minimal_combination(n_patterns: Option<usize>) {

    let n = get_nth_arg_to_int(2).unwrap_or(NET_N_DEFAULT);
    let filename: String = get_filename_from_int(NET_DIR, n).unwrap_or(get_default_name());
    let circuit: Netlist = file_iscas89_atlanta_to_netlist(&filename);
    let mut dag = Dag::from_netlist(&circuit);
    let faults: Vec<Fault> = dag.generate_fault_list(None, None);
    let mut undetected_faults: BTreeSet<Fault> = faults.iter().cloned().collect();
    let mut simulator = PPSFPSimulator::new(&dag);
    let run_random_phase = true; // se true, esegue prima la fase randomica per ridurre il numero di fault da coprire con la parte algoritmica (SAT + minimizzazione), altrimenti va direttamente alla parte algoritmica su tutti i fault

    // fase opzionale: generazione di un certo numero di pattern random per ridurre il numero di fault da coprire con l'approccio algoritmico 
    //(SOLO PER CIRCUITI GRANDI, per quelli piccoli se rand è attivo e trova tutti fault non si genera alcun pattern per la fase algoritmica,
    //quindi il minimal set sarà vuoto) -> in pratica voglio fare minimal set solo sui fault "difficili" da rilevare.
    let mut ppsfp_patterns: Vec<InputPattern> = Vec::new();
    if run_random_phase {
        println!("running random pattern generation phase before minimal cover computation");
        let mut ppsfp_results: Vec<HashMap<Fault, u32>> = Vec::new();
        let mut ppsfp_undetected_faults: BTreeSet<Fault> = BTreeSet::new();
        let n_patterns = n_patterns.unwrap_or_else(|| {
            let n_faults = faults.len();
            if n_faults == 0 { 2usize} 
            else { 
                let s = (n_faults as f64).sqrt().ceil() as usize;
                std::cmp::max(2, std::cmp::min(s, MAX_PATTERNS)) 
            }
        });
        println!("pattern generation ({n_patterns} patterns):");
        let mut pattern_generator = PatternGenerator::new(Some(DEFAULT_SEED as u64));
        ppsfp_patterns = pattern_generator.generate_n_patterns_from_netlist(&circuit, n_patterns);
        println!("parallel pattern single fault propagation (PPSFP):");
        (ppsfp_results, ppsfp_undetected_faults) = simulator.simulate_patterns_blocks(ppsfp_patterns.clone(), faults.clone(), Some(PARALLEL));
        undetected_faults = ppsfp_undetected_faults.clone();
    }
    
    // fase principale: calcolo del minimal cover (exact) dei pattern che rilevano tutti i fault rimanenti, usando l'approccio basato su SAT e minimizzazione degli insiemi
    let prev_undetected = undetected_faults.len();
    if prev_undetected == 0 { println!("all faults detected by random patterns, minimal input set will be computed on all faults for diagnostic purposes"); }
    else { println!("undetected faults before minimal cover computation: {}", prev_undetected); }
    // clone the set of undetected faults before passing ownership to the minimizer
    let undetected_before_minimizer = if prev_undetected > 0 {
         undetected_faults.clone() 
        } else {
             faults.iter().cloned().collect()
    };
    // TODO: compute_minimal_input_set deve generare un simulatore temporaneo per testare la copertura di ogni pattern durante
    // la minimizzazione e restituire anche la lista dei fault non rilevati da nessun pattern. idealmente undetectable.
    // riflessione:
    // - nuovo workflow potrebbe essere:
    //   fare sat solving per vedere quali sono i fault testabili e fare minimizzazione solo su quelli
    //   (per questo va integrata idealmente, per circuiti grandi, la parte randomica)
    // ricapitolando:
    //  - generazione fault list
    //  - se il circuito è "grande", parte pseudorandomica per ridurre il numero di fault da coprire con sat
    //    - (definizione di "grande": se la parte pseudorandomica non copre tutti i fault)
    //  - fare sat solving per ogni fault per vedere quali sono testabili, scartare gli untestable
    //  - fare minimizzazione solo sui fault testabili, per trovare il set minimo di pattern che li copre tutti
    //    - approccio: 
    //      - set cover esatto? -> esplosione combinatioria...
    //      - greedy set cover? -> non garantisce non garantisce soluzione ottima....
    //        -> in vista degli stati quantum: meglio set cover esatto tanto è magico, qui lo testo solo sui circuiti piccoli
    //                                         senza fare generazione pseudorandomica. al quantum diamo in pasto i circuiti grandi
    //                                         con fault set ridotto dalla parte pseudorandomica
    // sono stanco capo ho fame sono le 15..................
    // UPDATE: riflessione 2
    //         forse è meglio usare sat solo per vedere gli undetectable 
    //         e fare approccio bruteforce generando tutte le combinazioni di pattern e simulando per vedere quale copre più fault, 
    //fino a trovare il set minimo che copre tutti i fault testabili.
    // pass random patterns (if any) to the minimizer so they can be considered as candidate patterns
    let rand_patterns = if run_random_phase { Some(ppsfp_patterns.clone()) } else { None };
    let (minimal_input_set_with_sets, final_undetected_faults) = minimal_input_set::compute_minimal_input_set(dag.clone(), undetected_before_minimizer.clone(), rand_patterns);

    // salvataggio dei pattern generati in un file di testo 
    // (nella stessa cartella del benchmark, con nome derivato dal benchmark e suffisso "_minimal_input_set.txt")
    let minimal_input_set: Vec<InputPattern> = minimal_input_set_with_sets.iter().map(|(p, pos, _)| {
        let mut row = InputPattern::from_strings(circuit.inputs.clone());
        for input in &circuit.inputs {
            let bit = p.get(input).unwrap_or(0u32) >> pos;
            row.set(input, bit & 1u32);
        }
        row
    }).collect();
    let r_dir = Some(MINIMAL_INPUTS_SET_DIR);
    // Save only the tested bit-position (LSB, pos 0) rows for the minimal input set
    let _ = save_patterns_to_test_inputs(&minimal_input_set, &final_undetected_faults, &filename, r_dir, None, Some(0));
    // stampa dei risultati
    println!("\nMinimal input set results for {}:", filename);
    println!("  total faults: {}", faults.len());
    println!("  undetected before minimal cover: {}", prev_undetected);
    println!("  patterns in minimal set: {}", minimal_input_set.len());
    println!("  undetected after minimal cover: {}", final_undetected_faults.len());
    // The per-pattern single-bit simulation is performed inside `compute_minimal_input_set`.
    // Report detected count based on undetected faults before/after minimization.
    let detected_by_minimal = undetected_before_minimizer.len().saturating_sub(final_undetected_faults.len());
    println!("  faults detected by minimal cover: {}", detected_by_minimal);

    if !minimal_input_set_with_sets.is_empty() {
        println!("\nPatterns in minimal set (packed pattern + bit position -> detected faults):");
        let mut idx = 1;
        for (p, pos, fset) in &minimal_input_set_with_sets {
            println!("set {} (pos {}):", idx, pos);
            p.print();
            // Print the compact input->bit mapping for the selected bit position.
            let bits = p.get_input_pattern_from_position(*pos);
            let mut keys: Vec<String> = p.values.keys().cloned().collect();
            keys.sort();
            let mapping: Vec<String> = keys.iter().zip(bits.iter()).map(|(k, b)| format!("{}:{}", k, b)).collect();
            println!("  Tested combination (pos {}): {}", pos, mapping.join(", "));
            // Print faults detected by this pattern
            if fset.is_empty() {
                println!("  -> detects: (none)");
            } else {
                println!("  -> detects:");
                for f in fset.iter() {
                    println!("       - {}", f.to_string());
                }
            }
            idx += 1;
        }
    } else {
        println!("No patterns found for minimal cover.");
    }

    if !final_undetected_faults.is_empty() {
        println!("\nUndetected faults after minimal cover ({}):", final_undetected_faults.len());
        let mut i = 1;
        for f in final_undetected_faults.iter() {
            println!("  {}: {}", i, f.to_string());
            i += 1;
        }
    } else {
        println!("\nAll faults covered by minimal input set.");
    }

    if !minimal_input_set.is_empty() {
        println!("\nMinimal input set (canonical input order, one row per selected position):");
        let mut compact_rows: Vec<String> = Vec::with_capacity(minimal_input_set.len());
        for (idx, pat) in minimal_input_set.iter().enumerate() {
            let mut row = String::with_capacity(circuit.inputs.len());
            for input in &circuit.inputs {
                let bit = pat.get(input).unwrap_or(0u32) & 1u32;
                row.push(if bit == 1 { '1' } else { '0' });
            }
            compact_rows.push(row.clone());
            println!("  {}: {}", idx + 1, row);
        }
        println!("\nminimal input set = {{{}}}", compact_rows.join(", "));
    }

    return;
}

/// CLI helper function that mirrors the main workflow, but keeps the fault sets split into:
/// easy_detectable_faults (found by random phase), hard_detectable_faults (found only by SAT),
/// and undetectable_faults (still uncovered after SAT). The exact minimal cover is then computed
/// only for the hard-detectable faults.
fn minimal_combination_full_workflow(n_patterns: Option<usize>) {

    let n = get_nth_arg_to_int(2).unwrap_or(NET_N_DEFAULT);
    let filename: String = get_filename_from_int(NET_DIR, n).unwrap_or(get_default_name());
    let circuit: Netlist = file_iscas89_atlanta_to_netlist(&filename);
    let dag = Dag::from_netlist(&circuit);
    let faults: Vec<Fault> = dag.generate_fault_list(None, None);
    let n_faults = faults.len();
    let mut simulator = PPSFPSimulator::new(&dag);

    let mut ppsfp_patterns: Vec<InputPattern> = Vec::new();
    let mut sat_patterns: Vec<InputPattern> = Vec::new();
    let mut easy_detectable_faults: BTreeSet<Fault> = BTreeSet::new();
    let mut hard_detectable_faults: BTreeSet<Fault> = BTreeSet::new();
    let mut undetectable_faults: BTreeSet<Fault> = faults.iter().cloned().collect();

    println!("running random pattern generation phase before full minimal cover computation");
    let mut ppsfp_results: Vec<HashMap<Fault, u32>> = Vec::new();
    let n_patterns = n_patterns.unwrap_or_else(|| {
        if n_faults == 0 { 2usize }
        else {
            let s = (n_faults as f64).sqrt().ceil() as usize;
            std::cmp::max(2, std::cmp::min(s, MAX_PATTERNS))
        }
    });
    println!("pattern generation ({n_patterns} patterns):");
    let mut pattern_generator = PatternGenerator::new(Some(DEFAULT_SEED as u64));
    ppsfp_patterns = pattern_generator.generate_n_patterns_from_netlist(&circuit, n_patterns);
    println!("parallel pattern single fault propagation (PPSFP):");
    let (ppsfp_sim_result, ppsfp_undetected_faults) = simulator.simulate_patterns_blocks(ppsfp_patterns.clone(), faults.clone(), Some(PARALLEL));
    ppsfp_results = ppsfp_sim_result;
    easy_detectable_faults = faults.iter().cloned().filter(|f| !ppsfp_undetected_faults.contains(f)).collect();
    undetectable_faults = ppsfp_undetected_faults.clone();

    println!("\nFault split after random phase:");
    println!("  easy detectable faults: {}", easy_detectable_faults.len());
    println!("  faults still not detected after random phase: {}", undetectable_faults.len());

    let mut sat_results: HashMap<Fault, Vec<(String, u32)>> = HashMap::new();
    if !undetectable_faults.is_empty() {
        print!("\nalgorithmic pattern generation for remaining faults:\n");
        let opts = crate::options::get_options();
        println!("Using SAT backend: {}", opts.sat_backend);
        sat_results = crate::sat::sat_solving(&circuit, &dag, undetectable_faults.iter().collect(), Some(PARALLEL));

        println!("\nsimulation of SAT patterns for remaining faults:");
        println!("parallel pattern single fault propagation (PPSFP):");
        sat_patterns = sat_solutions_to_input_patterns(circuit.clone(), sat_results.clone());
        let (sat_sim_result, sat_undetected_faults) = simulator.simulate_patterns_blocks(sat_patterns.clone(), undetectable_faults.iter().cloned().collect(), Some(PARALLEL));
        println!("undetected faults after SAT patterns: {} ", sat_undetected_faults.len());

        hard_detectable_faults = undetectable_faults.difference(&sat_undetected_faults).cloned().collect();
        undetectable_faults = sat_undetected_faults.clone();

        print!("{}\n", if undetectable_faults.is_empty() {
            "  -> all remaining faults detected by SAT patterns".to_string()
        } else {
            let mut undetected_str = String::from("  -> undetected faults after SAT:\n");
            for fault in &undetectable_faults {
                undetected_str.push_str(&format!("       fault on wire {} s-a-{}\n", &fault.wire, if fault.sa1 {1} else {0}));
            }
            undetected_str
        });
    }

    println!("\nFault split summary:");
    println!("  total faults: {}", n_faults);
    println!("  easy_detectable_faults: {}", easy_detectable_faults.len());
    println!("  hard_detectable_faults: {}", hard_detectable_faults.len());
    println!("  undetectable_faults: {}", undetectable_faults.len());

    if !easy_detectable_faults.is_empty() {
        println!("\nEasy detectable faults:");
        for (idx, fault) in easy_detectable_faults.iter().enumerate() {
            println!("  {}: {}", idx + 1, fault.to_string());
        }
    }
    if !hard_detectable_faults.is_empty() {
        println!("\nHard detectable faults:");
        for (idx, fault) in hard_detectable_faults.iter().enumerate() {
            println!("  {}: {}", idx + 1, fault.to_string());
        }
    }
    if !undetectable_faults.is_empty() {
        println!("\nUndetectable faults after SAT:");
        for (idx, fault) in undetectable_faults.iter().enumerate() {
            println!("  {}: {}", idx + 1, fault.to_string());
        }
    }

    let minimal_input_set_input_faults = if !hard_detectable_faults.is_empty() {
        hard_detectable_faults.clone()
    } else {
        easy_detectable_faults.clone()
    };
    let mut candidate_patterns_for_minimizer: Vec<InputPattern> = Vec::new();
    candidate_patterns_for_minimizer.extend(ppsfp_patterns.clone());
    candidate_patterns_for_minimizer.extend(sat_patterns.clone());
    let rand_patterns_for_minimizer = if candidate_patterns_for_minimizer.is_empty() {
        None
    } else {
        Some(candidate_patterns_for_minimizer)
    };
    let (minimal_input_set_with_sets, remaining_selected_faults) = minimal_input_set::compute_minimal_input_set(dag.clone(), minimal_input_set_input_faults, rand_patterns_for_minimizer);

    let minimal_input_set: Vec<InputPattern> = minimal_input_set_with_sets.iter().map(|(p, pos, _)| {
        let mut row = InputPattern::from_strings(circuit.inputs.clone());
        for input in &circuit.inputs {
            let bit = p.get(input).unwrap_or(0u32) >> pos;
            row.set(input, bit & 1u32);
        }
        row
    }).collect();

    let r_dir = Some(MINIMAL_INPUTS_SET_DIR);
    let _ = save_patterns_to_test_inputs(&minimal_input_set, &remaining_selected_faults, &filename, r_dir, None, Some(0));

    println!("\nHard-detectable minimal cover results for {}:", filename);
    println!("  easy_detectable_faults: {}", easy_detectable_faults.len());
    println!("  hard_detectable_faults: {}", hard_detectable_faults.len());
    println!("  undetectable_faults: {}", undetectable_faults.len());
    println!("  patterns in minimal set for selected faults: {}", minimal_input_set.len());
    println!("  selected faults still uncovered after minimal cover: {}", remaining_selected_faults.len());

    if !minimal_input_set_with_sets.is_empty() {
        println!("\nPatterns in minimal set (assignments -> detected faults):");
        let mut compact_rows: Vec<String> = Vec::with_capacity(minimal_input_set.len());
        for (idx, (pat, pos, fset)) in minimal_input_set_with_sets.iter().enumerate() {
            let mut row = String::with_capacity(circuit.inputs.len());
            for input in &circuit.inputs {
                let bit = pat.get(input).unwrap_or(0u32) >> pos;
                row.push(if (bit & 1u32) == 1 { '1' } else { '0' });
            }
            compact_rows.push(row.clone());
            println!("set {}:", idx + 1);
            pat.print();
            let bits = pat.get_input_pattern_from_position(*pos);
            let mut keys: Vec<String> = pat.values.keys().cloned().collect();
            keys.sort();
            let mapping: Vec<String> = keys.iter().zip(bits.iter()).map(|(k, b)| format!("{}:{}", k, b)).collect();
            println!("  Tested combination (pos {}): {}", pos, mapping.join(", "));
            if fset.is_empty() {
                println!("  -> detects: (none)");
            } else {
                println!("  -> detects:");
                for f in fset.iter() {
                    println!("       - {}", f.to_string());
                }
            }
        }
        println!("\nminimal input set = {{{}}}", compact_rows.join(", "));
    } else {
        println!("\nNo minimal cover patterns found for the selected faults.");
    }

    if !remaining_selected_faults.is_empty() {
        println!("\nSelected faults still undetected after minimal cover ({}):", remaining_selected_faults.len());
        for (idx, fault) in remaining_selected_faults.iter().enumerate() {
            println!("  {}: {}", idx + 1, fault.to_string());
        }
    }
}

/// CLI helper function to generate the complete fault list for a given circuit and save it to a file. This is useful for diagnostic purposes, allowing users to see all the faults that were considered in the ATPG process. The function loads the netlist, generates the fault list using the DAG representation, saves it to a file, and prints it to the console.
fn fault_list() {
    let n = get_nth_arg_to_int(2).unwrap_or(NET_N_DEFAULT);
    let filename: String = get_filename_from_int(NET_DIR, n).unwrap_or(get_default_name());
    let netlist: Netlist = file_iscas89_atlanta_to_netlist(&filename);
    let dag = Dag::from_netlist(&netlist);
    let faults: Vec<Fault> = dag.generate_fault_list(None, None);
    let _ = crate::util::save_fault_list(&faults, &filename, None);
    print_fault_list(&faults);
}

/// Checks for special CLI commands and executes the corresponding helper functions. Returns true if a special command was detected and handled, false otherwise. This allows the main workflow to be bypassed for commands that are meant for data processing, benchmarking, or other utilities that don't require the full ATPG flow.
fn cli_helpers() -> bool {
    // Make CLI helper matching case-insensitive by lowercasing the first arg
    match get_nth_arg_to_string(1).map(|s| s.to_lowercase()).as_deref() {
        // CLI helper: compare results with Atalanta's undetectable faults
        Some("compare") | Some("comp") | Some("c") => { compare(); return true; },
        // CLI helper: convert all ISCAS89 Verilog files to combinational snapshot
        Some("snapshot_iscas89") | Some("snap") | Some("s") => { snapshot_iscas89(); return true; },
        // CLI helper: unroll ISCAS89 Verilog files for k frames
        Some("unroll_iscas89") | Some("unroll") | Some("u") => { unroll_iscas89(); return true; },
        // CLI helper: run benchmarks (with optional "full" mode for more detailed control and output)
        Some("bench") | Some("benchmark") | Some("b") => { bench(); return true; },
        // CLI: compute minimal (exact) cover for a specific circuit: `minimal <n>`
        Some("minimal") | Some("min") | Some("m") => { 
            let run_rand = if { get_nth_arg_to_string(3).map(|s| s.to_lowercase()).as_deref() == Some("rand") } { true } else { false };
            let n_patterns = get_nth_arg_to_int(4).map(|v| v as usize);
            minimal_combination(n_patterns);
            return true; 
        },
        // CLI: full main-workflow-style minimal cover with easy/hard/undetectable split
        Some("minfull") | Some("minwf") | Some("mf") => {
            let n_patterns = get_nth_arg_to_int(3).map(|v| v as usize);
            minimal_combination_full_workflow(n_patterns);
            return true;
        },
        Some("fault_list") | Some("fl") | Some("f") => { fault_list(); return true; },
        _ => {}
    }

    return false;
}

/// Special "side quest" function for Gustavo: generate all input combinations, analyze transitions, and print detailed mapping of input bits to transitions.
/// (devo rivederla per bene: deve restituire due input pattern che massimizzano le transizioni, e poi stampare la mappatura nome_input->bit per entrambi i pattern, in modo da capire quali input cambiano tra i due pattern e causano le transizioni)
fn gustavo_side_quest() {

    // `PatternGenerator::generate_all_input_combinations`.
    let n = get_nth_arg_to_int(1).unwrap_or(NET_N_DEFAULT);
    let filename: String = get_filename_from_int(NET_DIR, n).unwrap_or(get_default_name());
    println!("Loading netlist from: {}", filename);

    let circuit: Netlist = file_iscas89_atlanta_to_netlist(&filename);
    println!("Circuit inputs ({}): {:?}", circuit.inputs.len(), circuit.inputs);

    let pg = PatternGenerator::new(None);
    let combos = pg.generate_all_input_combinations(&circuit, Some(true));
    println!("Generated {} combinations", combos.len());

    let max_show = 16usize;
    for (i, p) in combos.iter().enumerate() {
        println!("Block {} (packed 32 combos):\n{}", i, p.to_string());
    }

    // Create DAG and simulator to analyze transitions
    let dag = Dag::from_netlist(&circuit);
    let mut sim = PPSFPSimulator::new(&dag);

    // For each packed block, find the local bit-position with most transitions
    // and report the global combination index (block*32 + pos)
    let mut best_count = 0;
    let mut best_pos = 0;
    for (block_idx, pat) in combos.iter().enumerate() {
        let ((p1, p2), count) = sim.most_transitions_for_compact_pattern(pat);
        if count == 0 {
            println!("Block {}: no transitions detected", block_idx);
        } else {
            let global_idx1 = block_idx * 32 + p1;
            let global_idx2 = block_idx * 32 + p2;
            println!("Block {}: best transition between local pos {} and {} -> global combinations {} and {} with {} changes",
                block_idx, p1, p2, global_idx1, global_idx2, count);

            // extract input vector (bits) at the chosen positions and print name->bit mapping
            let bits1 = pat.get_input_pattern_from_position(p1);
            let bits2 = pat.get_input_pattern_from_position(p2);
            println!("  Input bits at local pos {}: {:?}", p1, bits1);
            println!("  Input bits at local pos {}: {:?}", p2, bits2);

            let mut keys: Vec<String> = pat.values.keys().cloned().collect();
            keys.sort();
            let mapping1: Vec<String> = keys.iter().zip(bits1.iter()).map(|(k, b)| format!("{}:{}", k, b)).collect();
            let mapping2: Vec<String> = keys.iter().zip(bits2.iter()).map(|(k, b)| format!("{}:{}", k, b)).collect();
            println!("  Input vector 1: {}", mapping1.join(", "));
            println!("  Input vector 2: {}", mapping2.join(", "));


        }
    }
  
}
