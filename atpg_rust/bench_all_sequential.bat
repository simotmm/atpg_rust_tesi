@echo off
pushd "%~dp0"
echo starting sequential benchmarks...

rem lista di netlist da processare
set IDS=17 432 499 880 1355 1908 2670 3540 5315 27 382 420 641 713 1238 1423 1488 5378 9234 13207 15850 35932 38417 38584 7552 6288
rem set IDS=38417 38584 7552 6288 rem sono quelli rimasti, ma prima riprovo con tutti con la versione aggiornata del codice

for %%I in (%IDS%) do (
  echo.
  echo === running bench full %%I ===
  cargo run -- bench full %%I
  echo === completed %%I ===
  echo === updating README.md with benchmark %%I ===
  echo === adding test input %%I to results folder ===
  git add .
  echo README.md updated with benchmark %%I
  echo test input %%I added to results folder
  echo committing changes to git repository
  git commit -m "adding benchmark %%I in readme (3rd bench run) and test input %%I in results folder"
  echo changes committed. pushing to remote repository.
  git push
  echo Benchmark %%I processed and changes pushed to remote repository.
)

echo === all benchmarks completed ===
popd
echo waiting 10 seconds before shutting down.
timeout /t 10 /nobreak >nul
shutdown /s /t 0
