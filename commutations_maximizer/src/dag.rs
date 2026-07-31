use crate::netlist::{GateType, Netlist};
use crate::ppsfp::Fault;
use std::collections::{HashMap, HashSet};

const PRINT: bool = false;
const PRINT_ADJ_LISTS: bool = false;

#[derive(Debug, Clone)]
pub struct DagNode {
    pub id: usize,
    pub name: String,
    pub gate_type: GateType,
    pub outputs: Vec<String>,
    pub inputs: Vec<String>,
    pub pre_fault_cnf: Option<String>,
    pub is_final: bool,
}

#[derive(Debug, Clone)]
pub struct Dag {
    pub nodes: Vec<DagNode>,
    pub rev_adj: Vec<Vec<usize>>,
    pub succ_adj: Vec<Vec<usize>>,
    pub affected: Vec<usize>,
}

impl DagNode {
    pub fn is_input(&self) -> bool {
        self.inputs.is_empty()
    }

    pub fn is_output(&self) -> bool {
        self.outputs.is_empty()
    }

    pub fn is_gate(&self) -> bool {
        !self.is_input() && !self.is_output()
    }

    pub fn is_primary_output(&self) -> bool {
        self.is_final
    }
}

impl Dag {
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
                pre_fault_cnf: None,
                is_final: g.outputs.iter().any(|o| final_outputs.contains(o)),
            });
        }

        let n = nodes.len();
        let mut rev_adj: Vec<Vec<usize>> = vec![vec![]; n];
        let mut succ_adj: Vec<Vec<usize>> = vec![vec![]; n];
        let affected: Vec<usize> = vec![];

        for (i, g) in nl.gates.iter().enumerate() {
            for inp in &g.inputs {
                if let Some(&src) = id_map.get(inp) {
                    rev_adj[i].push(src);
                    succ_adj[src].push(i);
                }
            }
        }

        Dag {
            nodes,
            rev_adj,
            succ_adj,
            affected,
        }
    }

    pub fn evaluate_boolean(
        &self,
        primary_assign: &std::collections::HashMap<String, bool>,
        forced: Option<(String, bool)>,
    ) -> std::collections::HashMap<String, bool> {
        use std::collections::VecDeque;
        let mut result: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
        for (k, &v) in primary_assign.iter() {
            result.insert(k.clone(), v);
        }
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
                if indeg[s] == 0 {
                    q.push_back(s);
                }
            }
        }
        for &u in &topo {
            let node = &self.nodes[u];
            if node.inputs.len() == 0 {
                continue;
            }
            if let Some((ref wire, val)) = forced {
                if wire == &node.outputs[0] {
                    result.insert(node.outputs[0].clone(), val);
                    continue;
                }
            }
            let mut in_vals: Vec<bool> = Vec::with_capacity(node.inputs.len());
            for inp in &node.inputs {
                let v = *result.get(inp).unwrap_or(&false);
                in_vals.push(v);
            }
            let out = match node.gate_type {
                crate::netlist::GateType::AND => in_vals.iter().fold(true, |acc, &b| acc & b),
                crate::netlist::GateType::OR => in_vals.iter().fold(false, |acc, &b| acc | b),
                crate::netlist::GateType::NAND => !in_vals.iter().fold(true, |acc, &b| acc & b),
                crate::netlist::GateType::NOR => !in_vals.iter().fold(false, |acc, &b| acc | b),
                crate::netlist::GateType::XOR => {
                    if in_vals.len() >= 2 {
                        in_vals[0] ^ in_vals[1]
                    } else {
                        false
                    }
                }
                crate::netlist::GateType::XNOR => {
                    if in_vals.len() >= 2 {
                        !(in_vals[0] ^ in_vals[1])
                    } else {
                        false
                    }
                }
                crate::netlist::GateType::BUF => in_vals.get(0).cloned().unwrap_or(false),
                crate::netlist::GateType::NOT => !in_vals.get(0).cloned().unwrap_or(false),
                _ => false,
            };
            result.insert(node.outputs[0].clone(), out);
        }
        result
    }

    pub fn primed_clone(&self) -> Dag {
        // omitted for brevity in new project; not used
        self.clone()
    }

    pub fn generate_fault_list(
        &self,
        fault_wire: Option<String>,
        sa1: Option<bool>,
    ) -> Vec<crate::ppsfp::Fault> {
        self.all_faults()
    }

    fn all_faults(&self) -> Vec<crate::ppsfp::Fault> {
        vec![]
    }
}
