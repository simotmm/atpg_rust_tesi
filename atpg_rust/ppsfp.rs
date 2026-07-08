use std::collections::{HashMap, HashSet, BTreeSet};
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use crate::pattern_generator::InputPattern;
use crate::dag::Dag;
use crate::netlist::GateType;
use rayon::prelude::*; // per simulazione in parallelo su thread

const PRINT: bool = false;
const VERBOSE: bool = PRINT && true;

/// struct: fault
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fault {
    pub wire: String,
    pub sa1: bool,
    // optional site descriptor to represent source->sink style entries from Atalanta
    pub site: Option<String>,
}

/// implementation: fault
impl Fault {

    /// new fault
    pub fn new(wire: String, sa1: bool) -> Self {
        Fault { wire: wire, sa1: sa1, site: None }
    }

    /// create a fault with an explicit site descriptor (e.g. "src->dst")
    pub fn new_with_site(wire: String, sa1: bool, site: Option<String>) -> Self {
        Fault { wire, sa1, site }
    }

    /// set fault
    pub fn set(&mut self, wire: String, sa1: bool) {
        self.wire = wire;
        self.sa1 = sa1;
    }

    /// fault -> string
    pub fn to_string(&self) -> String {
        let s = if self.sa1 { 1 } else { 0 };
        let id = self.site.as_ref().unwrap_or(&self.wire);
        // Atalanta format: "ID[/->ID] /<0|1>"
        format!("{} /{}", id, s)
    }

    /// print fault 
    pub fn print(&self) {
        if !crate::options::get_options().quiet { println!("{}", self.to_string()); }
    }

}

/// struct PPSFPSimulator
pub struct PPSFPSimulator<'a> {
    dag: &'a Dag,                 // riferimento al DAG del circuito (non posseduto)
    good: HashMap<String, u32>,   // map: signal name -> value (32-bit pattern) for good circuit
    faulty: HashMap<String, u32>, // map: signal name -> value (32-bit pattern) for faulty circuit
    topo: Vec<usize>              // nuovo: ordine topologico ricavato da rev_adj
}

/// implementation: PPSFPSimulator
impl<'a> PPSFPSimulator<'a> {

    /// new simulator
    pub fn new(dag: &'a Dag) -> Self {

        // ===== costruiamo ordine topologico usando SOLO rev_adj =====
        let mut indeg = vec![0usize; dag.rev_adj.len()];
        for v in 0..dag.rev_adj.len() {
            indeg[v] = dag.rev_adj[v].len();
        }

        let mut q = std::collections::VecDeque::new();
        for i in 0..indeg.len() {
            if indeg[i] == 0 {
                q.push_back(i);
            }
        }

        let mut topo = Vec::new();
        while let Some(u) = q.pop_front() {
            topo.push(u);
            for &s in &dag.succ_adj[u] {
                indeg[s] -= 1;
                if indeg[s] == 0 {
                    q.push_back(s);
                }
            }
        }

        Self { dag, good: HashMap::new(), faulty: HashMap::new(), topo }
    }

    /// simulate good circuit
    pub fn simulate_good(&mut self, patterns: &InputPattern) {
        let print = PRINT && !crate::options::get_options().quiet;
        let verbose = VERBOSE && !crate::options::get_options().quiet;
        let mut debug_string = "".to_string();
        self.good.clear();

        if print {
            debug_string.push_str("good circuit simulation:\n");
        }

        // ordina gli input per coerenza visiva
        let mut ins: Vec<_> = patterns.values.iter().collect();
        ins.sort_by_key(|(k, _)| *k);

        if print {
            debug_string.push_str("  input pattern:\n");
            for (name, val) in &ins {
                debug_string.push_str(&format!("  {:>10}: {:032b}\n", name, val));
            }
        }

        // assegna input in ordine deterministico
        let mut keys: Vec<String> = patterns.values.keys().cloned().collect();
        keys.sort();
        for name in keys {
            if let Some(&val) = patterns.values.get(&name) {
                self.good.insert(name.clone(), val);
            }
        }

        if print && verbose {
            debug_string.push_str("  intermediate nodes output:\n");
        }

        // ========== simulazione in ordine topologico (NO dag.nodes) ==========
        for &u in &self.topo {
            let node = &self.dag.nodes[u]; // necessario solo per leggere gate_type e nomi
            if node.inputs.len() == 0 {
                continue; // input primario
            }

            let vals: Vec<u32> = node.inputs
                .iter()
                .map(|inp| self.good[inp])
                .collect();

            let out = simulate_gate(node.gate_type, &vals);
            self.good.insert(node.outputs[0].clone(), out);

            if print && !node.is_final && verbose {
                debug_string.push_str(&format!("  {:>10}: {:032b}\n", node.outputs[0], out));
            }
        }

        if print {
            debug_string.push_str("  final outputs:\n");
            for &u in &self.topo {
                let node = &self.dag.nodes[u];
                if node.is_final {
                    let out = self.good[&node.outputs[0]];
                    debug_string.push_str(&format!("  {:>10}: {:032b}\n", node.outputs[0], out));
                }
            }
        }
        if print { print!("{debug_string}"); }
    }

    /// simulate faulted circuit
    pub fn simulate_fault(&mut self, fault: &Fault) -> u32 {
        let print = PRINT && !crate::options::get_options().quiet;
        let verbose = VERBOSE && !crate::options::get_options().quiet;
        let mut debug_string = "".to_string();

        self.faulty.clear();
        for (k, v) in &self.good {
            self.faulty.insert(k.clone(), *v);
        }

        let forced = if fault.sa1 { !0u32 } else { 0u32 };
        self.faulty.insert(fault.wire.clone(), forced);

        if print {
            debug_string.push_str("\nfault circuit simulation:\n");
            debug_string.push_str(&format!("  fault: {} stuck-at-{}\n",
                fault.wire,
                if fault.sa1 {1} else {0}));
        }

        // Recompute faulty outputs in topological order to ensure all inputs
        // for a node are correctly updated before evaluation. This avoids
        // incorrect results caused by a DFS order that may evaluate a node
        // before all its predecessors have been updated.
        for &u in &self.topo {
            let node = &self.dag.nodes[u];
            if node.inputs.len() == 0 { continue; }

            // the start node (fault.wire) was forced above; skip recomputing it
            if node.outputs[0] == fault.wire { continue; }

            let vals: Vec<u32> = node.inputs.iter().map(|i| self.faulty[i]).collect();
            let out = simulate_gate(node.gate_type, &vals);
            self.faulty.insert(node.outputs[0].clone(), out);

            if print && verbose && !node.is_final {
                debug_string.push_str(&format!("  {:>10}: {:032b}\n",
                    node.outputs[0], out));
            }
        }

        // stampa finali
        if print {
            debug_string.push_str("  final outputs:\n");
            for &u in &self.topo {
                let node = &self.dag.nodes[u];
                if node.is_final {
                    let out = self.faulty[&node.outputs[0]];
                    debug_string.push_str(&format!("  {:>10}: {:032b}\n", node.outputs[0], out));
                }
            }
        }

        let diff = self.diff_outputs();

        if print {
            debug_string.push_str("  output difference (XOR(good, faulty)):\n");
            debug_string.push_str(&format!("  {:>10}: {:032b}\n", "XOR", diff));
        }

        if print { print!("{debug_string}"); }

        diff
    }

    /// XOR good/faulty final outputs (full 32-bit mask)
    pub fn diff_outputs(&self) -> u32 {
        let mut diff = 0u32;
        for &u in &self.topo {
            let node = &self.dag.nodes[u];
            if node.is_final {
                let out = &node.outputs[0];
                diff |= self.good[out] ^ self.faulty[out];
            }
        }
        diff
    }

    /// Simulate the circuit for a single input position (bit) inside a packed `InputPattern`.
    /// `pos` is the bit position (0 = LSB, 31 = MSB). Returns 0 if no output difference
    /// at that position, or 1 if any final output differs at that bit.
    pub fn simulate_single_input(&mut self, patterns: &InputPattern, pos: usize, fault: &Fault) -> u32 {
        if pos > 31 { panic!("simulate_single_input: pos {} out of range", pos); }

        // Build good map with single-bit values (0 or 1) extracted from packed pattern
        self.good.clear();
        let mut keys: Vec<String> = patterns.values.keys().cloned().collect();
        keys.sort();
        for name in keys.iter() {
            let full = *patterns.values.get(name).unwrap_or(&0u32);
            let bit = ((full >> pos) & 1) as u32;
            self.good.insert(name.clone(), bit);
        }

        // Propagate good values through topo
        for &u in &self.topo {
            let node = &self.dag.nodes[u];
            if node.inputs.len() == 0 { continue; }
            let vals: Vec<u32> = node.inputs.iter().map(|inp| *self.good.get(inp).unwrap_or(&0u32)).collect();
            let out = simulate_gate(node.gate_type, &vals) & 1u32; // keep single bit
            self.good.insert(node.outputs[0].clone(), out);
        }

        // Build faulty map starting from good
        self.faulty.clear();
        for (k, v) in &self.good { self.faulty.insert(k.clone(), *v); }

        // Force the fault wire at the single-bit value
        let forced = if fault.sa1 { 1u32 } else { 0u32 };
        self.faulty.insert(fault.wire.clone(), forced);

        // Recompute faulty outputs in topo order (skip the node that is the fault wire output)
        for &u in &self.topo {
            let node = &self.dag.nodes[u];
            if node.inputs.len() == 0 { continue; }
            if node.outputs[0] == fault.wire { continue; }
            let vals: Vec<u32> = node.inputs.iter().map(|i| *self.faulty.get(i).unwrap_or(&0u32)).collect();
            let out = simulate_gate(node.gate_type, &vals) & 1u32;
            self.faulty.insert(node.outputs[0].clone(), out);
        }

        // Compute diff across final outputs, but only for this single bit (values are 0/1)
        let mut diff_bit: u32 = 0;
        for &u in &self.topo {
            let node = &self.dag.nodes[u];
            if node.is_final {
                let g = *self.good.get(&node.outputs[0]).unwrap_or(&0u32);
                let f = *self.faulty.get(&node.outputs[0]).unwrap_or(&0u32);
                diff_bit |= g ^ f;
            }
        }
        diff_bit & 1u32
    }

    /// simulazione di più blocchi di pattern e più fault, con raccolta dei risultati in un unico passaggio
    pub fn simulate_patterns_blocks(&mut self, patterns: Vec<InputPattern>, faults: Vec<Fault>, parallel: Option<bool>) -> (Vec<HashMap<Fault, u32>>, BTreeSet<Fault>) {
        let mut result = Vec::new();
        let mut covered_faults = BTreeSet::new(); // Set per i fault coperti (deterministico)

        // Manteniamo una lista mutabile di fault rimanenti e una copia dell'insieme
        // originale per calcolare i non-coperti in modo deterministico.
        let mut faults_remaining: Vec<Fault> = faults;
        let all_faults: BTreeSet<Fault> = faults_remaining.iter().cloned().collect();
        let initial_total_faults = all_faults.len();

        // Simulazione per ogni pattern
        let total_patterns = patterns.len();

        // Track stagnation: if the number of remaining faults does not
        // decrease for `stagnation_limit` consecutive patterns, stop early.
        let stagnation_limit = 20usize;
        let mut consecutive_no_progress = 0usize;
        let mut prev_remaining = faults_remaining.len();

        for (pidx, pattern) in patterns.into_iter().enumerate() {
            let pnum = pidx + 1;
            if !crate::options::get_options().quiet && crate::PRINT_PROGRESS { println!("PPSFP: starting pattern {}/{}", pnum, total_patterns); }

            // se non ci sono più fault da testare, aggiungiamo mappe vuote e saltiamo
            if faults_remaining.is_empty() {
                result.push(HashMap::new());
                if !crate::options::get_options().quiet && crate::PRINT_PROGRESS { println!("PPSFP: no remaining faults; skipping pattern {}/{}", pnum, total_patterns); }
                continue;
            }

            let r = if parallel.unwrap_or(false) {
                self.simulate_patterns_block_parallel(&pattern, &faults_remaining, pnum, total_patterns) // parallelo
            } else {
                self.simulate_patterns_block(&pattern, &faults_remaining, pnum, total_patterns) // sequenziale
            };

            // salva risultati per questo pattern
            result.push(r.clone());

            // Aggiungere i fault coperti da questa simulazione al set
            if !r.is_empty() {
                let detected_set: std::collections::HashSet<Fault> = r.keys().cloned().collect();
                for fault in detected_set.iter() {
                    covered_faults.insert(fault.clone());
                }

                // rimuovi i fault già rilevati dalla lista dei rimanenti
                faults_remaining = faults_remaining.into_iter().filter(|f| !detected_set.contains(f)).collect();
            }

            let covered_count = covered_faults.len();
            let total_faults = initial_total_faults;
            let covered_pct = if total_faults > 0 { (covered_count as f64) * 100.0 / (total_faults as f64) } else { 0.0 };
            if !crate::options::get_options().quiet && crate::PRINT_PROGRESS { println!("PPSFP: pattern {}/{} done - detected in this pattern: {} - cumulative detected: {}/{} ({:.1}%)", pnum, total_patterns, result.last().map(|m| m.len()).unwrap_or(0), covered_count, total_faults, covered_pct); }

            // Check for stagnation: stop early if no remaining faults have
            // been removed for `stagnation_limit` consecutive patterns.
            let new_remaining = faults_remaining.len();
            if new_remaining < prev_remaining {
                consecutive_no_progress = 0;
            } else {
                consecutive_no_progress += 1;
            }
            prev_remaining = new_remaining;

            if consecutive_no_progress >= stagnation_limit {
                if !crate::options::get_options().quiet && crate::PRINT_PROGRESS {
                    println!("PPSFP: no progress for {} consecutive patterns - stopping early at pattern {}/{}", consecutive_no_progress, pnum, total_patterns);
                }
                break;
            }
        }

        // Fault non coperti (ordine deterministico usando BTreeSet)
        let uncovered_faults: BTreeSet<Fault> = all_faults.difference(&covered_faults).cloned().collect();

        (result, uncovered_faults)
    }

    /// simulazione multipattern (sequenziale)
    pub fn simulate_patterns_block(&mut self, patterns: &InputPattern, faults: &[Fault], pattern_idx: usize, total_patterns: usize) -> HashMap<Fault, u32> {
        let mut result = HashMap::new();
        self.simulate_good(patterns);

        let total_faults = faults.len();
        if total_faults == 0 { return result; }

        // throttle prints to reduce verbosity: print ~20 updates per pattern
        let print_interval = std::cmp::max(1, total_faults / 20);
        for (i, fault) in faults.iter().enumerate() {
            let idx = i + 1;
            let det = self.simulate_fault(fault);
            if det != 0 { // voglio che result contenga i pattern solo per i fault che vengono rilevati
                result.insert(fault.clone(), det);
            }

            // print only on interval or on final item
            if !crate::options::get_options().quiet && crate::PRINT_PROGRESS && (idx % print_interval == 0 || idx == total_faults) {
                let percent = (idx as f64) * 100.0 / (total_faults as f64);
                let detected_count = result.len();
                let detected_pct = (detected_count as f64) * 100.0 / (total_faults as f64);
                println!("PPSFP pattern {}/{} - fault {}/{} ({}, s-a-{}) - processed {:.1}% - detected {}/{} ({:.1}%)",
                    pattern_idx, total_patterns, idx, total_faults, fault.wire, if fault.sa1 {1} else {0}, percent, detected_count, total_faults, detected_pct);
            }
        }

        result
    }

    /// simulazione multipattern parallela per fault (ottimizzato con rayon)
    pub fn simulate_patterns_block_parallel(&mut self, patterns: &InputPattern, faults: &[Fault], pattern_idx: usize, total_patterns: usize) -> HashMap<Fault, u32> {

        // calcolo circuito good UNA sola volta
        self.simulate_good(patterns);

        // topo viene copiato nei simulatore locali
        let topo_clone = self.topo.clone();

        let total_faults = faults.len();
        let processed = Arc::new(AtomicUsize::new(0));
        let detected = Arc::new(AtomicUsize::new(0));
        // throttle prints to reduce verbosity: print ~20 updates per pattern
        let print_interval = std::cmp::max(1, total_faults / 20);

        faults
            .par_iter()
            .filter_map(|fault| {

                // ogni thread costruisce il suo simulatore
                let mut sim_clone = PPSFPSimulator {
                    dag: self.dag,
                    good: self.good.clone(),
                    faulty: HashMap::new(),
                    topo: topo_clone.clone()   
                };

                let det = sim_clone.simulate_fault(fault);

                let idx = processed.fetch_add(1, Ordering::SeqCst) + 1;
                if det != 0 { detected.fetch_add(1, Ordering::SeqCst); }

                let proc = idx;
                let pct = if total_faults > 0 { (proc as f64) * 100.0 / (total_faults as f64) } else { 0.0 };
                let det_cnt = detected.load(Ordering::SeqCst);
                let det_pct = if total_faults > 0 { (det_cnt as f64) * 100.0 / (total_faults as f64) } else { 0.0 };
                if !crate::options::get_options().quiet && crate::PRINT_PROGRESS && (proc % print_interval == 0 || proc == total_faults) {
                    println!("PPSFP parallel pattern {}/{} - processed {}/{} ({:.1}%) - detected {}/{} ({:.1}%) - last fault {} s-a-{}",
                        pattern_idx, total_patterns, proc, total_faults, pct, det_cnt, total_faults, det_pct, fault.wire, if fault.sa1 {1} else {0});
                }
                //per debug: benchmark
                println!("PPSFP parallel pattern {}/{} - processed {}/{} ({:.1}%) - detected {}/{} ({:.1}%) - last fault {} s-a-{}",
                    pattern_idx, total_patterns, proc, total_faults, pct, det_cnt, total_faults, det_pct, fault.wire, if fault.sa1 {1} else {0});

                if det != 0 { Some((fault.clone(), det)) } else { None }
            })
            .collect::<HashMap<_, _>>()
    }

    /// util: stringa della rappresentazione testuale dei pattern simulati
    pub fn to_string(&self) -> String {
        let mut s = String::new();
        s.push_str("PPSFP simulation results:\n");
        s.push_str("  good circuit patterns:\n");
        let mut good_entries: Vec<_> = self.good.iter().collect();
        good_entries.sort_by_key(|(k, _)| *k);
        for (input, pattern) in good_entries {
            s.push_str(&format!("    {} -> {:032b}\n", input, pattern));
        }
        s.push_str("  faulty circuit patterns:\n");
        let mut faulty_entries: Vec<_> = self.faulty.iter().collect();
        faulty_entries.sort_by_key(|(k, _)| *k);
        for (input, pattern) in faulty_entries {
            s.push_str(&format!("    {} -> {:032b}\n", input, pattern));
        }
        s
    }

    /// Return a clone of the `good` map (input/output values after last `simulate_good`)
    pub fn good_map_clone(&self) -> std::collections::HashMap<String,u32> {
        self.good.clone()
    }

    /// Return a clone of the `faulty` map (values after last `simulate_fault`)
    pub fn faulty_map_clone(&self) -> std::collections::HashMap<String,u32> {
        self.faulty.clone()
    }

    /// util: print simulation parameters
    pub fn print(&self) {
        println!("{}", self.to_string());
    }

    /// Given a compact `InputPattern` (packed 32 combinations per u32),
    /// find the pair of input combination positions (p, q) that maximizes
    /// the number of signal changes between those two positions and return
    /// the pair together with the count of changing signals.
    /// Returns `((p, q), count)` where `p` and `q` are 0-based positions
    /// in the compact pattern (range 0..31). 
    /// If no nodes/signals exist, returns `((0,0), 0)`.
    /// This simulates the good circuit to populate `self.good` and then counts
    /// the number of signals that differ between the two input combinations at positions p and q.
    pub fn most_transitions_for_compact_pattern(&mut self, pattern: &InputPattern) -> ((usize, usize), u32) {
        // If the number of signals (S) is below this threshold, use the simple
        // per-signal brute-force counting instead of building bitsets.
        const BRUTE_FORCE_THRESHOLD: usize = 128;

        // simulate packed (bit-parallel) good circuit
        self.simulate_good(pattern);

        // If no nodes, nothing to count
        if self.topo.is_empty() || self.good.is_empty() {
            return ((0, 0), 0);
        }

        // Prepare ordered list of signals so indices are deterministic
        let mut keys: Vec<&String> = self.good.keys().collect();
        keys.sort();
        let s = keys.len();

        // If the signal count is small, prefer a simple per-signal brute-force
        // loop to avoid the overhead of allocating bitsets.
        if s <= BRUTE_FORCE_THRESHOLD {
            // counts[p][q] for p<q
            let mut counts: Vec<Vec<u32>> = vec![vec![0u32; 32]; 32];
            for name in keys.iter() {
                let val = *self.good.get(*name).unwrap_or(&0u32); // val: 
                for p in 0..32 {
                    let bp = (val >> p) & 1;
                    for q in (p + 1)..32 {
                        let bq = (val >> q) & 1;
                        if bp != bq {
                            counts[p][q] += 1;
                        }
                    }
                }
            }

            let mut best_p = 0usize;
            let mut best_q = 0usize;
            let mut best_count: u32 = 0;
            for p in 0..32 {
                for q in (p + 1)..32 {
                    let c = counts[p][q];
                    if c > best_count {
                        best_count = c;
                        best_p = p;
                        best_q = q;
                    }
                }
            }
            return ((best_p, best_q), best_count);
        }

        // Otherwise build word-parallel bitsets for efficiency
        let words = (s + 63) / 64; // number of u64 words needed

        // Build bitsets: for each local bit position p (0..31) we keep a Vec<u64>
        // where bit i indicates whether signal i has bit p == 1.
        let mut bitsets: Vec<Vec<u64>> = vec![vec![0u64; words]; 32];
        for (idx, name) in keys.iter().enumerate() {
            let val = *self.good.get(*name).unwrap_or(&0u32);
            let w = idx / 64;
            let b = (idx % 64) as u64;
            let mask = 1u64 << b;
            for p in 0..32 {
                if ((val >> p) & 1) != 0 {
                    bitsets[p][w] |= mask;
                }
            }
        }

        // Now compute pairwise popcount of XOR between bitsets[p] and bitsets[q]
        let mut best_p = 0usize;
        let mut best_q = 0usize;
        let mut best_count: u32 = 0;
        for p in 0..32 {
            for q in (p + 1)..32 {
                let mut cnt: u32 = 0;
                for w in 0..words {
                    cnt = cnt.wrapping_add((bitsets[p][w] ^ bitsets[q][w]).count_ones());
                }
                if cnt > best_count {
                    best_count = cnt;
                    best_p = p;
                    best_q = q;
                }
            }
        }

        ((best_p, best_q), best_count)
    }


}

/// gate simulation: (gate, inputs(bits)) -> result (bits)
fn simulate_gate(gt: GateType, inputs: &[u32]) -> u32 {
    match gt {
        GateType::AND  =>   inputs.iter().fold(!0u32, |acc, &x| acc & x),
        GateType::OR   =>   inputs.iter().fold( 0u32, |acc, &x| acc | x),
        GateType::NAND =>  !inputs.iter().fold(!0u32, |acc, &x| acc & x),
        GateType::NOR  =>  !inputs.iter().fold( 0u32, |acc, &x| acc | x),
        GateType::XOR  =>   inputs[0] ^ inputs[1],
        GateType::XNOR => !(inputs[0] ^ inputs[1]),
        GateType::BUF  =>   inputs[0],
        GateType::NOT  =>  !inputs[0],
        _              =>   0
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::netlist::{Netlist, Gate, GateType};
    use crate::pattern_generator::InputPattern;

    fn simple_buf_netlist() -> Netlist {
        let mut nl = Netlist::new();
        nl.inputs = vec!["IN".to_string()];
        nl.outputs = vec!["OUT".to_string()];
        let g = Gate { name: "G1".to_string(), gate_type: GateType::BUF, outputs: vec!["OUT".to_string()], inputs: vec!["IN".to_string()] };
        nl.gates.push(g);
        nl
    }

    #[test]
    fn test_simulator_good_and_faulted() {
        let nl = simple_buf_netlist();
        let dag = crate::dag::Dag::from_netlist(&nl);
        let mut sim = PPSFPSimulator::new(&dag);

        // create pattern IN=0
        let mut pat = InputPattern::from_strings(vec!["IN".to_string()]);
        pat.insert_pattern("0");

        sim.simulate_good(&pat);
        // good output should be 0
        assert_eq!(sim.good.get("OUT").cloned().unwrap_or(0), 0u32);

        // fault on OUT stuck-at-1 should be detected
        let f = Fault::new("OUT".to_string(), true);
        let det = sim.simulate_fault(&f);
        assert!(det != 0);
    }

    #[test]
    fn test_most_transitions_for_compact_pattern_simple() {
        // small netlist: two inputs A,B -> two outputs O1,O2 via BUFs
        let mut nl = Netlist::new();
        nl.inputs = vec!["A".to_string(), "B".to_string()];
        nl.outputs = vec!["O1".to_string(), "O2".to_string()];
        let g1 = Gate { name: "G1".to_string(), gate_type: GateType::BUF, outputs: vec!["O1".to_string()], inputs: vec!["A".to_string()] };
        let g2 = Gate { name: "G2".to_string(), gate_type: GateType::BUF, outputs: vec!["O2".to_string()], inputs: vec!["B".to_string()] };
        nl.gates.push(g1);
        nl.gates.push(g2);

        let dag = crate::dag::Dag::from_netlist(&nl);
        let mut sim = PPSFPSimulator::new(&dag);

        let mut pat = InputPattern::from_strings(vec!["A".to_string(), "B".to_string()]);
        // set position 1 (bit 1) to 1 for both inputs; position 0 remains 0
        pat.set("A", 1u32 << 1);
        pat.set("B", 1u32 << 1);

        let ((p, q), count) = sim.most_transitions_for_compact_pattern(&pat);
        assert_eq!((p, q), (0usize, 1usize));
        // signals: A,B,O1,O2 => all four flip between pos 0 and 1
        assert_eq!(count, 4u32);
    }

    #[test]
    fn test_most_transitions_for_compact_pattern_large_bitset() {
        // build a larger netlist to exercise the bitset code path
        let n_inputs = 130usize; // > BRUTE_FORCE_THRESHOLD (128)
        let mut nl = Netlist::new();
        for i in 0..n_inputs {
            nl.inputs.push(format!("I{}", i));
            nl.outputs.push(format!("O{}", i));
            let gate = Gate { name: format!("G{}", i), gate_type: GateType::BUF, outputs: vec![format!("O{}", i)], inputs: vec![format!("I{}", i)] };
            nl.gates.push(gate);
        }

        let dag = crate::dag::Dag::from_netlist(&nl);
        let mut sim = PPSFPSimulator::new(&dag);

        let inputs: Vec<String> = (0..n_inputs).map(|i| format!("I{}", i)).collect();
        let mut pat = InputPattern::from_strings(inputs.clone());

        // set bit 1 to 1 for every input (pos 0 = 0, pos 1 = 1)
        for name in inputs.iter() {
            pat.set(name, 1u32 << 1);
        }

        let ((p, q), count) = sim.most_transitions_for_compact_pattern(&pat);
        assert_eq!((p, q), (0usize, 1usize));
        // total signals = inputs + outputs
        let total_signals = (n_inputs * 2) as u32;
        assert_eq!(count, total_signals);
    }
}