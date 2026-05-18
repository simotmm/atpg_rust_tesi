# atpg_rust

This document describes the `atpg_rust` project: execution flow, general logic, module architecture and a detailed explanation of the main functions.

Purpose: generate test vectors that detect stuck‑at faults in ISCAS/Atalanta netlists. The program combines pseudo‑random generation (PPSFP) and SAT-based algorithms to find patterns that cover undetected faults.

Contents
- Overview: concept and purpose
- Execution flow: main steps executed by `main`
- Main modules: file map and responsibilities
- Key data structures: main types and meaning
- Main functions: detailed descriptions and behavior
- Execution examples and runtime options

Note: navigation references to source files are included as links.

Quick overview
The tool reads a netlist (ISCAS89/Atalanta), builds a `Dag` with per-node CNFs, generates random patterns, simulates the good and faulty circuits in parallel (PPSFP) to mark detected faults; for undetected faults it builds SAT formulas (XOR between good and faulty circuit) and attempts to solve them to obtain vectors that cause output differences.

Execution
1. Input and parsing
- The input file is read and parsed into a `Netlist` (`parser.rs`).
- Main function: `file_iscas89_atlanta_to_netlist` (`parser.rs`).

2. DAG and CNF construction
- `Dag::from_netlist` converts the `Netlist` into a DAG where each node contains a CNF for its gate (`dag.rs`).
- `rev_adj` and `succ_adj` adjacency lists are precomputed for efficient traversals and fault injection.

3. Pattern generation
- `PatternGenerator` produces `InputPattern` bit-packed random patterns from a seed (`pattern_generator.rs`).

4. PPSFP simulation
- `PPSFPSimulator::simulate_patterns_blocks` simulates blocks of packed patterns and checks which faults are detected (`ppsfp.rs`).

5. SAT-based pattern generation
- For undetected faults the solver constructs a test formula: CNF(good cone) + CNF(faulty cone) + BD clauses.
- CNF minimization is optional; the solver backend can be internal (2-SAT + extension) or `varisat`.

6. Post-processing
- SAT solutions are converted to `InputPattern` and re-simulated for confirmation.
- Final patterns are saved using utilities in `util.rs`.

Files and responsibilities
- `main.rs`: orchestration and argument parsing.
- `parser.rs`: parse ISCAS/Verilog -> `Netlist`.
- `netlist.rs`: gate and netlist data types.
- `dag.rs`: DAG construction, boolean evaluator, CNF conversion, fault enumeration.
- `pattern_generator.rs`: `InputPattern` and `PatternGenerator`.
- `ppsfp.rs`: PPSFP simulator.
- `sat.rs`: SAT orchestration and wrapper for `varisat`/builtin solver.
- `simple_solver.rs`: internal DPLL/unit-propagation solver.
- `cnf.rs`: CNF utilities and gate→CNF conversion.
- `cnf_minimizer*.rs`: CNF minimization modules.
- `options.rs`: runtime options.
- `util.rs`: helper functions for printing, parsing args, saving outputs.

Data types
- `GateType`: enum representing gates (AND, OR, XOR, NAND, NOR, XNOR, BUF, NOT).
- `Gate`: `name`, `gate_type`, `outputs`, `inputs`.
- `Netlist`: `inputs`, `outputs`, `gates`.
- `Dag`/`DagNode`: node-level CNF and adjacency lists.
- `Fault`: `wire`, `sa1`, `site`.
- `InputPattern`: `HashMap<String,u32>` where each `u32` packs 32 patterns.

Key behaviors
- `Dag::generate_fault_list` enumerates faults using Atalanta-like rules (producer-level vs site-specific), considering fanout and final outputs.
- `PPSFPSimulator` performs bit-parallel simulation and uses `simulate_fault` to force a wire to `0` or `!0u32` to check detection across 32 packed vectors.
- `sat::sat_solving` builds test formulas and calls the chosen SAT backend; it can minimize the CNF and prefer prioritized variables from the influence cone.

SAT strategies
- The internal approach extracts the 2-SAT portion (solvable in linear time) and enumerates assignments up to a limit, then extends them to the remaining 3-SAT clauses via DPLL/unit-propagation.
- `varisat` can be used as an external backend by converting CNF to DIMACS and invoking the solver for better performance.

Minimization
- Built-in minimizer tries to remove literals/clauses while preserving satisfiability of the test goal.
- Optional Espresso integration exports the cover to Espresso and maps the result back to CNF; it is disabled by default due to external I/O overhead.

Verification
- SAT models are converted into `InputPattern` and re-simulated; boolean single-bit evaluation is used for diagnostics when a full boolean assignment is available.

Saving outputs
- `save_patterns_to_test_inputs` and `save_patterns_to_file` (in `util.rs`) write final patterns in formats compatible with Atalanta, and optionally save undetected fault lists.

Examples
- Build and run sample benchmark:

```powershell
cd atpg_rust
cargo run -- 432
```

Options include `mode` (`rand` / `alg` / `both`), `sat_backend` (`varisat` or `builtin`), and `minimizer` (`espresso` or `builtin`).



## Benchmark results

| File | Total (s) | Parse (s) | DAG (s) | Rand (s) | Rand detected | SAT (s) | SAT detected | Total detected | Faults | Coverage (%) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| c17_to_iscas89_atlanta.bench | 0.001 | 0.001 | 0.000 | 0.001 | 16/16 | 0.000 | 0/16 | 16/16 | 16 | 100.00 |
| c432_to_iscas89_atlanta.bench | 0.066 | 0.001 | 0.001 | 0.025 | 466/472 | 0.039 | 3/472 | 469/472 | 472 | 99.36 |
| c499_to_iscas89_atlanta.bench | 0.254 | 0.001 | 0.001 | 0.048 | 658/684 | 0.205 | 26/684 | 684/684 | 684 | 100.00 |
| c880_to_iscas89_atlanta.bench | 0.466 | 0.006 | 0.001 | 0.087 | 930/949 | 0.372 | 19/949 | 949/949 | 949 | 100.00 |
| c1355_to_iscas89_atlanta.bench | 1.519 | 0.007 | 0.001 | 0.235 | 1502/1564 | 1.276 | 62/1564 | 1564/1564 | 1564 | 100.00 |
| c1908_to_iscas89_atlanta.bench | 4.283 | 0.007 | 0.002 | 0.651 | 1713/1859 | 3.623 | 144/1859 | 1857/1859 | 1859 | 99.89 |
| c2670_to_iscas89_atlanta.bench | 50.806 | 0.010 | 0.006 | 2.046 | 2238/2629 | 48.743 | 305/2629 | 2543/2629 | 2629 | 96.73 |
