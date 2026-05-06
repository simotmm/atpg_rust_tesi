// FaultSimulation.h: interface for the FaultSimulation class.
//
//////////////////////////////////////////////////////////////////////

#ifndef __ATALANTA_FAULTSIMULATION_H__
#define __ATALANTA_FAULTSIMULATION_H__

namespace atalantadll {
#define copyLevel(dest,source) dest[0]=source[0]; dest[1]=source[1]
#define cmpLevel(v1,v2) ((v1[0]!=v2[0]) || (v1[1]!=v2[1]))

	const level FAULTYVALUE[SAX+1]={ZERO,ONE,X};
	const level LEVELTOEVENT[4]={1,2,0,3};
	const level PLEVELTBL[Z+1][2]={{ALL1,ALL0},{ALL0,ALL1},{ALL0,ALL0},{ALL1,ALL1}};

	typedef list<Fault*>::iterator FaultIter;

	const int UNSIMULATED=32;
	const int SIMULATED=33;

	struct Sstems
	{
		int stem;
		level val;
	};

	struct SUT
	{
		Gate *gate;			// stem of the faulty gate
		level type;			// value of the stem simulated
		int line;			// faulty line
		list<Fault*> extra;	// extra faults simulated
		list<Event*> event;
		int fn;
		int papa;
		level Val[2];
	};

	class Stem
	{
	public:
		int gate;
		int dominator;
		int checkup;
		Fault* fault[3];
		short flag[3];
	};

	class SHARE_EXPORT FaultSimulation:public ReadableFaultList
	{
	protected:
		int groupID;
		Fault *undfault;		//undetectable fault
		Fault *potentialFault;
		int nDetected;
		int nFut;
		bool dynamicOrderFlag;
		char xdetectmode;
		int nsStems;
		int sStem;
		level sSval;

		int numberOfStems;
		int rstem;
		Stem *stems;
		SUT sut[SIZEOFFUT];
		Fault *fut[SIZEOFFUT];
		Sstems sStems[3000];


		int  setb(int word,int p) { return word | BITMASK[p];};
		int  resetb(int word,int p) {return word & ~BITMASK[p];};
		void setx(int *v0,int *v1) {(*v0)=(*v1)=ALL0;};
		bool bitb(int word,int p) {return (word & BITMASK[p])==ALL0;};
		int whatIs(int v0,int v1)
		{
			return (v0==ALL1 && v1==ALL0) ? ZERO :
				(v0==ALL0 && v1==ALL1) ? ONE  :
				(v0==ALL0 && v1==ALL0) ? X : Z;
		}

		Gate*	checkSingleEvent(Fault *f,Gate *gut,int gid);
		Gate*	checkStem(Gate* gut,int gid);
		int		dropDetectedFaults();
		void	faultSim(int start,int stop,int gid);
		void	faultyGateEval(Gate *gut,level *val);
		void	feval(Gate *gut,int *val,int *v,int ggid);
		Gate*	injectFault(Gate *gate,int ftype,int fline,int bit);
		FaultIter selectNextFaults(FaultIter current);
		FaultIter selectOneFault(FaultIter current);
		Gate*	sSimToDominator(Gate* gut,Gate *dom,int gid);
		void	storeFaultyStatus();

		int setFFR();
		int setDominator();

	public:
		FaultSimulation():groupID(0),numberOfStems(0),xdetectmode('n'),nsStems(-1),sStem(-1),rstem(0) {};

		void initFaultSim();
		int simulation();

		virtual ~FaultSimulation() { };

	};
}
#endif // __ATALANTA_FAULTSIMULATION_H__
