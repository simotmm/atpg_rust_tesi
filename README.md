# ATPG — README generale

Questo documento fornisce una panoramica della cartella `atpg`, spiegando dove si trovano i due motori principali (implementazione Rust e Atalanta C/C++), come compilarli/usarli e a cosa servono gli script inclusi.

## Struttura principale
- `atpg_rust/` — implementazione in Rust del generatore ATPG e strumenti correlati.
- `atpg_atlanta/` — sorgenti di 3 versioni di atalanta (atalanta, atalanta-m (usato negli script), atalanta-m2). Contiene anche una `README` locale e un `makefile`/soluzione Visual Studio.
- script nella root — vari `.bat` e `.ps1` per eseguire, validare e stampare riepiloghi.

## Requisiti
- Rust toolchain (cargo, rustc) per `atpg_rust`.
- Strumenti di compilazione C/C++ per Atalanta: Visual Studio (soluzione `.sln`) o toolchain GNU (`make` se disponibile).
- Windows PowerShell / CMD per eseguire gli script forniti.

## atpg_rust (cartella `atpg_rust`)
1. Compilare:

```powershell
cd atpg_rust
cargo build --release
```

2. Eseguire (esempio per visualizzare l'aiuto):

```powershell
cd atpg_rust
cargo run --release -- -h
```

3. Note:
- `target/release/` conterrà l'eseguibile compilato.
- Troverai file sorgente principali come `main.rs`, `parser.rs`, `netlist.rs`, ecc. Consultare i commenti nel codice per opzioni dettagliate.

## Atalanta (cartella `atpg_atlanta/Atalanta`)
1. Compilazione con Visual Studio:

 - Apri `[atpg_atlanta/Atalanta/Atalanta.sln](atpg_atlanta/Atalanta/Atalanta.sln)` e compila la soluzione (Debug o Release).

2. Compilazione con make (se supportato):

```powershell
cd atpg_atlanta/Atalanta
make
```

3. Eseguire:

- L'eseguibile risultante (es. `Atalanta.exe`) viene usato per la simulazione/validazione dei pattern. Per dettagli sui parametri dell'eseguibile consultare il file `README` locale in quella cartella o la documentazione interna.

## Script nella root, esempi d'uso
- `run_atpg_rust.bat` — esegue l'ATPG Rust con impostazioni predefinite.
- `run_atalanta.bat` / `run_atalanta.ps1` — esegue Atalanta su input predefiniti.
- `run_and_validate_all.bat` — esegue l'intero flusso (generazione con Rust / Atalanta) e poi la validazione.
- `run_rust_and_validate_with_atlanta.bat` — esegue `atpg_rust` e valida i pattern prodotti con Atalanta.
- `validate_with_atlanta.bat` / `validate_with_atlanta.ps1` — valida file di pattern/netlist usando Atalanta.
- `print_atlanta_summary.ps1` — genera un riepilogo dei risultati prodotti da Atalanta (formato leggibile).
- `print_validation_summary.ps1` — stampa un riepilogo delle validazioni effettuate.
- `tests.bat` (in `atpg_rust/`) — esegue la batteria di test per la versione Rust.

Esempi rapidi (usando argomenti numerici per selezionare un benchmark):

- Eseguire `atpg_rust` sul benchmark numero `3`:

```powershell
.\run_atpg_rust.bat 3
```

- Eseguire Atalanta sul benchmark numero `3` (o sul file prodotto per il benchmark 3):

```powershell
.\run_atalanta.bat 3
```

- Lanciare tutta la pipeline (Rust → validazione con Atalanta) sul benchmark `3`:

```powershell
.\run_rust_and_validate_with_atlanta.bat 3
```

- Eseguire la validazione direttamente da PowerShell specificando l'indice `3`:

```powershell
.\validate_with_atlanta.ps1 3
```

Come funzionano gli argomenti numerici
- Gli script accettano un argomento intero (es. `1`, `2`, `3`) che viene usato per selezionare il file benchmark corrispondente nella cartella `atpg_rust/benchmarks/`.
- I file nella cartella `atpg_rust/benchmarks/converted_to_atlanta_iscas89` sono versioni ISCAS85 convertite nel formato leggibile da Atalanta; l'intero passato agli script mappa al relativo file benchmark (in genere secondo l'ordine alfabetico o l'indexing implementato dagli script).
- Se non viene fornito alcun argomento, gli script possono eseguire un set di default o iterare su più benchmark (comportamento dipendente dallo script specifico).

## Suggerimenti e risoluzione problemi
- Se manca `cargo`, installare Rust dal sito ufficiale (rustup).
- Se la compilazione di Atalanta fallisce con errori Visual Studio, aprire la soluzione in Visual Studio e ricostruire per risolvere eventuali dipendenze mancanti.
- Assicurarsi che gli script `.ps1` siano eseguiti con i permessi corretti (es. eseguire PowerShell come amministratore o modificare la `ExecutionPolicy` se necessario).

## Dove approfondire
- Esempi, casi di test e dettagli operativi si trovano nelle sottocartelle (`atpg_rust/README.md`, `atpg_atlanta/Atalanta/README`). Consultare quei file per opzioni avanzate e formati di input/output.
