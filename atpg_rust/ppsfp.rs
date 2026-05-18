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
    dag: &'a Dag,
    good: HashMap<String, u32>,
    faulty: HashMap<String, u32>,
    topo: Vec<usize>            // nuovo: ordine topologico ricavato da rev_adj
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

    /// XOR good/faulty finali
    pub fn diff_outputs(&self) -> u32 {
        let mut diff = 0;
        for &u in &self.topo {
            let node = &self.dag.nodes[u];
            if node.is_final {
                let out = &node.outputs[0];
                diff |= self.good[out] ^ self.faulty[out];
            }
        }
        diff
    }

    /// simulazione di più blocchi di pattern e più fault, con raccolta dei risultati in un unico passaggio
    /// TODO IMPORTANTE: implementare anche il fatto che i fault vadano diminuendo man mano che si trovano soluzioni
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

    /// util: print simulation parameters
    pub fn print(&self) {
        println!("{}", self.to_string());
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
}