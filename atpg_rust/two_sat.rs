use crate::cnf::{CNF, Literal};
use std::collections::{HashMap, VecDeque};

type Graph = Vec<Vec<usize>>;

fn build_implication_graph(cnf: &CNF) -> (Graph, Graph, Vec<String>) {
    // deterministically collect variables
    let mut vars: Vec<String> = cnf.clauses.iter().flat_map(|cl| cl.iter().map(|lit| lit.var.clone())).collect();
    vars.sort();
    vars.dedup();
    let m = vars.len();

    // mapping var -> index 0..m-1
    let var_index: HashMap<String, usize> = vars.iter().cloned().enumerate().map(|(i, v)| (v, i)).collect();

    let mut graph: Graph = vec![vec![]; 2 * m];
    let mut rev_graph: Graph = vec![vec![]; 2 * m];

    let lit_index = |l: &Literal| -> usize {
        let i = *var_index.get(&l.var).unwrap();
        2 * i + (if l.neg { 1 } else { 0 })
    };

    for clause in &cnf.clauses {
        if clause.len() == 2 {
            let a = &clause[0];
            let b = &clause[1];
            let ai = lit_index(a);
            let bi = lit_index(b);

            // add implications: (¬a -> b) and (¬b -> a)
            graph[ai ^ 1].push(bi);
            rev_graph[bi].push(ai ^ 1);
            graph[bi ^ 1].push(ai);
            rev_graph[ai].push(bi ^ 1);
        } else if clause.len() == 1 {
            let a = &clause[0];
            let ai = lit_index(a);
            // add implication: (¬a -> a)
            graph[ai ^ 1].push(ai);
            rev_graph[ai].push(ai ^ 1);
        }
    }

    (graph, rev_graph, vars)
}

fn kosaraju_scc(graph: &Graph, rev_graph: &Graph) -> Vec<i32> {
    let n = graph.len();
    let mut visited = vec![false; n];
    let mut stack = VecDeque::new();
    
    fn dfs1(v: usize, g: &Graph, vis: &mut Vec<bool>, stack: &mut VecDeque<usize>) {
        vis[v] = true;
        for &w in &g[v] {
            if !vis[w] { dfs1(w, g, vis, stack); }
        }
        stack.push_front(v);
    }

    for v in 0..n { if !visited[v] { dfs1(v, graph, &mut visited, &mut stack); } }

    let mut comp = vec![-1; n];
    let mut comp_id = 0usize;

    fn dfs2(v: usize, g: &Graph, comp: &mut Vec<i32>, id: i32) {
        comp[v] = id;
        for &w in &g[v] {
            if comp[w] == -1 { dfs2(w, g, comp, id); }
        }
    }

    while let Some(v) = stack.pop_front() {
        if comp[v] == -1 {
            dfs2(v, rev_graph, &mut comp, comp_id as i32);
            comp_id += 1;
        }
    }

    comp
}

pub fn solve_2sat(cnf: &CNF) -> Option<Vec<bool>> {
    let (graph, rev_graph, vars) = build_implication_graph(cnf);
    let comp = kosaraju_scc(&graph, &rev_graph);
    let m = vars.len();

    // unsat if var and its negation are in same comp
    for i in 0..m {
        if comp[2*i] == comp[2*i + 1] { return None; }
    }

    // assignment: variable true if comp[true] > comp[false] (deterministic)
    let mut assignment = vec![false; m];
    for i in 0..m {
        assignment[i] = comp[2*i] > comp[2*i + 1];
    }

    Some(assignment)
}

pub fn generate_all_assignments(cnf: &CNF, max_solutions: Option<usize>) -> Option<Vec<HashMap<String, bool>>> {
    // Enumerate up to max_solutions satisfying assignments deterministically using DFS
    let max_solutions = max_solutions.unwrap_or(1usize);
    let mut solutions: Vec<HashMap<String, bool>> = Vec::new();

    // collect vars deterministically
    let mut vars: Vec<String> = cnf.clauses.iter().flat_map(|cl| cl.iter().map(|lit| lit.var.clone())).collect();
    vars.sort(); vars.dedup();
    let m = vars.len();

    // --- HEURISTIC VARIABLE ORDERING ---
    // 1. Count appearances in ternary clauses
    let mut ternary_counts: HashMap<String, usize> = HashMap::new();
    for clause in &cnf.clauses {
        if clause.len() > 2 { // Consider clauses with 3 or more literals as "ternary" for this heuristic
            for literal in clause {
                *ternary_counts.entry(literal.var.clone()).or_insert(0) += 1;
            }
        }
    }

    // 2. Build implication graph to count constraints
    let (graph, _, _) = build_implication_graph(cnf);
    let var_index: HashMap<String, usize> = vars.iter().cloned().enumerate().map(|(i, v)| (v, i)).collect();

    // 3. Calculate scores and sort variables
    let mut scored_vars: Vec<(String, usize)> = vars.iter().map(|var| {
        let ternary_score = *ternary_counts.get(var).unwrap_or(&0);
        
        let index = *var_index.get(var).unwrap();
        let implication_score = graph[2 * index].len() + graph[2 * index + 1].len();

        let total_score = ternary_score + implication_score;
        (var.clone(), total_score)
    }).collect();

    // Sort variables by score in descending order.
    scored_vars.sort_by(|a, b| b.1.cmp(&a.1));
    let sorted_vars: Vec<String> = scored_vars.into_iter().map(|(var, _)| var).collect();
    // --- END HEURISTIC ---

    // helper: check satisfiable with prefix assignments
    let cnf_base = cnf.clone();
    fn is_sat_with_prefix(base: &CNF, prefix: &[(String, bool)]) -> bool {
        let mut augmented = base.clone();
        for (v, val) in prefix {
            augmented.clauses.push(vec![Literal { var: v.clone(), neg: !*val }]);
        }
        solve_2sat(&augmented).is_some()
    }

    fn dfs(i: usize, prefix: &mut Vec<(String, bool)>, vars: &Vec<String>, base: &CNF, sols: &mut Vec<HashMap<String,bool>>, max_solutions: usize) {
        if sols.len() >= max_solutions { return; }
        if i == vars.len() {
            // get a full satisfying assignment
            let mut augmented = base.clone();
            for (v, val) in prefix.iter() { augmented.clauses.push(vec![Literal { var: v.clone(), neg: !*val }]); }
                if let Some(ass) = solve_2sat(&augmented) {
                // Create a map from var name to its boolean value for the final solution
                let mut final_assignment_map = HashMap::new();
                let var_to_index: HashMap<String, usize> = vars.iter().cloned().enumerate().map(|(i, v)| (v, i)).collect();
                
                for (var_name, value) in prefix.iter() {
                    final_assignment_map.insert(var_name.clone(), *value);
                }

                // For variables not in the prefix, their values are determined by the final solve_2sat call
                let mut temp_cnf = base.clone();
                for (p_var, p_val) in prefix.iter() {
                    temp_cnf.clauses.push(vec![Literal { var: p_var.clone(), neg: !*p_val }]);
                }
                if let Some(final_ass) = solve_2sat(&temp_cnf) {
                    // Use deterministic variable ordering from implication graph
                    let (_g, _rg, final_vars) = build_implication_graph(&temp_cnf);
                    for (idx, var) in final_vars.iter().enumerate() {
                        if !final_assignment_map.contains_key(var) {
                            if idx < final_ass.len() {
                                final_assignment_map.insert(var.clone(), final_ass[idx]);
                            }
                        }
                    }
                }

                sols.push(final_assignment_map);
            }
            return;
        }

        for &val in &[false, true] {
            prefix.push((vars[i].clone(), val));
            if is_sat_with_prefix(base, prefix) {
                dfs(i+1, prefix, vars, base, sols, max_solutions);
            }
            prefix.pop();
            if sols.len() >= max_solutions { return; }
        }
    }

    let mut prefix: Vec<(String,bool)> = Vec::new();
    dfs(0, &mut prefix, &sorted_vars, &cnf_base, &mut solutions, max_solutions);

    if solutions.is_empty() { None } else { Some(solutions) }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solve_2sat_simple_true() {
        // Clause (A) -> A must be true
        let cnf = CNF::new(vec![vec![Literal { var: "A".to_string(), neg: false }]]);
        let sol = solve_2sat(&cnf).unwrap();
        // single variable A -> should be true
        assert_eq!(sol.len(), 1);
        assert_eq!(sol[0], true);
    }

    #[test]
    fn test_solve_2sat_unsat() {
        // Clauses (A) and (not A) -> unsat
        let cnf = CNF::new(vec![
            vec![Literal { var: "A".to_string(), neg: false }],
            vec![Literal { var: "A".to_string(), neg: true }],
        ]);
        assert!(solve_2sat(&cnf).is_none());
    }

    #[test]
    fn test_generate_all_assignments() {
        // Clause (A or B)
        let cnf = CNF::new(vec![vec![Literal { var: "A".to_string(), neg: false }, Literal { var: "B".to_string(), neg: false }]]);
        let sols = generate_all_assignments(&cnf, None).unwrap();
        assert!(!sols.is_empty());
        for map in sols {
            // maps should contain A and B
            assert!(map.contains_key("A"));
            assert!(map.contains_key("B"));
        }
    }

    #[test]
    fn test_generate_all_assignments_mapping() {
        // Verify that generate_all_assignments maps final_ass values to variable names
        // using the deterministic implication graph ordering.
        let cnf = CNF::new(vec![vec![Literal { var: "A".to_string(), neg: false }, Literal { var: "B".to_string(), neg: false }]]);
        let sols = generate_all_assignments(&cnf, None).unwrap();
        assert!(!sols.is_empty());

        for sol_map in sols {
            // Build temp CNF by adding unit clauses for the assignment
            let mut temp = cnf.clone();
            for (v, val) in sol_map.iter() {
                temp.clauses.push(vec![Literal { var: v.clone(), neg: !*val }]);
            }

            // Solve 2SAT for the augmented CNF and obtain assignment vector
            let ass_opt = solve_2sat(&temp);
            assert!(ass_opt.is_some());
            let ass = ass_opt.unwrap();

            // Get deterministic variable ordering from implication graph
            let (_g, _rg, vars_order) = build_implication_graph(&temp);
            for (i, var_name) in vars_order.iter().enumerate() {
                assert!(sol_map.contains_key(var_name));
                assert_eq!(sol_map[var_name], ass[i]);
            }
        }
    }
}
