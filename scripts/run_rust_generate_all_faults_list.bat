@echo off

set IDS=2 3 17 432 499 880 1355 1908 2670 3540 5315 27 382 420 641 713 1238 1423 1488 5378 9234 13207 15850 35932 38417 38584 7552 6288
cd ..
cd atpg_rust
for %%I in (%IDS%) do (
    cargo run -- f %%I
    echo completed faults list generation for benchmark %%I 
)
echo.
echo faults list generated for all benchmarks.
