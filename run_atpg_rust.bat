@echo off
if "%~1"=="" (
  set "CIRCUIT=2"
) else (
  set "CIRCUIT=%~1"
)
cd atpg_rust
cargo run %CIRCUIT% 