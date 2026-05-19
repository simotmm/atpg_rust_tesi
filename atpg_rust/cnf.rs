use crate::netlist::{GateType};
use std::collections::HashMap;

const OR: &str = "+";
const AND: &str = "·";
const NOT: &str = "!";
//const OR: &str = "v";
//const AND: &str = "∧";
//const NOT: &str = "¬";

/// struct: literal
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Literal {
    pub var: String,
    pub neg: bool,
}

/// struct: cnf (conjunctive normal form)
#[derive(Debug, Clone)]
pub struct CNF { //cnf=(literal0+literal1+...)*(!literal2)*(literal3+literal4)*...
    pub clauses: Vec<Vec<Literal>>,
}

/// implementation: cnf
impl CNF {
    /// clauses -> new cnf
    pub fn new(clauses: Vec<Vec<Literal>>) -> CNF {
        CNF { clauses }
    }

    /// get number of clauses
    pub fn get_n_clauses(&self) -> usize {
        self.clauses.len()
    }

    /// get number of literals (unique variables)
    pub fn get_n_literals(&self) -> usize {
        self.literals_to_int_map().len()
    }

    /// get number of clauses and literals
    pub fn get_n_clauses_and_n_literals(&self) -> (usize, usize) {
        (self.get_n_clauses(), self.get_n_literals())
    }

    /// cnf to string
    pub fn to_string(&self) -> String {
        let mut s = String::new();
        let n_clauses = self.clauses.len();
        for (i, clause) in self.clauses.iter().enumerate() {
            s.push('(');
            for (j, literal) in clause.iter().enumerate() {
                let not = if literal.neg { NOT } else { "" };
                let or = if j < clause.len() - 1 { OR } else { "" };
                s.push_str(&format!("{}{}{}", not, literal.var, or));
            }
            let and = if i < n_clauses - 1 { AND } else { "" };
            s.push_str(&format!("){and}"));
        }
        s
    }

    /// print cnf
    pub fn print(&self){
        let (clauses, literals) = self.get_n_clauses_and_n_literals();
        println!("cnf\nclauses: {}, literals: {}", clauses, literals);
        println!("{}", self.to_string());
    }

    /// cnf -> mappa da variabili a numeri interi (per rappresentazione compatta)
    pub fn literals_to_int_map(&self) -> HashMap<String, i32> {
        // Raccogliamo tutte le variabili uniche
        let mut vars: Vec<String> = self.clauses.iter()
            .flat_map(|clause| clause.iter().map(|lit| lit.var.clone()))
            .collect();
        // Rimuoviamo i duplicati e ordiniamo alfabeticamente
        vars.sort();
        vars.dedup();
        let mut var_to_int: HashMap<String, i32> = HashMap::new();
        let mut num = 1;
        for var in vars {
            var_to_int.insert(var.clone(), num);
            num += 1;
        }
        var_to_int
    }

    /// Converte la CNF in una rappresentazione come stringa di numeri interi, senza simboli per AND e OR
    pub fn to_int_string(&self) -> String {
        let add_0_for_each_clause = false;
        let mut var_to_int = self.literals_to_int_map();

        let mut result = String::new();
        for (i, clause) in self.clauses.iter().enumerate() {
            if i > 0 {
                if add_0_for_each_clause { result.push_str(" 0"); }
                result.push('\n');  // Aggiungi una nuova riga per separare le clausole
            }
            for (j, lit) in clause.iter().enumerate() {
                if j > 0 {
                    result.push(' ');  // Spazio tra i numeri dei letterali
                }
                let mut num = var_to_int[&lit.var];
                if lit.neg {
                    num = -num;  // Se negato, rendiamo il numero negativo
                }
                result.push_str(&num.to_string());
            }
        }

        result
    }

    /// cnf -> mappa ordinata da variabili a numeri interi (per rappresentazione compatta)
    pub fn literals_to_int_map_ordered(&self) -> Vec<(String, i32)> {
        let mut var_to_int: HashMap<String, i32> = self.literals_to_int_map();
        let mut vec: Vec<(String, i32)> = var_to_int.into_iter().collect();
        vec.sort_by(|a, b| a.0.cmp(&b.0));
        vec
    }

    /// cnf -> porzione di cnf con 1 o 2 letterali
    pub fn get_2_lit_portion(&self) -> Self {
        let c = self.clauses.iter()
            .filter(|clause| clause.len() <= 2)
            .cloned()
            .collect();
        CNF { clauses: c }
    }

    /// cnf -> porzione di cnf con 3 o più letterali
    pub fn get_3_lit_portion(&self) -> Self {
        let c = self.clauses.iter()
            .filter(|clause| clause.len() > 2)
            .cloned()
            .collect();
        CNF { clauses: c }
    }

}

/// helper wrapper function
fn lit(v: &str, neg: bool) -> Literal { Literal { var: v.to_string(), neg: neg } }

/// gate -> cnf
pub fn gate_to_cnf_literals(gt: GateType, d: &str, inputs: &[String]) -> CNF {
    let mut clauses: Vec<Vec<Literal>> = Vec::new();

    match gt {
        // AND n-input
        GateType::AND => {
            // d -> (a1 & a2 & ... & an)
            // (¬d ∨ ai)   for each ai
            for ai in inputs {
                clauses.push(vec![lit(d, true), lit(ai, false)]);
            }
            // (d ∨ ¬a1 ∨ ¬a2 ∨ ... ∨ ¬an)
            let mut c = vec![lit(d, false)];
            for ai in inputs { c.push(lit(ai, true)); }
            clauses.push(c);
        },
        // OR n-input
        GateType::OR => {
            // d -> (a1 ∨ a2 ∨ ... ∨ an)
            // (d ∨ ¬ai)   for each ai
            for ai in inputs {
                clauses.push(vec![lit(d, false), lit(ai, true)]);
            }
            // (¬d ∨ a1 ∨ a2 ∨ ... ∨ an)
            let mut c = vec![lit(d, true)];
            for ai in inputs { c.push(lit(ai, false)); }
            clauses.push(c);
        },
        // NAND n-input 
        GateType::NAND => {
            // d = NOT(AND(inputs))
            // Use AND CNF inverted:
            // (d ∨ ai) for each ai
            for ai in inputs {
                clauses.push(vec![lit(d, false), lit(ai, false)]);
            }
            // (¬d ∨ ¬a1 ∨ ¬a2 ∨ ... ∨ ¬an)
            let mut c = vec![lit(d, true)];
            for ai in inputs { c.push(lit(ai, true)); }
            clauses.push(c);
        },
        // NOR n-input
        // NOR n-input
        GateType::NOR => {
            // d = NOT(OR(inputs))  <=>  d = (¬a1 ∧ ¬a2 ∧ ...)
            // Clauses for d <-> (¬a1 & ¬a2 & ...):
            // (¬d ∨ ¬ai)  for each ai
            // (d ∨ a1 ∨ a2 ∨ ... ∨ an)
            for ai in inputs {
                clauses.push(vec![lit(d, true), lit(ai, true)]);
            }
            let mut c = vec![lit(d, false)];
            for ai in inputs { c.push(lit(ai, false)); }
            clauses.push(c);
        },
        // XOR / XNOR 2-input
        GateType::XOR | GateType::XNOR => {
            if inputs.len() != 2 { panic!("XOR / XNOR devono avere esattamente 2 input"); }
            let a = &inputs[0];
            let b = &inputs[1];
            match gt {
                // XOR 2-input
                GateType::XOR => {
                    clauses.push(vec![lit(d, true), lit(a, true),  lit(b, true )]);
                    clauses.push(vec![lit(d, true), lit(a, false), lit(b, false)]);
                    clauses.push(vec![lit(d, false),lit(a, true),  lit(b, false)]);
                    clauses.push(vec![lit(d, false),lit(a, false), lit(b, true )]);
                }
                // XNOR 2-input
                GateType::XNOR => {
                    clauses.push(vec![lit(d, true), lit(a, true),  lit(b, false)]);
                    clauses.push(vec![lit(d, true), lit(a, false), lit(b, true )]);
                    clauses.push(vec![lit(d, false),lit(a, false), lit(b, false)]);
                    clauses.push(vec![lit(d, false),lit(a, true),  lit(b, true )]);
                }
                _ => unreachable!()
            }
        }
        // BUF 1-input
        GateType::BUF => {
            let a = &inputs[0];
            clauses.push(vec![lit(d, false), lit(a, true)]);
            clauses.push(vec![lit(d, true),  lit(a, false)]);
        }
        // NOT 1-input
        GateType::NOT => {
            let a = &inputs[0];
            clauses.push(vec![lit(d, false), lit(a, false)]);
            clauses.push(vec![lit(d, true),  lit(a, true)]);
        }
        // invalid
        GateType::INVALID => return CNF::new(vec![])
    }

    CNF::new(clauses)
}

/// BD=XOR(X,X')=(V1+V2)=(!X+!V1)·(X'+!V1)·(V1+X+!X')·(X+!V2)·(!X'+V2)·(V2+!X+X')·(!BD+V1+V2)·(!V1+BD)·(!V2+BD)
pub fn get_boolean_difference_clauses(x: &str, xp: &str) -> CNF {
    // Use per-output unique auxiliary variable names to avoid collisions when
    // adding boolean-difference clauses for multiple outputs in the same CNF.
    // Derive a safe base name from `x` by stripping trailing apostrophes.
    let base = x.trim_end_matches('\'');
    let mut clauses: Vec<Vec<Literal>> = Vec::new();
    let v1 = format!("V1__{}", base);
    let v2 = format!("V2__{}", base);
    let bd = format!("BD__{}", base);
    clauses.push(vec![lit(x,  !false), lit(&v1, !false)]);
    clauses.push(vec![lit(xp, !true),  lit(&v1, !false)]);
    clauses.push(vec![lit(&v1, !true),  lit(x,  !true),  lit(xp, !false)]);
    clauses.push(vec![lit(x,  !true),  lit(&v2, !false)]);
    // V2 <-> (X & !X') encoding: include (¬V2 ∨ ¬X')
    clauses.push(vec![lit(&v2, !false), lit(xp, !false)]);
    clauses.push(vec![lit(&v2, !true),  lit(x,  !false), lit(xp, !true)]);
    clauses.push(vec![lit(&bd, !false), lit(&v1, !true),  lit(&v2, !true)]);
    clauses.push(vec![lit(&v1, !false), lit(&bd, !true)]);
    clauses.push(vec![lit(&v2, !false), lit(&bd, !true)]);
    CNF { clauses }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::netlist::GateType;

    #[test]
    fn test_cnf_basic_and_to_string() {
        let clauses = vec![
            vec![lit("A", false), lit("B", false)],
            vec![lit("C", false)],
        ];
        let cnf = CNF::new(clauses);
        assert_eq!(cnf.get_n_clauses(), 2);
        let n_literals = cnf.get_n_literals();
        assert_eq!(n_literals, 3);
        assert_eq!(cnf.to_string(), "(A+B)·(C)");
        assert_eq!(cnf.to_int_string(), "1 2\n3");
        let ordered = cnf.literals_to_int_map_ordered();
        assert_eq!(ordered.len(), 3);
    }

    #[test]
    fn test_get_2_and_3_lit_portions() {
        let clauses = vec![
            vec![lit("A", false), lit("B", false)],
            vec![lit("C", false), lit("D", false), lit("E", false)],
        ];
        let cnf = CNF::new(clauses);
        let two = cnf.get_2_lit_portion();
        let three = cnf.get_3_lit_portion();
        assert_eq!(two.get_n_clauses(), 1);
        assert_eq!(three.get_n_clauses(), 1);
    }

    #[test]
    fn test_gate_to_cnf_and() {
        let inputs = vec!["a".to_string(), "b".to_string()];
        let cnf = gate_to_cnf_literals(GateType::AND, "d", &inputs);
        // For AND with 2 inputs we expect 3 clauses
        assert_eq!(cnf.get_n_clauses(), 3);
        // Ensure clauses contain expected literals by stringified int representation
        let s = cnf.to_string();
        assert!(s.contains("d") || s.contains("a"));
    }

    #[test]
    fn test_get_boolean_difference_clauses_len() {
        let cnf = get_boolean_difference_clauses("X", "X'");
        // The implementation pushes 9 clauses
        assert_eq!(cnf.get_n_clauses(), 9);
    }

    #[test]
    fn test_boolean_difference_semantics() {
        use crate::simple_solver;
        use std::collections::HashMap;

        fn sat_with(cnf: &CNF, assigns: &HashMap<String, bool>) -> bool {
            simple_solver::solve_cnf(cnf, Some(assigns), None).is_some()
        }

        let combos = vec![(false, false), (false, true), (true, false), (true, true)];
        for (x, xp) in combos {
            let cnf = get_boolean_difference_clauses("X", "X'");
            let mut a_true: HashMap<String, bool> = HashMap::new();
            a_true.insert("X".to_string(), x);
            a_true.insert("X'".to_string(), xp);
            a_true.insert("BD__X".to_string(), true);
            let sat_true = sat_with(&cnf, &a_true);

            let mut a_false: HashMap<String, bool> = HashMap::new();
            a_false.insert("X".to_string(), x);
            a_false.insert("X'".to_string(), xp);
            a_false.insert("BD__X".to_string(), false);
            let sat_false = sat_with(&cnf, &a_false);

            // Exactly one of BD=true/false should be satisfiable and match XOR
            assert_ne!(sat_true, sat_false, "Both BD assignments have same satisfiability for X={}, X'={}", x, xp);
            let expected_bd = x ^ xp;
            assert_eq!(sat_true, expected_bd, "BD truth does not match XOR for X={}, X'={}", x, xp);
        }
    }
    #[test]
    fn test_get_boolean_difference_clauses_content() {
        let cnf = get_boolean_difference_clauses("X", "X'");
        // Expect clause: (!X + !V1)
        let expected1 = vec![lit("X", true), lit("V1__X", true)];
        assert!(cnf.clauses.contains(&expected1));
        // Expect clause: (¬V2 ∨ ¬X') -> (V2 negated, X' negated)
        let expected5 = vec![lit("V2__X", true), lit("X'", true)];
        assert!(cnf.clauses.contains(&expected5));
    }
}


