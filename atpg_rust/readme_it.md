
# atpg_rust

Questo documento descrive il progetto `atpg_rust`: il flusso d'esecuzione, la logica generale, l'architettura dei moduli e una spiegazione dettagliata delle funzioni principali.

Scopo: generare vettori di test che rilevino fault stuck-at in netlist ISCAS/Atlanta. Il programma combina generazione pseudo-random (PPSFP) e risoluzione SAT per cercare pattern che coprano i fault non rilevati.

**Contenuto**
 - **Overview**: concetto e scopo
 - **Flusso d'esecuzione**: passi principali eseguiti da `main`
 - **Moduli principali**: mappa dei file e responsabilità
 - **Strutture dati chiave**: tipi principali e significato
 - **Funzioni principali**: descrizioni dettagliate e comportamento
 - **Esempi di esecuzione** e opzioni runtime

**Nota**: per la navigazione del codice, i riferimenti ai file sono inclusi come link.

**Panoramica rapida**
Il tool prende un netlist (formato ISCAS89/Atlanta), costruisce un DAG con CNF associate ai nodi, genera pattern casuali, simula in parallelo il circuito buono e il circuito con fault (PPSFP) per verificare quali fault vengono rilevati; per i fault non rilevati costruisce formule SAT (XOR tra circuito buono e circuito faulted) e prova a risolverle per ottenere vettori che causino una differenza sulle uscite.

**Esecuzione**

1. Input e parsing
  - Il file di ingresso viene caricato e parsato in una struttura `Netlist` ([parser.rs](parser.rs#L1)).
  - Funzione principale: `file_iscas89_atlanta_to_netlist` ([parser.rs](parser.rs#L1-L40)).

2. Costruzione DAG e CNF
  - `Dag::from_netlist` converte la `Netlist` in un grafo diretto aciclico di nodi con CNF per ogni gate ([dag.rs](dag.rs#L1-L40)).
  - Si precomputano liste di adiacenza predecessor (`rev_adj`) e successor (`succ_adj`) per traversal e fault injection efficiente.

3. Generazione pattern pseudo-random
  - `PatternGenerator` crea `InputPattern` casuali basati su seed ([pattern_generator.rs](pattern_generator.rs#L1)).

4. Simulazione PPSFP (Parallel Pattern Single Fault Propagation)
  - `PPSFPSimulator::simulate_patterns_blocks` simula blocchi di pattern e valuta quali fault sono rilevati ([ppsfp.rs](ppsfp.rs#L1)).
  - Metodi chiave: `simulate_good`, `simulate_fault`, `simulate_patterns_block`, `simulate_patterns_block_parallel`.

5. Algoritmo SAT per fault residui
  - Per i fault non rilevati, si costruisce la formula di test CNF che combina CNF(circuito buono) + CNF(circuito faulted) + clausole di boolean difference (BD__).
  - `sat::sat_solving` orchestra la risoluzione SAT (scelta backend `varisat` o interno). Usa minimizzazione opzionale della CNF prima di chiamare il solver ([sat.rs](sat.rs#L1)).

6. Post-processing
  - Le soluzioni SAT vengono convertite in `InputPattern` e simulate nuovamente per conferma.
  - I pattern finali sono salvati tramite funzioni in `util.rs`.

### File e responsabilità (mappa rapida)
 - [main.rs](main.rs#L1): orchestrazione principale e parsing argomenti.
 - [parser.rs](parser.rs#L1): conversione file ISCAS/Verilog → `Netlist`.
 - [netlist.rs](netlist.rs#L1): definizioni `GateType`, `Gate`, `Netlist`.
 - [dag.rs](dag.rs#L1): costruzione DAG, valutatore booleano, conversione in CNF, enumerazione fault.
 - [pattern_generator.rs](pattern_generator.rs#L1): `InputPattern` e `PatternGenerator`.
 - [ppsfp.rs](ppsfp.rs#L1): simulatore PPSFP (single-word 32-bit vector simulation, parallelizzato con `rayon`).
 - [sat.rs](sat.rs#L1): orchestrazione SAT, wrapper per `varisat` e risolutore interno (2-SAT + estensione 3-SAT / DPLL).
 - [simple_solver.rs](simple_solver.rs#L1): risolutore interno DPLL/unit-propagation per CNF generiche.
 - [cnf.rs](cnf.rs#L1): struttura `CNF`, conversione gate→CNF, utilità su literals.
 - [cnf_minimizer.rs / cnf_minimizer_espresso.rs](cnf_minimizer.rs#L1): minimizzazione CNF (builtin o interfaccia Espresso).
 - [options.rs](options.rs#L1): opzioni runtime (minimizer, SAT backend, flags).
 - [util.rs](util.rs#L1): funzioni di utilità per stampa, parsing argomenti, salvataggio.

Sezione seguente: definizioni e comportamento dettagliato delle funzioni principali.

---

**Tipi e strutture dati chiave**

 - `GateType` ([netlist.rs](netlist.rs#L1)): enum che rappresenta le porte (AND, OR, XOR, NAND, NOR, XNOR, BUF, NOT). Metodo: `from_str` e `to_string`.
 - `Gate` ([netlist.rs](netlist.rs#L20)): `name`, `gate_type`, `outputs`, `inputs`.
 - `Netlist` ([netlist.rs](netlist.rs#L40)): `inputs`, `outputs`, `gates`; metodo `to_string()` per debug.
 - `Dag` e `DagNode` ([dag.rs](dag.rs#L1)): rappresentazione a nodi con `cnf` per ogni gate, `rev_adj` e `succ_adj` per traversali efficienti.
 - `Fault` ([ppsfp.rs](ppsfp.rs#L1)): `wire`, `sa1`, `site` (opzionale) — usato per enumerare e identificare stuck-at.
 - `InputPattern` ([pattern_generator.rs](pattern_generator.rs#L1)): mappa `input -> u32` dove i 32 bit rappresentano 32 vettori packati.

---

Funzioni e metodi principali (dettagli)

1) `main()` ([main.rs](main.rs#L1-L20))
 - Responsabilità: leggere argomenti CLI, impostare opzioni runtime, caricare file netlist, costruire `Dag`, generare fault, eseguire fase random (PPSFP) e fase algoritmica (SAT), stampare e salvare risultati.
 - Punti salienti del flusso in `main.rs`:
  - parsing argomenti: `get_nth_arg_to_int/string/bool` ([util.rs](util.rs#L1)).
  - `file_iscas89_atlanta_to_netlist(&filename)` → `Netlist` ([parser.rs](parser.rs#L1-L40)).
  - `Dag::from_netlist(&circuit)` → costruzione DAG ([dag.rs](dag.rs#L1-L40)).
  - `PPSFPSimulator::new(&dag)` crea simulatore.
  - `dag.generate_fault_list(fault_wire, sa1)` genera la lista di `Fault` da coprire.
  - Se abilitata fase random: genera N pattern con `PatternGenerator::generate_n_patterns_from_netlist`, li simula con `simulate_patterns_blocks`.
  - Se rimangono fault, costruisce formule e chiama `sat::sat_solving(...)` per ottenere soluzioni SAT (opzionalmente parallelo).
  - Concatena pattern da entrambe le fasi, le stampa e le salva con `save_patterns_to_test_inputs`.

2) Parser: `file_iscas89_atlanta_to_netlist` ([parser.rs](parser.rs#L1-L40))
 - Legge un file riga-per-riga, ignora commenti e righe vuote, riconosce `INPUT(...)`, `OUTPUT(...)` e definizioni di gate `OUT = GATETYPE(IN1, IN2)`.
 - Restituisce `Netlist` popolato con `inputs`, `outputs`, `gates` (ogni `Gate` con `outputs[0]` impostato sul nome del gate).

3) `Dag::from_netlist` ([dag.rs](dag.rs#L1-L40))
 - Converte `Netlist` in `Dag` creando per ogni gate un `DagNode` con CNF (usando `gate_to_cnf_literals` in `cnf.rs`).
 - Compone `rev_adj` (predecessori) e `succ_adj` (successori) per ogni nodo; queste liste sono usate per traversal bottom-up e per fault injection O(n).

4) `PatternGenerator` e `InputPattern` ([pattern_generator.rs](pattern_generator.rs#L1))
 - `InputPattern` mantiene una `HashMap<String,u32>`: per ogni primary input è memorizzata una word a 32 bit contenente 32 vettori packati (bitwise simulation).
 - Metodi principali:
  - `InputPattern::random_from_netlist(netlist, seed)` → riempie gli input con `rng.next_u32()`.
  - `InputPattern::insert_pattern` e `insert_patterns` → costruiscono pattern bitwise a partire da stringhe binarie (MSB->LSB mapping).
  - `PatternGenerator::generate_n_patterns_from_netlist` → genera N `InputPattern` incrementando il seed.

5) `PPSFPSimulator` ([ppsfp.rs](ppsfp.rs#L1))
 - Scopo: simulare il circuito su vettori packati a 32 bit e verificare per ciascun fault se la differenza tra circuiti good/faulty genera output non-zero.
 - Costruttore: `PPSFPSimulator::new(&dag)` costruisce un ordine topologico `topo` basato su `rev_adj`.
 - Metodi principali:
  - `simulate_good(&mut self, patterns: &InputPattern)` — calcola `good` (mappa signal->u32) eseguendo gate simulation in ordine topologico; salva i valori intermedi e finali.
  - `simulate_fault(&mut self, fault: &Fault) -> u32` — forza la wire del fault (`!0` per sa1, `0` per sa0), ricalcola i nodi downstream in ordine topologico e ritorna `diff` = XOR delle uscite finali `good ^ faulty` (se non-zero il fault è rilevato su almeno un bit della parola).
  - `simulate_patterns_blocks(patterns, faults, parallel)` — per ogni pattern simula `good` e testa tutti i fault (sequenziale o parallelo). Ritorna mappa pattern->(fault->det_word) e l'insieme di fault non rilevati (BTreeSet per ordine deterministico).
  - Implementazione parallela: `simulate_patterns_block_parallel` usa `rayon::par_iter` e clona lo stato `good` per thread; mantiene counters atomici per report progress.

6) `Dag::evaluate_boolean` e `primed_clone` ([dag.rs](dag.rs#L1))
 - `evaluate_boolean` valuta il DAG su una assegnazione booleana `HashMap<String,bool>` per verifiche di correttezza a singolo bit (usata per diagnostica sulle soluzioni SAT).
 - `primed_clone` costruisce una copia del DAG con tutte le uscite rinominate `X'` e aggiorna input/output e CNF di conseguenza; utile per costruire la parte faulty della formula di test.

7) SAT: `sat::sat_solving` e funzioni correlate ([sat.rs](sat.rs#L1))
 - `sat_solving` è il wrapper che sceglie tra esecuzione sequenziale o parallela.
 - Per ogni fault costruisce la formula di test test_formula = CNF(GOOD cone) + CNF(FAULTED cone) + clausole BD__ per le differenze sulle uscite.
 - Minimizzazione CNF: opzionale, chiamando `minimize_search_tree` o passando la CNF a Espresso (`cnf_minimizer_espresso`). Questo può ridurre la dimensione del problema SAT per il backend.
 - Backend SAT:
  - `varisat_solver_bits`: converte la CNF in formati DIMACS e invoca `varisat` per ottenere una assignment completa (modello) — ritorna pattern bits e mappa assegnazione.
  - `sat_solver_bits`: strategia interna che prova prima la porzione 2-SAT, genera assegnazioni 2-SAT con `two_sat` e poi estende con `three_sat_solver` (unit propagation + DPLL) per la porzione 3-SAT.

8) `three_sat_solver` (interno, [sat.rs](sat.rs#L1))
 - Algoritmo: riceve una lista di assegnazioni valide per la porzione 2-SAT e prova ad estenderle usando unit propagation e una versione DPLL su clausole rimanenti; se fallisce prova infine un risolutore completo (`simple_solver::solve_cnf`) con eventuale seeding di variabili prioritarie.

---

Esempi rapidi

 - Eseguire su un bench (predefinito):
  - `cargo run -- <n> <fault_wire?> <sa1?> <seed?> <mode?> <enable_sat?> <optimize?> <minimizer?> <sat_backend?>`
  - Esempio: `cargo run -- 432` (carica il file benchmark #432 nella cartella `benchmarks\\converted_to_atlanta_iscas89`).

Opzioni comuni (vedi [main.rs](main.rs#L1) e [options.rs](options.rs#L1)):
 - `mode`: `rand` / `alg` / `both` (default `both`).
 - `sat_backend`: `varisat` o `builtin`.
 - `minimizer`: `espresso` o `builtin` (opzionale).

---

**Interpretazione dei fault**

La notazione usata nel progetto (e compatibile con l'output in stile Atalanta) ha la forma generale:

 - `ID[/->ID] /<0|1>`

Dove la prima parte può essere solo il nome del wire (es. `A`) oppure includere un *site descriptor* che indica producer->sink (es. `N11->N543`). Dopo uno spazio e la barra `/` compare il valore dello stuck-at: `1` per stuck-at-1, `0` per stuck-at-0.

Esempi:

 - `N11->N543 /1` — indica un fault relativo al filo prodotto da `N11` osservato nella connessione verso `N543` (site `N11->N543`); il valore `/1` significa stuck-at-1. Nel codice questo viene rappresentato come `Fault { wire: "N11", sa1: true, site: Some("N11->N543") }` e, quando stampato con `Fault::to_string()`, verrà mostrato esattamente come `N11->N543 /1`.

 - `A /0` — indica un fault sul wire `A` senza site descriptor; `/0` significa stuck-at-0. Nel codice: `Fault::new("A", false)` e la stampa sarà `A /0`.

Comportamento in stampa e in enumerazione

 - Se il `Fault` ha un campo `site` valorizzato viene usato quel valore nella stringa di output, altrimenti viene usato il solo `wire`.
 - La generazione dei fault (vedi `Dag::all_faults`) può produrre sia fault "producer-level" (solo `wire`) sia fault con descrittore sito (`site`) per rappresentare site-specific entries simili a quelle di Atalanta.

Questa convenzione rende immediato distinguere se un fault è relativo al produttore del segnale o a una specifica connessione (site) lungo il fanout.

---

**Versione estesa: logica e strutture dati**

Di seguito descrivo in dettaglio la logica e le strutture dati usate in ciascuna fase richiesta.

**1) Generazione della fault list**

- Punto d'ingresso: `Dag::generate_fault_list` ([dag.rs](dag.rs#L1)).
- Logica:
  - Si parte dal DAG costruito da `Dag::from_netlist`. Per ogni `DagNode` si valuta il suo `out_count` (numero di successori, cioè fanout) usando `succ_adj`.
  - Per ogni nodo si costruisce una lista di `pfaults_per_gate` seguendo regole ispirate ad Atalanta:
    - fault sull'output del gate: se fanout>1 o `is_final` allora vengono considerati sia SA0 sia SA1 (o solo il più rilevante a seconda del tipo di gate). Per alcune porte (XOR/XNOR) si aggiungono entrambi; per OR/NOR si privilegia SA0; per AND/NAND si privilegia SA1.
    - fault su input-line (site faults): per gate con più ingressi, se la sorgente ha fanout>1 vengono generate fault site-specific `site = Some("src->dst")` per poter distinguere casistiche producer vs site.
  - Si identificano gli FFR/stem (gates con fanout>1 o final outputs) e si esegue una DFS/visita inversa per enumerare i fault in ordine compatibile con Atalanta (stems in reverse order, traversal dentro FFR): questo determina l'ordine finale della `fault_list`.
- Strutture dati coinvolte:
  - `DagNode` ([dag.rs](dag.rs#L1)): contiene `outputs`, `inputs`, `is_final`, `cnf`.
  - `Dag`: contiene `nodes: Vec<DagNode>`, `rev_adj: Vec<Vec<usize>>`, `succ_adj: Vec<Vec<usize>>`.
  - `Fault` ([ppsfp.rs](ppsfp.rs#L1)): `wire: String`, `sa1: bool`, `site: Option<String>`.

**2) Generazione delle CNF**

- File: `cnf.rs` (costruzione CNF), `dag.rs` (uso delle CNF per nodi) e helper `gate_to_cnf_literals` ([dag.rs](dag.rs#L1), [cnf.rs](cnf.rs#L1)).
- Logica:
  - Ogni `DagNode` viene associato a una `CNF` che rappresenta la relazione logica tra i suoi ingressi e l'uscita (es. per una NAND si costruiscono le clausole equivalenti in CNF). Questo avviene in `Dag::from_netlist` che chiama `gate_to_cnf_literals` per ogni gate.
  - Per costruire la formula di test per un fault si usa:
    - `CNF_good_cone = dag.cnf_cone_from_outputs()` — raccoglie ricorsivamente le CNF solo dei nodi necessari a determinare le uscite finali (`is_final`).
    - `faulty = dag.primed_clone()` per avere le variabili primed (es. `X'`) per il circuito con fault.
    - `CNF_faulted_cone = faulty.cnf_cone_from_outputs()` per la parte faultata.
    - `get_boolean_difference_clauses(x, x')` aggiunge variabili ausiliarie BD__ per dichiarare che una uscita differisce tra good e faulty. Infine si aggiunge una clausola che richiede almeno un `BD__` vero (altrimenti si ottiene solo soluzioni triviale senza differenza in uscita).
  - Formula test finale: `test_formula = CNF_good_cone + CNF_faulted_cone + BD__ clauses`.
- Strutture dati:
  - `CNF` ([cnf.rs](cnf.rs#L1)): `clauses: Vec<Vec<Literal>>` dove `Literal { var: String, neg: bool }`.
  - `Dag` e `DagNode.cnf` (CNF per ogni nodo).

**3) Simulazione PPSFP su random pattern**

- File: `pattern_generator.rs`, `ppsfp.rs`.
- Logica:
  - `PatternGenerator::generate_n_patterns_from_netlist` crea N `InputPattern` packando 32 vettori in ciascuna `u32` per input (bit-parallel simulation). Seed deterministico per riproducibilità.
  - `PPSFPSimulator::simulate_patterns_blocks(patterns, faults, parallel)` per ogni pattern chiama `simulate_good` (calcola `good: HashMap<String,u32>`) e poi per ogni fault `simulate_fault` che forza la wire del fault (`forced = !0u32` per SA1, `0u32` per SA0) e ricalcola gli output downstream in ordine topologico. Se XOR(good, faulty) sulle uscite finali è NON ZERO, il fault è considerato rilevato per almeno uno dei 32 vettori packed.
  - L'implementazione parallela `simulate_patterns_block_parallel` usa `rayon::par_iter` su faults; ogni thread costruisce una copia locale di simulatore (clonando `good`) e esegue `simulate_fault` localmente per evitare lock globali.
- Strutture dati:
  - `InputPattern` ([pattern_generator.rs](pattern_generator.rs#L1)): `values: HashMap<String,u32>`.
  - `PPSFPSimulator` ([ppsfp.rs](ppsfp.rs#L1)): `dag: &Dag`, `good: HashMap<String,u32>`, `faulty: HashMap<String,u32>`, `topo: Vec<usize>` (ordine topologico calcolato in `new`).

**4) Generazione algoritmica dei pattern con SAT**

- File: `sat.rs` (orchestrazione), `two_sat.rs` (2-SAT helper), `simple_solver.rs` (builtin DPLL/unit-propagation), `cnf_minimizer*.rs` (opzionale).
- Logica di alto livello (`sat::sat_solving`):
  - Per ogni fault costruire `test_formula` come sopra.
  - Opzionalmente minimizzare la CNF (`minimize_search_tree` o passare a Espresso tramite `cnf_minimizer_espresso`) usando una lista di `prioritized_vars` derivata dalla cone-of-influence del fault (preferire primary inputs nella cone) per accelerare la ricerca.
  - Scegliere backend SAT: interno (2-SAT + estensione 3-SAT + fallback a `simple_solver`) oppure `varisat` (invocato da `varisat_solver_bits`) in base a `options::get_options().sat_backend`.
  - Se SAT trova un'assegnazione valida (modello) per le variabili primarie, la si converte in `Vec<(String,u32)>` dove ogni primary input prende valori {0,1,2} (2 = don't-care) e si converte in `InputPattern` con `sat_solutions_to_input_patterns`.

**5) Come funziona la parte 2-SAT → 3-SAT builtin e integrazione varisat**

- 2-SAT passo (module `two_sat.rs`)
  - Si identifica la porzione del CNF che è 2-literal clauses (clauses con ≤2 literal). Questa porzione è risolvibile in tempo polinomiale: il modulo `two_sat` genera *tutte* le assegnazioni modellanti la porzione 2-SAT (entro un limite configurabile per evitare esplosione combinatoria).
  - Se la porzione 2-SAT è insoddisfacibile, l'intera formula è UNSAT e si può saltare la parte restante.

- Estensione a 3-SAT (`three_sat_solver` in `sat.rs`)
  - Per ogni assegnazione 2-SAT candidate si prova a estenderla sulla porzione 3-SAT rimanente attraverso:
    1. unit propagation adattata per la struttura Literal/CNF;
    2. un DPLL ricorsivo `dpll_cnf` che sceglie variabili non assegnate in ordine deterministico (BTreeSet) e prova assegnazioni true/false con propagation.
  - Se estensione ha successo si ritorna la soluzione completa. Questo approccio è efficiente nei casi in cui la maggior parte delle variabili è già fissata dalla porzione 2-SAT.

- Fallback builtin: `simple_solver::solve_cnf`
  - Se l'approccio basato su 2-SAT+estensione non fornisce una soluzione utile oppure non è abilitato, il codice chiama `simple_solver::solve_cnf` che implementa DPLL con seeding (opzionale) e ritorna una `HashMap<String,bool>` se trova soluzione.

- Backend `varisat`
  - In alternativa, se `options::get_options().sat_backend == "varisat"`, la CNF viene convertita in literal DIMACS e passata a `varisat` (`varisat_solver_bits`). `varisat` è un SAT solver industriale/performante e restituisce un modello se SAT.
  - Vantaggio: prestazioni migliori su formule grandi; svantaggio: dipendenza esterna e minor controllo su strategie di estensione 2-SAT.

**6) Minimizzazione booleana (builtin vs Espresso)**

- Scopo: ridurre dimensione della CNF di test per velocizzare il SAT solver.

- Minimizzatore builtin (`cnf_minimizer::minimize_search_tree`)
  - Algoritmo: ricerca euristica/greedy che prova a rimuovere letterali/clausole non necessari per mantenere la soddisfacibilità della formula o l'equivalenza rispetto alla query BD__. È integrato direttamente nel repo e non richiede dipendenze esterne.
  - Vantaggi: integrato, riproducibile, controllabile dal codice; può essere molto efficace su piccoli/medi coni.

- Integrazione con Espresso (`cnf_minimizer_espresso.rs`)
  - Espresso è uno strumento esterno per minimizzazione di funzioni booleane (two-level minimization). Il codice può esportare la CNF/cover a Espresso e leggere il risultato, mappandolo di nuovo in CNF.
  - Avvertenze / perché non raccomandata:
    - Overhead di I/O e invocazione esterna può superare il beneficio per problemi piccoli/medi.
    - Richiede che Espresso sia installato e compatibile con la conversione CNF→PLA→CNF fatta dal progetto; la conversione può introdurre distorsioni e casi limite (es: overflow di variabili ausiliarie, complessità di mapping tra formati).
    - Per questi motivi la chiamata a Espresso è opzionale ed è off by default; la minimizzazione builtin dovrebbe essere provata prima.

**7) Simulazione sui nuovi pattern (verifica delle soluzioni SAT)**

- Dopo che il solver (builtin o varisat) produce una assegnazione, il codice:
  - converte il modello in `Vec<(String,u32)>` ordinata secondo le `primary_inputs` e in `InputPattern` tramite `sat_solutions_to_input_patterns`;
  - esegue `simulator.simulate_patterns_blocks(sat_patterns, undetected_faults, Some(PARALLEL))` per confermare quali fault risultano effettivamente rilevati dai pattern SAT sul simulatore bit-parallel;
  - (diagnostica) se è disponibile una assignment booleana completa (`HashMap<String,bool>`), il codice invoca `dag.evaluate_boolean` e `faulty.evaluate_boolean` per confrontare bit singoli e diagnosticare eventuali discrepanze fra modello SAT e valutatore booleano.

**8) Salvataggio su file finale**

- Funzione: `save_patterns_to_test_inputs` / `save_patterns_to_file` (in `util.rs`).
- Logica:
  - I pattern finali (concatenazione di PPSFP random + SAT patterns) vengono convertiti in formato test input compatibile con Atalanta/bench files e salvati nella directory/filename calcolata da `get_filename_from_int`.
  - Se `SAVE_UNDETECTED_TO_FILE` è abilitato, viene salvata anche la lista di fault non rilevati.
- Formati di output comuni:
  - `.pat` o file textual con mapping `input_name -> bitstring` per ogni pattern, replicabile anche su formati legacy usati nella cartella `results/`.

