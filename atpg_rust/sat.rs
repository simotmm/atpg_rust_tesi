use crate::dag::{Dag, build_test_formula_optimized};
use crate::cnf::{CNF, Literal, get_boolean_difference_clauses};
use crate::netlist::Netlist;
use crate::ppsfp::Fault;
use crate::cnf_minimizer::minimize_search_tree;
use crate::util::press_any_key;
use varisat::ExtendFormula;
use std::collections::BTreeMap;
use std::collections::{HashMap, HashSet, VecDeque, BTreeSet};
use rayon::prelude::*; // per simulazione in parallelo su thread
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use crate::two_sat::{self, generate_all_assignments};
// runtime options (set by main)
use crate::options;

const PRINT: bool = false;
const PRINT_DEBUG: bool = false; // flag per abilitare stampe di debug più dettagliate (es: valutazione booleana passo-passo)
const VERBOSE: bool = false;
const DONT_CARE_VALUE: u32 = 2;
const EXTEND_2SAT: bool = false; // se true, prova ad estendere le soluzioni 2-SAT con il risolutore 3-SAT
const PRUNE_VARISAT_WITH_2SAT: bool = true; // se true, esegue un passo di pruning usando il modulo 2-SAT prima di chiamare varisat (mi sembra che rallenti soltanto...)

/// wrapper 2sat + 3sat
pub fn sat_solver_bits(cnf: CNF, primary_inputs: Option<&[String]>, prioritized_vars: Option<&[String]>) -> (Vec<(String, u32)>, Option<HashMap<String, bool>>) {

    let mut result = Vec::new();
    let two_sat_portion = cnf.get_2_lit_portion();
    let three_sat_portion = cnf.get_3_lit_portion();

    if !crate::options::get_options().quiet && PRINT && VERBOSE{
        print!("full cnf:         {}\n", cnf.to_string());
        print!("(literal, int) map: {:?}\n", cnf.literals_to_int_map_ordered());
        print!("full cnf as int:\np cnf {} {}\n", cnf.get_n_clauses(), cnf.get_n_literals());
        print!("{}\n", cnf.to_int_string());
        print!("2sat portion cnf: {}\n", two_sat_portion.clone().to_string());
        print!("2sat portion cnf as int:\n{}\n", two_sat_portion.to_int_string());
        print!("3sat portion cnf: {}\n", three_sat_portion.to_string());
        print!("3sat portion cnf as int:\n{}\n", three_sat_portion.to_int_string());
    }
    
    // First: try 2-SAT portion. If unsatisfiable, whole formula unsat -> return empty result.
    let extend_factor = 2;
    let max_2sat_assignments = if EXTEND_2SAT { Some(primary_inputs.unwrap().len()*extend_factor as usize) } else { None }; // limit per evitare esplosione combinatoria in casi con molte soluzioni 2-SAT
    
    // Use the dedicated two_sat module for generating 2-SAT assignments (correct implementation)
    let two_sat_assignments_opt = generate_all_assignments(&two_sat_portion, max_2sat_assignments);
    if two_sat_assignments_opt.is_none() {
        // 2-SAT unsat => whole formula unsat
        if !crate::options::get_options().quiet && PRINT { print!("2-SAT portion UNSAT -> formula UNSAT\n"); }
        return (result, None);
    }

    let two_sat_assignments = two_sat_assignments_opt.unwrap();

    // Try to extend each 2-SAT assignment with 3-SAT (DPLL + unit propagation)
    // The `None` Option will cause `three_sat_solver` to use the module-level `EXTEND_2SAT` flag.
    
    let final_assignment: Option<HashMap<String, bool>> = three_sat_solver(two_sat_assignments.clone(), three_sat_portion, cnf.clone(), prioritized_vars, None);
    if !crate::options::get_options().quiet && PRINT { print!("final results: {:?}\n\n", final_assignment); }
    if final_assignment.is_none() { return (result, None); }

    let final_assignment = final_assignment.unwrap();
    // Debug: print the final assignment returned by the 3-SAT solver
    if !crate::options::get_options().quiet && PRINT_DEBUG {
        println!("[DEBUG] SAT final assignment: {:?}", final_assignment);
    }

    // Debug: verify that the final assignment satisfies the full CNF passed to this function
    {
        use std::collections::HashSet;
        let mut unsatisfied: Vec<Vec<Literal>> = Vec::new();
        let mut missing_vars: HashSet<String> = HashSet::new();

        // count variables in CNF
        let var_map = cnf.literals_to_int_map();
        let n_vars_in_cnf = var_map.len();
        if !crate::options::get_options().quiet && PRINT_DEBUG {
            println!("[DEBUG] CNF variables count: {}. Assigned vars count: {}.", n_vars_in_cnf, final_assignment.len());
        }

        for clause in cnf.clauses.iter() {
            let mut clause_sat = false;
            let mut clause_has_unassigned = false;
            for lit in clause.iter() {
                match final_assignment.get(&lit.var) {
                    Some(&v) => {
                        let lit_val = if lit.neg { !v } else { v };
                        if lit_val { clause_sat = true; break; }
                    }
                    None => {
                        clause_has_unassigned = true;
                        missing_vars.insert(lit.var.clone());
                    }
                }
            }
            if !clause_sat && !clause_has_unassigned {
                // clause is definitely unsatisfied
                unsatisfied.push(clause.clone());
            }
        }

        if !unsatisfied.is_empty() || !missing_vars.is_empty() {
            if !crate::options::get_options().quiet && PRINT_DEBUG {
                println!("[DEBUG] Assignment does NOT fully satisfy CNF: {} unsatisfied clauses, {} missing/unassigned variables",
                    unsatisfied.len(), missing_vars.len());
                if !unsatisfied.is_empty() {
                    let s: Vec<Vec<String>> = unsatisfied.iter().map(|cl| cl.iter().map(|l| format!("{}{}", if l.neg {"!"} else {""}, l.var)).collect()).collect();
                    println!("[DEBUG] Unsatisfied clauses: {:?}", s);
                }
                if !missing_vars.is_empty() {
                    println!("[DEBUG] Missing/unassigned variables: {:?}", missing_vars);
                }
            }
        } else {
            if !crate::options::get_options().quiet && PRINT_DEBUG {
                println!("[DEBUG] Assignment satisfies CNF (no unsatisfied clauses, no unassigned vars in decisive clauses).");
            }
        }
    }

    // Conversione in Vec<(String, u32)> ordinata secondo primary_inputs
    if let Some(inputs) = primary_inputs {
        for var in inputs {
            if let Some(&val) = final_assignment.get(var) {
                result.push((var.clone(), if val { 1 } else { 0 }));
            } else {
                result.push((var.clone(), DONT_CARE_VALUE));
            }
        }
    } else {
        // Se non sono specificati primary_inputs, ordiniamo tutte le variabili alfabeticamente
        let mut vars: Vec<_> = final_assignment.iter().collect();
        vars.sort_by(|a, b| a.0.cmp(b.0));
        for (var, &val) in vars {
            result.push((var.clone(), if val { 1 } else { 0 }));
        }
    }

    (result, Some(final_assignment))
}

/// varisat-backed SAT solver for a single CNF instance (non-incremental)
fn varisat_solver_bits(cnf: CNF, primary_inputs: Option<&[String]>) -> (Vec<(String, u32)>, Option<HashMap<String, bool>>) {

    // pruning con 2-SAT: se la porzione 2-SAT è insoddisfacibile, allora tutta la formula è insoddisfacibile -> possiamo saltare la chiamata a varisat
    if PRUNE_VARISAT_WITH_2SAT {
        //println!("Running 2-SAT check for pruning before Varisat call...");
        //press_any_key(); // debug pause to check the CNF and 2-SAT portion before potentially skipping Varisat
        if !two_sat::is_2_sat_satisfiable(&cnf.get_2_lit_portion()) {
            //println!("2-SAT portion is UNSAT, pruning Varisat call and returning UNSAT result.");
            //press_any_key();
            return (Vec::new(), None);
        } else {
            //println!("2-SAT portion is SAT, proceeding to Varisat call.");
        }
        //press_any_key(); // debug pause to confirm pruning decision
    }
    
    let ordered = cnf.literals_to_int_map_ordered(); // Vec<(String, i32)>
    let mut name_to_dimacs: HashMap<String, isize> = HashMap::new();
    for (name, id) in ordered.iter() {
        name_to_dimacs.insert(name.clone(), *id as isize); 
    }

    let mut solver = varisat::solver::Solver::new();
    for clause in cnf.clauses.iter() {
        let lits: Vec<varisat::lit::Lit> = clause.iter().filter_map(|lit| {
            name_to_dimacs.get(&lit.var).map(|&num| {
                let dim: isize = if lit.neg { -num } else { num };
                varisat::lit::Lit::from_dimacs(dim)
            })
        }).collect();
        if !lits.is_empty() { solver.add_clause(&lits); }
    }

    let mut result_bits: Vec<(String, u32)> = Vec::new();
    let sat = match solver.solve() { Ok(b) => b, Err(_) => false };
    if !sat { return (result_bits, None); }
    let model = match solver.model() { Some(m) => m, None => return (result_bits, None) };

    let mut assign_map: HashMap<String, bool> = HashMap::new();
    for lit in model.iter() {
        let d = lit.to_dimacs();
        let var_id = d.abs();
        for (name, &id) in name_to_dimacs.iter() {
            if id == var_id { assign_map.insert(name.clone(), d > 0); break; }
        }
    }

    if let Some(inputs) = primary_inputs {
        for var in inputs { if let Some(&val) = assign_map.get(var) { result_bits.push((var.clone(), if val { 1 } else { 0 })); } else { result_bits.push((var.clone(), DONT_CARE_VALUE)); } }
    } else {
        let mut vars: Vec<_> = assign_map.iter().collect(); vars.sort_by(|a,b| a.0.cmp(b.0)); for (var, &val) in vars { result_bits.push((var.clone(), if val {1} else {0})); }
    }

    (result_bits, Some(assign_map))
}

/// 3sat
fn three_sat_solver(two_sat_assignments: Vec<HashMap<String, bool>>, three_sat_portion: CNF, full_cnf: CNF, prioritized_vars: Option<&[String]>, try_extend_opt: Option<bool>) -> Option<HashMap<String, bool>> {

    // decide whether to attempt to extend 2-SAT solutions based on the provided Option
    // or the module-level `EXTEND_2SAT` flag when None
    let try_extend = try_extend_opt.unwrap_or(EXTEND_2SAT);

    /// Unit propagation adattata per CNF con Literal
    fn unit_propagation_cnf(assignment: &mut HashMap<String, bool>, formula: &mut Vec<Vec<Literal>>) -> bool {
        let mut changed = true;

        while changed {
            changed = false;
            let mut new_formula = Vec::new();

            for clause in formula.iter() {
                // Clausola già soddisfatta?
                if clause.iter().any(|lit| assignment.get(&lit.var).map(|v| if lit.neg { !*v } else { *v }).unwrap_or(false)) {
                    continue;
                }

                // Rimuoviamo literal falsi
                let mut new_clause = Vec::new();
                for lit in clause.iter() {
                    match assignment.get(&lit.var) {
                        Some(&val) => {
                            if lit.neg != val {
                                new_clause.push(lit.clone());
                            }
                        }
                        None => new_clause.push(lit.clone()),
                    }
                }

                if new_clause.is_empty() {
                    return false; // clausola insoddisfacibile
                }

                // Clausola unitaria → propagazione
                if new_clause.len() == 1 {
                    let lit = &new_clause[0];
                    assignment.entry(lit.var.clone()).or_insert(!lit.neg);
                    changed = true;
                }

                new_formula.push(new_clause);
            }

            *formula = new_formula;
        }

        true
    }

    /// DPLL adattato per CNF con Literal
    fn dpll_cnf(assignment: &mut HashMap<String, bool>, formula: &mut Vec<Vec<Literal>>) -> bool {
        if formula.is_empty() {
            return true;
        }

        // Trova variabile non assegnata
        let mut unassigned_vars = BTreeSet::new();
        for clause in formula.iter() {
            for lit in clause.iter() {
                if !assignment.contains_key(&lit.var) {
                    unassigned_vars.insert(lit.var.clone());
                }
            }
        }

        // deterministically pick the first variable (BTreeSet keeps sorted order)
        if let Some(var) = unassigned_vars.iter().next().cloned() {
            for &val in &[true, false] {
                let mut new_assignment = assignment.clone();
                new_assignment.insert(var.clone(), val);

                let mut new_formula = formula.clone();
                if unit_propagation_cnf(&mut new_assignment, &mut new_formula) {
                    if dpll_cnf(&mut new_assignment, &mut new_formula) {
                        *assignment = new_assignment;
                        return true;
                    }
                }
            }
        }

        false
    }

    // tentativo 1: estendere soluzione 2SAT se esiste (abilitato da `try_extend`)
    // Preferiamo soluzioni che assegnano almeno una delle `prioritized_vars` se fornite.
    let mut fallback_solution: Option<HashMap<String,bool>> = None;
    if try_extend {
        let total_two_sat = two_sat_assignments.len();
        for (i, two_sat_assignment) in two_sat_assignments.into_iter().enumerate() {
        let idx = i + 1;
        let pct = if total_two_sat > 0 { (idx as f64) * 100.0 / (total_two_sat as f64) } else { 0.0 };
        if !crate::options::get_options().quiet && crate::PRINT_PROGRESS { println!("three_sat: extending 2-SAT assignment {}/{} ({:.1}%)", idx, total_two_sat, pct); }

        let mut assignment = two_sat_assignment; // ownership from into_iter()
        let mut formula = three_sat_portion.clauses.clone();
        if unit_propagation_cnf(&mut assignment, &mut formula) {
            if dpll_cnf(&mut assignment, &mut formula) {
                // deterministic ordering of assignment before returning
                let mut ordered: Vec<(String, bool)> = assignment.iter().map(|(k,v)| (k.clone(), *v)).collect();
                ordered.sort_by(|a,b| a.0.cmp(&b.0));
                let mut ordered_map = HashMap::new();
                for (k,v) in ordered { ordered_map.insert(k,v); }

                if let Some(pvars) = prioritized_vars {
                    // check whether this solution assigns any prioritized var
                    let mut assigns_pvar = false;
                    for pv in pvars {
                        if ordered_map.contains_key(pv) { assigns_pvar = true; break; }
                    }
                    if assigns_pvar {
                        return Some(ordered_map);
                    } else {
                        // keep as fallback but continue searching for prioritized assignment
                        if fallback_solution.is_none() { fallback_solution = Some(ordered_map); }
                    }
                } else {
                    return Some(ordered_map); // soluzione valida trovata estendendo 2SAT
                }
            }
        }
    }
    }

    // If we didn't find a prioritized solution, but we have a fallback from 2-SAT, remember it for later
    // (we'll return it only if no better solution is found in the prioritized/global search below).

     
    // tentativo 2: risolvere tutta la CNF senza vincolare 2SAT
    // Use the lightweight internal solver (`simple_solver`) with optional prioritized seeding.
    let mut base_formula = full_cnf.clone();

    // If prioritized vars are provided, try small prefix seeds first (limited depth)
    if let Some(pvars) = prioritized_vars {
        let k = pvars.len().min(6);
        let total_prefixes = 1usize << k;
        for mask in 0..total_prefixes {
            let mut seed_map: HashMap<String, bool> = HashMap::new();
            for i in 0..k {
                let val = ((mask >> i) & 1) == 1;
                seed_map.insert(pvars[i].clone(), val);
            }
            if let Some(sol) = crate::simple_solver::solve_cnf(&base_formula, Some(&seed_map), None) {
                return Some(sol);
            }
        }
    }

    // Final fallback: solve full CNF without seeding (may be slower but is a single call)
    if let Some(sol) = crate::simple_solver::solve_cnf(&base_formula, None, prioritized_vars) {
        return Some(sol);
    }

    // If no solution found, return fallback from 2-SAT extension if available
    if let Some(fb) = fallback_solution { return Some(fb); }

    None // nessuna soluzione trovata
}

/// wrapper sat solving: se parallel=true usa sat_solving_parallel, altrimenti sat_solving_sequential
pub fn sat_solving(circuit: &Netlist, dag: &Dag, faults: Vec<&Fault>, parallel_execution: Option<bool>) -> HashMap<Fault, Vec<(String, u32)>> {
    if parallel_execution.is_some() && parallel_execution.unwrap() == true {
        return sat_solving_parallel(circuit, dag, faults);
    }
    sat_solving_sequential(circuit, dag, faults)
}

/// wrapper sat solving sequenziale
fn sat_solving_sequential(circuit: &Netlist, dag: &Dag, faults: Vec<&Fault>) -> HashMap<Fault, Vec<(String, u32)>> {
    // mappa per memorizzare i risultati: per ogni fault, la lista di pattern (stringa) e numero di bit significativi
    let mut solutions: HashMap<Fault, Vec<(String, u32)>> = HashMap::new();
    let inputs = &circuit.inputs;
    let total = faults.len();
    let mut found_count: usize = 0;
    // Precompute GOOD circuit CNF + output boolean-difference clauses once to avoid
    // regenerating them for every fault (significant saving).
    let mut base_good_and_diff: CNF = dag.cnf_cone_from_outputs();
    for node in &dag.nodes {
        if node.is_final {
            let x = &node.outputs[0];
            let xp = format!("{x}'");
            base_good_and_diff.clauses.extend(get_boolean_difference_clauses(x, &xp).clauses);
        }
    }
    // Require at least one boolean-difference (BD__) to be true (i.e., some final output differs)
    let mut bd_clause: Vec<Literal> = Vec::new();
    for node in &dag.nodes {
        if node.is_final {
            let base = node.outputs[0].trim_end_matches('\'').to_string();
            bd_clause.push(Literal { var: format!("BD__{}", base), neg: false });
        }
    }
    // Add the BD__ clause to the base formula (GOOD + output-diff) that will be used for all faults
    if !bd_clause.is_empty() { base_good_and_diff.clauses.push(bd_clause); }

    // Process each fault sequentiallyly: build the faulty DAG, construct the CNF for the SAT query, and solve it
    for (i, fault_ref) in faults.iter().enumerate() {
        let fault = *fault_ref; // &Fault
        let idx = i + 1;
        let pct = if total > 0 { (idx as f64) * 100.0 / (total as f64) } else { 0.0 };

        if !crate::options::get_options().quiet && crate::PRINT_PROGRESS { println!("SAT: processing {}/{} ({:.1}%) - fault {} s-a-{}", idx, total, pct, fault.wire, if fault.sa1 {1} else {0}); }

        // Build a primed copy of the DAG for the faulty circuit (all node outputs primed)
        // and apply the stuck-at fault on the corresponding primed wire. Also compute
        // the affected nodes (cone) for prioritized variable selection.
        let mut faulty = dag.primed_clone();
        // find the node index in the original dag (if any)
        let start_idx_opt = dag.nodes.iter().position(|n| n.outputs[0] == fault.wire);

        // compute affected nodes (downstream) from original dag's succ_adj
        let mut affected: Vec<usize> = Vec::new();
        let mut stack: Vec<usize> = Vec::new();
        let mut visited = vec![false; dag.nodes.len()];

        // if the fault is on a gate output, start DFS from that node. If it's on a primary input, start DFS from all consumer nodes.
        if let Some(start_idx) = start_idx_opt {
            stack.push(start_idx);
            visited[start_idx] = true;
        } else {
            // fault on a primary input: find consumer nodes that have this wire as input
            let mut consumers: Vec<usize> = dag.nodes.iter().enumerate()
                .filter(|(_, n)| n.inputs.iter().any(|inp| inp == &fault.wire))
                .map(|(i, _)| i)
                .collect();
            if consumers.is_empty() {
                // no consumers -> nothing to do for SAT
                continue;
            }
            // start DFS from consumer nodes of the affected primary input
            for c in consumers {
                if !visited[c] {
                    visited[c] = true;
                    stack.push(c);
                }
            }
        }
        // DFS to find all affected nodes downstream of the fault site
        while let Some(u) = stack.pop() {
            affected.push(u);
            // visit successors in the original DAG (not the primed one, since we want the cone of influence based on the original structure)
            for &v in &dag.succ_adj[u] {
                if !visited[v] {
                    visited[v] = true;
                    stack.push(v);
                }
            }
        }
        // store affected nodes in the faulty DAG for later use (e.g., prioritization)
        faulty.affected = affected.clone();

        // apply stuck-at clause on the primed wire name
        let renamed_wire = format!("{}'", fault.wire);
        // If the fault is on a gate output, we can directly replace the CNF of that node with the stuck-at clause. If it's on a primary input, we need to rename the input in all affected nodes and add a new dummy node to carry the stuck-at clause for the primed PI.
        if let Some(start_idx) = start_idx_opt {
            faulty.nodes[start_idx].pre_fault_cnf = Some(faulty.nodes[start_idx].cnf.to_string());
            let stuck_literal = Literal { var: renamed_wire.clone(), neg: !fault.sa1 };
            faulty.nodes[start_idx].cnf = CNF { clauses: vec![vec![stuck_literal]] };
        } else {
            // primary input case: rename inputs in affected nodes to the primed name
            for &id in &affected {
                // rename the input wire to the primed version in the faulty DAG
                for inp in &mut faulty.nodes[id].inputs {
                    if inp == &fault.wire {
                        *inp = renamed_wire.clone();
                    }
                }
                // recompute CNF for the node after input rename
                faulty.nodes[id].cnf = crate::cnf::gate_to_cnf_literals(faulty.nodes[id].gate_type, &faulty.nodes[id].outputs[0].clone(), &faulty.nodes[id].inputs);
            }

            // create a dummy node to carry the stuck-at clause for the primed PI
            let new_id = faulty.nodes.len();
            let dummy_node = crate::dag::DagNode {
                id: new_id,
                name: format!("PI_FAULT_{}", fault.wire),
                gate_type: crate::netlist::GateType::INVALID,
                outputs: vec![renamed_wire.clone()],
                inputs: vec![],
                cnf: CNF { clauses: vec![vec![Literal { var: renamed_wire.clone(), neg: !fault.sa1 }]] },
                pre_fault_cnf: None,
                is_final: false,
            };
            faulty.nodes.push(dummy_node);
            faulty.affected.push(new_id);
        }

        // Build CNF as (GOOD + output-diff) + FULL FAULTED CIRCUIT (use full cone from outputs
        // for the faulty DAG to ensure primed variables have proper constraints)
        let mut test_formula: CNF = base_good_and_diff.clone();
        test_formula.clauses.extend(faulty.cnf_cone_from_outputs().clauses.clone());
        let mut cnf: CNF = test_formula.clone();

        // build prioritized vars from affected cone (prefer primary inputs in the cone)
        let mut pvars: Vec<String> = Vec::new();
        for &id in &faulty.affected {
            let v = faulty.nodes[id].outputs[0].clone();
            let base = if v.ends_with('\'') { v.trim_end_matches('\'').to_string() } else { v.clone() };
            if inputs.contains(&base) { pvars.push(base); }
        }
        // fallback: if no primary inputs in cone, use previous (all cone vars)
        if pvars.is_empty() {
            for &id in &faulty.affected {
                let v = faulty.nodes[id].outputs[0].clone();
                pvars.push(v.clone());
                if v.ends_with('\'') { pvars.push(v.trim_end_matches('\'').to_string()); }
            }
        }

        // Optional minimization step to reduce CNF size before solving (can help or hurt depending on the case)
        if options::get_options().optimize {
            if options::get_options().minimizer.as_deref() == Some("espresso") {
                // try espresso minimizer using prioritized primary inputs for this cone -> NON PIU' USATO
                if let Ok(min_cnf) = crate::cnf_minimizer_espresso::minimize_using_espresso(&test_formula, &pvars[..]) {
                    cnf = min_cnf;
                } else {
                    // fallback to builtin
                        if let Err(_) = minimize_search_tree(&mut cnf) {
                            if !crate::options::get_options().quiet && crate::PRINT_PROGRESS { println!("CNF unsat during minimization for fault {} s-a-{}", fault.wire, if fault.sa1 {1} else {0}); }
                        continue;
                    }
                    if cnf.get_n_literals() == 0 { cnf = test_formula.clone(); }
                }
            } else {
                if let Err(_) = minimize_search_tree(&mut cnf) {
                    if !crate::options::get_options().quiet && crate::PRINT_PROGRESS { println!("CNF unsat during minimization for fault {} s-a-{}", fault.wire, if fault.sa1 {1} else {0}); }
                    continue;
                }
                if cnf.get_n_literals() == 0 { cnf = test_formula.clone(); }
            }
        }

        if !crate::options::get_options().quiet && PRINT { fault.print(); }

        // build prioritized vars from affected cone (prefer primary inputs in the cone)
        let mut pvars: Vec<String> = Vec::new();
        for &id in &faulty.affected {
            let v = faulty.nodes[id].outputs[0].clone();
            let base = if v.ends_with('\'') { v.trim_end_matches('\'').to_string() } else { v.clone() };
            if inputs.contains(&base) {
                pvars.push(base);
            }
        }

        // fallback: if no primary inputs in cone, use previous (all cone vars)
        if pvars.is_empty() {
            for &id in &faulty.affected {
                let v = faulty.nodes[id].outputs[0].clone();
                pvars.push(v.clone());
                if v.ends_with('\'') { pvars.push(v.trim_end_matches('\'').to_string()); }
            }
        }

        let (pattern_bits, maybe_assignment) = if options::get_options().sat_backend == "varisat" {
            varisat_solver_bits(cnf.clone(), Some(&inputs))
        } else {
            sat_solver_bits(cnf.clone(), Some(&inputs), Some(&pvars))
        };

        if !pattern_bits.is_empty() {
            found_count += 1;
            // Debug: print the primary-input pattern bits produced by the SAT solver
            if !crate::options::get_options().quiet && PRINT_DEBUG {
                println!("[DEBUG] pattern_bits for fault {}: {:?}", fault.wire, pattern_bits);
            }
            // If SAT returned the full boolean assignment, evaluate the DAG directly
            if let Some(assign_map) = maybe_assignment {
                // build primary input boolean map
                let mut primary_assign: HashMap<String, bool> = HashMap::new();
                for inp in inputs.iter() {
                    let b = *assign_map.get(inp).unwrap_or(&false);
                    primary_assign.insert(inp.clone(), b);
                }
                // evaluate good circuit
                let good_eval = dag.evaluate_boolean(&primary_assign, None);
                // evaluate faulty (primed) circuit using the forced primed wire
                let forced_wire = format!("{}'", fault.wire);
                let faulty_eval = faulty.evaluate_boolean(&primary_assign, Some((forced_wire.clone(), fault.sa1)));

                if !crate::options::get_options().quiet && PRINT_DEBUG {
                    println!("[DEBUG] Boolean evaluation of DAG (single-bit) for fault {}:", fault.wire);
                }
                // Compare assigned values vs computed booleans for each node; report first mismatch
                let mut mismatch_found = false;
                for (nid, node) in dag.nodes.iter().enumerate() {
                    let base = node.outputs[0].clone();
                    let primed = format!("{}'", base);
                    let g = *good_eval.get(&base).unwrap_or(&false);
                    let f = *faulty_eval.get(&primed).unwrap_or(&false);
                    let assigned_x = assign_map.get(&base).cloned();
                    let assigned_xp = assign_map.get(&primed).cloned();
                    let bd_var = format!("BD__{}", base);
                    let bd_assigned = assign_map.get(&bd_var).cloned();

                    if let Some(ax) = assigned_x {
                        if ax != g {
                            if !crate::options::get_options().quiet && PRINT_DEBUG {
                                println!("[DEBUG][MISMATCH] node {} ('{}'): assigned {} != computed good {}", nid, base, ax, g);
                            }
                            if !crate::options::get_options().quiet && PRINT_DEBUG { println!("[DEBUG][MISMATCH] node CNF: {}", node.cnf.to_string()); }
                            if !crate::options::get_options().quiet && PRINT_DEBUG { println!("[DEBUG][MISMATCH] node gate type: {}", node.gate_type.to_string()); }
                            // print inputs assigned vs computed
                            for inp in &node.inputs {
                                let a = assign_map.get(inp).cloned();
                                let c = good_eval.get(inp).cloned().unwrap_or(false);
                                if !crate::options::get_options().quiet && PRINT_DEBUG { println!("[DEBUG][MISMATCH]   input {} -> assigned={:?}, computed={}", inp, a, c); }
                            }
                            // print boolean vector used by evaluator
                            let in_vals: Vec<bool> = node.inputs.iter().map(|i| *good_eval.get(i).unwrap_or(&false)).collect();
                            if !crate::options::get_options().quiet && PRINT_DEBUG { println!("[DEBUG][MISMATCH]   evaluator in_vals: {:?}", in_vals); }
                            mismatch_found = true;
                        }
                    }
                    if mismatch_found { break; }

                    if let Some(axp) = assigned_xp {
                        if axp != f {
                            if !crate::options::get_options().quiet && PRINT_DEBUG { println!("[DEBUG][MISMATCH] node {} ('{}'): assigned primed {} != computed faulty {}", nid, base, axp, f); }
                            // print CNF from the faulty DAG for this node (if available)
                            if nid < faulty.nodes.len() {
                                if !crate::options::get_options().quiet && PRINT_DEBUG { println!("[DEBUG][MISMATCH] faulty node CNF: {}", faulty.nodes[nid].cnf.to_string()); }
                                if !crate::options::get_options().quiet && PRINT_DEBUG { println!("[DEBUG][MISMATCH] node gate type: {}", node.gate_type.to_string()); }
                                for inp in &node.inputs {
                                    let a = assign_map.get(inp).cloned();
                                    let c = faulty_eval.get(inp).cloned().unwrap_or(false);
                                    if !crate::options::get_options().quiet && PRINT_DEBUG { println!("[DEBUG][MISMATCH]   input {} -> assigned={:?}, computed_faulty={}", inp, a, c); }
                                }
                                let in_vals_faulty: Vec<bool> = node.inputs.iter().map(|i| *faulty_eval.get(i).unwrap_or(&false)).collect();
                                if !crate::options::get_options().quiet && PRINT_DEBUG { println!("[DEBUG][MISMATCH]   evaluator in_vals_faulty: {:?}", in_vals_faulty); }
                            }
                            mismatch_found = true;
                        }
                    }
                    if mismatch_found { break; }

                    if let Some(bd) = bd_assigned {
                        let computed_xor = g ^ f;
                        if bd != computed_xor {
                            if !crate::options::get_options().quiet && PRINT_DEBUG { println!("[DEBUG][MISMATCH] node {} ('{}'): BD__ assigned={} but computed XOR(good,faulty)={}", nid, base, bd, computed_xor); }
                            // print clauses from the whole CNF that mention this BD__ var
                            let bd_clauses: Vec<String> = cnf.clauses.iter()
                                .filter(|cl| cl.iter().any(|l| l.var == bd_var))
                                .map(|cl| cl.iter().map(|l| format!("{}{}", if l.neg {"!"} else {""}, l.var)).collect::<Vec<_>>().join(" + "))
                                .collect();
                            if !crate::options::get_options().quiet && PRINT_DEBUG { println!("[DEBUG][MISMATCH] BD__ clauses: {:?}", bd_clauses); }
                            mismatch_found = true;
                        }
                    }
                    if mismatch_found { break; }
                }
                if !mismatch_found { if !crate::options::get_options().quiet && PRINT_DEBUG { println!("[DEBUG] No node-level mismatches found between SAT assignment and boolean evaluation."); } }
            }
            if !crate::options::get_options().quiet && crate::PRINT_PROGRESS { println!("SAT: {}/{} ({:.1}%) - found pattern for {} (found {}/{})", idx, total, pct, fault.wire, found_count, total); }
            solutions.insert(fault.clone(), pattern_bits);
        } else {
            if !crate::options::get_options().quiet && crate::PRINT_PROGRESS { println!("SAT: {}/{} ({:.1}%) - UNSAT for {}", idx, total, pct, fault.wire); }
            if !crate::options::get_options().quiet && PRINT {
                println!("SAT UNSAT for fault {} stuck-at{}", fault.wire, fault.sa1);
                // Diagnostic output to help understand why SAT failed for this fault
                println!("--- Diagnostic: sequential SAT failed for fault {} stuck-at-{} ---", fault.wire, fault.sa1);
                println!("Good CNF (cone): {}", dag.cnf_cone_from_outputs().to_string());
                println!("Faulty CNF (cone): {}", faulty.cnf_cone_from_outputs().to_string());
                println!("XOR (test) CNF (original): {}", test_formula.to_string());
                println!("XOR (test) CNF (optimized): {}", cnf.to_string());
                println!("Prioritized vars (pvars): {:?}", pvars);
            }
        }
    }
    //sort_assignments_by_order(&mut solutions, &circuit.inputs);
    solutions
}

/// wrapper sat solving con thread
fn sat_solving_parallel(circuit: &Netlist, dag: &Dag, faults: Vec<&Fault>) -> HashMap<Fault, Vec<(String, u32)>> {
    // Parallel map dei fault -> ciascun thread ritorna un HashMap<String, u32>
    let inputs = &circuit.inputs;
    let total = faults.len();
    // Precompute GOOD circuit CNF + output boolean-difference clauses once.
    let mut base_good_and_diff: CNF = dag.cnf_cone_from_outputs();
    for node in &dag.nodes {
        if node.is_final {
            let x = &node.outputs[0];
            let xp = format!("{x}'");
            base_good_and_diff.clauses.extend(get_boolean_difference_clauses(x, &xp).clauses);
        }
    }
    // Also require at least one BD__ to be true (force an output difference)
    let mut bd_clause: Vec<Literal> = Vec::new();
    for node in &dag.nodes {
        if node.is_final {
            let base = node.outputs[0].trim_end_matches('\'').to_string();
            bd_clause.push(Literal { var: format!("BD__{}", base), neg: false });
        }
    }
    if !bd_clause.is_empty() { base_good_and_diff.clauses.push(bd_clause); }

    let base_good_and_diff = Arc::new(base_good_and_diff);
    let processed = Arc::new(AtomicUsize::new(0));
    let found = Arc::new(AtomicUsize::new(0));
    let partial_solutions: Vec<HashMap<Fault, Vec<(String, u32)>> > = faults
        .par_iter()                     // iteratore parallelo
        .map(|fault| {
            let mut sol = HashMap::new();

            let idx = processed.fetch_add(1, Ordering::SeqCst) + 1;
            let pct = if total > 0 { (idx as f64) * 100.0 / (total as f64) } else { 0.0 };
            if !crate::options::get_options().quiet && crate::PRINT_PROGRESS { println!("SAT (parallel): processing {}/{} ({:.1}%) - fault {} s-a-{}", idx, total, pct, fault.wire, if fault.sa1 {1} else {0}); }

            // Build a primed copy of the DAG for the faulty circuit and apply the stuck-at fault
            let mut faulty = dag.primed_clone();
            // locate node index in original dag
            let start_idx_opt = dag.nodes.iter().position(|n| n.outputs[0] == (*fault).wire);
            if start_idx_opt.is_none() {
                return sol;
            }
            let start_idx = start_idx_opt.unwrap();
            // compute affected nodes (downstream) using original dag succ_adj
            let mut affected = Vec::new();
            let mut stack = vec![start_idx];
            let mut visited = vec![false; dag.nodes.len()];
            visited[start_idx] = true;
            while let Some(u) = stack.pop() {
                affected.push(u);
                for &v in &dag.succ_adj[u] {
                    if !visited[v] {
                        visited[v] = true;
                        stack.push(v);
                    }
                }
            }
            faulty.affected = affected.clone();

            // apply stuck-at clause on primed wire
            let renamed_wire = format!("{}'", (*fault).wire);
            faulty.nodes[start_idx].pre_fault_cnf = Some(faulty.nodes[start_idx].cnf.to_string());
            let stuck_literal = Literal { var: renamed_wire, neg: !(*fault).sa1 };
            faulty.nodes[start_idx].cnf = CNF { clauses: vec![vec![stuck_literal]] };

            // Build CNF XOR(good,faulty) using precomputed GOOD+diff
            // Extend with full faulty circuit CNF (from outputs) so primed variables are constrained

            let mut test_formula: CNF = (*base_good_and_diff).clone();
            test_formula.clauses.extend(faulty.cnf_cone_from_outputs().clauses.clone());
            let mut cnf: CNF = test_formula.clone();

            // compute prioritized vars for this faulty cone and use them for espresso minimization
            let mut pvars: Vec<String> = Vec::new();
            for &id in &faulty.affected {
                let v = faulty.nodes[id].outputs[0].clone();
                let base = if v.ends_with('\'') { v.trim_end_matches('\'').to_string() } else { v.clone() };
                if inputs.contains(&base) { pvars.push(base); }
            }
            if pvars.is_empty() {
                for &id in &faulty.affected {
                    let v = faulty.nodes[id].outputs[0].clone();
                    pvars.push(v.clone());
                    if v.ends_with('\'') { pvars.push(v.trim_end_matches('\'').to_string()); }
                }
            }

            if options::get_options().optimize {
                if options::get_options().minimizer.as_deref() == Some("espresso") {
                    if let Ok(min_cnf) = crate::cnf_minimizer_espresso::minimize_using_espresso(&test_formula, &pvars[..]) {
                        cnf = min_cnf;
                    } else {
                        if let Err(_) = minimize_search_tree(&mut cnf) {
                            if !crate::options::get_options().quiet && crate::PRINT_PROGRESS { println!("CNF unsat during minimization for fault {} s-a-{}", fault.wire, if fault.sa1 {1} else {0}); }
                            return sol;
                        }
                        if cnf.get_n_literals() == 0 { cnf = test_formula.clone(); }
                    }
                } else {
                    if let Err(_) = minimize_search_tree(&mut cnf) {
                        if !crate::options::get_options().quiet && crate::PRINT_PROGRESS { println!("CNF unsat during minimization for fault {} s-a-{}", fault.wire, if fault.sa1 {1} else {0}); }
                        return sol;
                    }
                    if cnf.get_n_literals() == 0 { cnf = test_formula.clone(); }
                }
            }

            // SAT with prioritized variables: prefer primary inputs in the cone
            let mut pvars: Vec<String> = Vec::new();
            for &id in &faulty.affected {
                let v = faulty.nodes[id].outputs[0].clone();
                let base = if v.ends_with('\'') { v.trim_end_matches('\'').to_string() } else { v.clone() };
                if inputs.contains(&base) { pvars.push(base); }
            }
            if pvars.is_empty() {
                for &id in &faulty.affected {
                    let v = faulty.nodes[id].outputs[0].clone();
                    pvars.push(v.clone());
                    if v.ends_with('\'') { pvars.push(v.trim_end_matches('\'').to_string()); }
                }
            }
            let (pattern_bits, maybe_assignment) = if options::get_options().sat_backend == "varisat" {
                varisat_solver_bits(cnf.clone(), Some(&inputs))
            } else {
                sat_solver_bits(cnf.clone(), Some(&inputs), Some(&pvars))
            };
            if pattern_bits.is_empty() {
                if !crate::options::get_options().quiet && crate::PRINT_PROGRESS { println!("SAT (parallel): {}/{} ({:.1}%) - UNSAT for {}", idx, total, pct, fault.wire); }
                if !crate::options::get_options().quiet && PRINT { println!("SAT UNSAT for fault {} stuck-at{}", fault.wire, fault.sa1); }
                return sol;
            }

            // If the solver returned a full assignment, run the boolean evaluator for extra diagnostics
            if let Some(assign_map) = maybe_assignment {
                let mut primary_assign: HashMap<String, bool> = HashMap::new();
                for inp in inputs.iter() {
                    let b = *assign_map.get(inp).unwrap_or(&false);
                    primary_assign.insert(inp.clone(), b);
                }
                let good_eval = dag.evaluate_boolean(&primary_assign, None);
                let forced_wire = format!("{}'", (*fault).wire);
                let faulty_eval = faulty.evaluate_boolean(&primary_assign, Some((forced_wire.clone(), (*fault).sa1)));
                if !crate::options::get_options().quiet && PRINT_DEBUG { println!("[DEBUG] (parallel) Boolean evaluation for fault {}:", (*fault).wire); }
                // Compare assigned values vs computed booleans for each node; report first mismatch
                let mut mismatch_found = false;
                for (nid, node) in dag.nodes.iter().enumerate() {
                    let base = node.outputs[0].clone();
                    let primed = format!("{}'", base);
                    let g = *good_eval.get(&base).unwrap_or(&false);
                    let f = *faulty_eval.get(&primed).unwrap_or(&false);
                    let assigned_x = assign_map.get(&base).cloned();
                    let assigned_xp = assign_map.get(&primed).cloned();
                    let bd_var = format!("BD__{}", base);
                    let bd_assigned = assign_map.get(&bd_var).cloned();

                    if let Some(ax) = assigned_x {
                        if ax != g {
                            if !crate::options::get_options().quiet && PRINT_DEBUG { println!("[DEBUG][MISMATCH] node {} ('{}'): assigned {} != computed good {}", nid, base, ax, g); }
                            if !crate::options::get_options().quiet && PRINT_DEBUG { println!("[DEBUG][MISMATCH] node CNF: {}", node.cnf.to_string()); }
                            // print inputs assigned vs computed
                            for inp in &node.inputs {
                                let a = assign_map.get(inp).cloned();
                                let c = good_eval.get(inp).cloned().unwrap_or(false);
                                if !crate::options::get_options().quiet && PRINT_DEBUG { println!("[DEBUG][MISMATCH]   input {} -> assigned={:?}, computed={}", inp, a, c); }
                            }
                            mismatch_found = true;
                        }
                    }
                    if mismatch_found { break; }

                    if let Some(axp) = assigned_xp {
                        if axp != f {
                            if !crate::options::get_options().quiet && PRINT_DEBUG { println!("[DEBUG][MISMATCH] node {} ('{}'): assigned primed {} != computed faulty {}", nid, base, axp, f); }
                            if nid < faulty.nodes.len() {
                                if !crate::options::get_options().quiet && PRINT_DEBUG { println!("[DEBUG][MISMATCH] faulty node CNF: {}", faulty.nodes[nid].cnf.to_string()); }
                                for inp in &node.inputs {
                                    let a = assign_map.get(inp).cloned();
                                    let c = faulty_eval.get(inp).cloned().unwrap_or(false);
                                    if !crate::options::get_options().quiet && PRINT_DEBUG { println!("[DEBUG][MISMATCH]   input {} -> assigned={:?}, computed_faulty={}", inp, a, c); }
                                }
                            }
                            mismatch_found = true;
                        }
                    }
                    if mismatch_found { break; }

                    if let Some(bd) = bd_assigned {
                        let computed_xor = g ^ f;
                        if bd != computed_xor {
                            if !crate::options::get_options().quiet { println!("[DEBUG][MISMATCH] node {} ('{}'): BD__ assigned={} but computed XOR(good,faulty)={}", nid, base, bd, computed_xor); }
                            let bd_clauses: Vec<String> = cnf.clauses.iter()
                                .filter(|cl| cl.iter().any(|l| l.var == bd_var))
                                .map(|cl| cl.iter().map(|l| format!("{}{}", if l.neg {"!"} else {""}, l.var)).collect::<Vec<_>>().join(" + "))
                                .collect();
                            if !crate::options::get_options().quiet { println!("[DEBUG][MISMATCH] BD__ clauses: {:?}", bd_clauses); }
                            mismatch_found = true;
                        }
                    }
                    if mismatch_found { break; }
                }
                    if !mismatch_found { if !crate::options::get_options().quiet { println!("[DEBUG] (parallel) No node-level mismatches found between SAT assignment and boolean evaluation."); } }
            }

            let found_idx = found.fetch_add(1, Ordering::SeqCst) + 1;
            if !crate::options::get_options().quiet && crate::PRINT_PROGRESS { println!("SAT (parallel): {}/{} ({:.1}%) - found pattern for {} (found {}/{})", idx, total, pct, fault.wire, found_idx, total); }

            sol.insert((*fault).clone(), pattern_bits);
            sol
        })
        .collect();

    // merge di tutti gli HashMap prodotti in parallelo
    let mut solutions = HashMap::new();
    for local in partial_solutions {
        solutions.extend(local);
    }
    //sort_assignments_by_order(&mut solutions, &circuit.inputs);
    solutions
}

