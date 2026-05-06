use crate::cnf::{CNF, Literal};
use std::collections::HashMap;

// Improved lightweight SAT solver using watched literals and a simple VSIDS-style score.
// Not a full CDCL solver, but significantly faster than naive clause-scanning.
pub fn solve_cnf(cnf: &CNF, initial_assignments: Option<&HashMap<String, bool>>, prioritized_vars: Option<&[String]>) -> Option<HashMap<String, bool>> {
    // Collect variables deterministically
    let mut vars: Vec<String> = cnf.clauses.iter().flat_map(|cl| cl.iter().map(|lit| lit.var.clone())).collect();
    vars.sort(); vars.dedup();
    let n_vars = vars.len();
    let var_index: HashMap<String, usize> = vars.iter().cloned().enumerate().map(|(i, v)| (v, i)).collect();

    // Convert clauses to signed-literal ints (1..n, -1..-n)
    let mut clauses: Vec<Vec<i32>> = Vec::with_capacity(cnf.clauses.len());
    for clause in &cnf.clauses {
        let mut c: Vec<i32> = Vec::with_capacity(clause.len());
        for lit in clause {
            if let Some(&vid) = var_index.get(&lit.var) {
                let code = if lit.neg { -((vid as i32) + 1) } else { (vid as i32) + 1 };
                c.push(code);
            }
        }
        clauses.push(c);
    }

    // Helper to map literal -> watchlist index
    let lit_to_idx = |lit: i32| -> usize { let v = (lit.abs() - 1) as usize; if lit > 0 { 2 * v } else { 2 * v + 1 } };

    // Watched lists and watched positions per clause
    let mut watch: Vec<Vec<usize>> = vec![Vec::new(); 2 * n_vars];
    let mut watch_pos: Vec<(usize, usize)> = vec![(0, 0); clauses.len()];

    // Initialize watches: 2 per clause where possible
    for (cid, clause) in clauses.iter().enumerate() {
        if clause.is_empty() { return None; }
        if clause.len() == 1 {
            watch_pos[cid] = (0, 0);
            watch[lit_to_idx(clause[0])].push(cid);
        } else {
            watch_pos[cid] = (0, 1);
            watch[lit_to_idx(clause[0])].push(cid);
            watch[lit_to_idx(clause[1])].push(cid);
        }
    }

    // Scores for VSIDS-like heuristic (start with occurrence counts)
    let mut scores: Vec<u32> = vec![0u32; n_vars];
    for (cid, clause) in clauses.iter().enumerate() {
        for &lit in clause.iter() {
            let v = (lit.abs() - 1) as usize;
            scores[v] = scores[v].saturating_add(1);
        }
    }

    // assignment: 0 unassigned, 1 true, -1 false; trail & propagation head
    let mut assign: Vec<i8> = vec![0; n_vars];
    let mut trail: Vec<i32> = Vec::new();
    let mut prop_head: usize = 0;

    // decisions stack: (var, tried_value_was_true, trail_len_before)
    let mut decisions: Vec<(usize, bool, usize)> = Vec::new();

    // assign literal helper (function to avoid closure borrow issues)
    fn assign_literal_fn(assign: &mut [i8], trail: &mut Vec<i32>, lit: i32) -> bool {
        let vid = (lit.abs() - 1) as usize;
        let val = if lit > 0 { 1 } else { -1 };
        if assign[vid] != 0 {
            return assign[vid] == val;
        }
        assign[vid] = val;
        trail.push(lit);
        true
    }

    // Evaluate literal with current assignment: returns 1 true, -1 false, 0 unassigned
    fn eval_lit_fn(assign: &[i8], lit: i32) -> i8 {
        let vid = (lit.abs() - 1) as usize;
        match assign[vid] {
            0 => 0,
            1 => if lit > 0 { 1 } else { -1 },
            -1 => if lit < 0 { 1 } else { -1 },
            _ => 0,
        }
    }

    // Unit propagation using watched literals. Returns Option<conflict_clause_id>
    fn propagate_fn(prop_head: &mut usize, trail: &mut Vec<i32>, assign: &mut [i8], watch: &mut [Vec<usize>], watch_pos: &mut [(usize, usize)], clauses: &Vec<Vec<i32>>) -> Option<usize> {
        let mut ph = *prop_head;
        while ph < trail.len() {
            let lit_true = trail[ph];
            ph += 1;
            let lit_false = -lit_true;
            let idx = { let v = (lit_false.abs() - 1) as usize; if lit_false > 0 { 2 * v } else { 2 * v + 1 } };
            let mut i = 0;
            while i < watch[idx].len() {
                let cid = watch[idx][i];
                let (w0, w1) = watch_pos[cid];
                let (wl_pos, other_pos) = if clauses[cid][w0] == lit_false { (w0, w1) } else { (w1, w0) };
                let other_lit = clauses[cid][other_pos];
                if eval_lit_fn(assign, other_lit) == 1 { i += 1; continue; }

                // try to find replacement literal
                let mut found = false;
                for k in 0..clauses[cid].len() {
                    if k == other_pos { continue; }
                    let lk = clauses[cid][k];
                    if eval_lit_fn(assign, lk) != -1 {
                        if wl_pos == w0 { watch_pos[cid].0 = k; } else { watch_pos[cid].1 = k; }
                        // remove cid from current watch list
                        let last = watch[idx].len() - 1;
                        watch[idx][i] = watch[idx][last];
                        watch[idx].pop();
                        // add cid to new watch list
                        let new_idx = { let v = (lk.abs() - 1) as usize; if lk > 0 { 2 * v } else { 2 * v + 1 } };
                        watch[new_idx].push(cid);
                        found = true;
                        break;
                    }
                }
                if found { continue; }

                let other_val = eval_lit_fn(assign, other_lit);
                if other_val == -1 { *prop_head = ph; return Some(cid); }
                // unit: assign other_lit
                if !assign_literal_fn(assign, trail, other_lit) { *prop_head = ph; return Some(cid); }
                i += 1;
            }
        }
        *prop_head = ph;
        None
    }

    // Seed initial assignments if present
    if let Some(init) = initial_assignments {
        for (name, &val) in init.iter() {
            if let Some(&vid) = var_index.get(name) {
                let lit = if val { (vid as i32) + 1 } else { -((vid as i32) + 1) };
                if !assign_literal_fn(&mut assign, &mut trail, lit) { return None; }
            }
        }
        if let Some(_c) = propagate_fn(&mut prop_head, &mut trail, &mut assign, &mut watch, &mut watch_pos, &clauses) { return None; }
    }

    // Main DPLL loop with watched-literal propagation and simple backtracking
    loop {
        // Check for completion
        if assign.iter().all(|&a| a != 0) {
            let mut out: HashMap<String, bool> = HashMap::new();
            for (i, name) in vars.iter().enumerate() { out.insert(name.clone(), assign[i] == 1); }
            return Some(out);
        }

        // Pick branching variable: prioritized vars if provided, otherwise highest score
        let mut pick: Option<usize> = None;
        if let Some(pvars) = prioritized_vars {
            for pv in pvars.iter() {
                if let Some(&vid) = var_index.get(pv) {
                    if assign[vid] == 0 { pick = Some(vid); break; }
                }
            }
        }
        if pick.is_none() {
            let mut best_score: u32 = 0;
            for v in 0..n_vars {
                if assign[v] == 0 && scores[v] >= best_score { best_score = scores[v]; pick = Some(v); }
            }
        }

        let var = match pick {
            Some(v) => v,
            None => match assign.iter().position(|&a| a == 0) { Some(p) => p, None => { // all assigned
                let mut out: HashMap<String,bool> = HashMap::new(); for (i, name) in vars.iter().enumerate() { out.insert(name.clone(), assign[i] == 1); } return Some(out);
            } }
        };

        // Make decision: try true first
        let prev_trail_len = trail.len();
        decisions.push((var, true, prev_trail_len));
        if !assign_literal_fn(&mut assign, &mut trail, (var as i32) + 1) {
            // immediate contradiction; will be handled by propagate
        }

        // Propagate and handle conflicts by backtracking
        loop {
            if let Some(conf_cid) = propagate_fn(&mut prop_head, &mut trail, &mut assign, &mut watch, &mut watch_pos, &clauses) {
                // bump scores for variables in conflict clause
                for &lit in clauses[conf_cid].iter() {
                    let v = (lit.abs() - 1) as usize;
                    scores[v] = scores[v].saturating_add(1);
                }
                if scores.iter().copied().max().unwrap_or(0) > (1 << 30) {
                    for s in scores.iter_mut() { *s >>= 1; }
                }

                // backtrack
                let mut flipped = false;
                while let Some((dvar, dval, dtrail)) = decisions.pop() {
                    while trail.len() > dtrail {
                        let l = trail.pop().unwrap(); let vv = (l.abs() - 1) as usize; assign[vv] = 0;
                    }
                    prop_head = trail.len();
                    if dval {
                        decisions.push((dvar, false, dtrail));
                        if assign_literal_fn(&mut assign, &mut trail, -((dvar as i32) + 1)) {
                            flipped = true; break;
                        } else { continue; }
                    } else { continue; }
                }
                if !flipped { return None; }
                continue;
            } else { break; }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit(v: &str, neg: bool) -> Literal { Literal { var: v.to_string(), neg } }

    #[test]
    fn solve_simple_true() {
        let cnf = CNF::new(vec![vec![lit("A", false)]]);
        let res = solve_cnf(&cnf, None, None).unwrap();
        assert_eq!(res["A"], true);
    }

    #[test]
    fn solve_simple_or() {
        let cnf = CNF::new(vec![vec![lit("A", false), lit("B", false)]]);
        let res = solve_cnf(&cnf, None, None).unwrap();
        assert!(res["A"] || res["B"]);
    }
}
