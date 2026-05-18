@echo off
pushd "%~dp0"
echo starting sequential benchmarks...

rem lista di netlist da processare
set IDS=38417 38584 7552 6288

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
echo waiting 10 seconds before shutting down...
timeout /t 10 /nobreak >nul
shutdown /s /t 0
