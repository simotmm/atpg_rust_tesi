/// type: gate
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateType {
    AND,
    OR,
    XOR,
    NAND,
    NOR,
    XNOR,
    BUF,
    NOT,
    INVALID,
}

/// implementation: gatetype
impl GateType {

    /// string -> gatetype
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "AND"  => GateType::AND,
            "OR"   => GateType::OR,
            "XOR"  => GateType::XOR,
            "NAND" => GateType::NAND,
            "NOR"  => GateType::NOR,
            "XNOR" => GateType::XNOR,
            "BUF"  => GateType::BUF,
            "BUFF" => GateType::BUF,
            "NOT"  => GateType::NOT,
            _      => GateType::INVALID,
        }
    }

    /// gatetype -> string
    pub fn to_string(&self) -> String {
        match self {
            GateType::AND     => "AND", 
            GateType::OR      => "OR", 
            GateType::XOR     => "XOR", 
            GateType::NAND    => "NAND", 
            GateType::NOR     => "NOR", 
            GateType::XNOR    => "XNOR", 
            GateType::BUF     => "BUF", 
            GateType::NOT     => "NOT", 
            GateType::INVALID => "[invalid_gate]" 
        }.to_string()
    }
}

/// struct: gate
#[derive(Debug, Clone)]
pub struct Gate {
    pub name: String,
    pub gate_type: GateType,
    pub outputs: Vec<String>,
    pub inputs: Vec<String>,
}

///implementation: gate
impl Gate {

    /// gate -> string
    pub fn to_string(&self) -> String {
        format!("{}: {}({:?}) = {:?}", self.name, self.gate_type.to_string(), self.inputs, self.outputs).to_string()
    }

}

/// struct: netlist
#[derive(Debug, Clone)]
pub struct Netlist {
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub gates: Vec<Gate>,
}

/// implementation: netlist
impl Netlist {

    /// new netlist
    pub fn new() -> Netlist {
        Netlist { inputs: Vec::new(), outputs: Vec::new(), gates: Vec::new()}
    }

    /// print netlist
    pub fn print(&self) {
        println!("{}", self.to_string());
    }

    /// netlist -> string 
    pub fn to_string(&self) -> String {
        let mut s: String = "netlist: {\n".to_string();
        let mut i: usize = 0;
        s.push_str(&format!("  inputs  ({}): {:?},\n", &self.inputs.len(), &self.inputs));
        s.push_str(&format!("  outputs ({}): {:?},\n", self.outputs.len(), self.outputs));
        s.push_str(&format!("  gates   ({}): [\n", self.gates.len()));
        for gate in &self.gates {
            s.push_str(&format!("    {}{}\n", gate.to_string(), if i < &self.gates.len()-1 { "," } else { "" }));
            i += 1;
        }
        s.push_str("  ]\n}\n");
        s
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gatetype_from_to_string() {
        assert_eq!(GateType::from_str("and"), GateType::AND);
        assert_eq!(GateType::from_str("buf"), GateType::BUF);
        assert_eq!(GateType::from_str("unknown"), GateType::INVALID);
        assert_eq!(GateType::AND.to_string(), "AND");
    }

    #[test]
    fn test_netlist_to_string_basic() {
        let mut nl = Netlist::new();
        nl.inputs.push("I1".to_string());
        nl.outputs.push("O1".to_string());
        let s = nl.to_string();
        assert!(s.contains("inputs"));
        assert!(s.contains("outputs"));
    }
}
