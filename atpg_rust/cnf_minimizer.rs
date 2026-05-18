use crate::cnf::{CNF, Literal};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

/// Modulo: cnf_minimizer
/// Tecniche per "minimizing the search tree" (parte 4)
/// Ogni funzione implementa una singola sotto-tecnica.

fn opposite(l: &Literal) -> Literal {
    Literal { var: l.var.clone(), neg: !l.neg }
}

/// Rimuove clausole tautologiche (contenenti X e !X).
/// Restituisce il numero di clausole rimosse.
pub fn remove_tautological_clauses(cnf: &mut CNF) -> usize {
    let mut new_clauses: Vec<Vec<Literal>> = Vec::new();
    let mut removed = 0;
    for clause in cnf.clauses.iter() {
        let mut is_taut = false;
        for l in clause.iter() {
            if clause.iter().any(|l2| l2.var == l.var && l2.neg != l.neg) {
                is_taut = true;
                break;
            }
        }
        if is_taut {
            removed += 1;
        } else {
            new_clauses.push(clause.clone());
        }
    }
    cnf.clauses = new_clauses;
    removed
}

/// Elimina letterali puri (compaiono solo con un segno) e rimuove
/// tutte le clausole soddisfatte da questi letterali.
/// Restituisce la lista dei letterali assegnati (in ordine di scoperta).
pub fn eliminate_pure_literals(cnf: &mut CNF) -> Vec<Literal> {
    let mut assigned: Vec<Literal> = Vec::new();

    loop {
        let mut sign_map: HashMap<String, (bool, bool)> = HashMap::new(); // (pos, neg)
        for clause in cnf.clauses.iter() {
            for lit in clause.iter() {
                let entry = sign_map.entry(lit.var.clone()).or_insert((false, false));
                if lit.neg { entry.1 = true; } else { entry.0 = true; }
            }
        }

        let mut pure_literals: Vec<Literal> = Vec::new();
        for (var, (pos, neg)) in sign_map.iter() {
            if *pos && !*neg {
                pure_literals.push(Literal { var: var.clone(), neg: false });
            } else if !*pos && *neg {
                pure_literals.push(Literal { var: var.clone(), neg: true });
            }
        }

        if pure_literals.is_empty() { break; }

        // Rimuovi clausole contenenti uno qualsiasi dei pure_literals
        let pure_set: HashSet<Literal> = pure_literals.iter().cloned().collect();
        let mut new_clauses: Vec<Vec<Literal>> = Vec::new();
        for clause in cnf.clauses.iter() {
            if clause.iter().any(|l| pure_set.contains(l)) {
                // clausola soddisfatta dall'assegnamento
                continue;
            }
            new_clauses.push(clause.clone());
        }
        assigned.extend(pure_literals.into_iter());
        cnf.clauses = new_clauses;
    }

    assigned
}

/// Propagazione unitaria (unit propagation, BCP semplice).
/// Applica ripetutamente unit clauses; ritorna Err(()) in caso di conflitto (clausola vuota).
/// Restituisce Ok(true) se la CNF è stata modificata, Ok(false) se non c'erano unit.
pub fn unit_propagation(cnf: &mut CNF) -> Result<bool, ()> {
    let mut changed = false;
    loop {
        let maybe_unit: Option<Literal> = cnf.clauses.iter()
            .find(|c| c.len() == 1)
            .map(|c| c[0].clone());

        let unit = match maybe_unit {
            None => break,
            Some(u) => u,
        };
        changed = true;

        let opp = opposite(&unit);
        let mut new_clauses: Vec<Vec<Literal>> = Vec::new();

        for clause in cnf.clauses.iter() {
            // se la clausola è soddisfatta dal unit, saltala
            if clause.iter().any(|l| l == &unit) { continue; }

            // altrimenti rimuovi il literal opposto dalla clausola
            let filtered: Vec<Literal> = clause.iter()
                .filter(|l| !(l.var == opp.var && l.neg == opp.neg))
                .cloned()
                .collect();

            if filtered.is_empty() {
                // conflitto: clausola vuota
                return Err(());
            }
            new_clauses.push(filtered);
        }

        cnf.clauses = new_clauses;
    }
    Ok(changed)
}

/// Eliminazione per subsumption: se una clausola A è sottoinsieme di B allora B può essere rimossa.
/// Restituisce il numero di clausole rimosse.
pub fn subsumption_elimination(cnf: &mut CNF) -> usize {
    let n = cnf.clauses.len();
    if n <= 1 { return 0; }
    let mut keep = vec![true; n];
    let mut removed = 0;

    // Pre-converti in set per controlli più semplici
    let sets: Vec<HashSet<Literal>> = cnf.clauses.iter().map(|c| c.iter().cloned().collect()).collect();

    for i in 0..n {
        if !keep[i] { continue; }
        for j in 0..n {
            if i == j || !keep[j] { continue; }
            let si = &sets[i];
            let sj = &sets[j];
            if si.len() <= sj.len() && si.is_subset(sj) {
                // i subsumes j -> rimuovi j
                keep[j] = false;
                removed += 1;
            }
        }
    }

    let mut new_clauses: Vec<Vec<Literal>> = Vec::new();
    for (idx, c) in cnf.clauses.iter().enumerate() {
        if keep[idx] { new_clauses.push(c.clone()); }
    }
    cnf.clauses = new_clauses;
    removed
}

/// Semplifica clausole: rimuove duplicati letterali interni e clausole tautologiche.
/// Restituisce il numero di clausole o letterali modificati/rimossi (indicativo).
pub fn simplify_clauses(cnf: &mut CNF) -> usize {
    let mut changed = 0;
    let mut new_clauses: Vec<Vec<Literal>> = Vec::new();

    for clause in cnf.clauses.iter() {
        let mut set: HashSet<Literal> = HashSet::new();
        for l in clause.iter() { set.insert(l.clone()); }

        // se tautologia -> salta clausola
        let mut is_taut = false;
        for l in set.iter() {
            if set.contains(&opposite(l)) { is_taut = true; break; }
        }
        if is_taut {
            changed += 1;
            continue;
        }

        if set.len() != clause.len() {
            changed += 1;
        }

        // ricrea clausola dall'insieme (ordine non garantito, ma valido)
        new_clauses.push(set.into_iter().collect());
    }

    cnf.clauses = new_clauses;
    changed
}

/// Identifica variabili "determined" secondo la relazione descritta nel paper
/// e rimuove tutte le clausole contenenti le variabili determinate.
/// Ritorna la lista delle variabili rimosse.
pub fn determine_and_remove(cnf: &mut CNF) -> Vec<String> {
    use std::collections::HashSet;

    let mut removed = Vec::new();

    // Colleziona variabili presenti nella CNF
    let vars: Vec<String> = cnf.clauses.iter().flat_map(|cl| cl.iter().map(|l| l.var.clone())).collect::<HashSet<_>>().into_iter().collect();
    let mut to_remove: HashSet<String> = HashSet::new();

    // Ottimizzazione: per ogni v clonato la CNF solo due volte (v=true, v=false)
    // e poi si analizza la polarità di tutte le w su quelle due formule.
    for v in &vars {
        // skip if v already marked for removal
        if to_remove.contains(v) { continue; }

        if !crate::options::get_options().quiet && crate::PRINT_PROGRESS { println!("determine_and_remove: testing v={} (vars={})", v, vars.len()); }

        // prepare CNF with v=true
        let mut tmp_true = cnf.clone();
        tmp_true.clauses.push(vec![Literal { var: v.clone(), neg: false }]);
        let ok_true = unit_propagation(&mut tmp_true).is_ok();

        // prepare CNF with v=false
        let mut tmp_false = cnf.clone();
        tmp_false.clauses.push(vec![Literal { var: v.clone(), neg: true }]);
        let ok_false = unit_propagation(&mut tmp_false).is_ok();

        // If either propagation failed, we cannot rely on v to determine others (mirror previous behavior)
        if !ok_true || !ok_false {
            continue;
        }

        // For each candidate w, check that in both tmp_true and tmp_false w doesn't appear with both polarities
        // (i.e., is pure or absent in each propagated formula). If so, we can remove w.
        // Build polarity maps for tmp_true and tmp_false
        let mut pol_true: HashMap<String, (bool, bool)> = HashMap::new();
        for clause in tmp_true.clauses.iter() {
            for lit in clause.iter() {
                let e = pol_true.entry(lit.var.clone()).or_insert((false, false));
                if lit.neg { e.1 = true; } else { e.0 = true; }
            }
        }
        let mut pol_false: HashMap<String, (bool, bool)> = HashMap::new();
        for clause in tmp_false.clauses.iter() {
            for lit in clause.iter() {
                let e = pol_false.entry(lit.var.clone()).or_insert((false, false));
                if lit.neg { e.1 = true; } else { e.0 = true; }
            }
        }

        for w in &vars {
            if v == w { continue; }
            if to_remove.contains(w) { continue; }

            let (tpos, tneg) = pol_true.get(w).cloned().unwrap_or((false, false));
            let (fpos, fneg) = pol_false.get(w).cloned().unwrap_or((false, false));

            // if in either propagated formula w has both polarities, cannot remove
            if (tpos && tneg) || (fpos && fneg) {
                continue;
            }

            // otherwise w can be removed
            to_remove.insert(w.clone());
        }
    }

    if !to_remove.is_empty() {
        cnf.clauses.retain(|cl| !cl.iter().any(|lit| to_remove.contains(&lit.var)));
        removed = to_remove.into_iter().collect();
    }

    removed
}

/// Semplice euristica di branching: scegli la variabile con maggior numero di occorrenze.
/// Restituisce `None` se non ci sono variabili.
pub fn choose_branching_variable(cnf: &CNF) -> Option<String> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for clause in cnf.clauses.iter() {
        for lit in clause.iter() {
            *counts.entry(lit.var.clone()).or_insert(0) += 1;
        }
    }
    counts.into_iter().max_by_key(|(_, c)| *c).map(|(v, _)| v)
}

/// Funzione di alto livello che applica ripetutamente le tecniche per ridurre
/// l'albero di ricerca fino al punto fisso. Restituisce Err(()) in caso di conflitto.
pub fn minimize_search_tree(cnf: &mut CNF) -> Result<bool, ()> {
    let mut any_changed = false;
    let global_start = Instant::now();
    if !crate::options::get_options().quiet && crate::PRINT_PROGRESS { println!("minimize_search_tree: start"); }
    loop {
        let mut changed = false;

        let removed_taut = remove_tautological_clauses(cnf);
        if removed_taut > 0 { changed = true; any_changed = true; }

        let simp = simplify_clauses(cnf);
        if simp > 0 { changed = true; any_changed = true; }

        // unit propagation può rilevare conflitti
        match unit_propagation(cnf) {
            Ok(u) => if u { changed = true; any_changed = true; },
            Err(_) => return Err(()),
        }

        let assigned_pures = eliminate_pure_literals(cnf);
        if !assigned_pures.is_empty() { changed = true; any_changed = true; }

        let subs = subsumption_elimination(cnf);
        if subs > 0 { changed = true; any_changed = true; }

        // Try to find and remove variables determined by others (see paper Sec.4.1)
        if !crate::options::get_options().quiet && crate::PRINT_PROGRESS { println!("minimize_search_tree: running determine_and_remove..."); }
        let t0 = Instant::now();
        let determined_removed = determine_and_remove(cnf);
        let took = t0.elapsed();
        if !crate::options::get_options().quiet && crate::PRINT_PROGRESS { println!("determine_and_remove removed {} vars in {:?}", determined_removed.len(), took); }
        if !determined_removed.is_empty() { changed = true; any_changed = true; }

        if !changed { break; }
    }
    if !crate::options::get_options().quiet && crate::PRINT_PROGRESS { println!("minimize_search_tree: total time {:?}", global_start.elapsed()); }
    Ok(any_changed)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::cnf::Literal;

    fn lit(v: &str, neg: bool) -> Literal { Literal { var: v.to_string(), neg } }

    #[test]
    fn test_remove_tautology() {
        let clauses = vec![vec![lit("A", false), lit("A", true)], vec![lit("B", false)]];
        let mut c = CNF::new(clauses);
        let r = remove_tautological_clauses(&mut c);
        assert_eq!(r, 1);
        assert_eq!(c.get_n_clauses(), 1);
    }

    #[test]
    fn test_unit_propagation_basic() {
        // (A) · (!A + B)  => dopo unit propagation: (B)
        let clauses = vec![vec![lit("A", false)], vec![lit("A", true), lit("B", false)]];
        let mut c = CNF::new(clauses);
        let res = unit_propagation(&mut c);
        assert!(res.is_ok());
        assert!(res.unwrap());
        // unit propagation è applicata al punto fisso: la formula può diventare soddisfatta (nessuna clausola)
        assert_eq!(c.get_n_clauses(), 0);
    }

    #[test]
    fn test_eliminate_pure_literals() {
        // (A + B) · (A) · (C + !D) -> A è puro positivo -> rimuove clausole con A
        let clauses = vec![vec![lit("A", false), lit("B", false)], vec![lit("A", false)], vec![lit("C", false), lit("D", true)]];
        let mut c = CNF::new(clauses);
        let assigned = eliminate_pure_literals(&mut c);
        assert!(assigned.iter().any(|l| l.var == "A"));
        // in questo esempio A, C e D sono puri: l'eliminazione rimuove tutte le clausole
        assert_eq!(c.clauses.len(), 0);
    }

    #[test]
    fn test_determine_and_remove_basic() {
        // V determines W since both (V=true) and (V=false) leave W only positive
        // Clauses: (V + W) · (!V + W) · (X)
        let clauses = vec![
            vec![lit("V", false), lit("W", false)],
            vec![lit("V", true),  lit("W", false)],
            vec![lit("X", false)],
        ];
        let mut c = CNF::new(clauses);
        let removed = determine_and_remove(&mut c);
        // W should be removed
        assert!(removed.contains(&"W".to_string()));
        // Remaining clauses should not contain W
        for cl in c.clauses.iter() {
            for lit in cl.iter() { assert_ne!(lit.var, "W"); }
        }
    }
}
