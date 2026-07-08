use crate::ppsfp::{PPSFPSimulator, Fault};
use crate::pattern_generator::InputPattern;
use crate::netlist::Netlist;
use crate::dag::Dag;
use std::collections::{BTreeSet, HashMap};

/// SAT wrapper che converte le soluzioni SAT in `InputPattern` e le simula
/// tra una soluzione e l'altra per rimuovere i fault già coperti.
/// Ritorna `(undetected_faults, patterns_used)`.
pub fn sat_solving_opt(circuit: &Netlist, dag: &Dag, faults: Vec<&Fault>, parallel_execution: Option<bool>) -> (BTreeSet<Fault>, Vec<InputPattern>) {
    // Chiamata al solver esistente per ottenere mappa fault -> assignment (vec of (name,bit))
    let sat_map: HashMap<Fault, Vec<(String, u32)>> = crate::sat::sat_solving(circuit, dag, faults.clone(), parallel_execution);

    // Ordiniamo le entry in modo deterministico per stabilità (stringa rappresentativa del fault)
    let mut entries: Vec<_> = sat_map.into_iter().collect();
    entries.sort_by(|a, b| a.0.to_string().cmp(&b.0.to_string()));

    // Converti ogni assegnazione in un `InputPattern`
    let mut patterns: Vec<InputPattern> = Vec::new();
    for (_f, assign) in entries.iter() {
        let mut pat = InputPattern::from_strings(circuit.clone().inputs.clone());
        for (name, val) in assign.iter() {
            let v: u32 = match *val { 1 => !0u32, 0 => 0u32, _ => 0u32 };
            pat.set(name, v);
        }
        patterns.push(pat);
    }

    // Simulazione adattiva: applica i pattern in ordine e rimuovi i fault rilevati
    let mut undetected: BTreeSet<Fault> = faults.iter().map(|f| (*f).clone()).collect();
    let mut simulator = PPSFPSimulator::new(dag);

    for pat in patterns.iter() {
        if undetected.is_empty() { break; }
        // usa un simulatore temporaneo per valutare questo pattern sui fault residui
        let mut sim_tmp = PPSFPSimulator::new(dag);
        sim_tmp.simulate_good(pat);

        let mut detected: Vec<Fault> = Vec::new();
        for f in undetected.iter() {
            let det = sim_tmp.simulate_fault(f);
            if det != 0 { detected.push(f.clone()); }
        }

        for d in detected { undetected.remove(&d); }
    }

    (undetected, patterns)
}
