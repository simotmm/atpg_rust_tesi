@echo off
pushd "%~dp0"
echo Starting sequential benchmarks...

rem lista di netlist da processare
set IDS=17 432 499 880 1355 1908 2670 3540 5315 6288 7552 27 382 420 641 713 1238 1423 1488 5378 9234 13207 15850 35932 38417 38584

for %%I in (%IDS%) do (
  echo.
  echo === Running bench full %%I ===
  cargo run -- bench full %%I
  echo === Completed %%I ===
)

echo All benchmarks completed.
popd
pause
