@echo off
if "%~1"=="" (
  set "CIRCUIT=2"
) else (
  set "CIRCUIT=%~1"
)
pushd "%~dp0..\atpg_rust" 2>nul || (
  echo ERRORE: cartella "atpg_rust" non trovata relativa allo script.
  echo Posizione script: %~dp0
  exit /b 1
)
cargo run -- %CIRCUIT%
set "EXIT_CODE=%ERRORLEVEL%"
popd
exit /b %EXIT_CODE%