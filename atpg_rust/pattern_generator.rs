use rand::{RngCore, SeedableRng};
use rand::rngs::StdRng;
use std::collections::HashMap;
use crate::netlist::Netlist;

const DEFAULT_SEED: u64 = 762000; // default seed
const _64BITS: bool = false;

/// struct: pattern
#[derive(Clone, Debug)]
pub struct InputPattern {
    pub values: HashMap<String, u32> //string: input, u32: pattern rappresentato su 32 bit
}

/// implementation: pattern
impl InputPattern {

    /// new pattern
    pub fn new() -> Self {
        InputPattern { values: HashMap::new() }
    }

    pub fn init_from_netlist(netlist: Netlist) -> Self {
        let mut pattern = InputPattern::new();
        for input in netlist.inputs {
            pattern.values.insert(input, 0);
        }
        pattern
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

    /// inizializza un pattern con gli input di un circuito dati come vettore di stringhe
    pub fn from_strings(inputs: Vec<String>) -> Self {
        let mut pattern = InputPattern::new();
        for input in inputs {
            pattern.values.insert(input, 0);
        }
        pattern
    }

    /// netlist -> random pattern
    pub fn random_from_netlist(netlist: Netlist, seed: Option<u64>) -> Self {
        let mut pattern = InputPattern::new();
        let active_seed = seed.unwrap_or(DEFAULT_SEED);
        let mut rng = StdRng::seed_from_u64(active_seed);
        for input in netlist.inputs {
            let word = rng.next_u32();
            pattern.values.insert(input, word);
        }
        pattern
    }

    /// input -> pattern
    pub fn get(&self, input: &str) -> Option<u32> {
        self.values.get(input).cloned()
    }

    /// set pattern for input
    pub fn set(&mut self, input: &str, val: u32) {
        self.values.insert(input.to_string(), val);
    }

    /// pattern -> string
    pub fn to_string(&self) -> String {
        let mut entries: Vec<_> = self.values.iter().collect();
        entries.sort_by_key(|(k, _)| *k);

        entries
            .into_iter()
            .map(|(k, v)| format!("  {}: {:032b}", k, v)) 
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// print pattern
    pub fn print(&self) {
        println!("{}\n", self.to_string())
    }

    /// insert pattern (vec strings) in patter (already initialized with inputs)
    pub fn insert_patterns(&mut self, words: Vec<&str>) {
        // Ordino gli input
        let mut keys: Vec<String> = self.values.keys().cloned().collect();
        keys.sort();

        // Controllo lunghezza parole
        for word in &words {
            //print!("word: {} ({}), keys: {}\n", word, word.len(), keys.len());
            if word.len() != keys.len() {
                panic!("insert_patterns: word length {} does not match number of inputs {}", word.len(), keys.len());
            }
        }

        // Inserisco ogni word partendo dal bit 31 verso 0
        for (i, word) in words.iter().enumerate() {
            let position = 31 - i; // MSB -> LSB

            let mut bits: Vec<u32> = Vec::with_capacity(word.len());
            for c in word.bytes() {
                match c {
                    b'0' => bits.push(0),
                    b'1' => bits.push(1),
                    _ => panic!("insert_patterns: invalid char, expected '0' or '1'"),
                }
            }

            self.insert_pattern_at_position(bits, position, keys.clone());
        }
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

    /// inserisce un pattern a partire da una posizione specificata
    pub fn insert_pattern_at_position(&mut self, word: Vec<u32>, position: usize, order: Vec<String>) {
        // Verifica che la lunghezza di word corrisponda al numero di valori in self.values
        if position > 31 || word.len() != self.values.len() {
            panic!("insert_pattern: word length {} does not match number of inputs {}", word.len(), self.values.len());
        }

        let mut i = 0; // Indicizza l'array word
        // Itera su order, che contiene le chiavi in ordine
        for key in order {
            // Cerca la chiave corrispondente nell'HashMap
            if let Some(value) = self.values.get_mut(&key) {
                let bit = word[i];
                //println!("inserimento del bit {bit} (\"{:?}\"[{i}]) alla posizione {position} del pattern ({key}) {:032b}", word, value);
                // Aggiungi il bit alla posizione desiderata
                set_bit(value, bit, position as u32);
                i += 1;
            } else {
                panic!("chiave non trovata nella hashmap: {key}");
            }
        }
    }

}

/// set bit alla posizione pos in val 
fn set_bit(val: &mut u32, bit: u32, pos: u32) {
    //println!("(val: {:032b}, bit: {bit}, pos: {pos})", *val);
    if pos > 31 { return; }
    // If bit is 0 or 1, set or clear accordingly. If bit is other (e.g., 2) treat it as don't-care and do nothing.
    if bit == 1 {
        *val |= 1 << pos;
    } else if bit == 0 {
        *val &= !(1 << pos);
    } else {
        // don't-care: leave existing bit unchanged
        return;
    }
    //println!("(val: {:032b})", *val);
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

    /// increment seed
    pub fn increment_seed(&mut self) {
        self.set_seed(if self.seed.is_some() { self.seed.unwrap()+1 as u64 } else { 1 });
    }

    /// pattern generation
    pub fn generate_random_pattern_from_netlist(&mut self, netlist: &Netlist) -> InputPattern {
        let p = InputPattern::random_from_netlist(netlist.clone(), self.seed);
        self.increment_seed();
        p
    }

    /// generate n patterns from netlist
    pub fn generate_n_patterns_from_netlist(&mut self, netlist: &Netlist, n: usize) -> Vec<InputPattern> {
        let mut p: Vec<InputPattern> = vec![];
        for _ in 0..n {
            p.push(self.generate_random_pattern_from_netlist(netlist));
        }
        p
    }
    
}

/// util: bits as str -> u32
pub fn bits_to_u32(s: &str) -> Option<u32> {
    if s.len() > 32 {
        return None;
    }

    let mut value = 0u32;
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
    fn test_bits_to_u32() {
        assert_eq!(bits_to_u32("101"), Some(5));
        assert_eq!(bits_to_u32(""), Some(0));
        assert_eq!(bits_to_u32("2"), None);
    }

    #[test]
    fn test_inputpattern_insert_and_get() {
        let mut p = InputPattern::from_strings(vec!["A".to_string(), "B".to_string(), "C".to_string()]);
        p.insert_pattern("101");
        // keys sorted alphabetically: A,B,C -> A=1,B=0,C=1
        assert_eq!(p.get("A"), Some(1));
        assert_eq!(p.get("B"), Some(0));
        assert_eq!(p.get("C"), Some(1));
    }

    #[test]
    fn test_random_from_netlist_deterministic() {
        let mut nl = Netlist::new();
        nl.inputs = vec!["X".to_string(), "Y".to_string()];
        let p1 = InputPattern::random_from_netlist(nl.clone(), Some(42));
        let p2 = InputPattern::random_from_netlist(nl.clone(), Some(42));
        assert_eq!(p1.get("X"), p2.get("X"));
        assert_eq!(p1.get("Y"), p2.get("Y"));
    }
}

