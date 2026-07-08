# ATPG — General README

This document provides an overview of the `atpg` folder, explaining where the two main engines are located (Rust implementation and Atalanta C/C++), how to build/use them, and the purpose of the included scripts.

## Main structure
- `atpg_rust/` — Rust implementation of the ATPG generator and related tools.
- `atpg_atlanta/` — sources and Windows builds for the Atalanta family. The scripts in this repo currently use the `atalanta-M` executable.
- `scripts/` — batch files to run Rust ATPG, generate fault lists, and compare/validate results.

## Requirements
- Rust toolchain (`cargo`, `rustc`) for `atpg_rust`.
- C/C++ build tools for Atalanta: Visual Studio (solution `.sln`) or a GNU toolchain (`make` if available).
- Windows PowerShell / CMD to run the provided scripts.

## atpg_rust (folder `atpg_rust`)
1. Build:

```powershell
cd atpg_rust
cargo build --release
```

2. Run (example: view help):

```powershell
cd atpg_rust
cargo run --release -- -h
```

3. Notes:
- `target/release/` will contain the compiled executable.
- Main source files include `main.rs`, `parser.rs`, `netlist.rs`, etc. See code comments for detailed options.

## Atalanta (folder `atpg_atlanta/Atalanta`)
1. Build with Visual Studio:

- Open [atpg_atlanta/Atalanta/Atalanta.sln](atpg_atlanta/Atalanta/Atalanta.sln) and build the solution (Debug or Release).

2. Build with make (if supported):

```powershell
cd atpg_atlanta/Atalanta
make
```

3. Run:
- The produced executable (e.g. `Atalanta.exe`) is used for simulation/validation of patterns. Consult the local `README` in that folder or internal documentation for executable parameters.

## Root scripts — quick examples
The active scripts are the `.bat` files in `scripts/` and `atpg_rust/`:
- `scripts/run_atpg_rust.bat` — runs the Rust ATPG for a benchmark id.
- `scripts/run_atalanta_with_rust_inputs.bat` — runs Atalanta-M on the `.bench` and `.pat` produced by Rust, then writes a comparison summary.
- `scripts/run_atalanta_and_generate_und_faults_list.bat` — generates the Atalanta fault list and the undetected-fault list for a benchmark.
- `scripts/run_rust_generate_all_faults_list.bat` — generates fault lists for the full benchmark set using Rust.
- `scripts/atalanta_und_faults_list_for_all.bat` — runs the undetected-fault-list flow for the preset benchmark set.
- `atpg_rust/tests.bat` — runs the Rust test suite.

Quick usage examples (the benchmark-specific scripts accept a numeric argument that selects a benchmark in `atpg_rust/benchmarks/`):

- Run the Rust ATPG wrapper on benchmark number `3`:

```powershell
.\scripts\run_atpg_rust.bat 3
```

- Run Atalanta-M on benchmark `3` (or on the file produced for benchmark 3):

```powershell
.\scripts\run_atalanta_with_rust_inputs.bat 3
```

- Run the whole pipeline (Rust → validation with Atalanta-M) on benchmark `3`:

```powershell
.\scripts\run_atalanta_and_generate_und_faults_list.bat 3
```

- Generate the fault lists for all preset benchmarks:

```powershell
.\scripts\run_rust_generate_all_faults_list.bat
```

How script numeric arguments work
- Scripts that take a benchmark id accept an integer argument (for example `2`, `3`, `432`) and map it to the corresponding benchmark file in `atpg_rust/benchmarks/`.
- The batch files that iterate over the full benchmark set use a preset list of ids and do not require an argument.
- The scripts under `scripts/` are Windows batch files; there are no active `.ps1` wrappers in this repository snapshot.

## Troubleshooting and tips
- If `cargo` is missing, install Rust via the official installer (`rustup`).
- If building Atalanta fails with Visual Studio errors, open the solution in Visual Studio and rebuild to resolve missing dependencies.
- The batch scripts use relative paths, so run them from the repository root.

## Where to dig deeper
- Examples, test cases and operating details are located in subfolders ([atpg_rust/README.md](atpg_rust/README.md), [atpg_atlanta/Atalanta/README](atpg_atlanta/Atalanta/README)). Consult those files for advanced options and formats.