# Thesis Work Report - 1
## Abstract
This document summarizes the progress to date on the thesis project aimed at efficient generation of test vectors for stuck‑at faults, based on the paper "Efficient generation for test patterns using boolean difference" and the experimental Rust implementation (`atpg_rust`). It describes the study phases, design, implementation, obtained results and open areas for further investigation.

## 1. Introduction and Objective
The goal of the work is to understand the ATPG problem for combinational circuits and build a tool that generates effective test patterns by combining pseudo‑random techniques (PPSFP) with algorithmic procedures based on Boolean Difference (BD) solved using SAT. The work is grounded on the referenced paper ("Efficient generation for test patterns using boolean difference") and on the experimental implementation `atpg_rust` as a research platform.

## 2. Theoretical Study
- Reference file: "Efficient generation for test patterns using boolean difference"
- Key concepts integrated into the project:
  - Boolean Difference (BD): construction of the XOR formula between the good circuit and the faulty circuit outputs; satisfiable ⇨ valid test vector, unsatisfiable ⇨ untestable fault.
  - Gate→3CNF representation (product of sums) for each logic gate: e.g. AND D = A*B ⇨ (!D + A) * (!D + B) * (D + !A + !B).
  - The formulas produced by this transformation contain a large portion of 2CNF clauses; the 2SAT portion is solvable in linear time and is exploited to guide extensions into the 3SAT portion.
  - Search-tree minimization techniques: removal of redundant clauses, introduction of activation variables (ActW), critical clauses and non-local implications to accelerate propagation.

## 3. Design and Development Phases of `atpg_rust`
Below are the main phases carried out in the implementation and the major design choices.

3.1 Benchmark and format choices
- First approach: ISCAS89 (ISCAS89 and ISCAS85). ISCAS89 benchmarks often contain sequential elements, so conversion/normalization of the format was preferred.
- Parsers and translators were implemented to convert ISCAS85/ISCAS89 netlists into the format used by Atalanta (compatibility with external simulators).

3.2 DAG construction for the circuit
- Implemented the `Dag` structure representing the combinational circuit as a directed acyclic graph.
- Both forward and reverse adjacency lists (`succ_adj` and `rev_adj`) were built to enable efficient topological traversals and to optimize fault injection and recomputation operations.

3.3 Fault list generation and management
- Initially every gate was expanded over its fanout generating two possible faults (/0 and /1) for each branch.
- Later the Atalanta-like logic was adopted for fault enumeration (see `Dag::all_faults` and the `Fault { wire, sa1, site }` representation), including the distinction between producer-level and site-specific faults.

3.4 Data structure for CNF
- Given the `Dag`, each gate is converted into its local CNF (product of sums) and the complete formula for a cone of influence is obtained by concatenating the CNFs of the visited nodes.
- The representation is a vector of vectors of literals (CNF: Vec<Vec<Literal>>) ready for DIMACS conversion when required by the SAT backend.

3.5 Pseudo-random pattern generation and simulation
- Implemented 32-bit packed pattern generation (`InputPattern` maps input→u32) for bit-parallel simulation.
- Developed the PPSFP simulator that simulates blocks of 32 patterns, with methods `simulate_good`, `simulate_fault`, `simulate_patterns_block(_parallel)`.

3.6 SAT solving
- For residual faults the test formula is constructed: CNF(good circuit) + CNF(faulty circuit) + BD clauses and additional constraints (ActW, critical clauses, non-local implications) to direct the search.
- Initially a generic solution was implemented (internal DPLL and hybrid 2SAT/3SAT solver). Later `varisat` was integrated as a backend (using DIMACS) for improved performance and stability.

3.7 Boolean minimization
- Implemented minimization methods described in the paper and experimented with integrating `espresso-iisojs`.
- Early versions were inefficient due to data structure choices; using `varisat` for SAT provided a more effective solution for the critical phase.

3.8 Validation scripts
- Scripts were written to simulate the outputs produced by `atpg_rust` with the `Atalanta-M` simulator and compare results (fault coverage, generated patterns) to verify correctness and compatibility.

## 4. Current Results
- Patterns produced by `atpg_rust`, once converted to the format accepted by `Atalanta-M`, generally provide good coverage results.
- In some cases discrepancies are observed in the number of faults generated/recorded: this seems related to the logic of fault generation/nominalization (site vs producer) and requires reviewing the fault list generation and correspondence with Atalanta formats.
- Pipeline performance is satisfactory with `varisat`; CNF and minimization optimizations require further work to be robust across all benchmarks.

## 5. Open Issues and Investigation Points
- Analyze discrepancies in fault counting: verify site/producer correspondence and the rules for enumerating the `fault_list`.
- Refactor data structures used in CNF minimization to reduce overhead and make exchanges with Espresso efficient (if still desired).
- Improve integration of activation/critical clauses and non-local implications to reduce the SAT search tree.

## 6. Next Steps
- Full audit of fault list generation and comparison tests with Atalanta on a set of corner cases.
- Optimize data structures for minimization and re-evaluate the Espresso interface or implement lighter local reduction techniques.
- Extend the benchmark suite to ISCAS89.
- Begin study on quantum material: reading "Mastering Quantum Computing with IBM QX" to prepare for transitioning towards ideas on test generation using quantum resources.
