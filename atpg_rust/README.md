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
| c3540_to_iscas89_atlanta.bench | 81.025 | 0.010 | 0.004 | 1.825 | 3301/3402 | 79.186 | 24/3402 | 3325/3402 | 3402 | 97.74 |
| c5315_to_iscas89_atlanta.bench | 8.494 | 0.010 | 0.005 | 2.141 | 5148/5160 | 6.338 | 2/5160 | 5150/5160 | 5160 | 99.81 |
| s27_to_iscas89_atlanta.bench | 0.005 | 0.004 | 0.000 | 0.001 | 17/17 | 0.000 | 0/17 | 17/17 | 17 | 100.00 |
| s420_to_iscas89_atlanta.bench | 0.147 | 0.008 | 0.001 | 0.023 | 110/282 | 0.116 | 102/282 | 212/282 | 282 | 75.18 |
| s641_to_iscas89_atlanta.bench | 0.250 | 0.007 | 0.001 | 0.053 | 517/582 | 0.189 | 39/582 | 556/582 | 582 | 95.53 |
| s713_to_iscas89_atlanta.bench | 0.475 | 0.006 | 0.001 | 0.087 | 519/656 | 0.381 | 46/656 | 565/656 | 656 | 86.13 |
| s1238_to_iscas89_atlanta.bench | 1.336 | 0.006 | 0.001 | 0.251 | 901/1093 | 1.078 | 172/1093 | 1073/1093 | 1093 | 98.17 |
| s1423_to_iscas89_atlanta.bench | 0.936 | 0.006 | 0.002 | 0.269 | 219/688 | 0.658 | 126/688 | 345/688 | 688 | 50.15 |
| s1488_to_iscas89_atlanta.bench | 0.331 | 0.009 | 0.002 | 0.164 | 915/950 | 0.156 | 31/950 | 946/950 | 950 | 99.58 |
| s5378_to_iscas89_atlanta.bench | 86.239 | 0.014 | 0.006 | 16.656 | 4353/5234 | 69.563 | 51/5234 | 4404/5234 | 5234 | 84.14 |
| s27_to_iscas89_atlanta.bench | 0.001 | 0.000 | 0.000 | 0.001 | 17/17 | 0.000 | 0/17 | 17/17 | 17 | 100.00 |
| s420_to_iscas89_atlanta.bench | 0.137 | 0.001 | 0.001 | 0.020 | 110/282 | 0.116 | 102/282 | 212/282 | 282 | 75.18 |
| s641_to_iscas89_atlanta.bench | 0.243 | 0.001 | 0.001 | 0.057 | 517/582 | 0.184 | 39/582 | 556/582 | 582 | 95.53 |
| s713_to_iscas89_atlanta.bench | 0.479 | 0.001 | 0.001 | 0.088 | 519/656 | 0.389 | 46/656 | 565/656 | 656 | 86.13 |
| s1238_to_iscas89_atlanta.bench | 1.343 | 0.001 | 0.002 | 0.262 | 901/1093 | 1.078 | 172/1093 | 1073/1093 | 1093 | 98.17 |
| s1423_to_iscas89_atlanta.bench | 0.916 | 0.001 | 0.002 | 0.265 | 219/688 | 0.648 | 126/688 | 345/688 | 688 | 50.15 |
| s1488_to_iscas89_atlanta.bench | 0.312 | 0.002 | 0.002 | 0.161 | 915/950 | 0.148 | 31/950 | 946/950 | 950 | 99.58 |
| s5378_to_iscas89_atlanta.bench | 90.058 | 0.005 | 0.006 | 33.452 | 4353/5234 | 56.595 | 51/5234 | 4404/5234 | 5234 | 84.14 |
| s9234_to_iscas89_atlanta.bench | 211.404 | 0.019 | 0.012 | 181.436 | 1182/6259 | 29.938 | 0/6259 | 1182/6259 | 6259 | 18.88 |
| s13207_to_iscas89_atlanta.bench | 673.126 | 0.020 | 0.017 | 265.617 | 4362/7839 | 407.473 | 1311/7839 | 5673/7839 | 7839 | 72.37 |
| s15850_to_iscas89_atlanta.bench | 1280.996 | 0.023 | 0.020 | 615.763 | 4881/10352 | 665.189 | 3/10352 | 4884/10352 | 10352 | 47.18 |
| s35932_to_iscas89_atlanta.bench | 11946.141 | 0.036 | 0.037 | 11394.426 | 2476/24700 | 551.643 | 0/24700 | 2476/24700 | 24700 | 10.02 |
| c17_to_iscas89_atlanta.bench | 0.007 | 0.006 | 0.000 | 0.001 | 16/16 | 0.000 | 0/16 | 16/16 | 16 | 100.00 |
| c432_to_iscas89_atlanta.bench | 0.065 | 0.001 | 0.000 | 0.024 | 466/472 | 0.040 | 3/472 | 469/472 | 472 | 99.36 |
| c499_to_iscas89_atlanta.bench | 0.252 | 0.005 | 0.001 | 0.040 | 658/684 | 0.206 | 26/684 | 684/684 | 684 | 100.00 |
| c880_to_iscas89_atlanta.bench | 0.456 | 0.010 | 0.001 | 0.078 | 930/949 | 0.367 | 19/949 | 949/949 | 949 | 100.00 |
| c1355_to_iscas89_atlanta.bench | 1.491 | 0.006 | 0.001 | 0.233 | 1502/1564 | 1.250 | 62/1564 | 1564/1564 | 1564 | 100.00 |
| c1908_to_iscas89_atlanta.bench | 4.293 | 0.009 | 0.002 | 0.644 | 1713/1859 | 3.637 | 144/1859 | 1857/1859 | 1859 | 99.89 |
| s27_to_iscas89_atlanta.bench | 0.001 | 0.000 | 0.000 | 0.001 | 17/17 | 0.000 | 0/17 | 17/17 | 17 | 100.00 |
| s420_to_iscas89_atlanta.bench | 0.139 | 0.001 | 0.001 | 0.023 | 110/282 | 0.115 | 102/282 | 212/282 | 282 | 75.18 |
| s641_to_iscas89_atlanta.bench | 0.245 | 0.001 | 0.001 | 0.055 | 517/582 | 0.188 | 39/582 | 556/582 | 582 | 95.53 |
| s713_to_iscas89_atlanta.bench | 0.478 | 0.001 | 0.001 | 0.086 | 519/656 | 0.390 | 46/656 | 565/656 | 656 | 86.13 |
| s38417_to_iscas89_atlanta.bench | 4158.404 | 0.046 | 0.050 | 3234.419 | 1132/24940 | 923.889 | 72/24940 | 1204/24940 | 24940 | 4.83 |
| **riga: ora ottimizzato** (si spera) | Total (s) | Parse (s) | DAG (s) | Rand (s) | Rand detected | SAT (s) | SAT detected | Total detected | Faults | Coverage (%) |

| c17_to_iscas89_atlanta.bench | 0.052 | 0.000 | 0.000 | 0.052 | 16/16 | 0.000 | 0/16 | 16/16 | 16 | 100.00 |
