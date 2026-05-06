// ResultsReader.h: interface for the ResultsReader class.
//
//////////////////////////////////////////////////////////////////////

#ifndef __ATALANTA_RESULTSREADER_H__
#define __ATALANTA_RESULTSREADER_H__

using namespace std;

namespace atalantadll {
	const int TESTPERLINE=13;

	class ResultsReader  
	{
		//Gate **net;	/////////////////////////initialize somewhere
		GateNet *net;

		bool checkBit(level word,int n) {return (word & (1<<n)) != 0;};

		void addTestVector( string * ivct, string * ovct, int no );
		/*void print_gate(register GATEPTR gut);
		void sprint_gate(register GATEPTR gut);


		void printgatename(FILE *fp, register GATEPTR gate, char wmode);
		void printionames(FILE *fp, int *array, register int n, char *head, register char wmode, register char iomode);
		void printgate(FILE *fp, register GATEPTR gate, char wmode);
		level logiclevel(register level V0, register level V1, register int n);
		void printgatevalues(FILE *fp, register GATEPTR gut, int n, char gmode);
		void print_net(FILE *fp, char *name, char wmode);
		int count_events(register FAULTPTR f);
		void print_event(FILE *file, FAULTPTR f, char mode);
		void print_event_tree(FILE *file, EVENTPTR event, char mode);
		void DFSWalk(GATEPTR child, char iomode);
		int FindUnobservableGates(FILE *fp, status wflag, status iomode);
		*/


	public:
		ResultsReader();

		int pgetTest(std::fstream *fp,level *input,int npi,int nbit);

		virtual ~ResultsReader();

	};
}
#endif // __ATALANTA_RESULTSREADER_H__
