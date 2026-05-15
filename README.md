# ATPG — General README

This document provides an overview of the `atpg` folder, explaining where the two main engines are located (Rust implementation and Atalanta C/C++), how to build/use them, and the purpose of the included scripts.

## Main structure
- `atpg_rust/` — Rust implementation of the ATPG generator and related tools.
- `atpg_atlanta/` — sources for three Atalanta versions (atalanta, atalanta-m (used by scripts), atalanta-m2). Contains a local `README` and a `makefile`/Visual Studio solution.
- Scripts in the root — various `.bat` and `.ps1` files to run, validate and print summaries.

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
- `run_atpg_rust.bat` — runs the Rust ATPG with default settings.
- `run_atalanta.bat` / `run_atalanta.ps1` — runs Atalanta on predefined input.
- `run_and_validate_all.bat` — runs the full flow (generation with Rust / Atalanta) and then validation.
- `run_rust_and_validate_with_atlanta.bat` — runs `atpg_rust` and validates produced patterns with Atalanta.
- `validate_with_atlanta.bat` / `validate_with_atlanta.ps1` — validates pattern/netlist files using Atalanta.
- `print_atlanta_summary.ps1` — produces a summary of results from Atalanta (human-readable).
- `print_validation_summary.ps1` — prints a summary of validations performed.
- `tests.bat` (in `atpg_rust/`) — runs the test suite for the Rust version.

Quick usage examples (scripts accept a numeric argument selecting a benchmark in `atpg_rust/benchmarks/`):

- Run `atpg_rust` on benchmark number `3`:

```powershell
.\run_atpg_rust.bat 3
```

- Run Atalanta on benchmark `3` (or on the file produced for benchmark 3):

```powershell
.\run_atalanta.bat 3
```

- Run the whole pipeline (Rust → validation with Atalanta) on benchmark `3`:

```powershell
.\run_rust_and_validate_with_atlanta.bat 3
```

- Validate directly from PowerShell specifying index `3`:

```powershell
.\validate_with_atlanta.ps1 3
```

How script numeric arguments work
- Scripts accept an integer argument (e.g. `1`, `2`, `3`) used to select the corresponding benchmark file in `atpg_rust/benchmarks/`.
- Files in `atpg_rust/benchmarks/converted_to_atlanta_iscas89` are ISCAS85 versions converted to Atalanta format; the passed index maps to the related benchmark file (typically following alphabetical order or the indexing implemented by the scripts).
- If no argument is provided, scripts may run a default set or iterate over multiple benchmarks (behavior depends on the specific script).

## Troubleshooting and tips
- If `cargo` is missing, install Rust via the official installer (`rustup`).
- If building Atalanta fails with Visual Studio errors, open the solution in Visual Studio and rebuild to resolve missing dependencies.
- Ensure PowerShell scripts have the correct execution policy or run PowerShell with appropriate privileges if necessary.

## Where to dig deeper
- Examples, test cases and operating details are located in subfolders (`atpg_rust/README.md`, `atpg_atlanta/Atalanta/README`). Consult those files for advanced options and formats.