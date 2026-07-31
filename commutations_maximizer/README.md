# Commutations Maximizer

Commutations Maximizer finds, for each compact block of input combinations (32 combinations per block), the pair of input vectors that maximizes the number of output signal toggles in a combinational circuit.

This repository is a small, standalone extraction useful for quickly exploring which pairs of input vectors cause the most signal transitions.

## Execution flow

1. The CLI (`src/main.rs`) reads a single command-line argument: the path to a circuit file (Verilog/bench style).
2. `parser` converts the input file into an in-memory `Netlist` (signals, inputs, gates).
3. `PatternGenerator` produces compact `InputPattern` packs: each input is a `u32` where its 32 bits represent 32 input combinations in parallel.
4. For each compact block (`InputPattern`):
   - `ppsfp` (bit-parallel simulator) evaluates the circuit, producing a `u32` per net that encodes its value across the 32 combinations.
   - `most_transitions_for_compact_pattern` inspects all local positions p<q (0..31) and computes how many nets differ between position p and q.
   - The simulator keeps the pair (p, q) with the highest transition count across all blocks.
5. The program prints the best block, the local positions, the global indices, and the input-to-bit mappings for both vectors.

## Key data structures

- `Netlist` (`src/netlist.rs`): representation of circuit signals, input names, gates and gate types.
- `InputPattern` (`src/pattern_generator.rs`): maps each input name to a `u32` where each bit represents a packed input combination (32 combos per word).
- Simulator state (`src/ppsfp.rs`): during simulation each net is evaluated to a `u32` (packed values), enabling parallel evaluation of 32 test vectors.
- `Dag` (`src/dag.rs`): topologically ordered gates for fast evaluation in `PPSFPSimulator`.

These compact representations trade off the number of inputs (N <= 5 recommended) for extremely fast bit-parallel simulation.

## CLI usage

Build and run the program from the project root. Examples below assume you are in the `commutations_maximizer` folder.

Basic run (debug build):

```bash
cargo run -- nets/c1.v
```

Release build run:

```bash
cargo run --release -- nets/c1.v
```

Notes:
- The program expects a single positional argument: the path to the circuit file. Example supported syntaxes include simple Verilog/bench style input lists (INPUT/OUTPUT and gate assignments).
- If the input file contains more than 5 inputs the compact method is not used and the program will exit with an error.

## Where results are saved

Results are written to the `results/` directory created next to the binary. The output filename is derived from the input file stem (basename without extension). Example:

- Input: `nets/c1.v` → Output: `results/c1_max_commutations_inputs.txt`

The file contains two lines: the first line is the bit string for the first vector, the second line is the bit string for the second vector (characters are `0`/`1` for each input in the Netlist order).

## Examples

Run and inspect output file:

```bash
cargo run -- nets/c17.v
# after run: cat results/c1_max_commutations_inputs.txt
```

Run release build and redirect stdout to a log:

```bash
cargo build --release
./target/release/commutations_maximizer nets/c17.v | tee run.log
```

## Build

```bash
cargo build
```

## Limitations

- This tool targets combinational circuits and supports at most 5 primary inputs. For circuits with more than 5 inputs the compact bit-parallel method is not applicable and the program will exit with an error.
- Compact mode packs 32 combinations into a `u32`; it is suitable when the circuit has at most 5 inputs (so the total number of combinations is manageable in blocks of 32).
- The parser is simple and may not accept all Verilog dialects.

## Development notes

- Key modules: `src/ppsfp.rs` (simulation), `src/pattern_generator.rs` (pattern packing), `src/parser.rs` (file parsing).
- To change output behavior edit `src/util.rs::save_bits_result` which controls the `results/` filename.

