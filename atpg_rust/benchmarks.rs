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
    if !crate::options::get_options().quiet { println!("-- running CNF benchmark suite (limited sample) --"); }
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
        if !crate::options::get_options().quiet { println!("\nBenchmark file: {}", path_str); }
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

        if !crate::options::get_options().quiet { println!("  parse:     {:?}\n  dag build: {:?}\n  cnf build: {:?}\n  optimize:  {:?}", dt_parse, dt_dag, dt_cnf, dt_opt); }
        let orig_clauses = cnf.get_n_clauses();
        if !crate::options::get_options().quiet {
            match opt_res {
                Ok(_) => println!("  clauses: original {} -> optimized {}", orig_clauses, cnf_opt.get_n_clauses()),
                Err(_) => println!("  clauses: original {} -> optimized: conflict/UNSAT during minimization", orig_clauses),
            }
        }

        count += 1;
    }

    if !crate::options::get_options().quiet { println!("\n-- benchmark suite finished --"); }

    if !crate::options::get_options().quiet { println!("\n-- running PPSFP micro-benchmark --"); }
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
    if !crate::options::get_options().quiet { println!("Synthetic netlist: inputs {}, nodes {}, faults {}", num_inputs, num_nodes, faults.len()); }

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
    if !crate::options::get_options().quiet { println!("Sequential NO-PRUNE: time {:?}, detected {}/{}", dt_no_prune, detected_union.len(), faults.len()); }

    // 2) Sequential - PRUNE
    let t1 = Instant::now();
    let mut sim2 = PPSFPSimulator::new(&dag);
    let (_res_prune, uncovered_prune) = sim2.simulate_patterns_blocks(patterns.clone(), faults.clone(), Some(false));
    let dt_prune = t1.elapsed();
    let detected_prune = faults.len() - uncovered_prune.len();
    if !crate::options::get_options().quiet { println!("Sequential PRUNE:     time {:?}, detected {}/{}", dt_prune, detected_prune, faults.len()); }

    // 3) Parallel - NO PRUNE
    let t2 = Instant::now();
    let mut sim3 = PPSFPSimulator::new(&dag);
    let mut detected_union_par = BTreeSet::<Fault>::new();
    for (i, p) in patterns.iter().enumerate() {
        let r = sim3.simulate_patterns_block_parallel(p, &faults, i + 1, n_patterns);
        for f in r.keys() { detected_union_par.insert(f.clone()); }
    }
    let dt_no_prune_par = t2.elapsed();
    if !crate::options::get_options().quiet { println!("Parallel NO-PRUNE:   time {:?}, detected {}/{}", dt_no_prune_par, detected_union_par.len(), faults.len()); }

    // 4) Parallel - PRUNE
    let t3 = Instant::now();
    let mut sim4 = PPSFPSimulator::new(&dag);
    let (_res_prune_par, uncovered_prune_par) = sim4.simulate_patterns_blocks(patterns.clone(), faults.clone(), Some(true));
    let dt_prune_par = t3.elapsed();
    let detected_prune_par = faults.len() - uncovered_prune_par.len();
    if !crate::options::get_options().quiet { println!("Parallel PRUNE:     time {:?}, detected {}/{}", dt_prune_par, detected_prune_par, faults.len()); }

    // Speedup ratios (guard against divide-by-zero)
    if !crate::options::get_options().quiet {
        if dt_prune.as_secs_f64() > 0.0 {
            println!("Sequential speedup (no-prune / prune): {:.2}x", dt_no_prune.as_secs_f64() / dt_prune.as_secs_f64());
        }
        if dt_prune_par.as_secs_f64() > 0.0 {
            println!("Parallel speedup   (no-prune / prune): {:.2}x", dt_no_prune_par.as_secs_f64() / dt_prune_par.as_secs_f64());
        }
    }

}

/// Run full benchmark suite: for each netlist starting with prefix 'c',
/// measure parse/dag build, random-phase detection, algorithmic-phase detection,
/// and append a summary to `README.md` in the repository root.
pub fn run_full_benchmarks_for_prefix(prefix: &str, specific: Option<&str>, seed_opt: Option<u64>, mode_opt: Option<&str>) {
    // backward-compatible wrapper
    run_full_benchmarks_for_prefixes(&[prefix], specific, seed_opt, mode_opt);
}

/// Run full benchmark suite for multiple prefixes.
pub fn run_full_benchmarks_for_prefixes(prefixes: &[&str], specific: Option<&str>, seed_opt: Option<u64>, mode_opt: Option<&str>) {
    use std::time::Instant;
    use std::fs::OpenOptions;
    use std::io::Write;
    use crate::util::{sat_solutions_to_input_patterns, save_patterns_to_test_inputs};

    let dir = "benchmarks/converted_to_atlanta_iscas89";

    let mut paths: Vec<String> = Vec::new();

    // If `specific` is a numeric id, construct the expected filename directly
    if let Some(spec) = specific {
        if spec.parse::<usize>().is_ok() {
            for prefix in prefixes {
                let filename = format!("{}{}_to_iscas89_atlanta.bench", prefix, spec);
                let full = format!("{}/{}", dir, filename);
                if std::path::Path::new(&full).exists() {
                    paths.push(full);
                    break;
                }
            }
            if paths.is_empty() {
                eprintln!("Specified numeric file not found: prefixes={:?}, id={}", prefixes, spec);
                return;
            }
        } else {
            // non-numeric `specific`: fall back to scanning directory and matching by name/substring
            let entries = match fs::read_dir(dir) {
                Ok(e) => e,
                Err(_) => {
                    eprintln!("Benchmark directory not found: {}", dir);
                    return;
                }
            };
            for entry in entries {
                if let Ok(e) = entry {
                    let path = e.path();
                    let path_str = path.to_string_lossy().to_string();
                    if (path_str.ends_with(".bench") || path_str.ends_with(".v")) {
                        if let Some(name) = std::path::Path::new(&path_str).file_name().and_then(|s| s.to_str()) {
                            if name.eq_ignore_ascii_case(spec) || name.contains(spec) || name.trim_end_matches(".bench").eq_ignore_ascii_case(spec) {
                                paths.push(path_str);
                            }
                        }
                    }
                }
            }
        }
    } else {
        // No specific requested: scan directory and match by prefixes
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => {
                eprintln!("Benchmark directory not found: {}", dir);
                return;
            }
        };
        for entry in entries {
            if let Ok(e) = entry {
                let path = e.path();
                let path_str = path.to_string_lossy().to_string();
                if (path_str.ends_with(".bench") || path_str.ends_with(".v")) {
                    if let Some(name) = std::path::Path::new(&path_str).file_name().and_then(|s| s.to_str()) {
                        if prefixes.iter().any(|p| name.starts_with(p)) {
                            paths.push(path_str);
                        }
                    }
                }
            }
        }
    }
    paths.sort();

    // open README for appending
    let filename = "README.md";
    let mut readme = match OpenOptions::new().create(true).append(true).open(filename) {
        Ok(f) => f,
        Err(e) => { eprintln!("Cannot open {}: {}", filename, e); return; }
    };

    // Read existing README to avoid duplicating the table header
    let existing = std::fs::read_to_string(filename).unwrap_or_default();
    if !existing.contains("| File | Total (s) |") {
        let prefix_display = prefixes.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(",");
        if let Err(e) = writeln!(readme, "\n## Automatic benchmark results (prefixes={})\n", prefix_display) { eprintln!("Failed writing README header: {}", e); }
        // Markdown table header
        if let Err(e) = writeln!(readme, "| File | Total (s) | Parse (s) | DAG (s) | Rand (s) | Rand detected | SAT (s) | SAT detected | Total detected | Faults | Coverage (%) |") { eprintln!("Failed writing README header: {}", e); }
        if let Err(e) = writeln!(readme, "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|") { eprintln!("Failed writing README header: {}", e); }
    }

    for path_str in paths {
            if !crate::options::get_options().quiet { println!("Running full benchmark for: {}", path_str); }
        println!("parsing file to netlist");
        let t0 = Instant::now();
        let netlist = file_iscas89_atlanta_to_netlist(&path_str);
        let dt_parse = t0.elapsed();
        println!("parsing file to netlist complete: time {:?}", dt_parse);

        println!("building DAG from netlist");
        let t1 = Instant::now();
        let dag = Dag::from_netlist(&netlist);
        let dt_dag = t1.elapsed();
        println!("building DAG complete: time {:?}", dt_dag);

        // faults
        println!("generating fault list from DAG");
        let faults = dag.generate_fault_list(None, None);
        let n_faults = faults.len();
        println!("generating fault list complete: {} faults", n_faults);

        // random-phase
        println!("starting random pattern generation and simulation phase");
        let t_rand = Instant::now();
        let mut generator = PatternGenerator::new(seed_opt);
        // determine mode: "rand", "alg", "both"
        let effective_mode: Option<&str> = if mode_opt.is_some() { mode_opt } else { None };
        let run_random_phase = matches!(effective_mode, None | Some("both") | Some("rand") | Some("random") | Some("r"));
        let run_algorithmic_phase = matches!(effective_mode, None | Some("both") | Some("alg") | Some("algorithm") | Some("a"));
        println!(" random phase enabled: {}", run_random_phase);
        println!(" algorithmic phase enabled: {}", run_algorithmic_phase);
        if !run_random_phase {
            // skip random-phase entirely
            // set rand_detected = 0 and rand_uncovered = faults.clone()
            let rand_detected = 0usize;
            let dt_rand = std::time::Duration::new(0,0);
            // proceed to algorithmic phase using all faults as uncovered
            let rand_uncovered: std::collections::BTreeSet<crate::ppsfp::Fault> = faults.iter().cloned().collect();
            // algorithmic-phase handling below will use rand_uncovered
        }
        let n_patterns = std::cmp::max(1, n_faults/100);
        let mut simulator = PPSFPSimulator::new(&dag);
        println!("simulating random patterns and determining uncovered faults");
        let (rand_uncovered, rand_detected, dt_rand, patterns) = if run_random_phase {
            let patterns = generator.generate_n_patterns_from_netlist(&netlist, n_patterns);
            let (_rand_results, rand_uncovered) = simulator.simulate_patterns_blocks(patterns.clone(), faults.clone(), Some(true));
            let rand_detected = n_faults - rand_uncovered.len();
            let dt_rand = t_rand.elapsed();
            (rand_uncovered, rand_detected, dt_rand, patterns)
        } else {
            (faults.iter().cloned().collect(), 0usize, std::time::Duration::new(0,0), Vec::new())
        };
        println!("random phase complete: detected {}, uncovered {}, time {:?}", rand_detected, rand_uncovered.len(), dt_rand);

        // algorithmic-phase (SAT) and determine final uncovered faults
        println!("starting algorithmic phase (SAT-based pattern generation and simulation)");
        let mut sat_detected = 0usize;
        let mut dt_sat = std::time::Duration::new(0,0);
        // final_uncovered starts as the set left after random phase
        let mut final_uncovered: std::collections::BTreeSet<crate::ppsfp::Fault> = rand_uncovered.iter().cloned().collect();
        if run_algorithmic_phase && final_uncovered.len() > 0 {
            let t_sat = Instant::now();
            let sat_results = crate::sat::sat_solving(&netlist, &dag, final_uncovered.iter().collect(), Some(true));
            let sat_patterns = sat_solutions_to_input_patterns(netlist.clone(), sat_results.clone());
            let (_sat_sim_res, sat_uncovered) = simulator.simulate_patterns_blocks(sat_patterns.clone(), final_uncovered.iter().cloned().collect(), Some(true));
            sat_detected = final_uncovered.len() - sat_uncovered.len();
            dt_sat = t_sat.elapsed();
            final_uncovered = sat_uncovered;
            // append SAT patterns to patterns list for saving below (all_patterns will be built later)
        }
        println!("algorithmic phase complete: detected {}, uncovered {}, time {:?}", sat_detected, final_uncovered.len(), dt_sat);

        println!("benchmark complete for {}: total detected {}/{}", path_str, rand_detected + sat_detected, n_faults);
        let total_detected = rand_detected + sat_detected;
        let coverage = if n_faults>0 { (total_detected as f64 / n_faults as f64)*100.0 } else { 0.0 };
        let total_time = dt_parse + dt_dag + dt_rand + dt_sat;
        println!("total time: {:?}, coverage: {:.2}%", total_time, coverage);
        // append summary to README
        let file_name = std::path::Path::new(&path_str).file_name().and_then(|s| s.to_str()).unwrap_or(&path_str);
        println!("Appending benchmark results to README.md for file: {}", file_name);
        if let Err(e) = writeln!(readme, "| {} | {:.3} | {:.3} | {:.3} | {:.3} | {}/{} | {:.3} | {}/{} | {}/{} | {} | {:.2} |",
            file_name,
            total_time.as_secs_f64(),
            dt_parse.as_secs_f64(),
            dt_dag.as_secs_f64(),
            dt_rand.as_secs_f64(),
            rand_detected, n_faults,
            dt_sat.as_secs_f64(),
            sat_detected, n_faults,
            total_detected, n_faults,
            n_faults,
            coverage) { eprintln!("Failed writing README row for {}: {}", file_name, e); }
        println!("Benchmark summary appended to README.md for file: {}", file_name);
        
        println!("Saving detected patterns to results folder");
        // save patterns for this netlist in results folder (optional)
        let mut all_patterns = patterns.clone();
        // collect sat patterns if present (we may have computed them above)
        if run_algorithmic_phase && rand_uncovered.len() > 0 {
            let sat_results = crate::sat::sat_solving(&netlist, &dag, rand_uncovered.iter().collect(), Some(true));
            let sat_patterns = sat_solutions_to_input_patterns(netlist.clone(), sat_results.clone());
            for p in sat_patterns { all_patterns.push(p); }
        }
        // pass final_uncovered (undetected faults after all phases) to save_patterns_to_test_inputs
        let _ = save_patterns_to_test_inputs(&all_patterns, &final_uncovered, &path_str, Some(true));
        println!("Detected patterns saved to results folder for file: {}", file_name);

        if !crate::options::get_options().quiet { println!("Finished: {} — coverage: {:.2}%", path_str, coverage); }
    }
}

