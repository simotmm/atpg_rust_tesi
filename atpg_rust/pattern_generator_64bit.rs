use rand::{RngCore, SeedableRng};
use rand::rngs::StdRng;
use std::collections::HashMap;
use crate::netlist::Netlist;

const DEFAULT_SEED: u64 = 762000; // default seed
const _64BITS: bool = false;

/// struct: pattern
pub struct InputPattern {
    pub values: HashMap<String, u64> //string: input, u64: pattern rappresentato su 64 bit
}

/// implementation: pattern
impl InputPattern {

    /// new pattern
    pub fn new() -> Self {
        InputPattern { values: HashMap::new() }
    }

    /// netlist, pattern as str -> pattern
    pub fn from_netlist(netlist: Netlist, word: &str) -> Self {
        let mut pattern = InputPattern::new();
        for input in netlist.inputs {
            pattern.values.insert(input, 0);
        }
        pattern.insert_pattern(word);
        pattern
    }

    /// netlist -> random pattern
    pub fn random_from_netlist(netlist: Netlist, seed: Option<u64>) -> Self {
        let mut pattern = InputPattern::new();
        let active_seed = seed.unwrap_or(DEFAULT_SEED);
        let mut rng = StdRng::seed_from_u64(active_seed);
        for input in netlist.inputs {
            let word = rng.next_u64();
            pattern.values.insert(input, word);
        }
        pattern
    }

    /// input -> pattern
    pub fn get(&self, input: &str) -> Option<u64> {
        self.values.get(input).cloned()
    }

    /// set pattern for input
    pub fn set(&mut self, input: &str, val: u64) {
        self.values.insert(input.to_string(), val);
    }

    /// pattern -> string
    pub fn to_string(&self) -> String {
        let mut entries: Vec<_> = self.values.iter().collect();
        entries.sort_by_key(|(k, _)| *k);

        entries
            .into_iter()
            .map(|(k, v)| format!("  {}: {:064b}", k, v)) 
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// print pattern
    pub fn print(&self) {
        println!("patterns generated:\n{}\n", self.to_string())
    }

    /// insert pattern ("101" -> A=1, B=0, C=1)
    pub fn insert_pattern(&mut self, word: &str) {
        // ordino gli input
        let mut keys: Vec<_> = self.values.keys().cloned().collect();
        keys.sort();

        if word.len() != keys.len() {
            panic!(
                "insert_pattern: word length {} does not match number of inputs {}",
                word.len(),
                keys.len()
            );
        }

        // assegno i bit in ordine
        for (i, key) in keys.into_iter().enumerate() {
            match word.as_bytes()[i] {
                b'0' => self.values.insert(key, 0),
                b'1' => self.values.insert(key, 1),
                _ => panic!("insert_pattern: invalid char, expected '0' or '1'"),
            };
        }
    }

}

/// struct: pattern generator (settable seed)
pub struct PatternGenerator {
    seed: Option<u64>
}

/// implementation: pattern generator (settable seed)
impl PatternGenerator {

    /// new pattern generator with seed
    pub fn new(seed: Option<u64>) -> Self {
        PatternGenerator { seed: seed }
    }

    /// set seed
    pub fn set_seed(&mut self, seed: u64) {
        self.seed = Some(seed);
    }

    /// set seed to none
    pub fn remove_seed(&mut self) {
        self.seed = None;
    }

    /// pattern generation
    pub fn generate_random_patterns_from_netlist(&self, netlist: &Netlist) -> InputPattern {
        InputPattern::random_from_netlist(netlist.clone(), self.seed)
    }

    /// generate n patterns from netlist
    pub fn generate_n_patterns_from_netlist(&self, netlist: &Netlist, n: usize) -> Vec<InputPattern> {
        let mut p: Vec<InputPattern> = vec![];
        for _ in 0..n {
            p.push(self.generate_random_patterns_from_netlist(netlist));
        }
        p
    }
    
}

/// utl: bits as str -> u64
pub fn bits_to_u64(s: &str) -> Option<u64> {
    if s.len() > 64 {
        return None;
    }

    let mut value: u64 = 0u64;
    for c in s.bytes() {
        value <<= 1;
        match c {
            b'0' => {}
            b'1' => value |= 1,
            _ => return None,
        }
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::netlist::Netlist;

    #[test]
    fn test_bits_to_u64_basic() {
        assert_eq!(bits_to_u64("101"), Some(5u64));
        assert_eq!(bits_to_u64(""), Some(0u64));
        assert_eq!(bits_to_u64("x"), None);
    }

    #[test]
    fn test_random_from_netlist_64bit() {
        let mut nl = Netlist::new();
        nl.inputs = vec!["A".to_string()];
        let p1 = InputPattern::random_from_netlist(nl.clone(), Some(7));
        let p2 = InputPattern::random_from_netlist(nl.clone(), Some(7));
        assert_eq!(p1.get("A"), p2.get("A"));
    }
}