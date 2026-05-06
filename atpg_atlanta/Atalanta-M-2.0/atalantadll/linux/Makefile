# atalantadll library makefile
# 
# By Myslik Radovan, 2008

VPATH=../src/
CXXFLAGS=-std=c++98 -W -Wall -pedantic -D_LINUX -DLEARNFLG -DATALANTA -DINCLUDE_HOPE -fPIC -O3 
LDFLAGS=-shared -fPIC -I$(VPATH)

SO=libatalantadll.so
OBJ=Atpg.o Error.o Fan.o FanNet.o Fault.o FaultSimulation.o GateNet.o Globals.o Hash.o ParralelPattern.o Random.o ReadableNet.o ResultsReader.o Simulation.o Params.o

$(SO): $(OBJ) 
	$(CXX) -o $@ $(OBJ) $(LDFLAGS)

$(OBJ): Atpg.h
Atpg.o: Atpg.h
Error.o: Error.h
Fan.o: Fan.h
FanNet.o: FanNet.h
Fault.o: Fault.h
FaultSimulation.o: FaultSimulation.h
Gate.o: Gate.h
GateNet.o: GateNet.h
Globals.o: Globals.h
Hash.o: Hash.h
Parameteres.o: Parametres.h
ParralelPattern.o: ParralelPattern.h
Params.o: Params.h
Random.o: Random.h
ReadableNet.o: ReadableNet.h
ResultsReader.o: ResultsReader.h
Simulation.o: Simulation.h
Stack.o: Stack.h
Truthtable.o: Truthtable.h

clean:
	rm -f $(OBJ)
	rm -f $(SO)
