# Efficient generation for test patterns using boolean difference

## Introduzione
### Obiettivo
Un circuito combinatorio può essere usato per individuare i fault "stuck-at" (fault che "forzano" l'uscita a un determinato valore 0/1 indipendetemente dagli input) applicando un vettore di input che produca una differenza nell'output. 
Gli obiettivi di un ATPG (automatic test pattern generation) sono: generare un insieme di test che rilevi ogni fault e identificare i fault non testabili.

### Complessità del problema
Nel caso peggiore la complessità è esponenziale nella dimensione del circuito; si vogliono evitare i casi peggiori tramite ricerche topologiche.

### Approccio
Il metodo consiste in due parti: 
 - 1: Dato un circuito e un fault nel circuito, costruire una formula che esprime la Differenza Booleana (xor) tra il circuito funzionante e quello guasto.
 - 2: Successivamente si applica un algoritmo di Boolean Satisfiability (SAT) alla formula della Differenza Booleana.
La Differenza Booleana descrive solo i vettori che distunguono i due circuiti quindi:
 - Soluzione SAT -> test valido.
 - Se la formula non può essere soddisfatta -> il fault è untestable.
Per la sua complessità algebrica la Differenza Booleana era ritenuta come impratica. Qui si dimostra che non è necessario semplificare la formula e che basta soddisfare la formula con un SAT solver.

#### Dettagli dell'approccio
Il sistema ATPG ha 3 componenti fondamentali: Wirelist translator, Pseudo-random Test Pattern Generator, Algorithmic Test Pattern Generator.
##### Wirelist Translator

Conversione della netlist in una struttura interna.
##### Pseudo-random Test Pattern Generator

Sono prodotti 32 pseudo-random pattern alla volta e sono simulati usando un sistema Parallel Pattern Single Fault Propagation (PPSFP) (modello Waicukauski), questa fase copre tra l'80% e il 99% dei fault. Quando un intero passaggio PPSFP produce meno di un numero predeterminato di pattern (al momento: 2) si passa alla fase di Algorithmic Test Pattern Generator, che copre il 100% dei fault. 
##### Algorithmic Test Pattern Generator

Per ogni fault rimanente ogni pattern è simulato usando un Single Pattern Single Fault Propagation Simulator in modo che ogni fault trovato venga rimosso dalla lista dei fault, se passa troppo tempo senza generare un pattern il fault viene considerato untestable.


## Estrazione della Formula
### Rappresentazione del circuito
Il circuito è un grafo diretto aciclico (DAG) in cui:
 - I nodi senza archi entranti (sources): sono le USCITE del circuito.
 - I nodi senza archi uscenti (sinks): sono gli INGRESSI del circuito.

Quindi visitando il DAG da un'uscita si incontrano tutti i nodi che la influenzano.

### Formula Caratteristica dei gate (3CNF) (prodotto delle somme)
Ogni nodo del grafo contiene la formula della porta logica corrispondente espressa nella forma "3-element conjuctive normal form" (3CNF), detta anche forma del prodotto delle somme; questa formula risulta vera se i valori assegnati alle variabili in input e output della porta logica sono consistenti.
(Esempio: porta AND, formula: D=AND(A,B)=A*B=(!D+A)*(!D+B)*(D+!A+!B))
(Esempio: Porta NOT, formula: E=!C=(C+E)*(!C+!E))
Quindi ogni porta logica contiene gli input in termini dell'output.
Percorrendo il grafo a partire da un'uscita del circuito (source) si ottiene in tempo lineare nella dimensione del circuito la formula completa contenente la congiunzione (*) di tutte le formule dei nodi visitati.
### Circuito guasto
Si crea una copia del circuito rinominando (esempio: D->D' (x': x con un valore stuck at (1/0))) solo le variabili che si trovano lungo ogni percorso tra il fault e l'output.
### Differenza Booleana
Definizione: la differenza booleana di un circuito rispetto a un determinato fault è data dall'operazione XOR tra l'output del circuito guasto (x') e l'output del circuito funzionante (x).
BD=XOR(X,X')=V1+V2=(X+!X')+(!X+X').
### Generazione del test pattern
Dato il fault, generare il test pattern corrispondente equivale a soddisfare la formula 3CNF che esprime BD, se la formula non è soddisfatta allora il fault non è testabile.


## Soddisfare la Formula
### Struttura 3SAT e considerazioni su 2SAT
soddisfare la formula 3CNF, corrisponde al problema 3SAT (classe np-completo (è stato trasformato da un problema esponenziale a un altro)) e può essere visto come una ricerca in un albero binario in cui ogni nodo è una variabile della formula e i due archi sono le due possibili assegnazioni booleane. Per soddisfare la formula si deve trovare un percorso consistente dalla radice a ogni foglia (consistente: nessuna clausola della formula deve essere valutata falsa).
La classe delle formule generate dai circuiti combinatori sono una sotto-classe di tutte le formule 3CNF in cui almeno 2/3 delle clausole generate dalla differenza booleana sono 2SAT (gli operatori unari a 2 input sono più dei due terzi di tutti gli operatori). Nella pratica dall'80% al 90% delle clausole sono 2CNF e il problema corrispondente, 2SAT, è lineare nel numero di clausole + numero di variabili.

### Algoritmo 2SAT
Si costruisce un grafo delle implicazioni:
Per ogni clausola (X+Y): si aggungono archi !X->Y e Y!->X.
Si riduce il grafo alle componenti fortemente connesse come singoli nodi (componenete fortemente connessa: insieme massimale di nodi tali che ogni nodo sia raggiungibile dagli altri dello stesso insieme -> classi di equivalenza). Se una classe di equivalenza contiene un letterale e la sua negazione la formula non è soddisfacibile.

### Iterazione sulle soluzioni 2SAT
L'obiettivo è iterare sulle soluzioni 2SAT per massimizzare le possibilità di estendere una soluzione 2SAT all'intera formula 3SAT.
Per farlo si ordinano le soluzioni dando priorità alle variabili con più vincoli (se un ramo della ricerca porta a una contraddizione lo si vede rapidamente)


## Minimizzazione dell'Albero
### Velocizzazione dell'algoritmo
L'algoritmo descritto è completo e se esiste una soluzione questa verrà trovata. Il processo può essere velocizzato facendo in modo che non vengano esplorate parti che non contengono soluzione, l'albero può essere quindi ridotto sottraendo e/o aggiungendo clausole alla formula.

### Rimozione delle clausole
Delle clausole possono essere rimosse con l'uso della relazione di determinazione. Si dice che la variabile V determina la variabile W quando entrambe le assegnazioni di V causano la comparsa di W in forma SOLO negata/non negata -> tutte le clausole contenenti W possono essere rimosse e W sarà assegnata dopo l'assegnazione di V. (replica del comportamento delle head line dell'algoritmo FAN). Questo NON riguarda le porte XOR (nota per me: cercare se ci sono stati aggiornamenti e in caso implementarli)

### Aggiunta delle clausole
Tre tipi di vincoli ridondanti vengono aggiunti per accelerare l'algoritmo.
#### Clausole attive
Una classe importante di informazioni ridondanti è un insieme di clausole che velocizzano la propagazione. Se un fault è rilevabile deve asserci almeno un percorso dal fault a un output primario tale che ogni filo sul percorso abbia valori faultati e non faultati differenti (potrebbe esserci più di un percorso, ma ci basta trovarne uno).
Si introduce una variavile ActW per ogni wire W sul cammino tra fault e output:
 - ActW = true -> il segnale differisce tra circuito faultato e non faultato
 - Si aggiungono claysole che implementano la propagazione dell'attivazione:
   - se un input di un gate è attivo -> l'uscita deve essere attiva
   - se un fanout è attivo -> almeno uno dei rami deve essere attivo
#### Clausole critiche
Se un gate è su un path attivo, sappiamo che il nodo corrispondente propagherà il fault (discrepanza), quindi gli input non attivi devono assumere valori critici in modo che il fault venga propagato:
 - AND/NAND: input non attivi = 1
 - OR/NOR: input non attivi = 0
 - XOR: nessun input non attivo deve cambiare valore
#### Implicazioni non locali
Le implicazioni non locali sono vincoli logici che derivano non dal comportamento di un singolo gate ma dalla struttura complessiva di un fanout che riconverge. Possiamo mettere in una lista tutte le implicazioni non locali di una variabile e poi annotare le implicazioni che usano una clausola ternaria. Ogni implicazione che usa un'implicazione ternaria deve derivare da un fanout convergente. Quindi: se la variabile B implica non localmente la variabile F allora la clausola (!B+F) può essere aggiunta alla formula per essere soddisfatta. (se ho capito bene, basta trovare questa implicazioni e aggiungerle per evitare di scendere in profondità, quindi riducendo l'albero di ricerca SAT)
