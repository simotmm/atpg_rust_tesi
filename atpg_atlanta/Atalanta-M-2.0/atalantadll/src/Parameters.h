#ifndef __ATALANTA_PARAM_H__
#define __ATALANTA_PARAM_H__

namespace atalantadll {
	//const int MAXGATE=300000;	//number of gate
	//const int MAXPI=20000;		//number of primary inputs
	//const int MAXPO=20050;		// number of primary outputs 
	//const int MAXFAULT=400000;   // number of faults 
	//const int MAXFIN=15;	    // number of fanin lines 
	//const int MAXFOUNT=15;	 	// maximum number of fanout lines 

	//const int MAXSTRING=200 ;   // maximum size of a string 
	const int HASHSIZE=19999;   // symbol table size, prime 
	//const int SPAREGATES=100;   // should be larger than SIZE_OF_FUC*2
	const int MAXINTEGER=99999999;
	const int INFINITY=99999999;

	// For ISCAS85 circuits
	const int MAXLINE=12000;	// size of a line

	// For ATALANTA only
	//const int MAXBUF=50000;		// number of buffers for backtrace
	//const int MAXOBJ=10000;
	//const int MAXTREE=40000;
	const int MAXTEST=10000;
	//const int SHUFFLES=30;
	const int TWO=2;
	const int STOP=-1;

	//const int LEARNFLG=1;

	// For HOPE only */
	const int SIZEOFFUT=32;     // number of faults under test in one pass
	const int WORDSIZE=32;	      // number of bits in a word

	// error messages */
	const int NUMERRORS=10;      // number of error message
	const int NOERROR=0;
	const int EOFERROR=1;
	const int CIRCUITERROR=2;
	const int MEMORYERROR=3;
	const int HASHERROR=4;
	const int FAULTERROR=5;

}
#endif // __ATALANTA_PARAM_H__
