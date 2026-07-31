use std::env;
use std::fs::{create_dir_all, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub fn save_bits_result(name: &str, bits1: &[u32], bits2: &[u32]) -> io::Result<PathBuf> {
    let dir = PathBuf::from("results");
    create_dir_all(&dir)?;

    // Use only the file stem (basename without extension) so paths like
    // "nets/c1.v" don't create nested directories under `results/`.
    let base = Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(name);

    let filename = format!("{}_max_commutations_inputs.txt", base);
    let path = dir.join(filename);

    let mut file = File::create(&path)?;
    writeln!(file, "{}", bits_to_string(bits1))?;
    writeln!(file, "{}", bits_to_string(bits2))?;
    Ok(path)
}

fn bits_to_string(bits: &[u32]) -> String {
    bits.iter()
        .map(|b| if *b == 0 { '0' } else { '1' })
        .collect()
}

/// get nth command line arg as int
pub fn get_nth_arg_to_int(n: i32) -> Option<i32> {
    env::args()
        .nth(n as usize)
        .and_then(|s| s.parse::<i32>().ok())
}

/// get nth command line arg as string
pub fn get_nth_arg_to_string(n: i32) -> Option<String> {
    env::args()
        .nth(n as usize)
        .and_then(|s| s.parse::<String>().ok())
}
