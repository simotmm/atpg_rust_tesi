use std::{env, fs::{self, create_dir}, io::{self, Read, Write}};
use std::path::Path;
use std::fs::File;
use crate::ppsfp::Fault;
use crate::netlist::Netlist;
use crate::pattern_generator::InputPattern;
use crate::pattern_generator::PatternGenerator;
use crate::SAVE_UNDETECTED_TO_FILE;
use std::collections::{HashMap, HashSet, BTreeSet};

const SOL_STRING_TABLE: bool = false;

/// get nth command line arg as int
pub fn get_nth_arg_to_int(n: i32) -> Option<i32> {
    env::args().nth(n as usize).and_then(|s| s.parse::<i32>().ok())
}

/// get nth command line arg as string
pub fn get_nth_arg_to_string(n: i32) -> Option<String> {
    env::args().nth(n as usize).and_then(|s| s.parse::<String>().ok())
}

/// get nth command line arg as boolean
pub fn get_nth_arg_to_bool(n: i32) -> Option<bool> {
    let x = get_nth_arg_to_int(n);
    if x == Some(0) { return Some(false); }
    else if x == Some(1) { return Some(true); }
    None
}

/// get the complete filename from int using regex
pub fn get_filename_from_int(dir: &str, numero: i32) -> Option<String> {
    let filenames: Vec<String> = vec![
        format!("b{numero}f.bench"),
        format!("s{numero}tc.bench"),
        format!("s{numero}f_C.bench"),
        format!("s{numero}tc_2.bench"),
        format!("c{numero}_to_iscas89_atlanta.bench"),
        format!("s{numero}_to_iscas89_atlanta.bench"),
        format!("s{numero}.bench"),
        format!("c{numero}.bench"),
    ];

    // Leggo tutti i file nella cartella
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                if let Some(file_name) = entry.file_name().to_str() {
                    //println!("filename: {file_name}");
                    for name in &filenames {
                        if name.to_string() == file_name {
                            return Some(format!("{}\\{}", dir, file_name).to_string());
                        }
                    }
                }
            }
        }
    }
    // If exact matches failed, try a flexible match: files that contain the number and end with .bench
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                if let Some(file_name) = entry.file_name().to_str() {
                    if file_name.to_lowercase().ends_with(".bench") && file_name.contains(&numero.to_string()) {
                        return Some(format!("{}\\{}", dir, file_name).to_string());
                    }
                }
            }
        }
    }
    None
}

/// netlist, sat solutions (HashMap<Fault, HashMap<String, u32>>) -> patterns (Vec<InputPattern>)
pub fn sat_solutions_to_input_patterns(circuit: Netlist, sat_solutions: HashMap<Fault, Vec<(String, u32)>>) -> Vec<InputPattern> {
    if sat_solutions.len() == 0 { return Vec::new(); }

    let mut patterns = Vec::new();
    // Use a deterministic pseudo-random start pattern so don't-care inputs produce varied bits
    let mut rng_gen = PatternGenerator::new(Some(762000));
    let start_pattern = rng_gen.generate_random_pattern_from_netlist(&circuit);
    let mut new_pattern = start_pattern.clone();
    let n_inputs = circuit.inputs.len();
    const N_BIT: i32 = 32;
    const START: i32 = N_BIT-1;
    let mut pos = START;

    for (f, input_vec) in sat_solutions {
        let bits: Vec<u32> = input_vec.iter().map(|(_, val)| *val).collect(); // l'ordine è mantenuto dalle funzioni precedenti
        println!("{}: {:?}", f.to_string(), bits);
        if bits.len() != n_inputs { continue; } 
        if pos < 0 {
            pos = START;
            patterns.push(new_pattern);
            new_pattern = start_pattern.clone();
        }
        new_pattern.insert_pattern_at_position(bits, pos as usize, circuit.clone().inputs);
        pos -= 1;
    }
    patterns.push(new_pattern);
    patterns
}

/// print sat solution
pub fn print_sat_solution(sat_solutions: &HashMap<Fault, Vec<(String, u32)>>) {
    if sat_solutions.len() == 0 { return; }
    println!("sat sol:\n{}", if SOL_STRING_TABLE { sat_solution_as_table_str(sat_solutions) } else { sat_solution_as_str(sat_solutions) });
}

/// sat solution -> string
fn sat_solution_as_str(sat_solutions: &HashMap<Fault, Vec<(String, u32)>>) -> String {
    let mut result = String::new();
    let mut faults: Vec<&Fault> = sat_solutions.keys().collect();
    faults.sort_by(|a, b| a.wire.cmp(&b.wire));

    for fault in faults {
        let fault_str = format!("{} s-a-{}: ", fault.wire, if fault.sa1 { 1 } else { 0 });

        //ordina per chiave (provvisorio, spesso l'ordine coincide con quello alfabetico, il vero ordine deve essere quello di comparsa in netlist.inputs)
        let mut sorted_bin_values: Vec<(String, u32)> = sat_solutions[fault].clone();
        sorted_bin_values.sort_by(|a, b| a.0.cmp(&b.0)); 

        let bin_values: String = sorted_bin_values.iter()
            .map(|(input, val)| format!("{input}:{} ", if *val == 0 { '0' } else { '1' })) // Stampa i valori binari
            .collect::<Vec<String>>()
            .join("");

        result.push_str(&format!("{}{}\n", fault_str, bin_values));
    }
    result
}

/// sat solution -> string (formattazione a tabella con intestazione)
/// esempio:
/// ``` 
///   fault   in1 in2 in3
/// D' s-a-0:  0   0   0
/// D' s-a-1:  0   0   0
/// E' s-a-1:  0   0   0
/// E' s-a-0:  0   0   0
/// X' s-a-1:  0   0   0
/// X' s-a-0:  0   0   0
/// ```
fn sat_solution_as_table_str(sat_solutions: &HashMap<Fault, Vec<(String, u32)>>) -> String {
    let mut result = String::new();

    // Crea un vettore delle chiavi (Fault) e ordinale in base a 'wire'
    let mut faults: Vec<&Fault> = sat_solutions.keys().collect();
    faults.sort_by(|a, b| a.wire.cmp(&b.wire)); // Ordina per 'wire'

    // Raccogli tutte le chiavi (entrate) da tutte le Vec interne
    let mut inputs: Vec<String> = sat_solutions
        .values()
        .flat_map(|assignments| assignments.iter().map(|(input, _)| input.clone())) // Estrai tutte le chiavi (in1, in2, etc.)
        .collect();
    
    // Rimuovi duplicati e ordina
    inputs.sort();
    inputs.dedup();

    // Determina la larghezza massima degli input (nomi degli input come "N1", "N2", ecc.)
    let max_input_len = inputs.iter().map(|input| input.len()).max().unwrap_or(0);

    // Determina la larghezza massima dei "fault" (nomi come "N1", "N2", ecc.)
    let max_fault_len = faults.iter().map(|fault| fault.wire.len()).max().unwrap_or(0);

    // Calcola la larghezza per il "fault" centrato sopra gli input
    let fault_column_padding = if max_input_len > 0 && max_fault_len > 0 {
        (max_input_len + max_fault_len) / 2
    } else {
        0
    };

    // Aggiungi l'intestazione per "fault"
    result.push_str(&format!("{:width$}fault    ", "", width = fault_column_padding));

    // Aggiungi i nomi degli input (N1, N2, N3, ...) centrati
    for input in &inputs {
        result.push_str(&format!("{:^width$} ", input, width = max_input_len));
    }
    result.push_str("\n");

    // Calcola la larghezza complessiva della sezione
    let total_width = max_input_len * inputs.len() + fault_column_padding * 2 + max_fault_len + 7; // Aggiungi spazio per "fault"

    // Itera su ogni fault e le relative assegnazioni
    for fault in faults {
        let assignments = &sat_solutions[fault];

        // Crea la stringa di tipo di fault (stuck-at-0 o stuck-at-1)
        let fault_str = format!(
            "{} s-a-{}: ", // Mantieni il ":" alla fine
            fault.wire,
            if fault.sa1 { 1 } else { 0 }
        );

        // Calcola la larghezza dinamica della colonna per il "fault"
        let fault_len = fault_str.len();
        let fault_padding = if max_input_len + max_fault_len > fault_len {
            (max_input_len + max_fault_len - fault_len) / 2
        } else {
            0
        };

        // Centra la stringa "fault" con il padding calcolato
        let centered_fault = format!("{:padding$}{}", "", fault_str, padding = fault_padding);

        // Crea i valori binari per gli inputs in ordine
        let bin_values: String = inputs.iter()
            .map(|input| {
                let val = *assignments.iter().find(|(k, _)| k == input).map(|(_, v)| v).unwrap_or(&0); // Ottieni il valore da assignments
                format!("{:^width$}", if val == 0 { '0' } else { '1' }, width = max_input_len) // Centra i valori binari
            })
            .collect::<Vec<String>>()
            .join(" ");

        // Aggiungi la riga formattata alla stringa finale
        result.push_str(&format!("{}{}\n", centered_fault, bin_values));
    }

    result
}

/// util: print fault list
pub fn print_fault_list(faults: &Vec<Fault>) {
    println!("fault list ({}):", faults.len());
    let mut i = 1;
    for fault in faults {
        println!("  {i}: {}", fault.to_string());
        i += 1;
    }
    println!();
}

/// util print pattern list
pub fn print_pattern_list(patterns: Vec<InputPattern>) {
    println!("pattern list ({}):", patterns.len());
    let mut i = 1;
    for pattern in &patterns { 
        println!("  pattern {i}:");
        pattern.print(); 
        i+=1;
    }
}

/// salva le pattern su file e, se presenti, stampa gli uncovered faults alla fine
pub fn save_patterns_to_file(patterns: &Vec<InputPattern>, uncovered_faults: &std::collections::BTreeSet<crate::ppsfp::Fault>, filename: &str) -> io::Result<()> {
    // Ensure results directory exists and place file there if filename has no directory
    let output_path = ensure_results_path(filename)?;
    let mut file = fs::File::create(&output_path)?;
    writeln!(file, "pattern list ({}):", patterns.len())?;
    let mut i = 1;
    for pattern in patterns {
        writeln!(file, "  pattern {i}:")?;
        // pattern.to_string() returns lines like "  A: 010..."; write them
        writeln!(file, "{}", pattern.to_string())?;
        i += 1;
    }

    if !uncovered_faults.is_empty() {
        writeln!(file, "")?;
        writeln!(file, "uncovered faults ({}):", uncovered_faults.len())?;
        for fault in uncovered_faults {
            writeln!(file, "  {}", fault.to_string())?;
        }
    }

    Ok(())
}

/// salva le pattern su file nel formato "verticale" (righe = bit, colonne = input)
/// Il file viene creato nello stesso folder del file di input e si chiama
/// <input_file_stem>_test_inputs.txt
pub fn save_patterns_to_test_inputs(patterns: &Vec<InputPattern>, uncovered_faults: &std::collections::BTreeSet<crate::ppsfp::Fault>, input_filename: &str, save_uncovered: Option<bool>) -> io::Result<()> {
    use std::path::Path;
    use std::env;
    let print_always = true; // se false, salva gli uncovered faults solo se presenti e se save_uncovered è Some(true)

    // output file in current working directory (same folder where the binary is run, typically the project's root with `cargo run`).
    let cwd = env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());

    let input_path = Path::new(input_filename);
    let stem = input_path.file_stem().and_then(|s| s.to_str()).unwrap_or("test_inputs");
    // create (or ensure) results folder inside current working directory and write file there
    let results_dir = cwd.join("results");
    if !results_dir.exists() { fs::create_dir_all(&results_dir)?; }
    let output_path = results_dir.join(format!("{}_test_inputs.txt", stem));

    let mut file = fs::File::create(&output_path)?;

    // Try to read the netlist to obtain the canonical inputs order (circuit.inputs)
    let netlist: Netlist = crate::parser::file_iscas89_atlanta_to_netlist(input_filename);
    let inputs_order: Vec<String> = netlist.inputs.clone();

    // If no inputs, create an empty file with possible uncovered faults
    if inputs_order.is_empty() {
        writeln!(file, "pattern list (0):")?;
        if !uncovered_faults.is_empty() {
            writeln!(file, "")?;
            for fault in uncovered_faults {
                writeln!(file, "{} s-a-{}", fault.wire, if fault.sa1 { 1 } else { 0 })?;
            }
        }
        return Ok(());
    }

    // For each pattern set, write 32 rows (MSB -> LSB), each row contains one bit per input
    // Columns order follows the netlist inputs ordering. No "set n:" lines and no spaces between bits.
    // Remove duplicate rows across the whole file, preserving first occurrence order.
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for p in patterns.iter() {
        let mut wrote_any = false;
        for pos in (0..32).rev() {
            let mut row = String::with_capacity(inputs_order.len());
            for input in &inputs_order {
                let v = p.values.get(input).unwrap_or(&0u32);
                let bit = ((*v >> pos) & 1) as u8;
                row.push(if bit == 1 { '1' } else { '0' });
            }
            // write only if not seen before
            if seen.insert(row.clone()) {
                writeln!(file, "{}", row)?;
                wrote_any = true;
            }
        }
        /* // optional: add a blank line between pattern sets for readability, but only if we wrote any new patterns in this set
        if wrote_any {
            writeln!(file, "")?; // blank line between pattern sets for readability
        }
        */
    }

    // append uncovered faults if present in the requested format: "<fault> <fault type>"
    if print_always || ( !uncovered_faults.is_empty() && save_uncovered.unwrap_or(false) ) {
        writeln!(file, "")?;
        writeln!(file, "undetectable faults ({}):", uncovered_faults.len())?;
        for fault in uncovered_faults {
            writeln!(file, "{}", fault.to_string())?;
        }
    }

    Ok(())
}

/// Ensure a `results` directory exists and return a PathBuf to place `filename` inside it
fn ensure_results_path(filename: &str) -> io::Result<std::path::PathBuf> {
    let out_dir = Path::new("results");
    if !out_dir.exists() {
        fs::create_dir_all(out_dir)?;
    }
    let p = Path::new(filename);
    if p.parent().is_none() {
        Ok(out_dir.join(filename))
    } else {
        Ok(p.to_path_buf())
    }
}

/// util print sim results
pub fn print_simulation_results(results: Vec<HashMap<Fault, u32>>, faults: Vec<Fault>, undetected_faults: BTreeSet<Fault>, patterns: Vec<InputPattern>, print_all_patterns: bool) {
    let mut i = 0;
    let n_faults = faults.len();
    let n_undetected_faults = undetected_faults.len();
    let faults_covered = n_faults - n_undetected_faults;
    let all_faults_detected = n_undetected_faults == 0;
    println!("simulation results:");
    if print_all_patterns { print_pattern_list(patterns); }
    for result in results.iter() {
        println!("\n(set {}):", i+1);
        for (f, det) in result.iter() {
            println!("  fault on wire {} s-a-{} -> XOR(good, faulty): {:032b}", f.wire, if f.sa1 { 1 } else { 0 }, det);
        }
        println!("  -> faults covered: {}/{} ({:.2}%)", 
            faults_covered, 
            n_faults,
            (faults_covered as f64/n_faults as f64)*100 as f64 );
        i += 1;
    }
    println!("  -> {}all faults detected", if all_faults_detected { "" } else { "not " });
    if !all_faults_detected {
        println!("  -> undetected faults ({}):", n_undetected_faults);
        let mut v: Vec<Fault> = undetected_faults.into_iter().collect();
        print_fault_list(&v);
    }
}

/// util pause function
pub fn press_any_key() {
    println!("\n[press enter to continue...]\n");
    let _ = io::stdin().read(&mut [0u8]).unwrap();
}

/// util min fn
pub fn min(a: usize, b: usize) -> usize {
    if a < b { return a; }
    b
}

pub fn max(a: usize, b: usize) -> usize {
    if a > b { return a; }
    b
}

/// Convert all Verilog ISCAS89 files in `input_dir` to a combinational
/// ATLANTA/bench representation (snapshot): every DFF Q output is turned
/// into a primary input. Results are written to `output_dir` with names
/// `<stem>_to_iscas89_atlanta.bench`.
pub fn snapshot_iscas89_folder(input_dir: &str, output_dir: &str) -> io::Result<()> {
    // Ensure output dir exists
    if !Path::new(output_dir).exists() {
        fs::create_dir_all(output_dir)?;
    }

    for entry in fs::read_dir(input_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() { continue; }
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
        if ext != "v" { continue; }

        let filename = path.to_str().unwrap_or("");
        // Collect DFF Q outputs by scanning instantiations like: dff DFF_0(CK,Q,D);
        let content = fs::read_to_string(&path)?;
        let mut dff_qs: Vec<String> = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("dff") {
                if let Some(start) = trimmed.find('(') {
                    if let Some(end) = trimmed.find(')') {
                        let inside = &trimmed[start + 1..end];
                        let parts: Vec<&str> = inside.split(',').map(|s| s.trim()).collect();
                        if parts.len() >= 3 {
                            let q = parts[1].trim().trim_end_matches(';').to_string();
                            if !q.is_empty() {
                                dff_qs.push(q);
                            }
                        }
                    }
                }
            }
        }

        // Parse the verilog into internal Netlist (this ignores dff instantiations)
        let netlist_result = std::panic::catch_unwind(|| crate::parser::file_iscas89_verilog_to_netlist(filename));
        let mut netlist = match netlist_result {
            Ok(n) => n,
            Err(_) => {
                eprintln!("Warning: parser panicked on file {}, skipping", filename);
                continue;
            }
        };

        // Add each DFF Q as a primary input (snapshot)
        for q in dff_qs.iter() {
            if !netlist.inputs.iter().any(|x| x == q) {
                netlist.inputs.push(q.clone());
            }
        }

        // Remove clock (CK) from inputs if present
        netlist.inputs.retain(|x| x.to_uppercase() != "CK");

        // Write output bench file
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("out");
        let out_path = Path::new(output_dir).join(format!("{}_to_iscas89_atlanta.bench", stem));
        let mut out_file = File::create(&out_path)?;

        // Write inputs
        for inp in &netlist.inputs {
            writeln!(out_file, "INPUT({})", inp)?;
        }
        // Write outputs
        for out in &netlist.outputs {
            writeln!(out_file, "OUTPUT({})", out)?;
        }
        // Write gates
        for gate in &netlist.gates {
            let out_name = gate.outputs.get(0).map(|s| s.as_str()).unwrap_or(gate.name.as_str());
            if gate.inputs.is_empty() {
                writeln!(out_file, "{} = {}", out_name, gate.gate_type.to_string())?;
            } else {
                writeln!(out_file, "{} = {}({})", out_name, gate.gate_type.to_string(), gate.inputs.join(", "))?;
            }
        }

        println!("Converted {} -> {}", filename, out_path.display());
    }

    Ok(())
}

/// Time-frame expansion (unrolling) for ISCAS89 Verilog files.
/// For each file in `input_dir` produces an unrolled combinational bench
/// in `output_dir` with `k_frames` temporal frames. Original primary inputs
/// are duplicated per frame (`name_t{frame}`) and flip‑flop initial states
/// are modeled as primary inputs at frame 0 (`Q_t0`). Final outputs are
/// taken from frame `k_frames` (suffix `_t{k_frames}`).
pub fn unroll_iscas89_folder(input_dir: &str, output_dir: &str, k_frames: usize) -> io::Result<()> {
    if k_frames == 0 { return Err(io::Error::new(io::ErrorKind::InvalidInput, "k_frames must be >= 1")); }
    if !Path::new(output_dir).exists() {
        fs::create_dir_all(output_dir)?;
    }

    for entry in fs::read_dir(input_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() { continue; }
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
        if ext != "v" { continue; }

        let filename = path.to_str().unwrap_or("");
        let content = fs::read_to_string(&path)?;

        // Collect DFF mappings (Q <- D)
        let mut dff_qs: Vec<String> = Vec::new();
        let mut dff_ds: Vec<String> = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("dff") {
                if let Some(start) = trimmed.find('(') {
                    if let Some(end) = trimmed.find(')') {
                        let inside = &trimmed[start + 1..end];
                        let parts: Vec<&str> = inside.split(',').map(|s| s.trim()).collect();
                        if parts.len() >= 3 {
                            let q = parts[1].trim().trim_end_matches(';').to_string();
                            let d = parts[2].trim().trim_end_matches(';').to_string();
                            if !q.is_empty() && !d.is_empty() {
                                dff_qs.push(q);
                                dff_ds.push(d);
                            }
                        }
                    }
                }
            }
        }

        // Parse combinational netlist (may panic for odd files; catch to continue)
        let netlist = match std::panic::catch_unwind(|| crate::parser::file_iscas89_verilog_to_netlist(filename)) {
            Ok(n) => n,
            Err(_) => { eprintln!("Warning: parser panicked on file {}, skipping unroll", filename); continue; }
        };

        use std::collections::HashSet;
        let q_set: HashSet<String> = dff_qs.iter().cloned().collect();

        // Prepare output file
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("out");
        let out_path = Path::new(output_dir).join(format!("{}_to_iscas89_atlanta_unrolled_{}f.bench", stem, k_frames));
        let mut out_file = File::create(&out_path)?;

        // Write primary inputs: for each frame 0..=k_frames duplicate netlist.inputs
        for t in 0..=k_frames {
            for inp in &netlist.inputs {
                writeln!(out_file, "INPUT({}_t{})", inp, t)?;
            }
        }
        // Write flipflop initial states as inputs at frame 0 (Q_t0)
        for q in &dff_qs {
            writeln!(out_file, "INPUT({}_t0)", q)?;
        }

        // Write outputs: take them from frame k_frames
        for out in &netlist.outputs {
            writeln!(out_file, "OUTPUT({}_t{})", out, k_frames)?;
        }

        // For each time frame, create combinational gates with suffixed names
        for t in 0..=k_frames {
            // create gates at frame t
            for gate in &netlist.gates {
                let out_name = gate.outputs.get(0).map(|s| s.as_str()).unwrap_or(gate.name.as_str());
                let out_t = format!("{}_t{}", out_name, t);
                if gate.inputs.is_empty() {
                    writeln!(out_file, "{} = {}", out_t, gate.gate_type.to_string())?;
                } else {
                    let mut inputs_t: Vec<String> = Vec::new();
                    for inp in &gate.inputs {
                        // if input is a flipflop output, use its Q at same frame
                        if q_set.contains(inp) {
                            inputs_t.push(format!("{}_t{}", inp, t));
                        } else {
                            inputs_t.push(format!("{}_t{}", inp, t));
                        }
                    }
                    writeln!(out_file, "{} = {}({})", out_t, gate.gate_type.to_string(), inputs_t.join(", "))?;
                }
            }

            // For frames 0..k_frames-1 create buffers mapping D_t -> Q_{t+1}
            if t < k_frames {
                for (q, d) in dff_qs.iter().zip(dff_ds.iter()) {
                    let d_t = format!("{}_t{}", d, t);
                    let q_next = format!("{}_t{}", q, t + 1);
                    writeln!(out_file, "{} = BUF({})", q_next, d_t)?;
                }
            }
        }

        println!("Unrolled {} -> {} ({} frames)", filename, out_path.display(), k_frames);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_min_max_basic() {
        assert_eq!(min(1, 2), 1);
        assert_eq!(max(1, 2), 2);
        assert_eq!(min(5, 5), 5);
        assert_eq!(max(5, 5), 5);
        assert_eq!(min(0, 10), 0);
        assert_eq!(max(0, 10), 10);
    }
}

/* 
/// format file iscas85 -> iscas89 atlanta
pub fn process_iscas85_to89atlanta_files_in_folder(folder_path: &str) {
    // Controllo se la cartella esiste
    if !std::path::Path::new(folder_path).exists() {
        eprintln!("La cartella {} non esiste!", folder_path);
        return;
    }

    // Leggo tutti i file nella cartella
    let entries = match fs::read_dir(folder_path) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("Errore nella lettura della cartella: {}", e);
            return;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Errore nella lettura di un file: {}", e);
                continue;
            }
        };

        let path = entry.path();

        // Controllo che sia un file
        if path.is_file() {
            if let Some(filename) = path.to_str() {
                let _ = iscas85_to_89_atlanta(filename);
            }
        }
    }
}
*/

/* 
/// format file iscas85 -> iscas89
pub fn process_iscas85_to89_files_in_folder(folder_path: &str) {
    // Controllo se la cartella esiste
    if !Path::new(folder_path).exists() {
        eprintln!("La cartella {} non esiste!", folder_path);
        return;
    }

    // Leggo tutti i file nella cartella
    let entries = match fs::read_dir(folder_path) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("Errore nella lettura della cartella: {}", e);
            return;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Errore nella lettura di un file: {}", e);
                continue;
            }
        };

        let path = entry.path();

        // Controllo che sia un file
        if path.is_file() {
            if let Some(filename) = path.to_str() {
                let _ = iscas85_to89(filename);
            }
        }
    }
}
*/

/*
/// format file iscas89 -> iscas89 without sequential elements (non sempre funziona)
pub fn process_iscas89_to_comb_files_in_folder(folder_path: &str) {
    // Controllo se la cartella esiste
    if !Path::new(folder_path).exists() {
        eprintln!("La cartella {} non esiste!", folder_path);
        return;
    }

    // Leggo tutti i file nella cartella
    let entries = match fs::read_dir(folder_path) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("Errore nella lettura della cartella: {}", e);
            return;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Errore nella lettura di un file: {}", e);
                continue;
            }
        };

        let path = entry.path();

        // Controllo che sia un file
        if path.is_file() {
            if let Some(filename) = path.to_str() {
                let _ = iscas89_to_comb(filename);
            }
        }
    }
}
*/
