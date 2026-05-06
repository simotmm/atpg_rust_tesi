use crate::parser::file_iscas89_atlanta_to_netlist;
use crate::dag::Dag;
use crate::ppsfp::Fault;
use std::path::Path;

pub fn dump_faults(arg2: Option<String>) {
    let filename: String = if let Some(s) = arg2 {
        if let Ok(idx) = s.parse::<i32>() {
            crate::util::get_filename_from_int("benchmarks\\converted_to_atlanta_iscas89", idx).unwrap_or(crate::get_default_name())
        } else { s }
    } else { crate::get_default_name() };
    let circuit = file_iscas89_atlanta_to_netlist(&filename);
    let dag = Dag::from_netlist(&circuit);
    let faults: Vec<Fault> = dag.generate_fault_list(None, None);
    for f in faults { println!("{}", f.to_string()); }
}

pub fn extract_cnf(net_id: i32, fault_name: String, minimizer_arg: String) {
    let filename: String = crate::util::get_filename_from_int("benchmarks\\converted_to_atlanta_iscas89", net_id).unwrap_or(crate::get_default_name());
    println!("Loading netlist: {filename}");
    let circuit = file_iscas89_atlanta_to_netlist(&filename);
    let dag = Dag::from_netlist(&circuit);
    let faults: Vec<Fault> = dag.generate_fault_list(Some(fault_name.clone()), None);
    if faults.is_empty() { println!("Fault {fault_name} not found in netlist {net_id}"); return; }
    let fault = faults[0].clone();

    let mut faulty = dag.primed_clone();
    let start_idx_opt = dag.nodes.iter().position(|n| n.outputs[0] == fault.wire);
    let renamed_wire = format!("{}'", fault.wire);
    if let Some(start_idx) = start_idx_opt {
        let stuck_literal = crate::cnf::Literal { var: renamed_wire.clone(), neg: !fault.sa1 };
        faulty.nodes[start_idx].pre_fault_cnf = Some(faulty.nodes[start_idx].cnf.to_string());
        faulty.nodes[start_idx].cnf = crate::cnf::CNF { clauses: vec![vec![stuck_literal]] };
    } else {
        println!("Warning: fault injection point not found as node output; attempting fallback injection");
        let new_id = faulty.nodes.len();
        let dummy_node = crate::dag::DagNode { id: new_id, name: format!("PI_FAULT_{}", fault.wire), gate_type: crate::netlist::GateType::INVALID, outputs: vec![renamed_wire.clone()], inputs: vec![], cnf: crate::cnf::CNF { clauses: vec![vec![crate::cnf::Literal { var: renamed_wire.clone(), neg: !fault.sa1 }]] }, pre_fault_cnf: None, is_final: false };
        faulty.nodes.push(dummy_node);
        faulty.affected.push(new_id);
    }

    let test_formula = crate::dag::build_test_formula_optimized(&dag, &faulty);
    let out_dir = Path::new("out");
    let _ = std::fs::create_dir_all(&out_dir);
    let base_name = format!("test_formula_net{}_{}", net_id, fault.wire);
    let orig_path = out_dir.join(format!("{}_orig.cnf.txt", base_name));
    let dimacs_path = out_dir.join(format!("{}_orig.dimacs.txt", base_name));
    println!("Writing original CNF to {}", orig_path.display());
    let _ = std::fs::write(&orig_path, test_formula.to_string());
    let _ = std::fs::write(&dimacs_path, test_formula.to_int_string());

    let mut optimized = test_formula.clone();
    match crate::cnf_minimizer::minimize_search_tree(&mut optimized) {
        Ok(_) => {
            let opt_path = out_dir.join(format!("{}_builtin_min.cnf.txt", base_name));
            let _ = std::fs::write(&opt_path, optimized.to_string());
            println!("Wrote builtin-minimized CNF to {}", opt_path.display());
        }
        Err(_) => { println!("Builtin minimizer reported conflict/unsat or error"); }
    }

    // Note: espresso is intentionally not invoked here; it is used only in Part III SAT flow.
    println!("Done.");
}
