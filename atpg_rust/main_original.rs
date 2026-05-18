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
use util::{min, max, print_fault_list, print_pattern_list, print_simulation_results, get_nth_arg_to_int, get_nth_arg_to_string, get_nth_arg_to_bool, get_filename_from_int, print_sat_solution, sat_solutions_to_input_patterns, press_any_key, save_patterns_to_file};

mod dag;
use dag::{Dag};

mod benchmarks;
use crate::benchmarks::run_benchmarks;

pub mod cnf;

mod sat;
mod two_sat;

use std::{collections::{HashMap, HashSet}};

use crate::{dag::build_test_formula_optimized, sat::{sat_solving_parallel, sat_solving_sequential}};

//use crate::util::process_iscas85_to89atlanta_files_in_folder;

const NET_DIR:             &str = r"benchmarks\converted_to_atlanta_iscas89"; // r"\" = "\\"
//const NET_DIR:            &str = r"benchmarks\atlanta_iscas89"; 
const NET_PREFIX_DEFAULT:  &str = "c";
const NET_N_DEFAULT:        i32 = 2; // n. del file benchmark
const NET_SUFFIX_DEFAULT : &str = "_to_iscas89_atlanta";
const FILE_FORMAT:         &str = "bench";
const RANDOM_PATTERNS:    bool = true;
const N_PATTERNS:        usize = 1;
const MANUAL_PATTERN:     &str = "000";
const DEFAULT_SEED:         i32 = 762000;
const PARALLEL:            bool = false; // temporarily disable parallelism for determinism checks
const PRINT:              bool = false;
const ALWAYS_SAT:         bool = true;
const SAT_FOR_ALL_FAULTS: bool = false;
fn get_default_name() -> String { format!("{NET_DIR}\\{NET_PREFIX_DEFAULT}{NET_N_DEFAULT}{NET_SUFFIX_DEFAULT}.{FILE_FORMAT}") }

fn main() {
}

fn main_a_steps() {
    println!("--- automatic test pattern generation and fault simulation for combinational circuits ---\n");
    // se il primo argomento è "bench" esegui la suite di benchmark e termina
    if get_nth_arg_to_string(1).as_deref() == Some("bench") {
        run_benchmarks();
        return;
    }

    //init_data();

    //if RANDOM_PATTERNS { pattern_generation(); } else { pattern_insertion(); }
    
    //fault_simulation();

    //cnf_generation();

    //sat_solving();

    simulate_sat_solution();

    //main0();
}

fn init_data() -> (Netlist, Dag, Vec<Fault>) {
    let netlist_number: i32 = get_nth_arg_to_int(1).unwrap_or(NET_N_DEFAULT);
    let fault_wire: Option<String> = get_nth_arg_to_string(2);
    let sa1: Option<bool> = get_nth_arg_to_bool(3);
    let seed: u64 = get_nth_arg_to_int(4).unwrap_or(DEFAULT_SEED) as u64;

    let filename: String = get_filename_from_int(NET_DIR, netlist_number).unwrap_or(get_default_name());
    println!("filename: {filename}");

    let circuit: Netlist = file_iscas89_atlanta_to_netlist(&filename);
    circuit.print();

    let dag: Dag = Dag::from_netlist(&circuit);
    dag.print();

    let faults: Vec<Fault> = dag.generate_fault_list(fault_wire, sa1);
    print_fault_list(&faults);

    (circuit, dag, faults)
}

fn pattern_insertion() -> (Netlist, Dag, Vec<Fault>, Vec<InputPattern>) {
    let (circuit, dag, faults) = init_data();
    println!("-- manual pattern insertion --");
    let inputs: Vec<&str> = vec![ // per il circuito 2 (3 input)
        "000",
        "001",
        "010",
        "100",
        "011",
        "110",
        "101",
        "111",
    ];

    let mut pattern = InputPattern::init_from_netlist(circuit.clone());
    pattern.insert_patterns(inputs);

    let patterns = vec![pattern];
    print_pattern_list(patterns.clone());

    (circuit, dag, faults, patterns)
}


fn simulate_sat_solution() {
    let (circuit, dag, faults, patterns, sat_result) = sat_solving();
    print!("-- SAT solving result simulation --\n");

    if sat_result.is_empty() && !ALWAYS_SAT {
        println!("No SAT patterns generated; all faults already covered by PPSFP.");
        println!("-- Simulating PPSFP patterns that cover all faults --");

        let sat_patterns = patterns.clone();
        print_pattern_list(sat_patterns.clone());
        let mut simulator = PPSFPSimulator::new(&dag);
        let (results, undetected_faults ) = simulator.simulate_patterns_blocks(sat_patterns.clone(), faults.clone(), Some(PARALLEL));
        // salva su file le pattern e, se presenti, gli uncovered
        let _ = save_patterns_to_file(&sat_patterns.clone(), &undetected_faults, "test_inputs.txt");
        print_simulation_results(results, faults.clone(), undetected_faults, sat_patterns, false);
        return;
    }

    let sat_patterns = sat_solutions_to_input_patterns(circuit, sat_result);
    print_pattern_list(sat_patterns.clone());

    let mut simulator = PPSFPSimulator::new(&dag);

    let (results, undetected_faults ) = simulator.simulate_patterns_blocks(sat_patterns.clone(), faults.clone(), Some(PARALLEL));
    // salva su file le pattern e, se presenti, gli uncovered
    let _ = save_patterns_to_file(&sat_patterns.clone(), &undetected_faults, "test_inputs.txt");
    print_simulation_results(results, faults.clone(), undetected_faults, sat_patterns, false);
}


fn sat_solving() -> (Netlist, Dag, Vec<Fault>, Vec<InputPattern>, HashMap<Fault, Vec<(String, u32)>>) {
    let (circuit, dag, faults, undetected_faults, patterns) = cnf_generation();
    print!("-- SAT solving --\n");
    // If no undetected faults and ALWAYS_SAT is false, skip SAT.
    if undetected_faults.len() == 0 && !ALWAYS_SAT {
        println!("All faults detected by PPSFP; skipping SAT.");
        return (circuit, dag, faults, patterns, HashMap::new());
    }

    // Determine SAT input: if ALWAYS_SAT is true, run SAT on all faults; otherwise only on undetected ones.
    let sat_input: Vec<&Fault> = if ALWAYS_SAT {
        faults.iter().collect()
    } else {
        undetected_faults.iter().collect()
    };
    let sat_result: HashMap<Fault, Vec<(String, u32)>> = sat_solving_sequential(&circuit, &dag, sat_input);

    print_sat_solution(&sat_result);

    (circuit, dag, faults, patterns, sat_result)
}


fn cnf_generation() -> (Netlist, Dag, Vec<Fault>, Vec<Fault>, Vec<InputPattern>) {
    let (circuit, dag, faults, undetected_faults, patterns) = fault_simulation();
    print!("\n-- cnf formulas generation (conjunctive normal form) --\n");

    for fault in faults.clone() {
        let mut faulty = dag.clone();
        faulty.inject_fault(fault.clone());
        let test_formula = build_test_formula_optimized(&dag, &faulty);
        // Minimizza la CNF usando il nuovo modulo cnf_minimizer (modifica in-place)
        let mut optimized_formula = test_formula.clone();
        match crate::cnf_minimizer::minimize_search_tree(&mut optimized_formula) {
            Ok(_) => { /* ottimizzazione applicata */ }
            Err(_) => { println!("Minimization detected conflict/UNSAT for this test formula"); }
        }
        fault.print();
        print!("good cnf (f(x)):        {}\n", dag.cnf_cone_from_outputs().to_string());
        print!("faulty cnf (f'(x)):     {}\n", faulty.cnf_cone_from_outputs().to_string());
        print!("XOR((f(x), (f'(x)) cnf: {}\n\n", test_formula.to_string());
        print!("optimized cnf: {}\n\n", optimized_formula.to_string());
    }

    (circuit, dag, faults, undetected_faults, patterns)
}


fn fault_simulation() -> (Netlist, Dag, Vec<Fault>, Vec<Fault>, Vec<InputPattern>) {
    let (mut circuit, dag, faults, patterns) = if RANDOM_PATTERNS { pattern_generation() } else { pattern_insertion() };

    println!("-- fault simulation --");

    let mut simulator = PPSFPSimulator::new(&dag);
    let (results, undetected_faults_bset ) = simulator.simulate_patterns_blocks(patterns.clone(), faults.clone(), Some(PARALLEL));
    // keep BTreeSet for printing, but return a Vec for later processing
    print_simulation_results(results, faults.clone(), undetected_faults_bset.clone(), patterns.clone(), false);
    let undetected_faults: Vec<Fault> = undetected_faults_bset.into_iter().collect();

    (circuit, dag, faults, undetected_faults, patterns)
}


fn pattern_generation() -> (Netlist, Dag, Vec<Fault>, Vec<InputPattern>) {
    let (circuit, dag, faults) = init_data();

    println!("-- pseudo-random pattern generation --\n");
    let mut pattern_generator = PatternGenerator::new(Some(DEFAULT_SEED as u64));
    let patterns: Vec<InputPattern> = pattern_generator.generate_n_patterns_from_netlist(&circuit, N_PATTERNS);
    print_pattern_list(patterns.clone());

    (circuit, dag, faults, patterns)
}


fn main0() {

    // -- settings: conversione file -- //
    //let _ = process_iscas85_to89atlanta_files_in_folder(r"benchmarks\iscas85");
    

    // -- parte I: wirelist translator -- //
    println!("part I: wirelist stranslator");

    //eventuale lettura da riga di comando
    let n: i32 = get_nth_arg_to_int(1).unwrap_or(NET_N_DEFAULT);
    let fault_wire: Option<String> = get_nth_arg_to_string(2);
    let sa1: Option<bool> = get_nth_arg_to_bool(3);
    let seed: u64 = get_nth_arg_to_int(4).unwrap_or(DEFAULT_SEED) as u64;

    //caricamento netlist da file
    let filename: String = get_filename_from_int(NET_DIR, n).unwrap_or(get_default_name());
    if !crate::options::get_options().quiet && PRINT { println!("filename: {filename}"); }
    let circuit: Netlist = file_iscas89_atlanta_to_netlist(&filename);

    // stampa netlist
    if !crate::options::get_options().quiet && PRINT { circuit.print(); }

    //press_any_key(); //pausa per debug


    // -- parte II: pseudo-random pattern generation, PPSFP (parallel pattern single fault propagation) -- //
    println!("part II: pseudo-random pattern generation and parallel pattern single fault propagation (PPSFP)");

    //conversione netlist -> dag
    let dag: Dag = Dag::from_netlist(&circuit.clone());

    //stampa dag
    if !crate::options::get_options().quiet && PRINT { dag.print(); }
    //press_any_key(); //pausa per debug

    //stampa cnf (conjunctive normal form, prodotto delle somme)
    if !crate::options::get_options().quiet && PRINT { println!("cnf: {}\n", dag.cnf_cone_from_outputs().to_string()); }
    //press_any_key(); //pausa per debug

    //generazione dei pattern
    println!("pattern generation:");
    //inizializzazione generatore con seed (default per debug: 762000)
    let mut pattern_generator = PatternGenerator::new(Some(seed));

    //inizializzazione simulatore con dag
    let mut simulator = PPSFPSimulator::new(&dag);

    //generazione dei pattern
    let patterns = if RANDOM_PATTERNS {
        pattern_generator.generate_n_patterns_from_netlist(&circuit, N_PATTERNS)
    } else {
        vec![InputPattern::from_netlist(circuit.clone(), MANUAL_PATTERN)] // inserimento manuale (per debug)
    };

    //stampa pattern
    if !crate::options::get_options().quiet && PRINT { for pattern in &patterns { pattern.print(); } }
    //press_any_key(); //pausa per debug

    //genera lista di fault
    //let mut faults = dag.generate_fault_list(fault_wire, sa1); // idea: per ogni iterazione del simulatore tolgo i fault coperti
    let faults: Vec<Fault> = dag.generate_fault_list(fault_wire, sa1);
    let tot_faults: usize = faults.len();

    //stampa lista dei fault
    if !crate::options::get_options().quiet && PRINT { print_fault_list(&faults); }
    //press_any_key(); //pausa per debug

    //PPSFP (parallel pattern single fault propagation)
    //simulazione dei pattern per tutti i fault
    println!("parallel pattern single fault propagation (PPSFP):");
    let (ppsfp_results, ppsfp_undetected_faults ) = simulator.simulate_patterns_blocks(patterns.clone(), faults.clone(), Some(PARALLEL));

    //stampa risultati
    let n_faults = faults.len();
    let n_ppsfp_undetected_faults = ppsfp_undetected_faults.len();
    let n_ppsfp_detected_faults = n_faults - n_ppsfp_undetected_faults;
    let all_faults_detected = n_ppsfp_undetected_faults == 0;
    if !crate::options::get_options().quiet && PRINT { 
        let mut i = 0;
        for ppsfp_result in ppsfp_results.iter() {
            println!("\nsimulation results (set {}):\n  patterns: \n{}", i+1, patterns[i].to_string());
            for (f, det) in ppsfp_result.iter() {
                println!("  fault on wire {} s-a-{} -> XOR(good, faulty): {:032b}", f.wire, if f.sa1 { 1 } else { 0 }, det);
            }
            println!("  -> faults covered: {}/{} ({:.2}%)", 
                n_ppsfp_detected_faults, 
                n_faults,
                (n_ppsfp_detected_faults as f64/n_faults as f64)*100 as f64 );
            i += 1;
        }
        println!("  -> {}all faults detected with random pattern generation", if all_faults_detected { "" } else { "not " });
    }
    if all_faults_detected && !ALWAYS_SAT { return; }
    //press_any_key(); //pausa per debug

    
    // -- parte III: algorithmic pattern generation -- //
    let undetected_faults = if SAT_FOR_ALL_FAULTS {
         faults.iter().cloned().collect() // debug: per fare sat su tutti fault in ogni caso
    } else {
        ppsfp_undetected_faults
    };
    let n_undetected_faults = undetected_faults.len();

    // costruzione stringa undetected faults
    let mut undetected_str = String::from("  -> undetected faults:\n");
    if !crate::options::get_options().quiet && PRINT {
        for fault in &undetected_faults {
            undetected_str.push_str(&format!("       fault on wire {} s-a-{}\n", &fault.wire, if fault.sa1 {1} else {0}));
        }
        print!("{undetected_str}");
    }
    press_any_key(); //pausa per debug

    // sat solving (risultato: hashmap<fault, map<input, bit(0/1)>>)
    print!("\n\nalgorithmic pattern generation:\n");
    let sat_results:HashMap<Fault, Vec<(String, u32)>> = if PARALLEL { 
        crate::sat::sat_solving_parallel(&circuit, &dag, undetected_faults.iter().collect())  /*parallela per fault, ottimizzato con rayon */ 
    } else { 
        crate::sat::sat_solving_sequential(&circuit, &dag, undetected_faults.iter().collect()) 
    };

    //press_any_key(); //pausa per debug
    // stampa sol
    print_sat_solution(&sat_results.clone());


    // -- risultati finali -- //
    // costruzione pattern finali (random+sat)

    let sat_patterns = sat_solutions_to_input_patterns(circuit, sat_results);
    let mut all_patterns = Vec::new();
    for pattern in patterns     { all_patterns.push(pattern.clone()); }
    for pattern in &sat_patterns { all_patterns.push(pattern.clone()); }
    
    //simulazione su tutti i blocchi di input (avrebbe senso farlo solo per i pattern sat, prima di fare il merge con gli altri, da cambiare) --> fatto giu'
    //let (final_results, uncovered_faults) = simulator.simulate_patterns_blocks(final_patterns.clone(), faults.clone());

    //simulazione su con i blocchi di input del sat solver sui fault rimanenti
    let (sat_sim_result, sat_sim_uncovered_faults) = simulator.simulate_patterns_blocks(sat_patterns, undetected_faults.iter().cloned().collect(), Some(PARALLEL));

    println!("undetected faults after simulation of sat patterns ({}): {:?}", sat_sim_uncovered_faults.len(), sat_sim_uncovered_faults);

    // risultati
    let n_sat_sim_uncovered_faults = sat_sim_uncovered_faults.len();
    let tot_uncovered_faults = if !ALWAYS_SAT {
         n_sat_sim_uncovered_faults 
        } else {
            min(sat_sim_uncovered_faults.len(), undetected_faults.len())
        };
    let n_final_results = n_faults - tot_uncovered_faults;
    let yay = n_final_results == n_faults;

    // stampa tutti i pattern di input
    let mut i = 1;
    for pattern in all_patterns {
        print!("set {i}:\n");
        pattern.print();
        i += 1;
    }

    // se ci sono, stampa i risultati finali (xor(good, faulty) per ogni fault)
    if n_final_results > 0 {
        i = 1;
        for res in sat_sim_result {
            println!("set {i}:");
            for (f, det) in res.iter() {
                println!("  fault on wire {} s-a-{} -> XOR(good, faulty): {:032b}", f.wire, if f.sa1 { 1 } else { 0 }, det);
            }
            i += 1;
        }
    }
    
    // statistiche finali
    println!("  -> {}all faults covered", if yay { "" } else { "not " });
    let mut def_undetected_str = String::from("  -> undetectable faults:\n");
    for fault in sat_sim_uncovered_faults.clone() {
        def_undetected_str.push_str(&format!("       fault on wire {} s-a-{}\n", &fault.wire, if fault.sa1 {1} else {0}));
    }
    print!("{}", if tot_uncovered_faults != 0 { def_undetected_str } else { "".to_string() });
    println!("  -> faults covered (with pseudo-random pattern generation): {}/{} ({:.2}%)", 
            n_ppsfp_detected_faults, 
            n_faults,
            (n_ppsfp_detected_faults as f64/n_faults as f64)*100 as f64);
    println!("  -> faults covered (with SAT solving):                      {}/{} ({:.2}%)", 
            undetected_faults.len() - sat_sim_uncovered_faults.clone().len(), 
            undetected_faults.len(),
            (undetected_faults.len() as f64 - sat_sim_uncovered_faults.len() as f64) / (undetected_faults.len() as f64) * 100 as f64);
    if !ALWAYS_SAT {
        println!("  -> faults covered (with random and SAT solving):           {}/{} ({:.2}%)", 
            n_final_results, 
            n_faults,
            (n_final_results as f64/n_faults as f64)*100 as f64);
    }
    
}


fn main_tests() {

    let n: i32 = get_nth_arg_to_int(1).unwrap_or(NET_N_DEFAULT);
    let fault_wire: Option<String> = get_nth_arg_to_string(2);
    let sa1: Option<bool> = get_nth_arg_to_bool(3);
    let seed: u64 = get_nth_arg_to_int(4).unwrap_or(DEFAULT_SEED) as u64;
    let filename: String = get_filename_from_int(NET_DIR, n).unwrap_or(get_default_name());
    println!("filename: {filename}");

    let circuit: Netlist = file_iscas89_atlanta_to_netlist(&filename);
    circuit.print();

    let dag: Dag = Dag::from_netlist(&circuit.clone());
    dag.print();

    let faults = dag.generate_fault_list(fault_wire, sa1);
    print_fault_list(&faults);

    let sat_results:HashMap<Fault, Vec<(String, u32)>> = if PARALLEL { 
        crate::sat::sat_solving_parallel(&circuit, &dag, faults.iter().collect())  /*parallela per fault, ottimizzato con rayon */ 
    } else { 
        crate::sat::sat_solving_sequential(&circuit, &dag, faults.iter().collect()) 
    };
    print_sat_solution(&sat_results);

    let sat_patterns = sat_solutions_to_input_patterns(circuit, sat_results);
    
    let mut simulator = ppsfp::PPSFPSimulator::new(&dag);

    let (sat_sim_result, sat_sim_uncovered_faults) = simulator.simulate_patterns_blocks(sat_patterns, faults, Some(PARALLEL));

    print!("sat result: {:?}\n", sat_sim_result);


    println!("undetected faults after simulation of sat patterns ({}): {:?}", sat_sim_uncovered_faults.len(), sat_sim_uncovered_faults);

}
