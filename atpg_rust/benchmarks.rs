use std::time::Instant;
use std::fs;
use crate::cnf_minimizer;
use crate::parser::file_iscas89_atlanta_to_netlist;
use crate::dag::Dag;
use crate::ppsfp::{PPSFPSimulator, Fault};
use crate::pattern_generator::PatternGenerator;
use crate::netlist::{Netlist, Gate, GateType};
use std::collections::BTreeSet;

pub fn run_benchmarks() {
    println!("-- running CNF benchmark suite (limited sample) --");
    let dir = "benchmarks/converted_to_atlanta_iscas89";
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => {
            eprintln!("Benchmark directory not found: {}", dir);
            return;
        }
    };

    // Collect and sort entries to make benchmark ordering deterministic
    let mut paths: Vec<String> = Vec::new();
    for entry in entries {
        if let Ok(e) = entry {
            let path = e.path();
            let path_str = path.to_string_lossy().to_string();
            if path_str.ends_with(".bench") || path_str.ends_with(".v") {
                paths.push(path_str);
            }
        }
    }
    paths.sort();

    let mut count: usize = 0;
    for path_str in paths {
        if count >= 6 { break; } // limit number of benchmarks run
        println!("\nBenchmark file: {}", path_str);
        let t0 = Instant::now();
        let netlist = file_iscas89_atlanta_to_netlist(&path_str);
        let dt_parse = t0.elapsed();

        let t1 = Instant::now();
        let dag = Dag::from_netlist(&netlist);
        let dt_dag = t1.elapsed();

        let t2 = Instant::now();
        let cnf = dag.cnf_cone_from_outputs();
        let dt_cnf = t2.elapsed();

        // clone CNF and run minimization in-place
        let mut cnf_opt = cnf.clone();
        let t3 = Instant::now();
        let opt_res = cnf_minimizer::minimize_search_tree(&mut cnf_opt);
        let dt_opt = t3.elapsed();

        println!("  parse:     {:?}\n  dag build: {:?}\n  cnf build: {:?}\n  optimize:  {:?}", dt_parse, dt_dag, dt_cnf, dt_opt);
        let orig_clauses = cnf.get_n_clauses();
        match opt_res {
            Ok(_) => println!("  clauses: original {} -> optimized {}", orig_clauses, cnf_opt.get_n_clauses()),
            Err(_) => println!("  clauses: original {} -> optimized: conflict/UNSAT during minimization", orig_clauses),
        }

        count += 1;
    }

    println!("\n-- benchmark suite finished --");

    println!("\n-- running PPSFP micro-benchmark --");
    run_ppsfp_benchmark();
}

/// Micro-benchmark to compare PPSFP behavior: pruning detected faults vs no-prune.
pub fn run_ppsfp_benchmark() {
    use std::time::Instant;

    // Synthetic netlist parameters (tunable). Can be overridden by env vars:
    // PPSFP_INPUTS, PPSFP_NODES, PPSFP_FINAL_OUTPUTS, PPSFP_PATTERNS
    let mut num_inputs = 16usize;
    let mut num_nodes = 800usize; // increase to stress the simulator
    let mut final_outputs = 16usize;

    // optionally override with environment variables for quicker runs
    if let Ok(v) = std::env::var("PPSFP_INPUTS") { if let Ok(x) = v.parse::<usize>() { num_inputs = x; } }
    if let Ok(v) = std::env::var("PPSFP_NODES") { if let Ok(x) = v.parse::<usize>() { num_nodes = x; } }
    if let Ok(v) = std::env::var("PPSFP_FINAL_OUTPUTS") { if let Ok(x) = v.parse::<usize>() { final_outputs = x; } }

    // Build a simple deterministic synthetic netlist
    let mut nl = Netlist::new();
    for i in 0..num_inputs {
        nl.inputs.push(format!("I{}", i));
    }

    for i in 0..num_nodes {
        let name = format!("G{}", i);
        let out = format!("N{}", i);
        let gate_type = if i < num_inputs {
            GateType::BUF
        } else {
            if i % 2 == 0 { GateType::XOR } else { GateType::AND }
        };
        let inputs = if i < num_inputs {
            vec![format!("I{}", i % num_inputs)]
        } else {
            let a = format!("N{}", i - 1);
            let b = format!("N{}", (i + num_nodes - 2) % i);
            vec![a, b]
        };
        nl.gates.push(Gate { name, gate_type, outputs: vec![out], inputs });
    }

    let start_idx = if num_nodes > final_outputs { num_nodes - final_outputs } else { 0 };
    nl.outputs = (start_idx..num_nodes).map(|i| format!("N{}", i)).collect();

    let dag = Dag::from_netlist(&nl);
    let faults = dag.generate_fault_list(None, None);
    println!("Synthetic netlist: inputs {}, nodes {}, faults {}", num_inputs, num_nodes, faults.len());

    // Generate a fixed set of pseudo-random patterns (PPSFP_PATTERNS optional env override)
    let mut generator = PatternGenerator::new(Some(762000u64));
    let mut n_patterns = 32usize;
    if let Ok(v) = std::env::var("PPSFP_PATTERNS") { if let Ok(x) = v.parse::<usize>() { n_patterns = x; } }
    let patterns = generator.generate_n_patterns_from_netlist(&nl, n_patterns);

    // 1) Sequential - NO PRUNE
    let t0 = Instant::now();
    let mut sim = PPSFPSimulator::new(&dag);
    let mut detected_union = BTreeSet::<Fault>::new();
    for (i, p) in patterns.iter().enumerate() {
        let r = sim.simulate_patterns_block(p, &faults, i + 1, n_patterns);
        for f in r.keys() { detected_union.insert(f.clone()); }
    }
    let dt_no_prune = t0.elapsed();
    println!("Sequential NO-PRUNE: time {:?}, detected {}/{}", dt_no_prune, detected_union.len(), faults.len());

    // 2) Sequential - PRUNE
    let t1 = Instant::now();
    let mut sim2 = PPSFPSimulator::new(&dag);
    let (_res_prune, uncovered_prune) = sim2.simulate_patterns_blocks(patterns.clone(), faults.clone(), Some(false));
    let dt_prune = t1.elapsed();
    let detected_prune = faults.len() - uncovered_prune.len();
    println!("Sequential PRUNE:     time {:?}, detected {}/{}", dt_prune, detected_prune, faults.len());

    // 3) Parallel - NO PRUNE
    let t2 = Instant::now();
    let mut sim3 = PPSFPSimulator::new(&dag);
    let mut detected_union_par = BTreeSet::<Fault>::new();
    for (i, p) in patterns.iter().enumerate() {
        let r = sim3.simulate_patterns_block_parallel(p, &faults, i + 1, n_patterns);
        for f in r.keys() { detected_union_par.insert(f.clone()); }
    }
    let dt_no_prune_par = t2.elapsed();
    println!("Parallel NO-PRUNE:   time {:?}, detected {}/{}", dt_no_prune_par, detected_union_par.len(), faults.len());

    // 4) Parallel - PRUNE
    let t3 = Instant::now();
    let mut sim4 = PPSFPSimulator::new(&dag);
    let (_res_prune_par, uncovered_prune_par) = sim4.simulate_patterns_blocks(patterns.clone(), faults.clone(), Some(true));
    let dt_prune_par = t3.elapsed();
    let detected_prune_par = faults.len() - uncovered_prune_par.len();
    println!("Parallel PRUNE:     time {:?}, detected {}/{}", dt_prune_par, detected_prune_par, faults.len());

    // Speedup ratios (guard against divide-by-zero)
    if dt_prune.as_secs_f64() > 0.0 {
        println!("Sequential speedup (no-prune / prune): {:.2}x", dt_no_prune.as_secs_f64() / dt_prune.as_secs_f64());
    }
    if dt_prune_par.as_secs_f64() > 0.0 {
        println!("Parallel speedup   (no-prune / prune): {:.2}x", dt_no_prune_par.as_secs_f64() / dt_prune_par.as_secs_f64());
    }

}
