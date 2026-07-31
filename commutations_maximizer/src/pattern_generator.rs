use crate::netlist::Netlist;
use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};
use std::collections::HashMap;

const DEFAULT_SEED: u64 = 762000; // default seed

#[derive(Clone, Debug)]
pub struct InputPattern {
    pub values: HashMap<String, u32>,
}

impl InputPattern {
    pub fn new() -> Self {
        InputPattern {
            values: HashMap::new(),
        }
    }

    pub fn init_from_netlist(netlist: Netlist) -> Self {
        let mut pattern = InputPattern::new();
        for input in netlist.inputs {
            pattern.values.insert(input, 0);
        }
        pattern
    }

    pub fn from_netlist(netlist: Netlist, word: &str) -> Self {
        let mut pattern = InputPattern::new();
        for input in netlist.inputs {
            pattern.values.insert(input, 0);
        }
        pattern.insert_pattern(word);
        pattern
    }

    pub fn from_strings(inputs: Vec<String>) -> Self {
        let mut pattern = InputPattern::new();
        for input in inputs {
            pattern.values.insert(input, 0);
        }
        pattern
    }

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

    pub fn get(&self, input: &str) -> Option<u32> {
        self.values.get(input).cloned()
    }

    pub fn set(&mut self, input: &str, val: u32) {
        self.values.insert(input.to_string(), val);
    }

    pub fn get_input_pattern_from_position(&self, position: usize) -> Vec<u32> {
        if position > 31 {
            panic!(
                "get_input_pattern_from_position: position {} out of range",
                position
            );
        }
        let mut keys: Vec<String> = self.values.keys().cloned().collect();
        keys.sort();
        let mut res: Vec<u32> = Vec::with_capacity(keys.len());
        for k in keys {
            let v = *self.values.get(&k).unwrap_or(&0u32);
            let bit = ((v >> position) & 1) as u32;
            res.push(bit);
        }
        res
    }

    pub fn to_string(&self) -> String {
        let mut entries: Vec<_> = self.values.iter().collect();
        entries.sort_by_key(|(k, _)| *k);
        entries
            .into_iter()
            .map(|(k, v)| format!("  {}: {:032b}", k, v))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn insert_patterns(&mut self, words: Vec<&str>) {
        let mut keys: Vec<String> = self.values.keys().cloned().collect();
        keys.sort();
        for word in &words {
            if word.len() != keys.len() {
                panic!(
                    "insert_patterns: word length {} does not match number of inputs {}",
                    word.len(),
                    keys.len()
                );
            }
        }
        for (i, word) in words.iter().enumerate() {
            let position = 31 - i;
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

    pub fn insert_pattern(&mut self, word: &str) {
        let mut keys: Vec<_> = self.values.keys().cloned().collect();
        keys.sort();
        if word.len() != keys.len() {
            panic!(
                "insert_pattern: word length {} does not match number of inputs {}",
                word.len(),
                keys.len()
            );
        }
        for (i, key) in keys.into_iter().enumerate() {
            match word.as_bytes()[i] {
                b'0' => self.values.insert(key, 0),
                b'1' => self.values.insert(key, 1),
                _ => panic!("insert_pattern: invalid char, expected '0' or '1'"),
            };
        }
    }

    pub fn insert_pattern_at_position(
        &mut self,
        word: Vec<u32>,
        position: usize,
        order: Vec<String>,
    ) {
        if position > 31 || word.len() != self.values.len() {
            panic!(
                "insert_pattern: word length {} does not match number of inputs {}",
                word.len(),
                self.values.len()
            );
        }

        let mut i = 0;
        for key in order {
            if let Some(value) = self.values.get_mut(&key) {
                let bit = word[i];
                set_bit(value, bit, position as u32);
                i += 1;
            } else {
                panic!("chiave non trovata nella hashmap: {key}");
            }
        }
    }
}

fn set_bit(val: &mut u32, bit: u32, pos: u32) {
    if pos > 31 {
        return;
    }
    if bit == 1 {
        *val |= 1 << pos;
    } else if bit == 0 {
        *val &= !(1 << pos);
    } else {
        return;
    }
}

pub struct PatternGenerator {
    seed: Option<u64>,
}
impl PatternGenerator {
    pub fn new(seed: Option<u64>) -> Self {
        PatternGenerator { seed: seed }
    }

    pub fn set_seed(&mut self, seed: u64) {
        self.seed = Some(seed);
    }

    pub fn remove_seed(&mut self) {
        self.seed = None;
    }

    pub fn increment_seed(&mut self) {
        self.set_seed(if self.seed.is_some() {
            self.seed.unwrap() + 1 as u64
        } else {
            1
        });
    }

    pub fn generate_random_pattern_from_netlist(&mut self, netlist: &Netlist) -> InputPattern {
        let p = InputPattern::random_from_netlist(netlist.clone(), self.seed);
        self.increment_seed();
        p
    }

    pub fn generate_n_patterns_from_netlist(
        &mut self,
        netlist: &Netlist,
        n: usize,
    ) -> Vec<InputPattern> {
        let mut p: Vec<InputPattern> = vec![];
        for _ in 0..n {
            p.push(self.generate_random_pattern_from_netlist(netlist));
        }
        p
    }

    /// Generates all possible input combinations for the given netlist. If `compact` is true, it packs 32 combinations into each InputPattern using bitwise operations.
    pub fn generate_all_input_combinations(
        &self,
        netlist: &Netlist,
        compact: Option<bool>,
    ) -> Vec<InputPattern> {
        let mut res: Vec<InputPattern> = Vec::new();
        let mut keys = netlist.inputs.clone();
        keys.sort();
        let n = keys.len();
        if n == 0 {
            res.push(InputPattern::from_strings(vec![]));
            return res;
        }
        let total = 1usize << n;
        if compact.unwrap_or(false) {
            if n > 32 {
                panic!(
                    "generate_all_input_combinations: number of inputs {} exceeds 32",
                    n
                );
            }
            let blocks = (total + 31) / 32;
            for _ in 0..blocks {
                res.push(InputPattern::from_strings(keys.clone()));
            }
            for comb in 0..total {
                let block = comb / 32;
                let pos = (comb % 32) as u32;
                for (i, key) in keys.iter().enumerate() {
                    let bit = ((comb >> (n - 1 - i)) & 1) as u32;
                    if let Some(val) = res[block].values.get_mut(key) {
                        set_bit(val, bit, pos);
                    } else {
                        panic!("generate_all_input_combinations: key not found: {}", key);
                    }
                }
            }
            return res;
        }
        for comb in 0..total {
            let mut p = InputPattern::from_strings(keys.clone());
            for (i, key) in keys.iter().enumerate() {
                let bit = ((comb >> (n - 1 - i)) & 1) as u32;
                p.set(key, bit);
            }
            res.push(p);
        }
        res
    }
}

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
