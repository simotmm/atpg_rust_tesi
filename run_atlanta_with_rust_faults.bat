@echo off
REM Usage: run_atlanta_with_rust_faults.bat <benchmark_number>
if "%~1"=="" (
  echo Usage: %~nx0 ^<benchmark_number^>
  exit /b 2
)
set "num=%~1"
set "scriptDir=%~dp0"
set "resultsDir=%scriptDir%atpg_rust\results"

REM Generate Atalanta-compatible fault file from Rust output
echo Generating Atalanta fault file from Rust output...
python "%scriptDir%atpg_rust\scripts\rust_to_atlanta_faults.py" %num%
set "faultFile=%resultsDir%\c%num%_rust_undetected_for_atlanta.flt"
if not exist "%faultFile%" (
  echo Fault file not created: %faultFile%
  exit /b 3
)

set "inputFile=%resultsDir%\c%num%_to_iscas89_atlanta_test_inputs.txt"
if not exist "%inputFile%" (
  echo Input file not found: %inputFile%
  dir /b "%resultsDir%\*%num%*"
  exit /b 4
)

set "exePath=%scriptDir%atpg_atlanta\atalanta-M\atalanta-M.exe"
if not exist "%exePath%" (
  echo Atalanta executable not found: %exePath%
  exit /b 5
)

set "benchDir=%scriptDir%atpg_rust\benchmarks\converted_to_atlanta_iscas89"
set "benchFile=%benchDir%\c%num%_to_iscas89_atlanta.bench"
if not exist "%benchFile%" (
  echo Bench file not found: %benchFile%
  exit /b 6
)

set "outputFile=%resultsDir%\atalanta_with_rust_faults_%num%.txt"
set "reportFile=%resultsDir%\atalanta_with_rust_report_%num%.txt"
set "undetectedFile=%resultsDir%\atalanta_with_rust_undetected_%num%.txt"

echo Running Atalanta with fault list: %faultFile%
pushd "%scriptDir%atpg_atlanta\atalanta-M"
"%exePath%" -S -t "%inputFile%" -f "%faultFile%" -P "%reportFile%" -U "%undetectedFile%" "%benchFile%" > "%outputFile%" 2>&1
set "rc=%ERRORLEVEL%"
popd
if %rc% neq 0 (
  echo Atalanta returned error code %rc%
)

echo --- Begin Atalanta output (first 200 lines) ---
powershell -NoProfile -Command "Get-Content -Path '%outputFile%' -TotalCount 200"
echo --- End Atalanta output ---

echo Report file: %reportFile%
echo Undetected faults file: %undetectedFile%
echo Full Atalanta stdout/stderr saved to: %outputFile%

exit /b 0
