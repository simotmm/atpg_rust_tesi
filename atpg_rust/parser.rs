use std::fs::{File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use crate::netlist::{Netlist, Gate, GateType};

/// parser iscas89 (format used in atlanta): filename -> netlist
pub fn file_iscas89_atlanta_to_netlist(filename: &str) -> Netlist {
    let file = File::open(Path::new(filename)).expect("Impossibile aprire file");
    let reader = io::BufReader::new(file);

    let mut netlist = Netlist::new();

    for line in reader.lines() {
        let mut trimmed = line.unwrap();
        trimmed = trimmed.trim().to_string();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Rimuove eventuale punto e virgola
        if trimmed.ends_with(';') {
            trimmed.pop();
        }

        let upper = trimmed.to_uppercase();

        // ---- INPUT / PINPUT ----
        if upper.starts_with("INPUT(") || upper.starts_with("PINPUT(") {
            if let (Some(start), Some(end)) = (trimmed.find('('), trimmed.rfind(')')) {
                let name = trimmed[start + 1 .. end].trim().to_string();
                netlist.inputs.push(name);
            }
            continue;
        }

        // ---- OUTPUT / POUTPUT ----
        if upper.starts_with("OUTPUT(") || upper.starts_with("POUTPUT(") {
            if let (Some(start), Some(end)) = (trimmed.find('('), trimmed.rfind(')')) {
                let name = trimmed[start + 1 .. end].trim().to_string();
                netlist.outputs.push(name);
            }
            continue;
        }

        // ---- GATE ----
        if let Some(eq_pos) = trimmed.find('=') {
            let left = trimmed[..eq_pos].trim().to_string();
            let right = trimmed[eq_pos + 1..].trim();

            let open = right.find('(').expect("Gate senza '('");
            let close = right.rfind(')').expect("Gate senza ')'");

            let gate_type_str = right[..open].trim().to_uppercase();
            let gate_type = GateType::from_str(&gate_type_str);

            let inside = &right[open + 1 .. close];
            let inputs: Vec<String> = inside
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            let gate = Gate {
                name: left.clone(),     // usa direttamente il nome dell'uscita del gate
                gate_type,
                outputs: vec![left],
                inputs,
            };

            netlist.gates.push(gate);
            continue;
        }
    }

    netlist
}

/// parser iscas89 (verilog): filename -> netlist
pub fn file_iscas89_verilog_to_netlist(filename: &str) -> Netlist {
    let path = Path::new(filename);
    let file = File::open(path).expect("Impossibile aprire file");
    let reader = io::BufReader::new(file);
    let mut netlist = Netlist::new();

    // Read all lines to allow multi-line accumulation for inputs/outputs
    let lines: Vec<String> = reader.lines().filter_map(|r| r.ok()).collect();
    let mut i: usize = 0;
    while i < lines.len() {
        let line = &lines[i];
        let mut trimmed = line.trim().to_string();

        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("module") || trimmed.starts_with("endmodule") {
            i += 1;
            continue;
        }

        // Accumulate multi-line input declarations ending with ';'
        if trimmed.starts_with("input") {
            let mut acc = trimmed.trim_start_matches("input").trim().to_string();
            while !acc.contains(';') && i + 1 < lines.len() {
                i += 1;
                acc.push_str(lines[i].trim());
            }
            // remove trailing semicolon if present
            if let Some(pos) = acc.rfind(';') { acc.truncate(pos); }
            for tok in acc.split(',') {
                let s = tok.trim();
                if !s.is_empty() {
                    netlist.inputs.push(s.to_string());
                }
            }
            i += 1;
            continue;
        }

        // Accumulate multi-line output declarations
        if trimmed.starts_with("output") {
            let mut acc = trimmed.trim_start_matches("output").trim().to_string();
            while !acc.contains(';') && i + 1 < lines.len() {
                i += 1;
                acc.push_str(lines[i].trim());
            }
            if let Some(pos) = acc.rfind(';') { acc.truncate(pos); }
            for tok in acc.split(',') {
                let s = tok.trim();
                if !s.is_empty() {
                    netlist.outputs.push(s.to_string());
                }
            }
            i += 1;
            continue;
        }

        // Parse combinational gate instantiations safely
        let l = trimmed.to_lowercase();
        if l.starts_with("and")
            || l.starts_with("or")
            || l.starts_with("xor")
            || l.starts_with("nand")
            || l.starts_with("nor")
            || l.starts_with("xnor")
            || l.starts_with("buf")
            || l.starts_with("not")
        {
            // find parentheses
            if let (Some(start), Some(end)) = (trimmed.find('('), trimmed.rfind(')')) {
                let gate_type_str = trimmed[..start].split_whitespace().next().unwrap_or("").trim();
                let gate_type = GateType::from_str(gate_type_str);
                let inside = &trimmed[start + 1..end];
                let tokens: Vec<String> = inside.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                if tokens.is_empty() { i += 1; continue; }
                let outputs = vec![tokens[0].clone()];
                let inputs: Vec<String> = if tokens.len() > 1 { tokens[1..].to_vec() } else { Vec::new() };
                let gate_name = outputs[0].clone();
                let gate = Gate { name: gate_name, gate_type, outputs, inputs };
                netlist.gates.push(gate);
            }
        }

        i += 1;
    }

    netlist
}

/// formato vecchio (mio) -> netlist
pub fn file_to_netlist(filename: &str) -> io::Result<Netlist> {
    let path = Path::new(filename);
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);

    let mut netlist = Netlist::new();

    for line_res in reader.lines() {
        let line = line_res?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') { continue; }
        if trimmed.to_uppercase() == "END" { break; }

        let tokens: Vec<&str> = trimmed.split_whitespace().collect();
        if tokens.is_empty() { continue; }

        match tokens[0].to_uppercase().as_str() {
            "INPUT" => { if tokens.len() == 2 { netlist.inputs.push(tokens[1].to_string()); } },
            "OUTPUT" => { if tokens.len() == 2 { netlist.outputs.push(tokens[1].to_string()); } },
            "GATE" => {
                if tokens.len() < 5 { continue; }
                let name = tokens[1].to_string();
                let gate_type = GateType::from_str(tokens[2]);
                let output = vec![tokens[3].to_string()];
                let inputs = if tokens.len() > 4 { tokens[4..].iter().map(|s| s.to_string()).collect() } else { Vec::new() };
                netlist.gates.push(Gate { name, gate_type, outputs: output, inputs });
            }
            _ => { eprintln!("Riga non riconosciuta: {}", line); }
        }
    }

    Ok(netlist)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_to_netlist_missing() {
        let res = file_to_netlist("non_existent_file.bench");
        assert!(res.is_err());
    }
}

/// Converte un file ISCAS85 in un file ISCAS89 (Atlanta)
/// Il nuovo file verrà scritto con nome: "<filename>_to_iscas89_atlanta.bench"
pub fn iscas85_to_89_atlanta(filename: &str) -> io::Result<String> {
    let file = File::open(filename)?;
    let mut reader = BufReader::new(file).lines();

    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    let mut gates = Vec::new();

    while let Some(line) = reader.next() {
        let line = line?;
        let line = line.trim();

        if line.is_empty() || line.starts_with("//") {
            continue;
        }

        if line.starts_with("input ") {
            let mut acc = line.trim_start_matches("input ").trim().to_string();

            while !acc.contains(';') {
                if let Some(next_line) = reader.next() {
                    let next_line = next_line?;
                    acc.push_str(next_line.trim());
                } else {
                    break;
                }
            }

            let acc = acc.trim_end_matches(';');
            inputs.extend(acc.split(',').map(|s| s.trim().to_string()));
            continue;
        }

        if line.starts_with("output ") {
            let mut acc = line.trim_start_matches("output ").trim().to_string();

            while !acc.contains(';') {
                if let Some(next_line) = reader.next() {
                    let next_line = next_line?;
                    acc.push_str(next_line.trim());
                } else {
                    break;
                }
            }

            let acc = acc.trim_end_matches(';');
            outputs.extend(acc.split(',').map(|s| s.trim().to_string()));
            continue;
        }

        if let Some(pos) = line.find(' ') {
            let gate_type_raw = &line[..pos].to_lowercase();
            let rest = &line[pos..];
            let valid_gates = ["nand", "and", "or", "nor", "xor", "xnor", "not", "buf"];

            if valid_gates.contains(&gate_type_raw.as_str()) {
                if let (Some(start), Some(end)) = (rest.find('('), rest.find(')')) {
                    let content = &rest[start + 1..end];
                    let mut parts = content.split(',').map(|s| s.trim());

                    if let Some(output) = parts.next() {
                        let inputs_vec: Vec<&str> = parts.collect();
                        let gate_str = if inputs_vec.is_empty() {
                            format!("{} = {}", output, gate_type_raw.to_uppercase())
                        } else {
                            format!(
                                "{} = {}({})",
                                output,
                                gate_type_raw.to_uppercase(),
                                inputs_vec.join(", ")
                            )
                        };
                        gates.push(gate_str);
                    }
                }
            }
        }
    }

    let path = Path::new(filename);
    let stem = path.file_stem().unwrap().to_string_lossy();
    let output_filename = format!("{}_to_iscas89_atlanta.bench", stem);

    let mut output_file = File::create(&output_filename)?;

    for input in &inputs {
        writeln!(output_file, "INPUT({})", input)?;
    }
    for output in &outputs {
        writeln!(output_file, "OUTPUT({})", output)?;
    }
    for gate in &gates {
        writeln!(output_file, "{}", gate)?;
    }

    Ok(output_filename)
}

/*
/// Converte un file ISCAS85 in un file ISCAS89.
/// Il nuovo file verrà scritto con nome: "<filename>_to_iscas89.v"
pub fn iscas85_to89(filename: &str) -> std::io::Result<String> {
    let input_path = Path::new(filename);
    let file_stem = input_path.file_stem().unwrap().to_string_lossy();
    let output_filename = format!("{}_to_iscas89.v", file_stem);
    let mut output = File::create(&output_filename)?;

    let f = File::open(filename)?;
    let reader = BufReader::new(f);

    // Scrive header ISCAS89
    writeln!(output, "// Converted from ISCAS85 to ISCAS89")?;
    writeln!(output, "module {}_to_iscas89();", file_stem)?;

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();

        // Copio commenti ed ignoro dichiarazioni module/end
        if trimmed.starts_with("//") {
            writeln!(output, "{}", trimmed)?;
            continue;
        }
        if trimmed.starts_with("module") || trimmed.starts_with("endmodule") {
            continue;
        }

        // Converto porte ISCAS85 → ISCAS89
        if trimmed.starts_with("nand") 
            || trimmed.starts_with("and")
            || trimmed.starts_with("or")
            || trimmed.starts_with("not") {

            // Esempio linea ISCAS85:
            // nand NAND2_1 (N10, N1, N3);
            // Obiettivo ISCAS89:
            // nand NAND2_1(N10, N1, N3);

            let converted = trimmed
                .replace(" (", "(")
                .replace(" );", ");");

            writeln!(output, "  {};", converted)?;
            continue;
        }

        // Copio linee tipo "input", "output", "wire"
        if trimmed.starts_with("input")
            || trimmed.starts_with("output")
            || trimmed.starts_with("wire")
        {
            writeln!(output, "  {}", trimmed)?;
            continue;
        }
    }

    writeln!(output, "endmodule")?;

    Ok(output_filename)
}

/// Converte un file ISCAS89 in una versione combinatoria.
/// Rimuove tutti i DFF e segnali collegati ai DFF.
/// Scrive il risultato in "<filename>_comb.v".
pub fn iscas89_to_comb(filename: &str) -> std::io::Result<String> {
    let input_path = Path::new(filename);
    let file_stem = input_path.file_stem().unwrap().to_string_lossy();
    let output_filename = format!("{}_comb.v", file_stem);

    let mut output = File::create(&output_filename)?;
    let reader = BufReader::new(File::open(filename)?);

    // Per memorizzare i segnali da rimuovere (collegati ai DFF)
    let mut dff_outputs: Vec<String> = Vec::new();
    let mut dff_inputs: Vec<String> = Vec::new();

    // Prima passata: identifica i segnali collegati ai dff
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();

        if trimmed.starts_with("dff") {
            // Esempio: dff D0(CK, Q, D);
            if let Some(start) = trimmed.find('(') {
                if let Some(end) = trimmed.find(')') {
                    let inside = trimmed[start + 1..end].to_string();
                    let parts: Vec<&str> = inside.split(',').map(|s| s.trim()).collect();
                    if parts.len() == 3 {
                        dff_outputs.push(parts[1].to_string());
                        dff_inputs.push(parts[2].to_string());
                    }
                }
            }
        }
    }

    // Ri-apre file per seconda passata
    let reader = BufReader::new(File::open(filename)?);

    writeln!(output, "// ISCAS89 combinational version")?;
    writeln!(output, "module {}_comb();", file_stem)?;

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();

        // Ignora DFF
        if trimmed.starts_with("dff ") || trimmed.starts_with("dff(") {
            continue;
        }

        // Rimuove segnali che erano output o input dei DFF
        let skip = dff_outputs.iter().any(|sig| trimmed.contains(sig))
                  || dff_inputs.iter().any(|sig| trimmed.contains(sig));

        if skip {
            continue;
        }

        // Rimuove "input CK" o clock simili
        if trimmed.starts_with("input") && trimmed.contains("CK") {
            continue;
        }

        // Evita duplicare la dichiarazione del modulo originale
        if trimmed.starts_with("module") || trimmed.starts_with("endmodule") {
            continue;
        }

        // Mantiene solo porte combinatorie o dichiarazioni utili
        if trimmed.starts_with("nand")
            || trimmed.starts_with("and")
            || trimmed.starts_with("or")
            || trimmed.starts_with("not")
            || trimmed.starts_with("nor")
            || trimmed.starts_with("buf")
            || trimmed.starts_with("xor")
            || trimmed.starts_with("xnor")
            || trimmed.starts_with("input")
            || trimmed.starts_with("output")
            || trimmed.starts_with("wire")
        {
            writeln!(output, "  {}", trimmed)?;
        }
    }

    writeln!(output, "endmodule")?;
    Ok(output_filename)
}
*/
