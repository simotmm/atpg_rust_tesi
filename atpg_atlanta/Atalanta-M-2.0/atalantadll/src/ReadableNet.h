// ReadableNet.h: interface for the ReadableNet class.
//
//////////////////////////////////////////////////////////////////////

#ifndef __ATALANTA_READABLENET_H__
#define __ATALANTA_READABLENET_H__

#include "Gate.h"
#include "FaultSimulation.h"
#include <iostream>
#include <fstream>

namespace atalantadll {
	class SHARE_EXPORT ReadableNet :public FaultSimulation  
	{
	protected:
		fstream *circuit2;
		//	fstream circuit;
		istream circuit;
		int levels;
		char cctMode;
		//	Hash	hashTable;

		int maxFout;

		char getSymbol(string *s);
		int gateType(string *symbol);

	public:
		ReadableNet():maxFout(0),levels(0), circuit(NULL) {
			cctMode = ISCAS89;
		};

#ifdef INCLUDE_HOPE
		int computeLevel();
		void addSpareGate();
		int levelize();	

#else
		int levelize(int n,Gate **stack);
#endif

		int addPO();


		int readCircuit();

#ifdef ISCAS85_NETLIST_MODE
		bool circIn(fstream *circuit);
#endif

//		SHARE_EXPORT int readBenchFile(string benchFile);
		virtual ~ReadableNet() {};

		int setBenchStream(streambuf *buf);
	};
}
#endif // __ATALANTA_READABLENET_H__
