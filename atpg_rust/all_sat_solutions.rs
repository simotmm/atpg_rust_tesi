
use crate::cnf::{CNF, Literal, get_boolean_difference_clauses};
use std::collections::{HashMap, HashSet};
use varisat::lit::Lit;
use varisat::ExtendFormula;
use crate::dag::Dag;
use crate::netlist::Netlist;
use crate::ppsfp::Fault;

/// Enumera tutti i modelli SAT distinti proiettati sui `primary_inputs` usando Varisat.
/// Restituisce un vettore di pattern, ogni pattern è `Vec<(String,u32)>` con 1=TRUE,0=FALSE,2=DON'T-CARE.
pub fn all_sat_solutions(cnf: &CNF, primary_inputs: Option<&[String]>) -> Vec<Vec<(String, u32)>> {
    // mappa variabile -> dimacs id e costruzione solver
    let mut solutions: Vec<Vec<(String, u32)>> = Vec::new();
    let ordered = cnf.literals_to_int_map_ordered(); // Vec<(String, i32)>
    let mut name_to_dimacs: HashMap<String, isize> = HashMap::new();
    for (name, id) in ordered.iter() { name_to_dimacs.insert(name.clone(), *id as isize); }

    // costruzione solver Varisat
    let mut solver = varisat::solver::Solver::new();

    // aggiunta clausole alla formula Varisat, traducendo da CNF a Dimacs usando la mappa name_to_dimacs
    for clause in cnf.clauses.iter() {
        let lits: Vec<Lit> = clause.iter().filter_map(|lit| {
            name_to_dimacs.get(&lit.var).map(|&num| {
                let dim: isize = if lit.neg { -num } else { num };
                Lit::from_dimacs(dim)
            })
        }).collect();
        if !lits.is_empty() { solver.add_clause(&lits); }
    }

    // se primary_inputs è specificato, usalo come ordine e filtro per i pattern; altrimenti usa tutte le variabili in ordine
    let inputs: Vec<String> = if let Some(p) = primary_inputs { p.iter().cloned().collect() } else { ordered.iter().map(|(n,_)| n.clone()).collect() };

    // enumerazione soluzioni SAT proiettate sui primary_inputs con loop di blocking clause (ogni soluzione trovata viene bloccata con una clausola che ne inverte l'assegnamento sui primary_inputs)
    loop {
        let sat = match solver.solve() { Ok(b) => b, Err(_) => false };
        if !sat { break; }

        let model = match solver.model() { Some(m) => m.clone(), None => break };

        let mut id_map: HashMap<isize, bool> = HashMap::new();
        for lit in model.iter() {
            let d = lit.to_dimacs();
            id_map.insert(d.abs(), d > 0);
        }

        let mut pattern: Vec<(String, u32)> = Vec::new();
        for var in inputs.iter() {
            if let Some(&id) = name_to_dimacs.get(var) {
                let val = id_map.get(&id).cloned().unwrap_or(false);
                pattern.push((var.clone(), if val {1} else {0}));
            } else {
                pattern.push((var.clone(), 2));
            }
        }

        solutions.push(pattern.clone());

        let mut block_lits: Vec<Lit> = Vec::new();
        for (var, val) in pattern.iter() {
            if let Some(&id) = name_to_dimacs.get(var) {
                if *val == 1 {
                    block_lits.push(Lit::from_dimacs(-(id as isize)));
                } else if *val == 0 {
                    block_lits.push(Lit::from_dimacs(id as isize));
                }
            }
        }

        if block_lits.is_empty() { break; }
        solver.add_clause(&block_lits);
    }

    solutions
}

/// Riduce il vettore di pattern al sottoinsieme minimo eliminando duplicati.
pub fn minimal_pattern_set(patterns: Vec<Vec<(String,u32)>>) -> Vec<Vec<(String,u32)>> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut result: Vec<Vec<(String,u32)>> = Vec::new();
    for p in patterns.into_iter() {
        let key = p.iter().map(|(n,v)| format!("{}={}|", n, v)).collect::<String>();
        if !seen.contains(&key) {
            seen.insert(key);
            result.push(p);
        }
    }
    result
}

/// Costruisce, per ogni `fault` nella lista, la CNF di detection:
/// good circuit + boolean-difference clauses + unit clause che impone
/// il valore stuck-at sulla variabile primed corrispondente.
pub fn build_fault_cnfs(dag: &Dag, faults: Vec<&Fault>) -> HashMap<Fault, CNF> {
    let mut fault_map: HashMap<Fault, CNF> = HashMap::new();

    let mut base_good_and_diff: CNF = dag.cnf_cone_from_outputs();
    for node in &dag.nodes {
        if node.is_final {
            let x = &node.outputs[0];
            let xp = format!("{}'", x);
            base_good_and_diff.clauses.extend(get_boolean_difference_clauses(x, &xp).clauses);
        }
    }
    let mut bd_clause: Vec<Literal> = Vec::new();
    for node in &dag.nodes {
        if node.is_final {
            let base = node.outputs[0].trim_end_matches('\'').to_string();
            bd_clause.push(Literal { var: format!("BD__{}", base), neg: false });
        }
    }
    if !bd_clause.is_empty() { base_good_and_diff.clauses.push(bd_clause); }

    for &fault in faults.iter() {
        let mut cnf = base_good_and_diff.clone();
        let primed = format!("{}'", fault.wire);
        let lit = if fault.sa1 { Literal { var: primed, neg: false } } else { Literal { var: primed, neg: true } };
        cnf.clauses.push(vec![lit]);
        fault_map.insert(fault.clone(), cnf);
    }

    fault_map
}

/// Costruisce le CNF per tutti i fault e ritorna l'insieme (non risolve il set-cover qui).
pub fn minimal_patterns_for_faults(dag: &Dag, faults: Vec<&Fault>, primary_inputs: Option<&[String]>, per_fault_limit: Option<usize>) -> Vec<Vec<(String,u32)>> {
    let fault_cnfs_map = build_fault_cnfs(dag, faults);
    // For each fault CNF, enumerate sat solution patterns (limited per per_fault_limit)
    let mut all_patterns_per_fault: Vec<Vec<Vec<(String,u32)>>> = Vec::new();
    for (_f, cnf) in fault_cnfs_map.iter() {
        let mut sols = all_sat_solutions(cnf, primary_inputs);
        if let Some(limit) = per_fault_limit { sols.truncate(limit); }
        all_patterns_per_fault.push(sols);
    }
    // naive: flatten and deduplicate
    let mut flat: Vec<Vec<(String,u32)>> = all_patterns_per_fault.into_iter().flatten().collect();
    minimal_pattern_set(flat)
}

/// Greedy set-cover: per ogni fault enumera fino a `per_fault_limit` pattern, poi
/// sceglie greedy il sottoinsieme di pattern che copre tutti i fault (sulla base
/// delle enumeration ottenute). Restituisce i pattern scelti.
pub fn minimal_set_cover_for_faults(dag: &Dag, faults: Vec<&Fault>, primary_inputs: Option<&[String]>, per_fault_limit: Option<usize>) -> Vec<Vec<(String,u32)>> {
    use std::collections::BTreeMap;

    // enumerate per-fault patterns
    let fault_cnfs_map = build_fault_cnfs(dag, faults.clone());
    let mut pattern_to_faults: BTreeMap<String, Vec<Fault>> = BTreeMap::new();
    let mut key_to_pattern: HashMap<String, Vec<(String,u32)>> = HashMap::new();

    for (fault, cnf) in fault_cnfs_map.iter() {
        let mut sols = all_sat_solutions(cnf, primary_inputs);
        if let Some(limit) = per_fault_limit { sols.truncate(limit); }
        for p in sols.into_iter() {
            let key = p.iter().map(|(n,v)| format!("{}={}|", n, v)).collect::<String>();
            pattern_to_faults.entry(key.clone()).or_default().push(fault.clone());
            key_to_pattern.entry(key.clone()).or_insert(p.clone());
        }
    }

    let mut uncovered: HashSet<Fault> = faults.into_iter().cloned().collect();
    let mut cover: Vec<Vec<(String,u32)>> = Vec::new();

    while !uncovered.is_empty() && !pattern_to_faults.is_empty() {
        // choose pattern covering max uncovered faults
        let mut best_key: Option<String> = None;
        let mut best_cov: usize = 0;
        for (k, flist) in pattern_to_faults.iter() {
            let cnt = flist.iter().filter(|f| uncovered.contains(f)).count();
            if cnt > best_cov {
                best_cov = cnt;
                best_key = Some(k.clone());
            }
        }
        if best_cov == 0 { break; } // no pattern covers remaining uncovered faults
        let bk = best_key.unwrap();
        let pat = key_to_pattern.get(&bk).unwrap().clone();
        // add to cover
        cover.push(pat.clone());
        // remove covered faults
        if let Some(flist) = pattern_to_faults.get(&bk) {
            for f in flist.iter() { uncovered.remove(f); }
        }
        // remove chosen pattern from consideration
        pattern_to_faults.remove(&bk);
    }

    cover
}

/// Exact set-cover: guarantees minimal cardinality by enumerating subsets (bitmask)
/// Falls back to greedy when problem size is too large.
pub fn exact_set_cover_for_faults(dag: &Dag, faults: Vec<&Fault>, primary_inputs: Option<&[String]>, per_fault_limit: Option<usize>) -> Vec<Vec<(String,u32)>> {
    use std::collections::BTreeMap;

    let fault_cnfs_map = build_fault_cnfs(dag, faults.clone());
    let mut pattern_to_faults: BTreeMap<String, Vec<Fault>> = BTreeMap::new();
    let mut key_to_pattern: HashMap<String, Vec<(String,u32)>> = HashMap::new();

    for (fault, cnf) in fault_cnfs_map.iter() {
        let mut sols = all_sat_solutions(cnf, primary_inputs);
        if let Some(limit) = per_fault_limit { sols.truncate(limit); }
        for p in sols.into_iter() {
            let key = p.iter().map(|(n,v)| format!("{}={}|", n, v)).collect::<String>();
            pattern_to_faults.entry(key.clone()).or_default().push(fault.clone());
            key_to_pattern.entry(key.clone()).or_insert(p.clone());
        }
    }

    let keys: Vec<String> = pattern_to_faults.keys().cloned().collect();
    let n = keys.len();
    if n == 0 { return Vec::new(); }

    let total_faults = faults.len();
    let max_bits = std::mem::size_of::<usize>() * 8;
    if total_faults == 0 || total_faults > max_bits { return minimal_set_cover_for_faults(dag, faults, primary_inputs, per_fault_limit); }
    if n > 26 { // avoid 2^n explosion; fallback
        return minimal_set_cover_for_faults(dag, faults, primary_inputs, per_fault_limit);
    }

    // map faults -> index
    let mut fault_index: HashMap<Fault, usize> = HashMap::new();
    for (i, f) in faults.iter().enumerate() { fault_index.insert((*f).clone(), i); }

    // pattern masks
    let mut pattern_masks: Vec<usize> = Vec::new();
    for k in keys.iter() {
        let mut mask: usize = 0;
        if let Some(flist) = pattern_to_faults.get(k) {
            for f in flist.iter() {
                if let Some(&idx) = fault_index.get(f) { mask |= 1usize << idx; }
            }
        }
        pattern_masks.push(mask);
    }

    let all_mask: usize = if total_faults == usize::BITS as usize {
        usize::MAX
    } else { (1usize << total_faults) - 1 };

    // enumerate subset sizes
    for size in 1..=n {
        let limit: usize = 1usize << n;
        for mask in 1..limit {
            if mask.count_ones() as usize != size { continue; }
            let mut cov: usize = 0;
            for i in 0..n {
                if (mask >> i) & 1 == 1 { cov |= pattern_masks[i]; }
            }
            if cov == all_mask {
                // build selected patterns
                let mut res: Vec<Vec<(String,u32)>> = Vec::new();
                for i in 0..n {
                    if (mask >> i) & 1 == 1 { let k = &keys[i]; res.push(key_to_pattern.get(k).unwrap().clone()); }
                }
                return res;
            }
        }
    }

    // no exact cover found -> return greedy fallback
    minimal_set_cover_for_faults(dag, faults, primary_inputs, per_fault_limit)
}

