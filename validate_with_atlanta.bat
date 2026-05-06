@echo off
REM validate_with_atlanta.bat
REM Usage: validate_with_atlanta.bat <benchmark_number>

:: param check
if "%~1"=="" (
  echo Usage: %~nx0 ^<benchmark_number^>
  exit /b 2
)
set "num=%~1"
set "scriptDir=%~dp0"
set "resultsDir=%scriptDir%atpg_rust\results"
set "inputFile=%resultsDir%\c%num%_to_iscas89_atlanta_test_inputs.txt"
if not exist "%inputFile%" (
  echo Input file not found: %inputFile%
  echo Available files matching %num%:
  dir /b "%resultsDir%\*%num%*" 2>nul
  exit /b 3
)

set "exePath=%scriptDir%atpg_atlanta\atalanta-M\atalanta-M.exe"
if not exist "%exePath%" (
  echo Atalanta executable not found: %exePath%
  exit /b 4
)

set "outputFile=%resultsDir%\validation_results_%num%.txt"
rem choose converted .bench file from atpg_rust\benchmarks\converted_to_atlanta_iscas89
set "benchDir=%scriptDir%atpg_rust\benchmarks\converted_to_atlanta_iscas89"
set "benchFile=%benchDir%\c%num%_to_iscas89_atlanta.bench"
if not exist "%benchFile%" (
  echo Bench file not found: %benchFile%
  exit /b 5
)
set "cfileName=%benchFile%"
set "reportFile=%resultsDir%\validation_report_%num%.txt"
set "undetectedFile=%resultsDir%\validation_undetected_%num%.txt"

echo Using input: %inputFile%
echo Atalanta exe: %exePath%
echo Circuit file: %cfileName%
echo Saving results to: %outputFile%

pushd "%scriptDir%atpg_atlanta\atalanta-M"
"%exePath%" -S -t "%inputFile%" -P "%reportFile%" -U "%undetectedFile%" "%cfileName%" > "%outputFile%" 2>&1
set "rc=%ERRORLEVEL%"
popd
if %rc% neq 0 (
  echo Atalanta returned error code %rc%
)

echo --- Begin Atalanta output (first 200 lines) ---
powershell -NoProfile -Command "Get-Content -Path '%outputFile%' -TotalCount 200" 
echo --- End Atalanta output ---

if exist "%reportFile%" (
  powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0print_atlanta_summary.ps1" -ReportPath "%reportFile%" -UndetectedPath "%undetectedFile%"
) else (
  echo Report file not found: %reportFile%
)

:: Also attempt to print the combined Rust/Atalanta summary if Rust output exists
set "RUST_OUT=%resultsDir%\rust_output_%num%.txt"
if not defined SKIP_COMBINED_SUMMARY (
  if exist "%RUST_OUT%" (
    powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0print_validation_summary.ps1" -RustOutputPath "%RUST_OUT%" -AtalantaReportPath "%reportFile%" -AtalantaUndetectedPath "%undetectedFile%"
  ) else (
    echo Note: Rust output not found at %RUST_OUT%; combined summary not generated.
  )
) else (
  rem SKIP_COMBINED_SUMMARY set; skipping combined summary to avoid duplicate
)

echo Full results saved to: %outputFile%
