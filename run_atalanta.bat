@echo off
setlocal

if "%~1"=="" (
  echo Usage: %~nx0 ^<benchmark_number^>
  exit /b 2
)
set "num=%~1"
set "BASEDIR=%~dp0"
set "EXE=%BASEDIR%atpg_atlanta\atalanta-M\atalanta-M.exe"
set "BENCH=%BASEDIR%atpg_rust\benchmarks\converted_to_atlanta_iscas89\c%num%_to_iscas89_atlanta.bench"
set "INPUT=%BASEDIR%atpg_rust\results\c%num%_to_iscas89_atlanta_test_inputs.txt"

if not exist "%EXE%" (
  echo ERROR: Atalanta-M executable not found: "%EXE%"
  exit /b 3
)
if not exist "%BENCH%" (
  echo ERROR: Bench file not found: "%BENCH%"
  exit /b 4
)


echo Running Atalanta-M on benchmark %num%...

powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0run_atalanta.ps1" %num%

endlocal
