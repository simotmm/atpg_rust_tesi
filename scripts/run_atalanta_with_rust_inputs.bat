@echo off
rem Run Atalanta-M on .bench and .pat produced by atpg_rust and save comparison
pushd "%~dp0\.." 2>nul || (
    echo ERRORE: impossibile cambiare alla radice del progetto.
    echo Posizione script: %~dp0
    exit /b 1
)

setlocal EnableDelayedExpansion
echo.
if "%~1"=="" (
    echo Uso: %~nx0 ^<numero_benchmark^>
    echo Esempio: %~nx0 2670
    popd
    exit /b 1
)
set "id=%~1"

rem If user provided a .pat path as second argument, use it directly
if not "%~2"=="" (
    set "pat_file=%~2"
    if not exist "%pat_file%" (
        echo ERRORE: file .pat specificato non trovato: "%pat_file%"
        popd
        exit /b 1
    ) else (
        echo Usando .pat specificato: "%pat_file%"
        goto :foundpat
    )
)

set "bench_dir=atpg_rust\benchmarks\converted_to_atlanta_iscas89"
set "bench_suffix=_to_iscas89_atlanta.bench"
set "bench_file="
set "bench_prefix=c"
set "candidate=%bench_dir%\%bench_prefix%%id%%bench_suffix%"
if exist "%candidate%" (
    set "bench_file=%candidate%"
) else (
    set "bench_prefix=s"
    set "candidate=%bench_dir%\%bench_prefix%%id%%bench_suffix%"
    if exist "%candidate%" (
        set "bench_file=%candidate%"
    )
)
if not defined bench_file (
    rem fallback: cerca qualsiasi bench che contenga l'id
    for /f "delims=" %%B in ('dir /b /s "atpg_rust\benchmarks\*%id%*.bench" 2^>nul') do (
        set "bench_file=%%B"
        goto :foundbench
    )
)
if not defined bench_file (
    echo ERRORE: nessun file .bench trovato per "%id%" in atpg_rust\benchmarks
    popd
    exit /b 1
)
:foundbench
echo bench: !bench_file!
for %%A in ("!bench_file!") do set "stem=%%~nA"

set "pat_file="
if defined stem (
    rem expected output from atpg_rust: <stem>_test_inputs.pat
    set "expected=results\%stem%_test_inputs.pat"
    set "expected2=atpg_rust\results\%stem%_test_inputs.pat"
    if exist "%expected%" (
        set "pat_file=%expected%"
    ) else if exist "%expected2%" (
        set "pat_file=%expected2%"
    )
    rem if not found, try any .pat that contains the stem (search inside results and atpg_rust\results)
    if not defined pat_file (
        for /f "delims=" %%P in ('dir /b /s "results\%stem%*.pat" 2^>nul') do (
            set "pat_file=%%P"
            goto :foundpat
        )
    )
    if not defined pat_file (
        for /f "delims=" %%P in ('dir /b /s "atpg_rust\results\%stem%*.pat" 2^>nul') do (
            set "pat_file=%%P"
            goto :foundpat
        )
    )
)
rem if still not found, try to find any pat containing the numeric id (fallback)
if not defined pat_file (
    for /f "delims=" %%P in ('dir /b /s "results\*%id%*.pat" 2^>nul') do (
        set "pat_file=%%P"
        goto :foundpat
    )
)
if not defined pat_file (
    for /f "delims=" %%P in ('dir /b /s "atpg_rust\results\*%id%*.pat" 2^>nul') do (
        set "pat_file=%%P"
        goto :foundpat
    )
)

if not defined pat_file (
    echo ERRORE: nessun file .pat trovato nei risultati (results) per "%id%" o per lo stem "%stem%".
    echo Assicurati di avere eseguito atpg_rust e di avere il file "*_test_inputs.pat" in "results"
    popd
    exit /b 1
)
:foundpat
echo input pattern: !pat_file!
echo.

set "output_dir=validation_results\atalanta_und_faults_from_rust_inputs"
if not exist "%output_dir%" (
    mkdir "%output_dir%"
)

set "exe_dir=atpg_atlanta\atalanta-M"
set "exe_path=%exe_dir%\atalanta-M.exe"
if not exist "%exe_path%" (
    echo ERRORE: eseguibile Atalanta-M non trovato in "%exe_path%"
    popd
    exit /b 1
)

set "flt=%output_dir%\!stem!_atalanta.flt"
set "rep=%output_dir%\!stem!_atalanta.rep"
set "ud=%output_dir%\!stem!_atalanta.ud"
set "summary=%output_dir%\!stem!_comparison.txt"
rem set "und_list_txt=%output_dir%\!stem!_atalanta_und_faults_list.txt"

echo generazione della lista dei fault
echo -> (file: !flt!)
"%exe_path%" -F "!flt!" "!bench_file!"
echo -> fault salvati in "!flt!"

echo simulazione dei vettori (.pat) con atalanta-M per ottenere i fault non rilevati
"%exe_path%" -S -t "!pat_file!" -P "!rep!" -U "!ud!" -v "!bench_file!"
echo -> fault non rilevati salvati in "!ud!"

set "TOTAL=0"
set "UNDET=0"
if exist "!flt!" (
    set TOTAL=0
    for /f "usebackq delims=" %%L in ("!flt!") do (
        if not "%%L"=="" set /a TOTAL+=1
    )
)
if exist "!ud!" (
    set UNDET=0
    for /f "usebackq delims=" %%L in ("!ud!") do (
        if not "%%L"=="" set /a UNDET+=1
    )
)

set /a DETECTED=TOTAL - UNDET

echo bench: !stem!>"!summary!"
echo total_faults: !TOTAL!>>"!summary!"
echo detected: !DETECTED!>>"!summary!"
echo undetected: !UNDET!>>"!summary!"
echo ud_file: !ud!>>"!summary!"

rem Removed undetected faults file generation

popd
echo.
echo risultati salvati in "%output_dir%" 
echo -summary: "!summary!"
echo -tutti i fault: "!flt!"
echo -fault non rilevati: "!ud!"
exit /b 0
