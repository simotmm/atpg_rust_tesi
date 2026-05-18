//nome iniziale del modulo: "dag_rev_adj_opt.rs"
//parte dall'idea del paper di avere le adiacenze dei predecessori
//integra anche le adiacenze dei successori per una fault injection O(n)
use std::collections::{HashMap, HashSet};
use crate::netlist::{GateType, Netlist};
use crate::ppsfp::Fault;
use crate::cnf::{CNF, Literal, gate_to_cnf_literals, get_boolean_difference_clauses};  // Usa le strutture e le funzioni da cnf.rs

const PRINT: bool = false;
const PRINT_ADJ_LISTS: bool = false;

/// struct: dagnode
#[derive(Debug, Clone)]
pub struct DagNode {
    pub id: usize,
    pub name: String,
    pub gate_type: GateType,
    pub outputs: Vec<String>,
    pub inputs: Vec<String>,
    pub cnf: CNF,
    pub pre_fault_cnf: Option<String>,
    pub is_final: bool,
}

/// struct: dag
#[derive(Debug, Clone)]
pub struct Dag {
    pub nodes: Vec<DagNode>,
    pub rev_adj: Vec<Vec<usize>>,  // predecessori (per cone bottom up)
    pub succ_adj: Vec<Vec<usize>>, // successori (per fault injection O(n))
    pub affected: Vec<usize>       // per test formula
}

/// imlplementation: dag
impl Dag {

    /// netlist -> dag
    pub fn from_netlist(nl: &Netlist) -> Self {
        let final_outputs: HashSet<String> = nl.outputs.iter().cloned().collect();
        let mut nodes = Vec::with_capacity(nl.gates.len());
        let mut id_map = HashMap::<String, usize>::new();

        for (i, g) in nl.gates.iter().enumerate() {
            let out = g.outputs[0].clone();
            id_map.insert(out.clone(), i);

            nodes.push(DagNode {
                id: i,
                name: g.name.clone(),
                gate_type: g.gate_type.clone(),
                outputs: g.outputs.clone(),
                inputs: g.inputs.clone(),
                cnf: gate_to_cnf_literals(g.gate_type, &out, &g.inputs),
                pre_fault_cnf: None,
                is_final: g.outputs.iter().any(|o| final_outputs.contains(o))
            });
        }

        let n = nodes.len();
        let mut rev_adj: Vec<Vec<usize>> = vec![vec![]; n];
        let mut succ_adj: Vec<Vec<usize>> = vec![vec![]; n];
        let affected: Vec<usize> = vec![];

        for (i, g) in nl.gates.iter().enumerate() {
            for inp in &g.inputs {
                if let Some(&src) = id_map.get(inp) {
                    rev_adj[i].push(src);  // predecessore
                    succ_adj[src].push(i); // successore
                }
            }
        }

        Dag { nodes, rev_adj, succ_adj, affected }
    }

    /// Evaluate the DAG boolean outputs given a partial assignment of primary inputs
    /// `primary_assign` maps primary input names -> bool. If `forced` is Some((wire, val))
    /// the node whose output name matches `wire` will be forced to `val` (used for stuck-at).
    /// Returns a map var -> bool with computed values for all node outputs (and primary inputs).
    pub fn evaluate_boolean(&self, primary_assign: &std::collections::HashMap<String, bool>, forced: Option<(String, bool)>) -> std::collections::HashMap<String, bool> {
        use std::collections::VecDeque;
        let mut result: std::collections::HashMap<String, bool> = std::collections::HashMap::new();

        // initialize primary inputs
        for (k, &v) in primary_assign.iter() {
            result.insert(k.clone(), v);
        }

        // build topo order using rev_adj as indegree
        let mut indeg = vec![0usize; self.rev_adj.len()];
        for v in 0..self.rev_adj.len() {
            indeg[v] = self.rev_adj[v].len();
        }
        let mut q = VecDeque::new();
        for i in 0..indeg.len() {
            if indeg[i] == 0 {
                q.push_back(i);
            }
        }
        let mut topo: Vec<usize> = Vec::new();
        while let Some(u) = q.pop_front() {
            topo.push(u);
            for &s in &self.succ_adj[u] {
                indeg[s] = indeg[s].saturating_sub(1);
                if indeg[s] == 0 { q.push_back(s); }
            }
        }

        // evaluate nodes in topo order
        for &u in &topo {
            let node = &self.nodes[u];
            if node.inputs.len() == 0 { continue; }

            // if this node's output is forced, apply forced value
            if let Some((ref wire, val)) = forced {
                if wire == &node.outputs[0] {
                    result.insert(node.outputs[0].clone(), val);
                    continue;
                }
            }

            // gather input boolean values (default false if missing)
            let mut in_vals: Vec<bool> = Vec::with_capacity(node.inputs.len());
            for inp in &node.inputs {
                let v = *result.get(inp).unwrap_or(&false);
                in_vals.push(v);
            }

            let out = match node.gate_type {
                crate::netlist::GateType::AND  => in_vals.iter().fold(true, |acc, &b| acc & b),
                crate::netlist::GateType::OR   => in_vals.iter().fold(false, |acc, &b| acc | b),
                crate::netlist::GateType::NAND => !in_vals.iter().fold(true, |acc, &b| acc & b),
                crate::netlist::GateType::NOR  => !in_vals.iter().fold(false, |acc, &b| acc | b),
                crate::netlist::GateType::XOR  => { if in_vals.len() >= 2 { in_vals[0] ^ in_vals[1] } else { false } },
                crate::netlist::GateType::XNOR => { if in_vals.len() >= 2 { !(in_vals[0] ^ in_vals[1]) } else { false } },
                crate::netlist::GateType::BUF  => { in_vals.get(0).cloned().unwrap_or(false) },
                crate::netlist::GateType::NOT  => { !in_vals.get(0).cloned().unwrap_or(false) },
                _ => false,
            };

            result.insert(node.outputs[0].clone(), out);
        }

        result
    }

    /// Create a primed clone of the DAG where every node's output signal
    /// name is renamed by appending an apostrophe (') and internal inputs
    /// that refer to node outputs are updated accordingly. Primary inputs
    /// (signals that are not outputs of any node) are left unchanged.
    pub fn primed_clone(&self) -> Dag {
        // Build rename map: original output -> primed output
        let mut rename: HashMap<String, String> = HashMap::new();
        for node in &self.nodes {
            let orig = node.outputs[0].clone();
            rename.insert(orig.clone(), format!("{}'", orig));
        }

        // Clone the DAG and apply renaming
        let mut primed = self.clone();
        for node in &mut primed.nodes {
            // rename output
            if let Some(new_out) = rename.get(&node.outputs[0]) {
                node.outputs[0] = new_out.clone();
            }
            // rename inputs when they refer to other node outputs
            for inp in &mut node.inputs {
                if let Some(new_inp) = rename.get(inp) {
                    *inp = new_inp.clone();
                }
            }
            // recompute CNF for the renamed node
            node.cnf = gate_to_cnf_literals(node.gate_type, &node.outputs[0].clone(), &node.inputs);
        }

        primed
    }

    /// print dag
    pub fn print(&self) {
        println!("directed acyclic graph:");
        println!("node list ({}):", self.nodes.len());
        let mut i = 1;
        for n in &self.nodes {
            let row = if let Some(ref pf) = n.pre_fault_cnf {
                format!("{i}: [id:{}] {:2} -> {:?}, cnf: {} -> faulted -> {}",n.id, n.name, n.outputs, pf, n.cnf.to_string())
            } else {
                format!("{i}: [id:{}] {:2} -> {:?}, cnf: {}", n.id, n.name, n.outputs, n.cnf.to_string())
            };
            println!("  {}", row);
            i += 1;
        }
        println!();
        
        if PRINT_ADJ_LISTS {
            println!("reversed adjacency list (predecessors):");
            for (u, list) in self.rev_adj.iter().enumerate() {
                println!("  {}: {:?}", u, list);
            }
            println!("successor adjacency list:");
            for (u, list) in self.succ_adj.iter().enumerate() {
                println!("  {}: {:?}", u, list);
            }
        }
        
    }

    /// CNF minimale possibile per il circuito GOOD, senza includere nodi non necessari
    pub fn cnf_cone_from_outputs(&self) -> CNF {
        let mut clauses = Vec::new();

        let mut stack: Vec<usize> = self.nodes
            .iter()
            .filter(|n| n.is_final)
            .map(|n| n.id)
            .collect();

        let mut visited = vec![false; self.nodes.len()];

        while let Some(u) = stack.pop() {
            if visited[u] { continue; }
            visited[u] = true;

            // aggiungi CNF del nodo
            clauses.extend(self.nodes[u].cnf.clauses.clone());

            // risali i predecessori
            for &p in &self.rev_adj[u] {
                stack.push(p);
            }
        }

        CNF { clauses }
    }

    /// CNF "faulted" ridotta al suo cone-of-influence
    pub fn cnf_cone_faulted(&self) -> CNF {
        let mut clauses = Vec::new();
        for &id in &self.affected {
            clauses.extend(self.nodes[id].cnf.clauses.clone());
        }
        CNF { clauses }
    }

    /// dag -> fault list
    pub fn generate_fault_list(&self, fault_wire: Option<String>, sa1: Option<bool>) -> Vec<Fault> {
        let faults = if fault_wire.is_some() && self.wire_exists(&fault_wire.clone().unwrap()) { 
            let mut v : Vec<Fault> = vec![];
            if sa1.is_none() {
                v.push(Fault::new(fault_wire.clone().unwrap(), true));
                v.push(Fault::new(fault_wire.clone().unwrap(), false));
            }
            else{
                v.push(Fault::new(fault_wire.unwrap(), sa1.unwrap()));
            }
            v
        }
        else { 
            self.all_faults() 
        };
        faults
    }

    /// true se esiste un wire con quel nome nel dag (cioè come output di un nodo)
    fn wire_exists(&self, wire: &str) -> bool {
        self.nodes.iter().any(|n| n.outputs.get(0).map_or(false, |o| o == wire))
    }

    /// util: lista di tutti i fault
    fn all_faults(&self) -> Vec<Fault> {
        // Re-implement fault enumeration to match Atalanta's FFR/stem traversal
        // Build producer map and consumer (fanout) counts for each signal
        let mut producer: HashMap<String, usize> = HashMap::new();
        for node in &self.nodes {
            producer.insert(node.outputs[0].clone(), node.id);
        }

        // consumer count (fanout) for any signal name
        let mut fanout_count: HashMap<String, usize> = HashMap::new();
        for node in &self.nodes {
            for inp in &node.inputs {
                *fanout_count.entry(inp.clone()).or_insert(0) += 1;
            }
        }

        // per-gate pfault lists (ordered like Atalanta) stored as Fault objects
        let mut pfaults_per_gate: Vec<Vec<crate::ppsfp::Fault>> = vec![Vec::new(); self.nodes.len()];

        for (idx, node) in self.nodes.iter().enumerate() {
            let w = node.outputs[0].clone();
            let out_count = self.succ_adj[idx].len();

            // OUTPUT faults
            if out_count > 1 {
                pfaults_per_gate[idx].push(crate::ppsfp::Fault::new(w.clone(), false));
                pfaults_per_gate[idx].push(crate::ppsfp::Fault::new(w.clone(), true));
            } else if out_count == 1 {
                let sink = self.succ_adj[idx][0];
                let sink_node = &self.nodes[sink];

                if sink_node.inputs.len() > 1 || node.is_final {
                    match sink_node.gate_type {
                        crate::netlist::GateType::XOR | crate::netlist::GateType::XNOR => {
                            pfaults_per_gate[idx].push(crate::ppsfp::Fault::new(w.clone(), false));
                            pfaults_per_gate[idx].push(crate::ppsfp::Fault::new(w.clone(), true));
                        }
                        crate::netlist::GateType::OR | crate::netlist::GateType::NOR => {
                            pfaults_per_gate[idx].push(crate::ppsfp::Fault::new(w.clone(), false)); // SA0
                        }
                        _ => {
                            pfaults_per_gate[idx].push(crate::ppsfp::Fault::new(w.clone(), true)); // SA1
                        }
                    }
                } else {
                    // conservatively add both
                    pfaults_per_gate[idx].push(crate::ppsfp::Fault::new(w.clone(), false));
                    pfaults_per_gate[idx].push(crate::ppsfp::Fault::new(w.clone(), true));
                }
            } else {
                // no successors
                pfaults_per_gate[idx].push(crate::ppsfp::Fault::new(w.clone(), false));
                pfaults_per_gate[idx].push(crate::ppsfp::Fault::new(w.clone(), true));
            }

            // INPUT-line faults: follow Atalanta: for multi-input gates consider faults
            // on each source wire only when the source has fanout > 1
            if node.inputs.len() > 1 {
                for inp in &node.inputs {
                    // determine producer wire name and its fanout
                    let prod_wire = if let Some(&pid) = producer.get(inp) {
                        self.nodes[pid].outputs[0].clone()
                    } else {
                        inp.clone()
                    };

                    let src_fanout = fanout_count.get(&prod_wire).cloned().unwrap_or(0);
                    if src_fanout > 1 {
                        // create site descriptor like "src->dst" to match Atalanta
                        let site = Some(format!("{}->{}", prod_wire, node.outputs[0]));
                        match node.gate_type {
                            crate::netlist::GateType::XOR | crate::netlist::GateType::XNOR => {
                                pfaults_per_gate[idx].push(crate::ppsfp::Fault::new_with_site(prod_wire.clone(), false, site.clone()));
                                pfaults_per_gate[idx].push(crate::ppsfp::Fault::new_with_site(prod_wire.clone(), true, site.clone()));
                            }
                            crate::netlist::GateType::AND | crate::netlist::GateType::NAND => {
                                pfaults_per_gate[idx].push(crate::ppsfp::Fault::new_with_site(prod_wire.clone(), true, site.clone()));
                            }
                            crate::netlist::GateType::OR | crate::netlist::GateType::NOR => {
                                pfaults_per_gate[idx].push(crate::ppsfp::Fault::new_with_site(prod_wire.clone(), false, site.clone()));
                            }
                            _ => {
                                pfaults_per_gate[idx].push(crate::ppsfp::Fault::new_with_site(prod_wire.clone(), false, site.clone()));
                                pfaults_per_gate[idx].push(crate::ppsfp::Fault::new_with_site(prod_wire.clone(), true, site.clone()));
                            }
                        }
                    }
                }
            }
        }

        // Build stem list: gates that are roots of FFRs (fanout>1 or final outputs)
        let mut stems: Vec<usize> = Vec::new();
        for (idx, node) in self.nodes.iter().enumerate() {
            if self.succ_adj[idx].len() > 1 || node.is_final {
                stems.push(idx);
            }
        }

        // Enumerate faults per FFR similarly to Atalanta: for each stem, traverse
        // backward through predecessors that have single fanout and collect pfaults.
        let mut fault_list: Vec<Fault> = Vec::new();

        // traverse stems in reverse order as Atalanta does
        for &stem in stems.iter().rev() {
            // DFS stack seeded with the stem
            let mut stack = vec![stem];
            let mut visited = vec![false; self.nodes.len()];
            visited[stem] = true;

            while let Some(u) = stack.pop() {
                // append all pfaults of node u in order (already Fault objects)
                for f in &pfaults_per_gate[u] {
                    fault_list.push(f.clone());
                }

                // push predecessors that are inside the FFR (their out_count == 1)
                for &p in &self.rev_adj[u] {
                    if !visited[p] {
                        if self.succ_adj[p].len() == 1 {
                            visited[p] = true;
                            stack.push(p);
                        }
                    }
                }
            }
        }

        // da debug: confronta con enumerazione semplice (tutti i fault di ogni nodo) per verificare che non manchi nulla
        // Return full fault list (including site-specific and producer-specific
        // entries) to match Atalanta's enumeration.
        fault_list

        /*
        // If a producer-level (site==None) fault exists for a given wire/sa,
        // prefer that and remove per-site duplicates. This mirrors Atalanta's
        // tendency to list producer-only faults instead of many site-specific
        // entries for the same producer.
        let mut producer_pairs: std::collections::HashSet<(String, bool)> = std::collections::HashSet::new();
        for f in &fault_list {
            if f.site.is_none() {
                producer_pairs.insert((f.wire.clone(), f.sa1));
            }
        }

        let filtered: Vec<Fault> = fault_list.into_iter().filter(|f| {
            if f.site.is_none() { true } else { !producer_pairs.contains(&(f.wire.clone(), f.sa1)) }
        }).collect();

        filtered
         */ // riduce il numero di fault, da vedere se lasciarlo o no per debug (confronto con enumerazione semplice) e per avere più pattern (es: per input line faults)


    }

    /// fault injection O(n) usando succ_adj
    pub fn inject_fault(&mut self, fault: Fault) -> bool {
        let wire = &fault.wire;
        let sa1 = fault.sa1;

        if !crate::options::get_options().quiet && PRINT { print!("fault injection: "); }

        let start = match self.nodes.iter().position(|n| &n.outputs[0] == wire) {
            Some(id) => id,
            None => {
                eprintln!("  wire '{}' not found. Fault injection failed.", wire);
                return false;
            }
        };

        // Determina nodi downstream usando succ_adj
        let mut affected = Vec::new();
        let mut stack = vec![start];
        let mut visited = vec![false; self.nodes.len()];
        visited[start] = true;

        while let Some(u) = stack.pop() {
            affected.push(u);
            for &v in &self.succ_adj[u] {
                if !visited[v] {
                    visited[v] = true;
                    stack.push(v);
                }
            }
        }

        //affected.sort_unstable(); //per confrontare output in debug, poi da togliere
        self.affected = affected.clone();
        self.nodes[start].pre_fault_cnf = Some(self.nodes[start].cnf.to_string());

        // Rinominazione wire
        let mut rename: HashMap<String, String> = HashMap::new();
        let renamed_wire = format!("{}'", wire);
        rename.insert(wire.to_string(), renamed_wire.clone());
        for &id in &affected {
            let old_out = self.nodes[id].outputs[0].clone();
            if !rename.contains_key(&old_out) {
                rename.insert(old_out.clone(), format!("{old_out}'"));
            }
        }

        // Aggiorna outputs, inputs e CNF
        for &id in &affected {
            let node = &mut self.nodes[id];

            if let Some(new_out) = rename.get(&node.outputs[0]) {
                node.outputs[0] = new_out.clone();
            }

            for inp in &mut node.inputs {
                if let Some(new_inp) = rename.get(inp) {
                    *inp = new_inp.clone();
                }
            }

            node.cnf = gate_to_cnf_literals(node.gate_type, &node.outputs[0].clone(), &node.inputs);
        }

        // Imposta clausola stuck-at
        let stuck_literal = Literal { var: renamed_wire, neg: !sa1 };
        self.nodes[start].cnf = CNF { clauses: vec![vec![stuck_literal]] };

        if !crate::options::get_options().quiet && PRINT { println!("fault injection on wire {} complete (stuck-at {}).\n", wire, if sa1 { 1 } else { 0 }); }
        true
    }

    // funzioni utili per avere cnf per output singoli
    /*
    /// dag -> print cnf for each output
    pub fn print_cnfs_as_literals(&self) {
        for node in &self.nodes {
            if node.is_final {
                print!("- node {}:\n  ", node.name);
                for out in &node.outputs {
                    print!("{}\n  ", self.formula_for_output_as_cnf(&out).unwrap().to_string())
                }
            }
        }
    }

    /// output -> cnf for that output
    pub fn formula_for_output_as_cnf(&self, signal: &str) -> Option<CNF> {
        let id_map: HashMap<String, usize> = self.nodes.iter().map(|n| (n.outputs[0].clone(), n.id)).collect();
        let id = id_map.get(signal)?;
        let mut visited = vec![false; self.nodes.len()];
        let mut clauses = Vec::new();
        self.collect_clauses(*id, &mut visited, &mut clauses);
        Some(CNF::new(clauses))
    }
    */

}

/// costruisce la formula di test come cnf
/// ottimizzato con i cone usando rev_adj
/// (paper compliant: con variabili ausiliarie BD, V1, V2)
pub fn build_test_formula_optimized(dag_good: &Dag, dag_faulted: &Dag) -> CNF {
    let mut cnf = dag_good.cnf_cone_from_outputs();
    cnf.clauses.extend(dag_faulted.cnf_cone_faulted().clauses);

    for node in &dag_good.nodes {
        if node.is_final {
            let x = &node.outputs[0];
            let xp = &format!("{x}'");
            cnf.clauses.extend(get_boolean_difference_clauses(x, xp).clauses);
        }
    }
    cnf
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::netlist::{Netlist, Gate, GateType};

    fn simple_netlist_buf() -> Netlist {
        let mut nl = Netlist::new();
        nl.inputs = vec!["IN".to_string()];
        nl.outputs = vec!["OUT".to_string()];
        let g = Gate { name: "G1".to_string(), gate_type: GateType::BUF, outputs: vec!["OUT".to_string()], inputs: vec!["IN".to_string()] };
        nl.gates.push(g);
        nl
    }

    #[test]
    fn test_from_netlist_and_cnf_cone() {
        let nl = simple_netlist_buf();
        let dag = Dag::from_netlist(&nl);
        assert_eq!(dag.nodes.len(), 1);
        let cnf = dag.cnf_cone_from_outputs();
        // BUF creates 2 clauses per implementation
        assert!(cnf.get_n_clauses() >= 1);
    }

    #[test]
    fn test_generate_and_inject_fault() {
        let nl = simple_netlist_buf();
        let mut dag = Dag::from_netlist(&nl);
        let faults = dag.generate_fault_list(None, None);
        // for single node expect two faults (sa0 and sa1)
        assert!(faults.len() >= 2);

        // inject a fault on OUT
        let f = Fault::new("OUT".to_string(), true);
        let injected = dag.inject_fault(f.clone());
        assert!(injected);
        // affected should contain the node 0
        assert!(dag.affected.contains(&0usize));
        // pre_fault_cnf set
        assert!(dag.nodes[0].pre_fault_cnf.is_some());
    }
}

// Costruisce la formula cnf di test come stringa (divisa per parti per dettagli) 
/*
/// Costruisce la formula di test come stringa
pub fn get_test_formula_as_str(dag_good: &Dag, dag_faulted: &Dag) -> String {
    let mut formula = String::new();
    formula.push_str(" /* GOOD CIRCUIT      */ ");
    formula.push_str(&dag_good.cnf_cone_from_outputs().to_string());
    formula.push_str(AND);
    formula.push_str("\n /* FAULTED CIRCUIT   */ ");
    formula.push_str(&dag_faulted.cnf_cone_faulted().to_string());
    formula.push_str(AND);
    formula.push_str("\n /* OUTPUT DIFFERENCE */ ");
    for node in &dag_good.nodes {
        if node.is_final {
            let x = &node.outputs[0];
            let xp = &format!("{}'", x);
            formula.push_str(&get_boolean_difference_clauses(x, xp).to_string());
        }
    }
    formula
}
*/


