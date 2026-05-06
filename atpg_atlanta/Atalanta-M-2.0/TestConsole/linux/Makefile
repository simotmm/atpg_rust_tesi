IPATH=../../atalantadll/src/
VPATH=../src/
CXXFLAGS=-std=c++98 -W -Wall -pedantic -D_LINUX -fPIC -O3 -I$(IPATH)
LDFLAGS=-fPIC -latalantadll -L$(VPATH) -I$(IPATH) -I$(VPATH)

BIN=Main
OBJ=Main.o
SO=Main

$(SO): $(OBJ) #$(SO)
	$(CXX) -o $@ $(OBJ) $(LDFLAGS)

#$(SO):
#	$(MAKE) -C $(VPATH) libatalantadll.so

$(OBJ): Main.cpp
Main.o: Main.h

clean:
	rm -f $(OBJ)
	rm -f $(BIN)
