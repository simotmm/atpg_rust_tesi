// Gate.h: interface for the Gate class.
//
//////////////////////////////////////////////////////////////////////

#ifndef __ATALANTA_GATE_H__
#define __ATALANTA_GATE_H__

#include <list>
#include "Global.h"
#include "Defines.h"

using namespace std;

namespace atalantadll {
	enum LineType {LFREE,HEAD,BOUND};
	const level TABLE[Z+1][2]={{ALL1,ALL0},{ALL0,ALL1},{ALL0,ALL0},{ALL1,ALL1}};

	const int conflictTbl[3][5]=
/*	0	     1	      x	     d	    dbar	old     /new	*/
{{	PASS,	FAIL,	PASS1,	FAIL,	PASS	},	/* 0 */
 {	FAIL,	PASS,	PASS1,	PASS,	FAIL	},	/* 1 */
 {	PASS,	PASS,	PASS,	PASS,	PASS	}	/* x */
};

	class HashData;
	class FaultList;
	class Fault;

	struct Eden
	{
		int node;
		char val;

		Eden(int node,char val):node(node),val(val) {};
	};

	struct Learn
	{
		int node;
		char sval;
		char tval;

		Learn(int node,char sval,char tval): node(node),sval(sval),tval(tval) {};

	};

	class Gate  
	{
	public: 
		int index;
		logic fn;				// type of gate
		short ninput;			// number of fanins
		Gate **inlis;			// fan-in list
		short noutput;			// number of fan-outs
		Gate **outlis;			// fan-out list
		short dpi, dpo;			// depth from PIs & POs
		status changed;			// flag for event queue operations
		HashData *symbol;		// pointer to the symbol table
		boolean freach1,        // is reached from faulty gate
			freach;				// is reached from faulty gate
		status xpath;			// x path exists or not 
		LineType ltype;			// head line, free line, bound line 
		int numzero,numone;		// expected number of zeros and ones 
		list<Gate*> uPath;		// unique path
		list<Gate*> uPath1;		// unique path
		int cont0,cont1;		// 0 and 1 controllability
		int fos;				// fanout stem indication 
		//int nfault;
		level cobserve,observe;	// cumulated and local detectabilities 
		list<Fault*> pfault;
		Fault **dfault;

#ifdef LEARNFLG
		list<Learn> pLearn;
#endif
		Gate *next;
#ifdef ISCAS85_NETLIST_MODE
		int gid;				// gate identification
#endif
#ifdef INCLUDE_HOPE
		level GV[2];        // Good value; (V0,V1) */
		level FV[2],gid;    // Faulty Value; (V0,V1,Gid)
		int stem;           // indication of the stems
		status sense;       // simulated or not --- version 3
		level SGV;          // Good Value for SPF
#else
		level output;			// output value for fan
		level output1;			// output value for fsim
#endif

	public:
		Gate() {};

		bool isFree() {return ltype==LFREE;};
		bool isHead() {return ltype==HEAD;};
		bool isBound() {return ltype==BOUND;};
		int  isJustified() {return changed;};
		//#ifndef INCLUDE_HOPE
		int  isUnJustified() {return !changed && output!=X;};
		//#endif
		bool isFanout() {return noutput>1;};
		int  isReachableFormFault() {return freach;};
		bool isCheckPoint() {return (fn>=PI || noutput>1);};

		bool isConflict() {return numzero>0 && numone>0;};

		void setLine(int n0,int n1) {numone=n1;numzero=n0;};

		virtual ~Gate() {};

	};

	char* const dLevelToString[5]={"0","1","x","1","0"};
}
#endif // __ATALANTA_GATE_H__
