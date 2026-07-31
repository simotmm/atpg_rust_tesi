use crate::dag::Dag;
use crate::netlist::GateType;
use crate::pattern_generator::InputPattern;
use rayon::prelude::*;
use std::collections::{BTreeSet, HashMap};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

const PRINT: bool = false;
const VERBOSE: bool = PRINT && true;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fault {
    pub wire: String,
    pub sa1: bool,
    pub site: Option<String>,
}

impl Fault {
    pub fn new(wire: String, sa1: bool) -> Self {
        Fault {
            wire: wire,
            sa1: sa1,
            site: None,
        }
    }

    pub fn new_with_site(wire: String, sa1: bool, site: Option<String>) -> Self {
        Fault { wire, sa1, site }
    }

    pub fn set(&mut self, wire: String, sa1: bool) {
        self.wire = wire;
        self.sa1 = sa1;
    }

    pub fn to_string(&self) -> String {
        let s = if self.sa1 { 1 } else { 0 };
        let id = self.site.as_ref().unwrap_or(&self.wire);
        format!("{} /{}", id, s)
    }

    pub fn print(&self) {
        if !crate::options::get_options().quiet {
            println!("{}", self.to_string());
        }
    }
}

pub struct PPSFPSimulator<'a> {
    dag: &'a Dag,
    good: HashMap<String, u32>,
    faulty: HashMap<String, u32>,
    topo: Vec<usize>,
}

impl<'a> PPSFPSimulator<'a> {
    pub fn new(dag: &'a Dag) -> Self {
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
        Self {
            dag,
            good: HashMap::new(),
            faulty: HashMap::new(),
            topo,
        }
    }

    pub fn simulate_good(&mut self, patterns: &InputPattern) {
        let print = PRINT && !crate::options::get_options().quiet;
        let verbose = VERBOSE && !crate::options::get_options().quiet;
        let mut debug_string = "".to_string();
        self.good.clear();
        let mut ins: Vec<_> = patterns.values.iter().collect();
        ins.sort_by_key(|(k, _)| *k);
        let mut keys: Vec<String> = patterns.values.keys().cloned().collect();
        keys.sort();
        for name in keys {
            if let Some(&val) = patterns.values.get(&name) {
                self.good.insert(name.clone(), val);
            }
        }
        for &u in &self.topo {
            let node = &self.dag.nodes[u];
            if node.inputs.len() == 0 {
                continue;
            }
            let vals: Vec<u32> = node.inputs.iter().map(|inp| self.good[inp]).collect();
            let out = simulate_gate(node.gate_type, &vals);
            self.good.insert(node.outputs[0].clone(), out);
        }
    }

    pub fn simulate_fault(&mut self, fault: &Fault) -> u32 {
        self.faulty.clear();
        for (k, v) in &self.good {
            self.faulty.insert(k.clone(), *v);
        }
        let forced = if fault.sa1 { !0u32 } else { 0u32 };
        self.faulty.insert(fault.wire.clone(), forced);
        for &u in &self.topo {
            let node = &self.dag.nodes[u];
            if node.inputs.len() == 0 {
                continue;
            }
            if node.outputs[0] == fault.wire {
                continue;
            }
            let vals: Vec<u32> = node.inputs.iter().map(|i| self.faulty[i]).collect();
            let out = simulate_gate(node.gate_type, &vals);
            self.faulty.insert(node.outputs[0].clone(), out);
        }
        let diff = self.diff_outputs();
        diff
    }

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

    pub fn simulate_single_input(
        &mut self,
        patterns: &InputPattern,
        pos: usize,
        fault: &Fault,
    ) -> u32 {
        if pos > 31 {
            panic!("simulate_single_input: pos {} out of range", pos);
        }
        self.good.clear();
        let mut keys: Vec<String> = patterns.values.keys().cloned().collect();
        keys.sort();
        for name in keys.iter() {
            let full = *patterns.values.get(name).unwrap_or(&0u32);
            let bit = ((full >> pos) & 1) as u32;
            self.good.insert(name.clone(), bit);
        }
        for &u in &self.topo {
            let node = &self.dag.nodes[u];
            if node.inputs.len() == 0 {
                continue;
            }
            let vals: Vec<u32> = node
                .inputs
                .iter()
                .map(|inp| *self.good.get(inp).unwrap_or(&0u32))
                .collect();
            let out = simulate_gate(node.gate_type, &vals) & 1u32;
            self.good.insert(node.outputs[0].clone(), out);
        }
        self.faulty.clear();
        for (k, v) in &self.good {
            self.faulty.insert(k.clone(), *v);
        }
        let forced = if fault.sa1 { 1u32 } else { 0u32 };
        self.faulty.insert(fault.wire.clone(), forced);
        for &u in &self.topo {
            let node = &self.dag.nodes[u];
            if node.inputs.len() == 0 {
                continue;
            }
            if node.outputs[0] == fault.wire {
                continue;
            }
            let vals: Vec<u32> = node
                .inputs
                .iter()
                .map(|i| *self.faulty.get(i).unwrap_or(&0u32))
                .collect();
            let out = simulate_gate(node.gate_type, &vals) & 1u32;
            self.faulty.insert(node.outputs[0].clone(), out);
        }
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

    pub fn simulate_patterns_blocks(
        &mut self,
        patterns: Vec<InputPattern>,
        faults: Vec<Fault>,
        parallel: Option<bool>,
    ) -> (Vec<HashMap<Fault, u32>>, BTreeSet<Fault>) {
        let mut result = Vec::new();
        let mut covered_faults = BTreeSet::new();
        let mut faults_remaining: Vec<Fault> = faults;
        let all_faults: BTreeSet<Fault> = faults_remaining.iter().cloned().collect();
        let initial_total_faults = all_faults.len();
        let total_patterns = patterns.len();
        let stagnation_limit = 20usize;
        let mut consecutive_no_progress = 0usize;
        let mut prev_remaining = faults_remaining.len();
        for (pidx, pattern) in patterns.into_iter().enumerate() {
            let pnum = pidx + 1;
            if faults_remaining.is_empty() {
                result.push(HashMap::new());
                continue;
            }
            let r = if parallel.unwrap_or(false) {
                self.simulate_patterns_block_parallel(
                    &pattern,
                    &faults_remaining,
                    pnum,
                    total_patterns,
                )
            } else {
                self.simulate_patterns_block(&pattern, &faults_remaining, pnum, total_patterns)
            };
            result.push(r.clone());
            if !r.is_empty() {
                let detected_set: std::collections::HashSet<Fault> = r.keys().cloned().collect();
                for fault in detected_set.iter() {
                    covered_faults.insert(fault.clone());
                }
                faults_remaining = faults_remaining
                    .into_iter()
                    .filter(|f| !detected_set.contains(f))
                    .collect();
            }
            let new_remaining = faults_remaining.len();
            if new_remaining < prev_remaining {
                consecutive_no_progress = 0;
            } else {
                consecutive_no_progress += 1;
            }
            prev_remaining = new_remaining;
            if consecutive_no_progress >= stagnation_limit {
                break;
            }
        }
        let uncovered_faults: BTreeSet<Fault> =
            all_faults.difference(&covered_faults).cloned().collect();
        (result, uncovered_faults)
    }

    pub fn simulate_patterns_block(
        &mut self,
        patterns: &InputPattern,
        faults: &[Fault],
        pattern_idx: usize,
        total_patterns: usize,
    ) -> HashMap<Fault, u32> {
        let mut result = HashMap::new();
        self.simulate_good(patterns);
        let total_faults = faults.len();
        if total_faults == 0 {
            return result;
        }
        let print_interval = std::cmp::max(1, total_faults / 20);
        for (i, fault) in faults.iter().enumerate() {
            let idx = i + 1;
            let det = self.simulate_fault(fault);
            if det != 0 {
                result.insert(fault.clone(), det);
            }
        }
        result
    }

    pub fn simulate_patterns_block_parallel(
        &mut self,
        patterns: &InputPattern,
        faults: &[Fault],
        pattern_idx: usize,
        total_patterns: usize,
    ) -> HashMap<Fault, u32> {
        self.simulate_good(patterns);
        let topo_clone = self.topo.clone();
        let total_faults = faults.len();
        let processed = Arc::new(AtomicUsize::new(0));
        let detected = Arc::new(AtomicUsize::new(0));
        let print_interval = std::cmp::max(1, total_faults / 20);
        faults
            .par_iter()
            .filter_map(|fault| {
                let mut sim_clone = PPSFPSimulator {
                    dag: self.dag,
                    good: self.good.clone(),
                    faulty: HashMap::new(),
                    topo: topo_clone.clone(),
                };
                let det = sim_clone.simulate_fault(fault);
                let idx = processed.fetch_add(1, Ordering::SeqCst) + 1;
                if det != 0 {
                    detected.fetch_add(1, Ordering::SeqCst);
                }
                if det != 0 {
                    Some((fault.clone(), det))
                } else {
                    None
                }
            })
            .collect::<HashMap<_, _>>()
    }

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

    pub fn good_map_clone(&self) -> std::collections::HashMap<String, u32> {
        self.good.clone()
    }

    pub fn faulty_map_clone(&self) -> std::collections::HashMap<String, u32> {
        self.faulty.clone()
    }

    pub fn print(&self) {
        println!("{}", self.to_string());
    }

    pub fn most_transitions_for_compact_pattern(
        &mut self,
        pattern: &InputPattern,
    ) -> ((usize, usize), u32) {
        const BRUTE_FORCE_THRESHOLD: usize = 128;
        self.simulate_good(pattern);
        if self.topo.is_empty() || self.good.is_empty() {
            return ((0, 0), 0);
        }
        let mut keys: Vec<&String> = self.good.keys().collect();
        keys.sort();
        let s = keys.len();
        if s <= BRUTE_FORCE_THRESHOLD {
            let mut counts: Vec<Vec<u32>> = vec![vec![0u32; 32]; 32];
            for name in keys.iter() {
                let val = *self.good.get(*name).unwrap_or(&0u32);
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
        let words = (s + 63) / 64;
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

fn simulate_gate(gt: GateType, inputs: &[u32]) -> u32 {
    match gt {
        GateType::AND => inputs.iter().fold(!0u32, |acc, &x| acc & x),
        GateType::OR => inputs.iter().fold(0u32, |acc, &x| acc | x),
        GateType::NAND => !inputs.iter().fold(!0u32, |acc, &x| acc & x),
        GateType::NOR => !inputs.iter().fold(0u32, |acc, &x| acc | x),
        GateType::XOR => inputs[0] ^ inputs[1],
        GateType::XNOR => !(inputs[0] ^ inputs[1]),
        GateType::BUF => inputs[0],
        GateType::NOT => !inputs[0],
        _ => 0,
    }
}
