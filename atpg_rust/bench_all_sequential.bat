@echo off
pushd "%~dp0"
echo starting sequential benchmarks...

rem lista di netlist da processare
set IDS=17 432 499 880 1355 1908 2670 3540 5315 6288 7552 27 382 420 641 713 1238 1423 1488 5378 9234 13207 15850 35932 38417 38584

for %%I in (%IDS%) do (
  echo.
  echo === running bench full %%I ===
  cargo run -- bench full %%I
  echo === completed %%I ===
  echo === updating README.md with benchmark %%I ===
  git add README.md
  echo README.md updated with benchmark %%I
  echo committing changes to README.md
  git commit -m "add benchmark %%I in readme"
  echo changes committed. pushing to remote repository
  git push
  echo Benchmark %%I processed and changes pushed to remote repository.
)

echo === all benchmarks completed ===
popd
pause
