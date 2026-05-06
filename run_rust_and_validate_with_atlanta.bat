@echo off
setlocal enabledelayedexpansion

:: validate-atlanta-m.bat - generate PAT from Rust ATPG, run Atalanta-M, report undetected faults
:: Usage: validate-atlanta-m.bat [CIRCUIT]
if "%~1"=="" (
  set "CIRCUIT=2"
) else (
  set "CIRCUIT=%~1"
)

set "BASEDIR=%~dp0"
set "RUST_DIR=%BASEDIR%atpg_rust"
set "AT_DIR=%BASEDIR%atpg_atlanta\atalanta-M"

echo.
echo ===== Validate circuit %CIRCUIT% (Atalanta-M) =====

echo [1/3] Running Rust ATPG (atpg_rust)...
pushd "%RUST_DIR%" || (echo ERROR: Cannot enter %RUST_DIR% & exit /b 1)
call cargo run -- %CIRCUIT% > "%RUST_DIR%\results\rust_output_%CIRCUIT%.txt" 2>&1 || (echo ERROR: cargo run failed & popd & exit /b 1)
popd

set "RES_FILE=%RUST_DIR%\results\c%CIRCUIT%_to_iscas89_atlanta_test_inputs.txt"
if not exist "%RES_FILE%" (
  echo ERROR: results file not found: "%RES_FILE%"
  exit /b 1
)

set "PAT_FILE=%RUST_DIR%\results\c%CIRCUIT%_to_iscas89_atlanta_test_inputs.pat"
echo Extracting pure PAT lines to "%PAT_FILE%" (lines with only 0/1)
powershell -NoProfile -Command "Get-Content '%RES_FILE%' | Where-Object { $_ -match '^[01]+$' } | Set-Content -Encoding ASCII '%PAT_FILE%'" >nul 2>&1

:: Copy PAT into Atalanta working dir to avoid path issues
set "PAT_COPY=%AT_DIR%\c%CIRCUIT%.pat"
copy /Y "%PAT_FILE%" "%PAT_COPY%" >nul 2>&1

echo [2/3] Running Atalanta-M validation via validate_with_atlanta.bat...
REM call the validation wrapper we created
if not exist "%BASEDIR%validate_with_atlanta.bat" (
  echo ERROR: validate_with_atlanta.bat not found in %BASEDIR%
  exit /b 1
)
set "SKIP_COMBINED_SUMMARY=1"
call "%BASEDIR%validate_with_atlanta.bat" %CIRCUIT%
set "AT_EXIT=%ERRORLEVEL%"
set "SKIP_COMBINED_SUMMARY="

:: Paths to files produced by validate_with_atlanta
set "REPORT_FILE=%RUST_DIR%\results\validation_report_%CIRCUIT%.txt"
set "UD_FILE=%RUST_DIR%\results\validation_undetected_%CIRCUIT%.txt"
set "AT_LOG=%BASEDIR%atalanta_output.log"
set "AT_LOG_FILTERED=%BASEDIR%atalanta_output.filtered.log"

echo [3/3] Validation summary:
echo Note: Atalanta summary was already printed by validate_with_atlanta.bat; printing combined Rust+Atalanta summary now.


:: Print combined Rust + Atalanta validation summary (if Rust output exists)
set "RUST_OUT=%RUST_DIR%\results\rust_output_%CIRCUIT%.txt"
if exist "%RUST_OUT%" (
  powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0print_validation_summary.ps1" -RustOutputPath "%RUST_OUT%" -AtalantaReportPath "%REPORT_FILE%" -AtalantaUndetectedPath "%UD_FILE%"
) else (
  echo Note: Rust output not found at %RUST_OUT%; combined summary not generated.
)

endlocal
exit /b %FINAL_EXIT%
