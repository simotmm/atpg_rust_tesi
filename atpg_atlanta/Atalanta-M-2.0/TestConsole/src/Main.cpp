// TestCosole.cpp : Defines the entry point for the console application.
//
#include "Atpg.h"
#include "Simulation.h"

#include <stdio.h>

using namespace atalantadll;

int main(int argc, char** argv)
{
	int res = 0;
	fstream bench, pat;
	int b = false;
	try {
		Atalanta *simulation=new Atalanta();
		Params *p = new Params();

		res=simulation->run(argc, argv);

		/*bench.open("b18_opt_C.bench", ios::in);
		b = bench.is_open();
		pat.open("b18_opt_C.pat", ios::out);
		p->setBenchStream(bench.rdbuf());
		p->setSPatternStream(pat.rdbuf());
		p->setWTestMode(1);
		p->setCctMode('9');
		simulation->setParams(p);
		simulation->run();*/
	}
	catch(string s) {
		cerr << s;
	}
	catch(...) {
		cerr << "Uknown exception";
	}
	return res;	
}