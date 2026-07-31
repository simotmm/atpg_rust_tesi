/*
thesis side quest
Simone Tumminelli s333017 2026
*/

#![allow(unused)] //rimuove i warning per unused import
mod dag;
mod netlist;
mod options;
mod parser;
mod pattern_generator;
mod ppsfp;
mod util;
use crate::pattern_generator::PatternGenerator;
use crate::ppsfp::PPSFPSimulator;

fn main() {
    let filename = util::get_nth_arg_to_string(1).unwrap_or_else(|| {
        eprintln!("Error: Could not read filename from command line");
        std::process::exit(1);
    });

    println!("Loading netlist from: {}", filename);
    let circuit = parser::verilog_to_netlist(&filename);
    println!(
        "Circuit inputs ({}): {:?}",
        circuit.inputs.len(),
        circuit.inputs
    );
    if circuit.inputs.len() > 5 {
        eprintln!(
            "Error: Circuit has more than 5 inputs ({})",
            circuit.inputs.len()
        );
        std::process::exit(2);
    }

    let pg = PatternGenerator::new(None);
    let combos = pg.generate_all_input_combinations(&circuit, Some(true));
    println!("Generated {} blocks (packed 32 combos each)", combos.len());
    for (i, p) in combos.iter().enumerate() {
        println!("Block {} (packed 32 combos):\n{}", i, p.to_string());
    }

    let dag = dag::Dag::from_netlist(&circuit);
    let mut sim = PPSFPSimulator::new(&dag);

    let mut best_count = 0;
    let mut best_block_idx: Option<usize> = None;
    let mut best_p1 = 0;
    let mut best_p2 = 0;

    for (block_idx, pat) in combos.iter().enumerate() {
        let ((p1, p2), count) = sim.most_transitions_for_compact_pattern(pat);
        if count > best_count {
            best_count = count;
            best_block_idx = Some(block_idx);
            best_p1 = p1;
            best_p2 = p2;
        }
    }

    if best_count == 0 {
        println!("No transitions detected across all blocks");
        return;
    }

    let block_idx = best_block_idx.unwrap();
    let p1 = best_p1;
    let p2 = best_p2;
    let global_idx1 = block_idx * 32 + p1;
    let global_idx2 = block_idx * 32 + p2;
    println!("Best overall: block {} between local pos {} and {} -> global combinations {} and {} with {} changes",
        block_idx, p1, p2, global_idx1, global_idx2, best_count);

    let pat = &combos[block_idx];
    let bits1 = pat.get_input_pattern_from_position(p1);
    let bits2 = pat.get_input_pattern_from_position(p2);
    println!("  Input bits at local pos {}: {:?}", p1, bits1);
    println!("  Input bits at local pos {}: {:?}", p2, bits2);

    let mut keys: Vec<String> = pat.values.keys().cloned().collect();
    keys.sort();
    let mapping1: Vec<String> = keys
        .iter()
        .zip(bits1.iter())
        .map(|(k, b)| format!("{}:{}", k, b))
        .collect();
    let mapping2: Vec<String> = keys
        .iter()
        .zip(bits2.iter())
        .map(|(k, b)| format!("{}:{}", k, b))
        .collect();
    println!("  Input vector 1: {}", mapping1.join(", "));
    println!("  Input vector 2: {}", mapping2.join(", "));

    match util::save_bits_result(&filename, &bits1, &bits2) {
        Ok(path) => println!("Saved bits to {}", path.display()),
        Err(e) => eprintln!("Failed to save bits result: {}", e),
    }
}
