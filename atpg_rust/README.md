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
| c17_to_iscas89_atlanta.bench | 0.001 | 0.000 | 0.000 | 0.001 | 16/16 | 0.000 | 0/16 | 16/16 | 16 | 100.00 |
| c432_to_iscas89_atlanta.bench | 0.174 | 0.000 | 0.000 | 0.018 | 466/472 | 0.154 | 3/472 | 469/472 | 472 | 99.36 |
| c499_to_iscas89_atlanta.bench | 0.269 | 0.001 | 0.001 | 0.040 | 658/684 | 0.227 | 26/684 | 684/684 | 684 | 100.00 |
| c880_to_iscas89_atlanta.bench | 0.470 | 0.001 | 0.001 | 0.075 | 930/949 | 0.392 | 19/949 | 949/949 | 949 | 100.00 |
| c1355_to_iscas89_atlanta.bench | 2.111 | 0.001 | 0.001 | 0.239 | 1502/1564 | 1.869 | 62/1564 | 1564/1564 | 1564 | 100.00 |
| s27_to_iscas89_atlanta.bench | 0.103 | 0.000 | 0.000 | 0.102 | 17/17 | 0.000 | 0/17 | 17/17 | 17 | 100.00 |
| s641_to_iscas89_atlanta.bench | 15.668 | 0.002 | 0.001 | 1.960 | 517/582 | 13.705 | 39/582 | 556/582 | 582 | 95.53 |
| s713_to_iscas89_atlanta.bench | 21.917 | 0.001 | 0.002 | 2.444 | 519/656 | 19.470 | 46/656 | 565/656 | 656 | 86.13 |
| s1488_to_iscas89_atlanta.bench | 18.036 | 0.002 | 0.003 | 3.906 | 915/950 | 14.124 | 31/950 | 946/950 | 950 | 99.58 |
| s420_to_iscas89_atlanta.bench | 36.555 | 0.001 | 0.001 | 0.848 | 110/282 | 35.705 | 102/282 | 212/282 | 282 | 75.18 |
| c1908_to_iscas89_atlanta.bench | 59.587 | 0.003 | 0.003 | 0.762 | 1713/1859 | 58.818 | 144/1859 | 1857/1859 | 1859 | 99.89 |
| c5315_to_iscas89_atlanta.bench | 53.302 | 0.010 | 0.010 | 11.872 | 5148/5160 | 41.411 | 2/5160 | 5150/5160 | 5160 | 99.81 |
| s1423_to_iscas89_atlanta.bench | 38.386 | 0.002 | 0.002 | 4.561 | 219/688 | 33.821 | 126/688 | 345/688 | 688 | 50.15 |
| s1238_to_iscas89_atlanta.bench | 44.078 | 0.002 | 0.002 | 4.884 | 901/1093 | 39.190 | 172/1093 | 1073/1093 | 1093 | 98.17 |
| c2670_to_iscas89_atlanta.bench | 195.747 | 0.005 | 0.005 | 8.693 | 2238/2629 | 187.044 | 305/2629 | 2543/2629 | 2629 | 96.73 |
| c3540_to_iscas89_atlanta.bench | 698.657 | 0.006 | 0.007 | 8.206 | 3301/3402 | 690.438 | 24/3402 | 3325/3402 | 3402 | 97.74 |
| s5378_to_iscas89_atlanta.bench | 757.392 | 0.011 | 0.010 | 235.066 | 4353/5234 | 522.305 | 51/5234 | 4404/5234 | 5234 | 84.14 |
| c7552_to_iscas89_atlanta.bench | 1002.339 | 0.013 | 0.015 | 171.993 | 6941/7316 | 830.318 | 293/7316 | 7234/7316 | 7316 | 98.88 |
| s9234_to_iscas89_atlanta.bench | 1968.719 | 0.019 | 0.100 | 1743.644 | 1182/6259 | 224.957 | 0/6259 | 1182/6259 | 6259 | 18.88 |
