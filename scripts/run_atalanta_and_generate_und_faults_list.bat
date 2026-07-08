@echo off
rem voglio usare atalanta-M per generare la lista di fault non rilevati per c432, così da poterla confrontare con quella generata da atpg_rust e vedere se coincidono
rem porta la working directory alla radice del progetto indipendentemente da dove viene lanciato lo script
pushd "%~dp0\.." 2>nul || (
    echo ERRORE: impossibile cambiare alla radice del progetto.
    echo Posizione script: %~dp0
    exit /b 1
)

setlocal EnableDelayedExpansion

if "%~1"=="" (
    set "id=432"
) else (
    set "id=%~1"
)

set "bench_suffix=_to_iscas89_atlanta"
set "bench_extension=.bench"
set "bench_dir=atpg_rust\benchmarks\converted_to_atlanta_iscas89"

rem provo prima con prefisso 'c', se mancante uso 's'
set "bench_prefix=c"
set "bench_file=!bench_dir!\!bench_prefix!!id!!bench_suffix!!bench_extension!"
echo bench file 1: !bench_file!
if not exist "!bench_file!" (
    echo bench file 2: !bench_file!
    set "bench_prefix=s"
    set "bench_file=!bench_dir!\!bench_prefix!!id!!bench_suffix!!bench_extension!"
)

if not exist "!bench_file!" (
    echo bench file 3: !bench_file!
    echo ERRORE: bench file non trovato con prefissi 'c' o 's': "%bench_dir%\*%id%%bench_suffix%%bench_extension%"
    popd
    exit /b 1
)

echo Using bench prefix: %bench_prefix%

set "output_dir=validation_results"
set "output_file=%output_dir%\%bench_prefix%%id%_atalanta_und_faults_list.txt"

set "executable=atalanta-M.exe"
set "exe_dir=atpg_atlanta\atalanta-M"
set "exe_path=%exe_dir%\%executable%"
set "options=-U"

rem creo la cartella di output se non esiste
if not exist "%output_dir%" (
    mkdir "%output_dir%"
)

rem verifico che il bench e l'eseguibile esistano
if not exist "!bench_file!" (
    echo ERRORE: bench file non trovato: "!bench_file!"
    popd
    exit /b 1
)
if not exist "%exe_path%" (
    echo ERRORE: eseguibile non trovato: "%exe_path%"
    popd
    exit /b 1
)

echo Running: "%exe_path%" %options% "%output_file%" "!bench_file!"

"%exe_path%" %options% "%output_file%" "!bench_file!"
set "EXIT_CODE=%ERRORLEVEL%"

rem conto le righe non nulle nel file di output (prima di tornare alla dir originale)
if exist "%output_file%" (
    for /f %%N in ('powershell -NoProfile -Command "(Get-Content -LiteralPath \"%output_file%\") | Where-Object { $_.Trim() -ne '' } | Measure-Object | Select-Object -ExpandProperty Count"') do set "NLINES=%%N"
) else (
    set "NLINES=0"
)

popd
echo atalanta-M execution completed
echo undetected faults list saved to "%output_file%"
echo atalanta undetected faults for %bench_prefix%%id%: %NLINES%

exit /b %EXIT_CODE%