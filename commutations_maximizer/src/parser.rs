use crate::netlist::{Gate, GateType, Netlist};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;

/// Parses a Verilog file and constructs a Netlist struct.
pub fn verilog_to_netlist(filename: &str) -> Netlist {
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

        if trimmed.is_empty()
            || trimmed.starts_with("//")
            || trimmed.starts_with("module")
            || trimmed.starts_with("endmodule")
        {
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
            if let Some(pos) = acc.rfind(';') {
                acc.truncate(pos);
            }
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
            if let Some(pos) = acc.rfind(';') {
                acc.truncate(pos);
            }
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
                let gate_type_str = trimmed[..start]
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim();
                let gate_type = GateType::from_str(gate_type_str);
                let inside = &trimmed[start + 1..end];
                let tokens: Vec<String> = inside
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if tokens.is_empty() {
                    i += 1;
                    continue;
                }
                let outputs = vec![tokens[0].clone()];
                let inputs: Vec<String> = if tokens.len() > 1 {
                    tokens[1..].to_vec()
                } else {
                    Vec::new()
                };
                let gate_name = outputs[0].clone();
                let gate = Gate {
                    name: gate_name,
                    gate_type,
                    outputs,
                    inputs,
                };
                netlist.gates.push(gate);
            }
        }

        i += 1;
    }

    netlist
}
