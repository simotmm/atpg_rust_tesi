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

impl GateType {
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "AND" => GateType::AND,
            "OR" => GateType::OR,
            "XOR" => GateType::XOR,
            "NAND" => GateType::NAND,
            "NOR" => GateType::NOR,
            "XNOR" => GateType::XNOR,
            "BUF" => GateType::BUF,
            "BUFF" => GateType::BUF,
            "NOT" => GateType::NOT,
            _ => GateType::INVALID,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            GateType::AND => "AND",
            GateType::OR => "OR",
            GateType::XOR => "XOR",
            GateType::NAND => "NAND",
            GateType::NOR => "NOR",
            GateType::XNOR => "XNOR",
            GateType::BUF => "BUF",
            GateType::NOT => "NOT",
            GateType::INVALID => "[invalid_gate]",
        }
        .to_string()
    }
}

#[derive(Debug, Clone)]
pub struct Gate {
    pub name: String,
    pub gate_type: GateType,
    pub outputs: Vec<String>,
    pub inputs: Vec<String>,
}

impl Gate {
    pub fn to_string(&self) -> String {
        format!(
            "{}: {}({:?}) = {:?}",
            self.name,
            self.gate_type.to_string(),
            self.inputs,
            self.outputs
        )
        .to_string()
    }
}

#[derive(Debug, Clone)]
pub struct Netlist {
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub gates: Vec<Gate>,
}

impl Netlist {
    pub fn new() -> Netlist {
        Netlist {
            inputs: Vec::new(),
            outputs: Vec::new(),
            gates: Vec::new(),
        }
    }

    pub fn print(&self) {
        println!("{}", self.to_string());
    }

    pub fn to_string(&self) -> String {
        let mut s: String = "netlist: {\n".to_string();
        let mut i: usize = 0;
        s.push_str(&format!(
            "  inputs  ({}): {:?},\n",
            &self.inputs.len(),
            &self.inputs
        ));
        s.push_str(&format!(
            "  outputs ({}): {:?},\n",
            self.outputs.len(),
            self.outputs
        ));
        s.push_str(&format!("  gates   ({}): [\n", self.gates.len()));
        for gate in &self.gates {
            s.push_str(&format!(
                "    {}{}\n",
                gate.to_string(),
                if i < &self.gates.len() - 1 { "," } else { "" }
            ));
            i += 1;
        }
        s.push_str("  ]\n}\n");
        s
    }
}
