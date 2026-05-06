// FanNet.h: interface for the FanNet class.
//
//////////////////////////////////////////////////////////////////////

#ifndef __ATALANTA_FANNET_H__
#define __ATALANTA_FANNET_H__

#include "GateNet.h"


namespace atalantadll {
	const int SELECTMODE=0;	// 0: easiest first, 1: hardest first

	struct TreeNode
	{
		Gate* gate;		// pointer to the gate 
		bool flag;		// flag for backtracking
		int pstack;			// pointer for implication 

	public:	
		TreeNode(Gate *gate,bool flag):gate(gate),flag(flag) {pstack=0;};

		bool isFlagged() {return flag;};
	};

	class FanNet:public GateNet
	{
	protected:
		Stack unjustified,initObj,currObj,fanObj,finalObj,dFrontier;
		Stack *headObj;
		list<TreeNode> tree;
		vector <level> inVal;//level inVal[MAXPI];	
		int dyID;
		int nTestEach;
		int nTestEachLimit;
		int maxlevel;
		char noFaultSim;
		char learnMode;
		char genAllPat;

		Stack** eventList;

		void scheduleOutput(Gate *gate) {
			for(int i=0;i<gate->noutput;i++) pushEvent(gate->outlis[i]);
		}
		void scheduleInput(Gate *gate,int i) 
		{
			pushEvent(gate->inlis[i]);
			scheduleOutput(gate->inlis[i]);
		}
		void pushGate(Gate *gut) {eventList[gut->dpi]->push(gut);};

		status	backTrace(status state);
		bool	backTrack(Gate *faultyGate, int *last);
		Gate*	closestPO(Stack* objective,int *pclose);
		void	dScheduleOutput(Gate *gut,int *dFrontier);
		int		dynamicUniqueSensitize(Stack *dominators,int maxdpi,Gate* faultyGate);
		status	eval(Gate* gate,Fault *cf);
		void	faultyLineIsFree(Fault* cf);
		level	faultyGateEval(Gate *g,Fault *cf);
		void	findFinalOjective(bool *backtrace,bool faultPropgateToPo,Gate** lastDFrontier);
		void	gateEval1(Gate *gate,level *val,logic *f);
		void	initNet(Gate* faultyGate,int maxDpi);
		bool	imply(int maxDpi,bool backward,int last,Fault *cf);
		void	justifyFreeLines(Fault *of,Fault* cf);
		void	restoreFaults(Fault *fal);
		Gate*	selectHardest(Stack* objective,int *pclose);
		int		setFaultyGate(Fault* fault);
		void	updateDFrontier();
		int		uniqueSensitize(Gate* gate,Gate* faultyGate);
		bool	xPath(Gate *gate);
		status implyLearn(Gate *gut, level val);
		status implyLearn1(Gate *gut, level val);

		int aNot(int var)
		{
			return  (var==ONE)  ? ZERO : 
				(var==ZERO) ? ONE  :
				(var==D)	? DBAR :
				(var==DBAR) ? D:
				X;
		}

		void pushEvent(Gate* gate)
		{
			if(!gate->changed)
			{
				eventList[gate->dpi]->push(gate);
#ifdef _ALG_DEBUG
				dbgFile << "pushevent index: " << gate->index << "\n";
#endif
				gate->changed=true;
			}
		}

		void scheduleGate(Gate* gut);

	public:
		FanNet():unjustified(1000),initObj(1000),currObj(1000),fanObj(1000),finalObj(1000),
			dFrontier(1000),dyID(INFINITY), noFaultSim('n'), learnMode('n'), genAllPat('n') {
			unjustified.push(0);
		};

		void allocateEventList();

#ifdef INCLUDE_HOPE 
		void goodSim(int ntest);
#endif

		void setEachLimit(int limit) {nTestEachLimit=limit;};

		void getTime(double *userTime,double *systemTime,double *total);
		void setFanoutStemp(Gate** stem,int nstem);
		void setUniquePath(int maxDpi);
		int setDominator(int maxDpi);
		int setCctParameters();

		status fan(int maxdpi,Fault* cf, int maxbacktrack, int *nbacktrack);
		status fan1(int maxdpi,Fault* cf,int maxbacktrack, int *nbacktrack);

		virtual ~FanNet() { };

	};
}
#endif // __ATALANTA_FANNET_H__
