use crate::pattern_generator::InputPattern;
use crate::dag::Dag;
use crate::ppsfp::{Fault, PPSFPSimulator};
use crate::sat::generate_all_sat_solutions;
use std::collections::{BTreeSet, HashMap};

type Candidate = (InputPattern, usize, BTreeSet<Fault>);

/// come intendo procedere:
/// 1. per ogni fault: generare tutti i pattern che lo rilevano usando il sat solver
/// 2. costruire una matrice binaria (o una struttura dati equivalente) che rappresenta
///    quali pattern rilevano quali fault
/// 3. applicare un algoritmo di minimizzazione degli insiemi per trovare il sottoinsieme 
///    minimo di pattern che copre tutti i fault (set cover?))
pub fn compute_minimal_input_set(mut dag: Dag, faults: BTreeSet<Fault>, initial_patterns: Option<Vec<InputPattern>>) -> (Vec<Candidate>, BTreeSet<Fault>) {
    
    let mut undetected_faults: BTreeSet<Fault> = faults.iter().cloned().collect();
    // Collect primary inputs from DAG: inputs that are not outputs of any node
    let mut all_outputs: BTreeSet<String> = BTreeSet::new();
    for n in &dag.nodes { all_outputs.insert(n.outputs[0].clone()); }
    let mut primary_inputs: Vec<String> = Vec::new();
    for n in &dag.nodes {
        for inp in &n.inputs {
            if !all_outputs.contains(inp) { primary_inputs.push(inp.clone()); }
        }
    }
    primary_inputs.sort(); primary_inputs.dedup();

    // map: pattern+pos -> (packed InputPattern, row position, detected faults)
    let mut pattern_map: HashMap<String, Candidate> = HashMap::new();

    // simulator to test single-bit coverage from packed candidates
    let mut sim = PPSFPSimulator::new(&dag);

    let insert_candidate = |
        candidate: &InputPattern,
        pos: usize,
        faults_to_test: &BTreeSet<Fault>,
        sim: &mut PPSFPSimulator<'_>,
        pattern_map: &mut HashMap<String, Candidate>,
    | {
        let mut detected_by_pat: BTreeSet<Fault> = BTreeSet::new();
        for fault in faults_to_test.iter() {
            if sim.simulate_single_input(candidate, pos, fault) != 0 {
                detected_by_pat.insert(fault.clone());
            }
        }
        if !detected_by_pat.is_empty() {
            let key = format!("{}\n@pos={}", candidate.to_string(), pos);
            pattern_map.entry(key).or_insert_with(|| (candidate.clone(), pos, BTreeSet::new())).2.extend(detected_by_pat);
        }
    };

    let pack_rows = |rows: Vec<Vec<u32>>, inputs: Vec<String>| -> Vec<InputPattern> {
        let mut packed_patterns: Vec<InputPattern> = Vec::new();
        for chunk in rows.chunks(32) {
            let mut packed = InputPattern::from_strings(inputs.clone());
            for (pos, row_bits) in chunk.iter().enumerate() {
                packed.insert_pattern_at_position(row_bits.clone(), pos, inputs.clone());
            }
            packed_patterns.push(packed);
        }
        packed_patterns
    };

    // For small circuits, enumerate every input assignment exactly, but pack up to 32
    // assignments into a single InputPattern and then test each row by position.
    if primary_inputs.len() <= 12 {
        let total = 1usize << primary_inputs.len();
        let mut rows: Vec<Vec<u32>> = Vec::with_capacity(total);
        for comb in 0..total {
            let mut row_bits: Vec<u32> = Vec::with_capacity(primary_inputs.len());
            for (idx, _inp) in primary_inputs.iter().enumerate() {
                let bit = ((comb >> (primary_inputs.len() - 1 - idx)) & 1) as u32;
                row_bits.push(bit);
            }
            rows.push(row_bits);
        }
        for packed in pack_rows(rows, primary_inputs.clone()) {
            for pos in 0..32usize {
                insert_candidate(&packed, pos, &undetected_faults, &mut sim, &mut pattern_map);
            }
        }
    }

    // If initial patterns were provided (e.g., random patterns from main), simulate them and insert into pattern_map.
    if let Some(init_pats) = initial_patterns {
        for pat in init_pats.iter() {
            for pos in 0..32usize {
                insert_candidate(pat, pos, &undetected_faults, &mut sim, &mut pattern_map);
            }
        }
    }

    // SAT-derived candidates: collect rows, pack them in blocks of 32, then test each row.
    let mut sat_rows: Vec<Vec<u32>> = Vec::new();
    for fault in undetected_faults.clone() {
        let pruning_for_distinct_inputs = true; // we want distinct patterns for different faults
        let models = generate_all_sat_solutions(&dag, &fault, Some(pruning_for_distinct_inputs));
        // If the fault is stuck-at-1, one SAT model is sufficient for our purposes — don't enumerate more for this fault.
        let take_only_one_for_this_fault = fault.sa1 == true;
        for m in models {
            let mut row_bits: Vec<u32> = Vec::with_capacity(primary_inputs.len());
            for inp in &primary_inputs {
                let mut bit = 0u32;
                for (name, v) in m.iter() {
                    if name == inp {
                        bit = *v & 1u32;
                        break;
                    }
                }
                row_bits.push(bit);
            }

            sat_rows.push(row_bits);

            if take_only_one_for_this_fault {
                // move to next fault after first valid model for this fault
                break;
            }
        }
    }

    for packed in pack_rows(sat_rows, primary_inputs.clone()) {
        for pos in 0..32usize {
            insert_candidate(&packed, pos, &undetected_faults, &mut sim, &mut pattern_map);
        }
    }

    // Use the exact cover search when possible; it falls back to the best partial cover if
    // some faults cannot be covered by any pattern.
    let (minimal_input_set_with_sets, remaining) = perfect_set_cover(pattern_map, undetected_faults.clone());
    undetected_faults = remaining;
    (minimal_input_set_with_sets, undetected_faults)
}

/// Exact set-cover search with pruning.
/// Returns the smallest set of patterns that covers the maximum number of faults.
pub fn perfect_set_cover(pattern_map: HashMap<String, Candidate>, universe: BTreeSet<Fault>) -> (Vec<Candidate>, BTreeSet<Fault>) {
    let mut candidates: Vec<(String, InputPattern, usize, BTreeSet<Fault>)> = pattern_map
        .into_iter()
        .filter(|(_, (_, _, faults))| !faults.is_empty())
        .map(|(key, (pattern, pos, faults))| (key, pattern, pos, faults))
        .collect();

    candidates.sort_by(|a, b| {
        b.3.len()
            .cmp(&a.3.len())
            .then_with(|| a.0.cmp(&b.0))
    });

    let mut suffix_unions: Vec<BTreeSet<Fault>> = vec![BTreeSet::new(); candidates.len() + 1];
    for idx in (0..candidates.len()).rev() {
        let mut union = suffix_unions[idx + 1].clone();
        for fault in candidates[idx].3.iter() {
            union.insert(fault.clone());
        }
        suffix_unions[idx] = union;
    }

    let (greedy_selected, greedy_remaining) = {
        let greedy_map: HashMap<String, Candidate> = candidates
            .iter()
            .map(|(key, pattern, pos, faults)| (key.clone(), (pattern.clone(), *pos, faults.clone())))
            .collect();
        greedy_set_cover(greedy_map, universe.clone())
    };

    let mut best_selected = greedy_selected;
    let mut best_remaining = greedy_remaining;

    fn update_best(
        selected: &[Candidate],
        remaining: &BTreeSet<Fault>,
        best_selected: &mut Vec<Candidate>,
        best_remaining: &mut BTreeSet<Fault>,
    ) {
        let better_remaining = remaining.len() < best_remaining.len();
        let same_remaining_but_shorter = remaining.len() == best_remaining.len() && selected.len() < best_selected.len();
        if better_remaining || same_remaining_but_shorter {
            *best_selected = selected.to_vec();
            *best_remaining = remaining.clone();
        }
    }

    fn dfs(
        idx: usize,
        candidates: &[(String, InputPattern, usize, BTreeSet<Fault>)],
        suffix_unions: &[BTreeSet<Fault>],
        remaining: &BTreeSet<Fault>,
        selected: &mut Vec<Candidate>,
        best_selected: &mut Vec<Candidate>,
        best_remaining: &mut BTreeSet<Fault>,
    ) {
        let current_uncovered = remaining.len();
        if current_uncovered == 0 {
            update_best(selected, remaining, best_selected, best_remaining);
            return;
        }

        if idx >= candidates.len() {
            update_best(selected, remaining, best_selected, best_remaining);
            return;
        }

        let possible_cover_count = suffix_unions[idx].intersection(remaining).count();
        let min_uncovered_possible = current_uncovered.saturating_sub(possible_cover_count);
        if min_uncovered_possible > best_remaining.len() {
            return;
        }
        if min_uncovered_possible == best_remaining.len() && selected.len() >= best_selected.len() {
            return;
        }

        let (_, pattern, pos, faults) = &candidates[idx];

        if !faults.is_disjoint(remaining) {
            let mut next_remaining = remaining.clone();
            for fault in faults.iter() {
                next_remaining.remove(fault);
            }
            selected.push((pattern.clone(), *pos, faults.clone()));
            dfs(
                idx + 1,
                candidates,
                suffix_unions,
                &next_remaining,
                selected,
                best_selected,
                best_remaining,
            );
            selected.pop();
        }

        dfs(
            idx + 1,
            candidates,
            suffix_unions,
            remaining,
            selected,
            best_selected,
            best_remaining,
        );
    }

    let mut selected: Vec<Candidate> = Vec::new();
    let initial_remaining = universe.clone();
    dfs(
        0,
        &candidates,
        &suffix_unions,
        &initial_remaining,
        &mut selected,
        &mut best_selected,
        &mut best_remaining,
    );

    (best_selected, best_remaining)
}

/// Greedy set-cover: given a map pattern_key -> (InputPattern, set of Faults it detects)
/// and the universe of faults to cover, returns (selected patterns, remaining uncovered faults)
pub fn greedy_set_cover(mut pattern_map: HashMap<String, Candidate>, universe: BTreeSet<Fault>) -> (Vec<Candidate>, BTreeSet<Fault>) {
    let mut selected: Vec<Candidate> = Vec::new();
    let mut remaining = universe.clone();

    while !remaining.is_empty() {
        let mut best_key: Option<String> = None;
        let mut best_cover_count = 0usize;
        for (k, (_p, _pos, fset)) in pattern_map.iter() {
            let cover = fset.intersection(&remaining).count();
            if cover > best_cover_count {
                best_cover_count = cover;
                best_key = Some(k.clone());
            }
        }
        if best_cover_count == 0 { break; }
        let best_k = best_key.unwrap();
        let (best_p, best_pos, best_set) = pattern_map.remove(&best_k).unwrap();
        selected.push((best_p, best_pos, best_set.clone()));
        for f in best_set.iter() { remaining.remove(f); }
    }
    
    (selected, remaining)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::ppsfp::Fault;

    #[test]
    fn test_greedy_set_cover_basic() {
        // faults: f1,f2,f3
        let f1 = Fault::new("A".to_string(), false);
        let f2 = Fault::new("B".to_string(), false);
        let f3 = Fault::new("C".to_string(), false);

        let mut pmap: HashMap<String, Candidate> = HashMap::new();

        // pattern p1 covers f1 and f2 (set i1=1)
        let mut p1 = InputPattern::from_strings(vec!["i1".to_string(), "i2".to_string()]);
        p1.set("i1", 1);
        let mut s1 = BTreeSet::new(); s1.insert(f1.clone()); s1.insert(f2.clone());
        pmap.insert(format!("{}\n@pos=0", p1.to_string()), (p1.clone(), 0, s1));

        // pattern p2 covers f2 and f3 (set i2=1)
        let mut p2 = InputPattern::from_strings(vec!["i1".to_string(), "i2".to_string()]);
        p2.set("i2", 1);
        let mut s2 = BTreeSet::new(); s2.insert(f2.clone()); s2.insert(f3.clone());
        pmap.insert(format!("{}\n@pos=0", p2.to_string()), (p2.clone(), 0, s2));

        // universe
        let mut uni = BTreeSet::new(); uni.insert(f1.clone()); uni.insert(f2.clone()); uni.insert(f3.clone());

        let (selected, remaining) = greedy_set_cover(pmap, uni);
        // greedy should pick one pattern first (either p1 or p2) then pick the other covering remaining fault
        assert!(selected.len() <= 2);
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_greedy_set_cover_no_coverage() {
        let f1 = Fault::new("A".to_string(), false);
        let mut pmap: HashMap<String, Candidate> = HashMap::new();
        // empty map
        let mut uni = BTreeSet::new(); uni.insert(f1.clone());
        let (selected, remaining) = greedy_set_cover(pmap, uni);
        assert!(selected.is_empty());
        assert!(!remaining.is_empty());
    }

    #[test]
    fn test_perfect_set_cover_basic() {
        let f1 = Fault::new("A".to_string(), false);
        let f2 = Fault::new("B".to_string(), false);
        let f3 = Fault::new("C".to_string(), false);

        let mut pmap: HashMap<String, Candidate> = HashMap::new();

        let mut p1 = InputPattern::from_strings(vec!["i1".to_string(), "i2".to_string()]);
        p1.set("i1", 1);
        let mut s1 = BTreeSet::new(); s1.insert(f1.clone()); s1.insert(f2.clone());
        pmap.insert(format!("{}\n@pos=0", p1.to_string()), (p1.clone(), 0, s1));

        let mut p2 = InputPattern::from_strings(vec!["i1".to_string(), "i2".to_string()]);
        p2.set("i2", 1);
        let mut s2 = BTreeSet::new(); s2.insert(f2.clone()); s2.insert(f3.clone());
        pmap.insert(format!("{}\n@pos=0", p2.to_string()), (p2.clone(), 0, s2));

        let mut uni = BTreeSet::new(); uni.insert(f1.clone()); uni.insert(f2.clone()); uni.insert(f3.clone());

        let (selected, remaining) = perfect_set_cover(pmap, uni);
        assert_eq!(selected.len(), 2);
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_perfect_set_cover_partial_when_unreachable() {
        let f1 = Fault::new("A".to_string(), false);
        let f2 = Fault::new("B".to_string(), false);

        let mut pmap: HashMap<String, Candidate> = HashMap::new();
        let mut p1 = InputPattern::from_strings(vec!["i1".to_string()]);
        p1.set("i1", 1);
        let mut s1 = BTreeSet::new(); s1.insert(f1.clone());
        pmap.insert(format!("{}\n@pos=0", p1.to_string()), (p1.clone(), 0, s1));

        let mut uni = BTreeSet::new(); uni.insert(f1.clone()); uni.insert(f2.clone());

        let (selected, remaining) = perfect_set_cover(pmap, uni);
        assert_eq!(selected.len(), 1);
        assert_eq!(remaining.len(), 1);
        assert!(remaining.contains(&f2));
    }
}