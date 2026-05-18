@echo off
rem iscas85
start "bench on netlist c17"  cmd /k cargo run -- bench full 17
timeout /t 1 /nobreak >nul
start "bench on netlist c432"  cmd /k cargo run -- bench full 432
timeout /t 1 /nobreak >nul
start "bench on netlist c499"  cmd /k cargo run -- bench full 499
timeout /t 1 /nobreak >nul
start "bench on netlist c880"  cmd /k cargo run -- bench full 880
timeout /t 1 /nobreak >nul
start "bench on netlist c1355" cmd /k cargo run -- bench full 1355
timeout /t 1 /nobreak >nul
start "bench on netlist c1908" cmd /k cargo run -- bench full 1908
timeout /t 1 /nobreak >nul
start "bench on netlist c2670" cmd /k cargo run -- bench full 2670
timeout /t 1 /nobreak >nul
start "bench on netlist c3540" cmd /k cargo run -- bench full 3540
timeout /t 1 /nobreak >nul
start "bench on netlist c5315" cmd /k cargo run -- bench full 5315
timeout /t 1 /nobreak >nul
start "bench on netlist c6288" cmd /k cargo run -- bench full 6288
timeout /t 1 /nobreak >nul
start "bench on netlist c7552" cmd /k cargo run -- bench full 7552
rem iscas89
start "bench on netlist s27"  cmd /k cargo run -- bench full 27
timeout /t 1 /nobreak >nul
start "bench on netlist s382" cmd /k cargo run -- bench full 382
timeout /t 1 /nobreak >nul
start "bench on netlist c432" cmd /k cargo run -- bench full 420
timeout /t 1 /nobreak >nul
start "bench on netlist s641" cmd /k cargo run -- bench full 641
timeout /t 1 /nobreak >nul
start "bench on netlist s713" cmd /k cargo run -- bench full 713
timeout /t 1 /nobreak >nul
start "bench on netlist s1238" cmd /k cargo run -- bench full 1238
timeout /t 1 /nobreak >nul
start "bench on netlist s1423" cmd /k cargo run -- bench full 1423
timeout /t 1 /nobreak >nul
start "bench on netlist s1488" cmd /k cargo run -- bench full 1488
timeout /t 1 /nobreak >nul
start "bench on netlist s5378" cmd /k cargo run -- bench full 5378
timeout /t 1 /nobreak >nul
start "bench on netlist s9234" cmd /k cargo run -- bench full 9234
timeout /t 1 /nobreak >nul
start "bench on netlist s13207" cmd /k cargo run -- bench full 13207
timeout /t 1 /nobreak >nul
start "bench on netlist s15850" cmd /k cargo run -- bench full 15850
timeout /t 1 /nobreak >nul
start "bench on netlist s35932" cmd /k cargo run -- bench full 35932
timeout /t 1 /nobreak >nul
start "bench on netlist s38417" cmd /k cargo run -- bench full 38417
timeout /t 1 /nobreak >nul
start "bench on netlist s38584" cmd /k cargo run -- bench full 38584