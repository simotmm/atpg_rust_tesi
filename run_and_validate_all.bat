@echo off
start "validation on netlist 2"    cmd /k run_rust_and_validate_with_atlanta.bat 2
timeout /t 1 /nobreak >nul
start "validation on netlist 3"    cmd /k run_rust_and_validate_with_atlanta.bat 3
timeout /t 1 /nobreak >nul
start "validation on netlist 17"   cmd /k run_rust_and_validate_with_atlanta.bat 17
timeout /t 1 /nobreak >nul
start "validation on netlist 432"  cmd /k run_rust_and_validate_with_atlanta.bat 432
timeout /t 1 /nobreak >nul
start "validation on netlist 499"  cmd /k run_rust_and_validate_with_atlanta.bat 499
timeout /t 1 /nobreak >nul
start "validation on netlist 880"  cmd /k run_rust_and_validate_with_atlanta.bat 880
timeout /t 1 /nobreak >nul
start "validation on netlist 1355" cmd /k run_rust_and_validate_with_atlanta.bat 1355
timeout /t 1 /nobreak >nul
start "validation on netlist 1908" cmd /k run_rust_and_validate_with_atlanta.bat 1908
timeout /t 1 /nobreak >nul
start "validation on netlist 2670" cmd /k run_rust_and_validate_with_atlanta.bat 2670
timeout /t 1 /nobreak >nul
start "validation on netlist 3540" cmd /k run_rust_and_validate_with_atlanta.bat 3540
timeout /t 1 /nobreak >nul
start "validation on netlist 5315" cmd /k run_rust_and_validate_with_atlanta.bat 5315
timeout /t 1 /nobreak >nul
start "validation on netlist 6288" cmd /k run_rust_and_validate_with_atlanta.bat 6288
timeout /t 1 /nobreak >nul
start "validation on netlist 7552" cmd /k run_rust_and_validate_with_atlanta.bat 7552
timeout /t 1 /nobreak >nul
start "validation on netlist 2670" cmd /k run_rust_and_validate_with_atlanta.bat 2670
