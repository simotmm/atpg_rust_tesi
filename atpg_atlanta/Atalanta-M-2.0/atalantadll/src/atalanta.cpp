// atalanta.cpp : Defines the entry point for the console application.
//
#include "Atpg.h"
#include "Simulation.h"

#include <stdio.h>
#include <vector>

using namespace atalantadll;

int main(int argc, char** argv)
{
	int res = 0;
	fstream bench, pat;
	
	/*vector<int> vt;
	int  tmp[10];
	int iii;

	
	vt.push_back(12);
	vt.push_back(13);
	vt.push_back(45);
	for(iii = 0; iii < vt.size(); iii++) {
		tmp[iii] = vt[iii];
	}
	vt.erase(vt.begin() + 1);
	for(iii = 0; iii < vt.size(); iii++) {
		tmp[iii] = vt[iii];
	}*/
/*	list<Params> ps;
	list<Params>::iterator it;

	ps.push_back(*(new Params()));   //1
	ps.push_back(*(new Params()));   //2
	ps.push_back(*(new Params()));   //3
	ps.push_back(*(new Params()));   //4
	ps.push_back(*(new Params()));   //5
	ps.push_back(*(new Params()));   //6
	ps.push_back(*(new Params()));   //7


	char *adr = (char*)malloc(100);
	sprintf((char *)adr, "%p\n", it);
	it = ps.begin();
	sprintf((char *)adr, "%p\n", it);
	it = ps.end();
	sprintf((char *)adr, "%p\n", it);
	it = ps._Myhead;
	sprintf((char *)adr, "%p\n", it);
	free(adr);
*/

	
	try {
		Atalanta *simulation=new Atalanta();
		Params *p[5];

		//res=simulation->run(argc, argv);

		p[0] = new Params();
		bench.open("c17.bench", ios::in);
		pat.open("c17.pat", ios::out);
		p[0]->setBenchStream(bench.rdbuf());
		p[0]->setSPatternStream(pat.rdbuf());
		p[0]->setWTestMode(1);
		p[0]->setCctMode('9');
		p[0]->setIseed(23);
		p[1] = new Params();
		p[1]->setBenchFile("c432.bench");
		p[1]->setSPatternFile("c432.pat");
		p[1]->setWTestMode(1);
		p[1]->setCctMode('9');
		p[1]->setIseed(23);
		p[2] = new Params();
		*p[2] = p[1];
		p[2]->setBenchFile("c6288.bench");
		p[2]->setSPatternFile("c6288.pat");
		p[3] = new Params();
		*p[3] = p[1];
		p[3]->setBenchFile("c7552.bench");
		p[3]->setSPatternFile("c7552.pat");
		p[4] = new Params();
		*p[4] = p[1];
		p[4]->setBenchFile("c880.bench");
		p[4]->setSPatternFile("c880.pat");
		simulation->setParams(p[0]);
		simulation->run();
		simulation->setParams(p[1]);
		simulation->run();
		simulation->setParams(p[2]);
		simulation->run();
		simulation->setParams(p[3]);
		simulation->run();
		simulation->setParams(p[4]);
		simulation->run();
	}
	catch(string s) {
		cerr << s;
	}
	catch(...) {
		cerr << "Uknown exception";
	}
	return res;	
}



