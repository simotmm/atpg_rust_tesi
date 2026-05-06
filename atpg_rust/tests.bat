@echo off
start "atpg on netlist 2"    cmd /k cargo run 2
timeout /t 1 /nobreak >nul
start "atpg on netlist 3"    cmd /k cargo run 3
timeout /t 1 /nobreak >nul
start "atpg on netlist 17"   cmd /k cargo run 17
timeout /t 1 /nobreak >nul
start "atpg on netlist 432"  cmd /k cargo run 432
timeout /t 1 /nobreak >nul
start "atpg on netlist 499"  cmd /k cargo run 499
timeout /t 1 /nobreak >nul
start "atpg on netlist 880"  cmd /k cargo run 880
timeout /t 1 /nobreak >nul
start "atpg on netlist 1355" cmd /k cargo run 1355
timeout /t 1 /nobreak >nul
start "atpg on netlist 1908" cmd /k cargo run 1908
timeout /t 1 /nobreak >nul
start "atpg on netlist 2670" cmd /k cargo run 2670
timeout /t 1 /nobreak >nul
start "atpg on netlist 3540" cmd /k cargo run 3540
timeout /t 1 /nobreak >nul
start "atpg on netlist 5315" cmd /k cargo run 5315
timeout /t 1 /nobreak >nul
start "atpg on netlist 6288" cmd /k cargo run 6288
timeout /t 1 /nobreak >nul
start "atpg on netlist 7552" cmd /k cargo run 7552
timeout /t 1 /nobreak >nul
start "atpg on netlist 2670" cmd /k cargo run 2670