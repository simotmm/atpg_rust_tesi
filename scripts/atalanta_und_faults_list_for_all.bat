@echo off

set IDS=17 432 499 880 1355 1908 2670 3540 5315 27 382 420 641 713 1238 1423 1488 5378 9234 13207 15850 35932 38417 38584 7552 6288

for %%I in (%IDS%) do (
    echo.
    echo === running undetected faults list generation for benchmark %%I ===
    call "%~dp0run_atalanta_and_generate_und_faults_list.bat" %%I
    echo === completed undetected faults list generation for benchmark %%I ===
)