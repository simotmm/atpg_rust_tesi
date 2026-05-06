// Fault.h: interface for the Fault class.
//
//////////////////////////////////////////////////////////////////////

#ifndef __ATALANTA_FAULT_H__
#define __ATALANTA_FAULT_H__

#include <list>
#include "ParralelPattern.h"
//#include <fstream>

using namespace std;

namespace atalantadll {
	char* const fault2str[4]={" /0"," /1"," /0"," /1"};
	const int inverseParity[2][2]={{0,1},{1,0}};	
	const int parityOfGate[MAXGTYPE]={0,1,0,1,0,0,0,0,0,0,1,};

	struct Event
	{
		int node;
		level value;
	};

	class Gate;
	class GateNet;

	class Fault  
	{
	public:	
		Gate *gate;			// faulty gate 
		int line;			// faulty line, -1 if output fault 
		fault_type type;	// fault type 
		int detected;		// detected or not 
		level observe;		// detectability 
		int index;          // added by me. Index of the fault. Computed by IndexFaults
#ifdef INCLUDE_HOPE
		int npot;			// # potentially detected --- HOPE
		list<Event*> event;	// event list --- HOPE
#endif

		Fault();

		void printFault(fstream *fp,bool mode, char cctMode);
		void addFault();

		virtual ~Fault();

	};

	class FaultList:public ParralelPattern
	{
	protected:
		Fault **faultList;

		int numberOfFaults;

#ifdef INCLUDE_HOPE
		list<Event*> hopeEventList;
		list<Fault*> hopeFaultList;
		list<Fault*> *oddList,*evenList;

		inline void setParity(Gate *gut,int par);
		inline void mark(Gate *gut);
		inline bool isStem(Gate *gut);
		inline bool isNotMarked(Gate *gut);

		void DFSpo(Gate* parent, Gate* child);
		void FFRfault(Gate* gut);
		void defaultLineFault(Gate* gut,int line);
		void insertFault(Gate* gut,int line,fault_type type);

		void FWDfaults();
		int restoreHopeFaultList();
#endif 

	public:
		FaultList():ParralelPattern() {};

		int restoreDetectedFaultList();
		int checkRedundantFaults();
		int setAllFaultList(int noStem,Gate **stem);
	};

	class ReadableFaultList:public FaultList
	{
	protected:
		istream *inputf;
		char simMode;

		Gate **myStem;
		int myNumberOfStems;

		bool isWhitespace(char c) {return (c==' ' || c=='\r' || c=='-' || c=='\t' || c=='\n');}
		bool isHeadSymbol(char c) {return (c=='/' || c=='>');}
		bool isValid(char c) {return ((c>='0' && c<='9') || (c>='A' && c<='Z') || 
			(c>='a' && c<='z') || (c=='[' || c==']') || (c=='_'));}

		char getFaultSymbol(string *s);

	public:
		ReadableFaultList() {
			simMode = 'f';
		};

		void readFaults(std::streambuf *fn);
		int readFaultsFsim(istream *file,int noStem,Gate **stem);
#ifdef INCLUDE_HOPE
		void readFaultsHope(istream *file);
#endif

	};
}
#endif // __ATALANTA_FAULT_H__
