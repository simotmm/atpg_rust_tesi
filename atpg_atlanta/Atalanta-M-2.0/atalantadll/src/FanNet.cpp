// FanNet.cpp: implementation of the FanNet class.
//
//////////////////////////////////////////////////////////////////////
#include <list>
#include <fstream>

#include "Error.h"

#include "Defines.h"
#include "Truthtable.h"
#include "Parameters.h"

#include "Hash.h"

#include "Gate.h"
#include "Stack.h"
#include "GateNet.h"

#include <memory>
#include "Globals.h"

#include "FanNet.h"
#include "ParralelPattern.h"
#include "Fault.h"

using namespace std;

namespace atalantadll {
	//////////////////////////////////////////////////////////////////////
	// Construction/Destruction
	//////////////////////////////////////////////////////////////////////
	void FanNet::scheduleGate(Gate* gut)
	{
		Gate *temp;
		for(int ti=0;ti<gut->noutput;ti++)
		{
			temp=gut->outlis[ti];
			if(!temp->changed)
			{
				temp->changed=true;
				eventList[temp->dpi]->push(temp);
#ifdef _ALG_DEBUG
				dbgFile << "scheduleGate event push index:" << temp->index << "\n";
#endif
			}
		}
	}

	void FanNet::allocateEventList(void)
	{
		int i;
		int *levelPopulation=new int[maxlevel+2];
		memset(levelPopulation,0,sizeof(int)*(maxlevel+2));

		for(i=0;i<numberOfGates;i++) levelPopulation[net[i]->dpi]++;

		eventList=new Stack *[maxlevel+2];
		for(i=0;i<maxlevel+2;i++) eventList[i]=new Stack(levelPopulation[i]);

		delete levelPopulation;
	}

#ifdef INCLUDE_HOPE
	void FanNet::goodSim(int ntest)
	{
		int i;
		Gate *gut;
		level val;

		/* schedule events in flip-flops 
		--- FFs are modeled as Delay Elements */
		if(ntest==1)
		{
			for(i=0;i<numberOfFlipFlops;i++)
			{
				gut=net[flipFlops[i]];
				switch(initialMode)
				{
				case '0': val=0; break;
				case '1': val=1; break;
				default: val=X; break;
				}
				if(gut->SGV != val)
				{
					gut->SGV=val;
					gut->GV[0]=TABLE[val][0];
					gut->GV[1]=TABLE[val][1];
					scheduleGate(gut); //////////
				}
			}
		} else 
			for(i=0;i<numberOfFlipFlops;i++)
			{
				gut=net[primaryIn[i]];
				val=gut->inlis[0]->SGV;
				if(gut->SGV,val)
				{
					gut->SGV=val;
					gut->GV[0]=TABLE[val][0];
					gut->GV[1]=TABLE[val][1];
					scheduleGate(gut); /////////
				}
			}

			// schedule event in primary inputs 
			for(i=0;i<numberOfPrimaryInputs;i++)
			{
				gut=net[primaryIn[i]];
				if(gut->SGV!=inVal[i])
				{
					gut->SGV=inVal[i];
					gut->GV[0]=TABLE[inVal[i]][0];
					gut->GV[1]=TABLE[inVal[i]][1];
					scheduleGate(gut);
				}
			}

			for(i=0;i<PPOlevel;i++)
			{
				while(!eventList[i]->isEmpty())
				{
					gut=eventList[i]->pop();
					gut->changed=false;

					if(gut->ninput==1)
					{
						gut->SGV=val=truthtbl1[gut->fn][gut->inlis[0]->SGV];
						gut->GV[0]=TABLE[val][0];
						gut->GV[1]=TABLE[val][1];
						scheduleGate(gut);
					} else
					{
						val=truthtbl1[gut->fn][gut->inlis[0]->SGV];
						for(int j=1;j<gut->ninput;j++)
							val=truthtbl2[gut->fn][val][gut->inlis[j]->SGV];
						if(gut->SGV!=val)
						{
							gut->SGV=val;
							gut->GV[0]=TABLE[val][0];
							gut->GV[1]=TABLE[val][1];
							scheduleGate(gut);
						}
					}
				}
			}

			//Pseudo-Primary outputs
			while(!eventList[PPOlevel]->isEmpty())
			{
				gut=eventList[PPOlevel]->pop();
				gut->changed=false;
			}
	}
#endif


#ifdef INCLUDE_HOPE
	int FanNet::setCctParameters()
	{
		int i,j;
		int headCount=0;

		// define line type (free,head,bound) and distance from input
		if(genAllPat=='y')
		{
			for(i=0;i<numberOfGates;i++)
				net[i]->ltype= (net[i]->fn==PI) ? HEAD : BOUND;
			headCount=numberOfPrimaryInputs;
		} else
			for(i=0;i<numberOfGates;i++)
			{
				net[i]->ltype=LFREE;
				if(net[i]->fn != PI)
					for(j=0;j<net[i]->ninput;j++)
						if(!net[i]->inlis[j]->isFree()) net[i]->ltype=BOUND;
				if(net[i]->isFree() && net[i]->noutput != 1) net[i]->ltype=HEAD;
				if(net[i]->isHead()) headCount++;
				if(net[i]->isBound())
					for(j=0;j<net[i]->ninput;j++)
						if(net[i]->inlis[j]->isFree())
						{
							net[i]->inlis[j]->ltype=HEAD;
							headCount++;
						}

			}

			for(i=0;i<numberOfPrimaryInputs;i++) headlines[i]=-1;
			j=headCount;
			for(i=numberOfGates-1;i>=0;i--) {
				if(net[i]->isHead()) 
					headlines[--j]=i;
			}


			// alloacate space for sets (needed for the fan algorithm)
			headObj=new Stack(headCount);	
			if(!stack) stack=new Stack(numberOfGates);	

			//////return(maxLevel);
			return 0;
	}
#else
	int FanNet::setCctParameters()
	{
		int i,j,depth;
		int maxDpi=0;
		int headCount=0;

		// define line type (free,head,bound) and distance from input
		for(i=0;i<numberOfGates;i++)
		{
			net[i]->ltype=LFREE;
			depth=-1;
			if(net[i]->fn != PI)
				for(j=0;j<net[i]->ninput;j++)
				{
					if(!net[i]->inlis[j]->isFree()) net[i]->ltype=BOUND;
					depth=MAX(net[i]->inlis[j]->dpi,depth);
				}
				if(net[i]->isFree() && net[i]->noutput!=1) net[i]->ltype=HEAD;
				net[i]->dpi=depth+1;
				if(net[i]->isHead()) headCount++;
				if(net[i]->isBound())
					for(j=0;j<net[i]->ninput;j++)
						if(net[i]->inlis[j]->isFree())
						{
							net[i]->inlis[j]->ltype=HEAD;
							headCount++;
						}
						maxDpi=MAX(net[i]->dpi,maxDpi);
		}

		// allocate memory for event_listt and reset event counter
		maxDpi++;
		eventList=new Stack*[maxDpi];
		int *depths=new int[maxDpi];
		memset(depths,0,maxDpi*sizeof(int));

		for(i=0;numberOfPrimaryInputs;i++) headlines[i]=-1;
		j=headCount;

		for(i=numberOfGates-1;i>0;i--)
		{
			if(net[i]->isHead()) headlines[--j]=i;
			// count the number of gates in each depth
			depths[net[i]->dpi]++;
		}

		// allocate space for each event list
		for(i=0;i<maxDpi;i++) eventList[i]=new Stack(depths[i]);

		// alloacate space for sets (needed for the fan algorithm)
		headObj=new Stack(headCount);	
		if(!stack) stack=new Stack(numberOfGates);	

		//////////ALLOCATE(tree.list,TREETYPE,MAXTREE);

		delete depths;

		return maxDpi;

	}
#endif

	void FanNet::dScheduleOutput(Gate *gut,int *dFrontier)
	{
		Gate *tempGate;
		for(int i=0;i<gut->noutput;i++)
		{
			tempGate=gut->outlis[i];
			if(!tempGate->changed)
			{
				eventList[tempGate->dpi]->push(tempGate);
#ifdef _ALG_DEBUG
				dbgFile << "dscheduleoutput index:" << tempGate->index << "\n";
#endif
				(*dFrontier)++;
				tempGate->changed=true;
			}
		}
	}

	int FanNet::setDominator(int maxDpi)
	{
		int i,j;
		Gate *gut,*g;
		Gate *dominator;
		int gateCount;
		int nDominator=0;

		for(i=numberOfGates-1;i>=0;i--)
		{
			gut=net[i];
			if(gut->noutput<=1) {
				gut->uPath.clear();
			}
			else
			{
				gateCount=0;
				dScheduleOutput(gut,&gateCount);
				for(j=gut->dpi+1;j<maxDpi;j++)
					while(!eventList[j]->isEmpty())
					{
						dominator=eventList[j]->pop();
						dominator->changed=false;
						if(gateCount<=0) continue;
						gateCount--;
						if(gateCount==0)
						{
							gut->uPath.push_back(dominator);
							nDominator++;
							break;
						}
						if(dominator->noutput==0)
						{
							gut->uPath.clear();
							gateCount=0;
						}
						else if(dominator->noutput>1)
						{
							if(dominator->uPath.size()==0)
							{
								gut->uPath.clear();
								gateCount=0;
							} else
							{
								g=dominator->uPath.front();
								if(!g->changed)
								{
									eventList[g->dpi]->push(g);
#ifdef _ALG_DEBUG
				dbgFile << "setdominator event push index:" << g->index << "\n";
#endif
									g->changed=true;
									gateCount++;
								}
							}	
						}
						else if(!dominator->outlis[0]->changed)
						{
							g=dominator->outlis[0];
							eventList[g->dpi]->push(g);
#ifdef _ALG_DEBUG
				dbgFile << "setdominator(1) event push index:" << g->index << "\n";
#endif
							g->changed=true;
							gateCount++;
						}
					}
			}
		}
		return nDominator;
	}

	void FanNet::setUniquePath(int maxDpi)
	{
		int i,j,k;
		Gate *gut,*g;
		int count=0;

		for(i=numberOfGates-1;i>=0;i--)
		{
			gut=net[i];
			if(gut->uPath.size()==0) continue;
			count=0;
			gut->freach=i;

			for(j=0;j<gut->noutput;j++)
			{
				g=gut->outlis[j];
				if(!g->changed)
				{
					eventList[g->dpi]->push(g);
#ifdef _ALG_DEBUG
				dbgFile << "setuniquepath event push index:" << g->index << "\n";
#endif
					g->changed=true;
					count++;
				}
			}
			for(j=gut->dpi+1;j<maxDpi;j++)
				while(!eventList[j]->isEmpty())
				{
					gut=eventList[j]->pop();
					gut->changed=false;
					gut->freach=i;
					count--;
					if(count==0)
					{
						for(k=0;k<gut->ninput;k++)
						{
							g=gut->inlis[k];
							if(g->freach==i) net[i]->uPath.push_back(g);
						}
						break;
					}
					for(k=0;k<gut->noutput;k++)
					{
						g=gut->outlis[k];
						if(!g->changed)
						{
							eventList[g->dpi]->push(g);
#ifdef _ALG_DEBUG
				dbgFile << "setuniquepath(1) event push index:" << g->index << "\n";
#endif
							g->changed=true;
							count++;
						}
					}
				}
		}
	}

	void FanNet::setFanoutStemp(Gate **stem,int nstem)
	{
		int i,j;
		Gate* p;

		j=0;
		for(i=0;i<numberOfGates;i++)
			if(net[i]->noutput != 1) stem[j++]=net[i];
		stack->clear();
		for(i=0;i<nstem;i++)
		{
			stack->push(stem[i]);
			while(!stack->isEmpty())
			{
				p=stack->pop();
				p->fos=stem[i]->index;
				for(j=0;j<p->ninput;j++)
					if(p->inlis[j]->noutput==1) stack->push(p->inlis[j]);
			}
		}
	}

	void FanNet::getTime(double *userTime,double *systemTime,double *total)
	{
		/*struct tms timesbuffer;
		time_t totaltime;
		time_t utime;
		time_t stime;

		times(&timesbuffer);
		utime=timesbuffer.tms_utime;
		stime=timesbuffer.tms_stime;
		totaltime=timesbuffer.tms_utime+timesbuffer.tms_stime; // In 60th seconds
		*usertime=(double)utime/60.0;
		*systemtime=(double)stime/60.0;
		*total=(double)totaltime/60.0;*/
		*userTime=0;
		*systemTime=0;
		*total=0;  
	}

	void FanNet::initNet(Gate *faultyGate,int maxDpi)
	{
		int i,j;
		Gate* p;

		// clear changed ochange and set freach
		for(i=0;i<numberOfGates;i++)
		{
			if(net[i]->isFree()) 
				net[i]->changed=true;
			else
				net[i]->changed=false;
			net[i]->freach=false;
			net[i]->output=X;
			net[i]->xpath=1;
		}

		// clear all sets
		for(i=0;i<maxDpi;i++) eventList[i]->clear();
		dFrontier.clear();
		unjustified.clear();
		initObj.clear();
		currObj.clear();
		fanObj.clear();
		headObj->clear();
		finalObj.clear();
		stack->clear();
		tree.clear();

		// flag all reachable gates from the faulty gate
		pushEvent(faultyGate);
		for(i=faultyGate->dpi;i<maxDpi;i++)
			while(!eventList[i]->isEmpty())
			{
				p=eventList[i]->pop();
				p->changed=false;
				p->freach=true;
				for(j=0;j<p->noutput;j++) pushEvent(p->outlis[j]);
			}
	}

	void FanNet::faultyLineIsFree(Fault *cf)
	{
		int i;
		level value;
		Gate *p;

		p=cf->gate;
		if(cf->line!=OUTFAULT) p=p->inlis[cf->line];
		p->output= cf->type==SA0 ? D : DBAR;
		stack->push(p->outlis[0]);
		while(!stack->isEmpty())
		{
			p=stack->pop();
			value=(p->fn==AND || p->fn==NAND) ? ONE : ZERO;
			for(i=0;i<p->ninput;i++)
				if(p->inlis[i]->output==X)
					p->inlis[i]->output=value;
				else
					p->output=a_truthtbl1[p->fn][p->inlis[i]->output];
			if(p->isFree()) stack->push(p->outlis[0]);
		}

		p->changed=true;
		stack->push(p);
		scheduleOutput(p);
		cf->gate=p;
		cf->line=OUTFAULT;
		cf->type= (p->output==D) ? SA0 : SA1;   
	}


	int FanNet::setFaultyGate(Fault *fault)
	{
		int i,last=0;
		Gate *p,*faultyLine;
		level v1,v2;

		p=fault->gate;
		faultyLine= fault->line==OUTFAULT ? p : p->inlis[fault->line];
		v2= fault->type==SA0 ? D : DBAR;
		// input stuck-at faults
		if(fault->line>=0)
		{
			//input line fault
			faultyLine->output= v2==D ? ONE : ZERO; //faulty line;
			stack->push(faultyLine);
			switch(p->fn)
			{
			case AND:
			case NAND:
				p->changed=true;
				for(i=0;i<p->ninput;i++)
					if(i!=fault->line)
						if(p->inlis[i]->output==X)
						{
							p->inlis[i]->output=ONE;
							stack->push(p->inlis[i]);
						}
						else if(p->inlis[i]->output != ONE) return -1;
						p->output= p->fn==NAND ? aNot(v2) : v2;
						stack->push(p);
						break;
			case OR:
			case NOR:
				p->changed=true;
				for(i=0;i<p->ninput;i++)
					if(i!=fault->line)
						if(p->inlis[i]->output==X)
						{
							p->inlis[i]->output=ZERO;
							stack->push(p->inlis[i]);
						}
						else if(p->inlis[i]->output != ZERO) return -1;
						p->output=p->fn==NOR ? aNot(v2) : v2;
						stack->push(p);
						break;
			case NOT:
				p->output=aNot(v2);
				stack->push(p);
				break;
			case BUFF:
			case PO:
				p->output=v2;
				stack->push(p);
				break;
			}

			//schedule events
			if(p->output!=X) scheduleOutput(p);
			for(i=0;i<p->ninput;i++)
				if(p->inlis[i]->output!=X)
				{
					last=MAX(p->inlis[i]->dpi,last);
					scheduleInput(p,i);
				}

		} else
		{
			// output line fault
			p->output=v2;
			stack->push(p);
			scheduleOutput(p);

			if(p->isHead()) 
				p->changed=true;
			else
				if(p->ninput==1)
				{
					p->changed=true;
					v1= v2==D ? ONE : ZERO;
					p->inlis[0]->output=a_truthtbl1[p->fn][v1];
					stack->push(p->inlis[0]);
					scheduleInput(p,0);
					last=p->inlis[0]->dpi;
				}
				else if( (v2==D &&    (p->fn==AND  || p->fn==NOR)) ||
					(v2==DBAR && (p->fn==NAND || p->fn==OR )))
				{
					p->changed=true;
					v1=(p->fn==AND || p->fn==NAND) ? ONE : ZERO;
					for(i=0;i<p->ninput;i++)
						if(p->inlis[i]->output==X)
						{
							p->inlis[i]->output=v1;
							last=MAX(p->inlis[i]->dpi,last);
							stack->push(p->inlis[i]);
							scheduleInput(p,i);
						}
				} else
				{
					pushEvent(p);
					last=p->dpi;
				}
		}

		return last;
	}

	int FanNet::uniqueSensitize(Gate* gate,Gate* faultyGate)
	{
		int i;
		Gate *curr,*next;
		int last=-1; // the largest depth of sensitized lines
		bool flag;
		level v1;

		// sensitize the current gate */
		if(gate!=faultyGate)
		{
			v1= (gate->fn==AND || gate->fn==NAND) ? ONE : (gate->fn==OR || gate->fn==NOR) ? ZERO : X;
			if(v1!=X)
				for(i=0;i<gate->ninput;i++)
					if(gate->inlis[i]->output==X)
					{
						gate->inlis[i]->output=v1;
						last=MAX(gate->inlis[i]->dpi,last);
						stack->push(gate->inlis[i]);
						scheduleInput(gate,i);
					}
		}

		// sensitize next path
		curr=gate;
		while(curr!=0)
		{
			if(curr->noutput==0) break;
			else if(curr->noutput==1) next=curr->outlis[0];
			else if(curr->uPath.size()==0) break;
			else next=curr->uPath.front();

			v1=(next->fn==AND || next->fn==NAND) ? ONE : (next->fn==OR || next->fn==NOR) ? ZERO : X;
			if(v1!=X)
			{
				if(curr->noutput==1)
				{
					//one fanout
					for(i=0;i<next->ninput;i++)
						if(next->inlis[i]!=curr && next->inlis[i]->output==X)
						{
							next->inlis[i]->output=v1;
							last=MAX(next->inlis[i]->dpi,last);
							stack->push(next->inlis[i]);
							scheduleInput(next,i);
						}
				} else
				{
					//multiple fanout
					for(i=0;i<next->ninput;i++)
						if(next->inlis[i]->output==X)
						{
							flag=true;

							list<Gate*>::iterator current,final;
							current=curr->uPath.begin();
							final=curr->uPath.end();
							current++;

							while(current!=final)
							{
								if(next->inlis[i]==*current)
								{
									flag=false;
									break;
								}
								current++;
							}

							if(flag)
							{
								next->inlis[i]->output=v1;
								last=MAX(next->inlis[i]->dpi,last);
								stack->push(next->inlis[i]);
								scheduleInput(next,i);
							}
						}
				}
			}
			curr=next;
		}
		return last;
	}

	level FanNet::faultyGateEval(Gate *g,Fault *cf)
	{
		int i,j;
		level val;
		logic f;

		if(g->ninput==0) return g->output;

		if(cf->line==OUTFAULT)
		{
			j=0;
			val=g->inlis[0]->output;
		} else
		{
			j=cf->line;
			val=g->inlis[j]->output;
			if(val==ZERO && cf->type==SA1) val=DBAR;
			else if(val==ONE && cf->type==SA0) val=D;
		}

		if(g->ninput==1) 
			val=a_truthtbl1[g->fn][val];
		else
		{
			f=(g->fn==NAND) ? AND : (g->fn==NOR) ? OR : g->fn;		
			for(i=0;i<j;i++)
				val=a_truthtbl2[f][val][g->inlis[i]->output];
			for(++i;i<g->ninput;i++)
				val=a_truthtbl2[f][val][g->inlis[i]->output];
			if(g->fn==NAND||g->fn==NOR) val=aNot(val);
		}


		if(cf->line==OUTFAULT)
		{
			if(val==ZERO && cf->type==SA1) val=DBAR;
			else if(val==ONE && cf->type==SA0) val=D;
		}

		return val;
	}

	void FanNet::gateEval1(Gate *gate,level *val,logic *f)
	{
		if(gate->ninput==1) 
			*val=a_truthtbl1[gate->fn][gate->inlis[0]->output];
		else if(gate->ninput==2)
			*val=a_truthtbl2[gate->fn][gate->inlis[0]->output][gate->inlis[1]->output];
		else
		{
			*f=(gate->fn==NAND) ? AND : (gate->fn==NOR) ? OR : gate->fn;
			*val=a_truthtbl2[*f][gate->inlis[0]->output][gate->inlis[1]->output];
			for(int i=2;i<gate->ninput;i++)
				*val=a_truthtbl2[*f][*val][gate->inlis[i]->output];
			if(gate->fn==NAND || gate->fn==NOR) *val=aNot(*val);
		}
	}

	status FanNet::eval(Gate* gate,Fault *cf)
	{
		int i,j;
		level val,v1;
		int numX;
		logic f;
		Gate **p;

		gate->changed=false;
		p=gate->inlis;
#ifdef _ALG_DEBUG
		dbgFile << "eval begin, index:" << gate->index << "\n";
#endif

		// if a line is a head line, stop 
		if(gate->isHead()) {gate->changed=true; return FORWARD;};

		// faulty gate evaluation 
		if(gate==cf->gate)
		{
			val=faultyGateEval(gate,cf);
			if(val==X)
			{
				if(gate->output!=X)
				{
					for(i=numX=0;i<gate->ninput;i++)
						if(p[i]->output==X) { numX++; j=i; }
						if(numX==1)
						{	// backward implication
							val=(gate->output==D) ? ONE : (gate->output==DBAR) ? ZERO : gate->output;
							val=a_truthtbl1[gate->fn][val];
							switch(gate->fn)
							{
							case XOR:
							case XNOR:
								v1 = (j==0) ? p[1]->output : p[0]->output;
								if(v1==ONE) val=a_truthtbl1[NOT][val];
							}
							p[j]->output=val;
							gate->changed=true;
							stack->push(p[j]);
							scheduleInput(gate,j);
							return BACKWARD;
						}
						else {
							unjustified.push(gate);
#ifdef _ALG_DEBUG
							dbgFile << "eval unustified push gate:" << gate->index << "\n";
#endif
						}
				}
			}
			else if(val==gate->output) gate->changed=true;
			else if(gate->output==X)
			{	// forward imp
				gate->changed=true;
				gate->output=val;
				stack->push(gate);
				scheduleOutput(gate);
			}
			else return CONFLICT;
			return FORWARD;
		}

		// fault free gate evaluation
		gateEval1(gate,&val,&f);

		if(val==gate->output)
		{		// no event
			if(val!=X) gate->changed=true;
			return FORWARD;
		}

		if(gate->output==X)
		{		// forward implication 
			gate->output=val;
			stack->push(gate);
			gate->changed=true;
			scheduleOutput(gate);
			return FORWARD;
		}

		if(val!=X) return CONFLICT;		// conflict 

		// backward implication 
		switch(gate->fn)
		{
		case AND:
		case NAND:
		case OR:
		case NOR:
			v1 = (gate->fn==AND || gate->fn==NOR) ? ONE : ZERO;
			if(gate->output==v1)
			{
				gate->changed=true;
				for(i=0;i<gate->ninput;i++)
					if(p[i]->output==X)
					{
						p[i]->output=a_truthtbl1[gate->fn][v1];
						stack->push(p[i]);
						scheduleInput(gate,i);
					}
					i=BACKWARD;
			} else
			{
				for(i=numX=0; i<gate->ninput; i++) {
#ifdef _ALG_DEBUG
					dbgFile << "eval p[" << i << "] - gate:" << p[i]->index << ", output:" << p[i]->output << "\n";
#endif
					if(p[i]->output==X) { numX++; j=i;}
				}
					if(numX==1)
					{
						p[j]->output = a_truthtbl1[gate->fn][gate->output];
						gate->changed=true;
						stack->push(p[j]);
						scheduleInput(gate,j);
						i=BACKWARD;
					} else
					{
						unjustified.push(gate);
#ifdef _ALG_DEBUG
							dbgFile << "eval unustified push gate(1):" << gate->index << "\n";
#endif
						i=FORWARD;
					}
			}
			break;
		case BUFF:
		case NOT:
		case PO:
			p[0]->output=a_truthtbl1[gate->fn][gate->output];
			gate->changed=true;
			stack->push(p[0]);
			scheduleInput(gate,0);
			i=BACKWARD;
			break;
		case XOR:
		case XNOR:
			for(i=numX=0;i<gate->ninput;i++)
				if(p[i]->output==X) { numX++; j=i;}
				if(numX==1)
				{
					v1=(j==0) ? p[1]->output : p[0]->output;
					val=a_truthtbl1[gate->fn][gate->output];
					if(v1==ONE) val=a_truthtbl1[NOT][val];
					p[j]->output=val;
					gate->changed=true;
					stack->push(p[j]);
					scheduleInput(gate,j);
					i=BACKWARD;
				} else
				{
					unjustified.push(gate);
#ifdef _ALG_DEBUG
							dbgFile << "eval unustified push gate(2):" << gate->index << "\n";
#endif
					i=FORWARD;
				}
				break;
		}

#ifdef LEARNFLG 
		if(learnMode=='y' && !gate->pLearn.empty())
			if(gate->output == ZERO || gate->output==ONE)
				switch(implyLearn(gate,gate->output)) {
					case BACKWARD: i=BACKWARD; break;
					case CONFLICT: i=CONFLICT; break;
				}
#endif
				return i;
	}

	bool FanNet::imply(int maxDpi,bool backward,int last,Fault *cf)
	{
		int i,start;
		status st;
		Gate *g;

		if(backward)
			start=last;
		else
			start=0;

		while(true)
		{
			// backward implication
			if(backward) {
				for(i=start;i>=0;i--) {
#ifdef _ALG_DEBUG
					dbgFile << "imply backward i:" << i << ", eventlist:" << eventList[i]->getCount() << "\n";
#endif
					while(!eventList[i]->isEmpty())
					{
						g=eventList[i]->pop();
						if((st=eval(g,cf))==CONFLICT) 
							return false;
					}
				}
		  }
					// forward implication
					backward=false;

					for(i=0;i<maxDpi;i++)
					{
						while(!eventList[i]->isEmpty())
						{
							if((st=eval(eventList[i]->pop(),cf))==CONFLICT) 
								return false;
							else if(st==BACKWARD)
							{
								start=i-1;
								backward=true;
								break;
							}
						}
						if(backward) break;
					}
					if(!backward) break;
		}
		return true;
	}

	void FanNet::updateDFrontier()
	{
		int i,j;
		Gate *g;
		int first;

		first=INFINITY;
		for(i=0;i<=dFrontier.getCount()-1;)
		{
			g=dFrontier[i];
			switch(g->output)
			{
			case D:
			case DBAR:
				for(j=0;j<g->noutput;j++) dFrontier.push(g->outlis[j]);
				dFrontier.deleteItem(i);
				break;
			case X:
				if(g->index<first) first=g->index;
				i++;
				break;
			default:			// 1 or 0
				dFrontier.deleteItem(i);
			}
		}
	}

	bool FanNet::xPath(Gate *gate)
	{
		int i;

		// base step --- if no X-path exist, return FALSE 
		if(gate->output!=X || gate->xpath==0)
		{
			gate->xpath=0;
			return false;
		}

		// base step --- if an X-path exist, return TRUE
		if((gate->fn==PO) || (gate->xpath==2))
		{
			gate->xpath=2;
			return true;
		}

		// induction step --- else, go to next step
		for(i=0;i<gate->noutput;i++)
			if(xPath(gate->outlis[i]))
			{
				gate->xpath=2;
				return true;
			}

			gate->xpath=0;
			return false;
	}

	Gate* FanNet::closestPO(Stack* objective,int *pclose)
	{
		int i,distance;
		Gate *p;

		if(objective->isEmpty()) return 0;

		*pclose=objective->getCount()-1;   
		distance=(*objective)[*pclose]->dpo;
		for(i=(objective->getCount())-1;i>=0;i--)
			if((*objective)[i]->dpo<distance)
			{
				distance=(*objective)[i]->dpo;
				*pclose=i;
			}
			p=(*objective)[*pclose];
			return p;
	}

	Gate* FanNet::selectHardest(Stack* objective,int *pclose)
	{
		int i,distance;
		Gate *p;

		if(objective->isEmpty()) return 0;
		*pclose=objective->getCount()-1;
		distance=(*objective)[*pclose]->dpo;
		for(i=objective->getCount()-1;i>=0;i--)
			if((*objective)[i]->dpo>distance)
			{
				distance=(*objective)[i]->dpo;
				*pclose=i;
			}
			p=(*objective)[*pclose];
			return p;
	}

	status FanNet::backTrace(status state)
	{
		int i,j;
		level v1;
		Gate *aCurrObj,**input;
		int n0,n1,nn0,nn1;
		int easiest,easyCont;
#ifdef _ALG_DEBUG
								dbgFile << "backtrace begin\n" ;
#endif

		// box 1: Initialization of objective and its logic level 
		if(state==81) {
			currObj.copyFrom(&initObj);
			for(i=0;i<=initObj.getCount()-1;i++)
			{
				aCurrObj=initObj[i];
				switch(aCurrObj->output)
				{
				case ZERO:
				case DBAR:
					aCurrObj->setLine(1,0); //unjustified lines
					break;
				case ONE:
				case D:
					aCurrObj->setLine(0,1);
					break;
				default:	//dFrontier
					switch(aCurrObj->fn)
					{
					case AND:
					case NOR: 
						aCurrObj->setLine(0,1);	break;
					case NAND:
					case OR:
					case XOR:
					case XNOR:
						aCurrObj->setLine(1,0);	break;
					}
				}
			}
			state=82;
		}

		while(true)
		{
			switch(state)
			{
			case 82:	// Box 2,3,4 of figure 8
				if(currObj.isEmpty())	//box 2
					if(fanObj.isEmpty())
						state=103;		//box 4
					else
						state=86;
				else
				{
					aCurrObj=currObj.pop();	//box 3
					state=85;
				}
				break;

			case 85:	// Box 5,9,10,11,12 of figure 8
				if(aCurrObj->isHead())
					headObj->push(aCurrObj);	//box 5
				else
				{	//box 9,10,11
					switch(aCurrObj->fn)
					{
					case AND:
						n0=aCurrObj->numzero;
						n1=aCurrObj->numone;
						v1=ZERO;
						break;
					case OR:
						n0=aCurrObj->numzero;
						n1=aCurrObj->numone;
						v1=ONE;
						break;
					case NAND:
						n0=aCurrObj->numone;
						n1=aCurrObj->numzero;
						v1=ZERO;
						break;
					case NOR:
						n0=aCurrObj->numone;
						n1=aCurrObj->numzero;
						v1=ONE;
						break;
					case NOT:
						n0=aCurrObj->numone;
						n1=aCurrObj->numzero;
						v1=X;
						break;
					case XOR:
						j=0;
						if((v1=aCurrObj->inlis[j]->output)==X)
							v1=aCurrObj->inlis[++j]->output;
						if(v1==ONE)
						{
							n0=aCurrObj->numone;
							n1=aCurrObj->numzero;
						} else
						{
							n0=aCurrObj->numzero;
							n1=aCurrObj->numone;
						}
						v1=X;
						break;
					case XNOR:
						j=0;
						if((v1=aCurrObj->inlis[j]->output) ==X)
							v1=aCurrObj->inlis[++j]->output;
						if(v1==ZERO)
						{
							n0=aCurrObj->numone;
							n1=aCurrObj->numzero;
						} else
						{
							n0=aCurrObj->numzero;
							n1=aCurrObj->numone;
						}
						v1=X;
						break;
					default:	// BUFF,PO,PI
						n0=aCurrObj->numzero;
						n1=aCurrObj->numone;
						v1=X;
						break;
					}

					// Find the easiest input
					input=aCurrObj->inlis;
					easyCont=INFINITY;
					easiest=0;

					if(v1==ZERO)
					{	//and, nand
						for(i=0;i<aCurrObj->ninput;i++)
							if(input[i]->output==X && easyCont>input[i]->cont0)
							{
								easyCont=input[i]->cont0;
								easiest=i;
							}
					} else
					{	//or,nor,xor,xnor
						for(i=0;i<aCurrObj->ninput;i++)
							if(input[i]->output==X && easyCont>input[i]->cont1)
							{
								easyCont=input[i]->cont1;
								easiest=i;
							}
					}

					for(i=0;i<aCurrObj->ninput;i++)
					{
						if(input[i]->output==X)
						{
							if(i==easiest) {nn0=n0;nn1=n1;}
							else if(v1==ZERO) {nn0=0;nn1=n1;}
							else if(v1==ONE)  {nn0=n0;nn1=0;}
							else	//xor,xnor
								if(n0>n1) 
								{nn0=n0;nn1=n1;}
								else
								{nn0=n1;nn1=n0;}

								if(nn0>0 || nn1>0)
									if(input[i]->isFanout())
									{
										if(input[i]->numzero==0 && input[i]->numone==0) fanObj.push(input[i]);
										input[i]->numzero+=nn0;
										input[i]->numone+=nn1;
									} else
									{
										input[i]->setLine(nn0,nn1);
										currObj.push(input[i]);
									}
						}
					}
				}
				state=82;
				break;

			case 86:	// Box 6,7,8 of figure 8
				aCurrObj=closestPO(&fanObj,&i); //box 6
				fanObj.deleteItem(i);

				if(aCurrObj->output!=X) {state=82;break;}
				if(aCurrObj->isReachableFormFault()) {state=85; break;}	//box 7
				if(!aCurrObj->isConflict()) {state=85; break;}	//box 8
				finalObj.push(aCurrObj);	//box 12 in figure 10
				state=93;
				break;
			default:
				return state;
			}
		}
	}


	void FanNet::findFinalOjective(bool *backtrace,bool faultPropgateToPo,Gate** lastDFrontier)
	{
		int i;
		Gate *p;
		status state;

		if(*backtrace) {
			state=107;	// box 1
#ifdef _ALG_DEBUG
		dbgFile << "findFinalObjective set state to 107\n";
#endif
		}
		else 
			if(fanObj.isEmpty()) {
				state=103; //box 2
#ifdef _ALG_DEBUG
		dbgFile << "findFinalObjective set state to 103\n";
#endif
			}
			else {
#ifdef _ALG_DEBUG
		dbgFile << "findFinalObjective set state to 86\n";
#endif
				state=86;
			}

		while(true)
			switch(state)
		{
			case 103: 			// box 3,4,5,6 
				if(headObj->isEmpty()) {
					state=107;	//box 3
#ifdef _ALG_DEBUG
		dbgFile << "findFinalObjective set state to 107(2)\n";
#endif
				}
				else
				{
					p=(*headObj)[0];
					for(i=1;i<=headObj->getCount()-1;i++)
						(*headObj)[i-1]=(*headObj)[i];
					headObj->pop();

					if(p->output==X)
					{ //box 4,5
						finalObj.push(p);	//box 6
						state=93;
#ifdef _ALG_DEBUG
		dbgFile << "findFinalObjective set state to 93\n";
#endif

					}
					else {
#ifdef _ALG_DEBUG
		dbgFile << "findFinalObjective set state to 103(2)\n";
#endif
						state=103;
					}
				}
				break;
			case 107:			// box 7,8,9,10,11
				*backtrace=false;	//box 7
				for(i=0;i<numberOfGates;i++)
				{	// initialization
					net[i]->numzero=0;
					net[i]->numone=0;
				}
				initObj.clear();
				currObj.clear();
				fanObj.clear();
				headObj->clear();
				finalObj.clear();

				if(!unjustified.isEmpty())
				{	//box 8
					//initObj=unjustified;	//box 9
					initObj.copyFrom(&unjustified);	//box 9
					if(faultPropgateToPo)
					{	//box 10
						*lastDFrontier=0;
						state=81;
#ifdef _ALG_DEBUG
		dbgFile << "findFinalObjective set state to 81\n";
#endif
						break;
					}
				}

				switch(SELECTMODE)
				{
				case 0:	//easiest D first;
					*lastDFrontier=closestPO(&dFrontier,&i);
					initObj.push(*lastDFrontier);
					break;
				case 1:
					*lastDFrontier=selectHardest(&dFrontier,&i);
					initObj.push(*lastDFrontier);
					break;
				case 2:
					*lastDFrontier=closestPO(&dFrontier,&i);
					if(unjustified.isEmpty()) initObj.push(*lastDFrontier);
					break;
				case 3:
					*lastDFrontier=selectHardest(&dFrontier,&i);
					if(unjustified.isEmpty()) initObj.push(*lastDFrontier);
					break;
				}

				state=81;
#ifdef _ALG_DEBUG
		dbgFile << "findFinalObjective set state to 81(2)\n";
#endif
				break;
			case 86:
			case 81:
				state=backTrace(state);
#ifdef _ALG_DEBUG
		dbgFile << "findFinalObjective set state to " << state << "\n";
#endif
				break;
			default:
				return;				// exit
		}
	}

	bool FanNet::backTrack(Gate *faultyGate, int *last)
	{
		struct TreeNode *backNode;
		Gate *p;
		int i,j;
		level value;
#ifdef _ALG_DEBUG
		dbgFile << "backtrack begin, index:" << faultyGate->index << "\n";
#endif
		backNode = &(tree.back());
		while(tree.size()!=0)
			if(tree.back().isFlagged()) 
				tree.pop_back();
			else
			{
				// update & remove duplicate unjustified lines 
				for(i=unjustified.getCount()-1;i>=0;i--)         //-1 je navic
				{
					p=unjustified[i];
					if(p->isJustified()) 
						unjustified.deleteItem(i);
					else
					{
						p->changed=true;
						finalObj.push(p);
					}
				}

				while(!finalObj.isEmpty()) finalObj.pop()->changed=false;

				// restore and schedule events 
				value=aNot(tree.back().gate->output);
				tree.back().flag=true;
				for(i=tree.back().pstack;i<=stack->getCount()-1;i++)
				{
					p=(*stack)[i];
					p->output=X;
					p->changed=false;
					for(j=0;j<p->noutput;j++) p->outlis[j]->changed=false;
				}
				*last=0;

				for(i=tree.back().pstack+1;i<=stack->getCount()-1;i++)
				{
					p=(*stack)[i];
					for(j=0;j<p->noutput;j++)
					{
						if(p->outlis[j]->output!=X)
						{
							if(!p->outlis[j]->isJustified()) unjustified.push(p->outlis[j]);
							pushEvent(p->outlis[j]);
#ifdef _ALG_DEBUG
							dbgFile << "backtrack pushevent, index:" << p->outlis[j]->index << "\n";
#endif
						}
						if(*last<p->outlis[j]->dpi) *last=p->outlis[j]->dpi;
					}
				}

				stack->setCount(tree.back().pstack+1);
				tree.back().gate->output=value;
				if(tree.back().gate->isHead()) 
					tree.back().gate->changed=true;
				else {
#ifdef _ALG_DEBUG
							dbgFile << "backtrack pushevent1, index:" << tree.back().gate->index << "\n";
#endif
					pushEvent(tree.back().gate);
				}
				scheduleOutput(tree.back().gate);

				//update dFrontier
				dFrontier.clear();
				dFrontier.push(faultyGate);
				updateDFrontier();

				// update unjustified set 
				for(i=unjustified.getCount()-1;i>=0;i--)     //-1 je navic
					if(unjustified[i]->output==X) unjustified.deleteItem(i);

				//reset xpath
				for(i=faultyGate->index;i<numberOfGates;i++) net[i]->xpath=1;
				return true;
			}
			return false;
	}

	void FanNet::restoreFaults(Fault *fal)
	{
		int i,j,k;
		Gate *p;
		level value,gtype;

		fanObj.clear();
		p=fal->gate;
		if(fal->line!=OUTFAULT) p=p->inlis[fal->line];
		fanObj.push(p);

		while(true)
		{
			p=p->outlis[0];
			for(i=0;i<p->ninput;i++)
				if(p->inlis[i]->output==ZERO || p->inlis[i]->output==ONE)
					fanObj.push(p->inlis[i]);
			if(p->isHead()) break;
		}

		while(!fanObj.isEmpty())
		{
			p=fanObj.pop();
			if(p->output==D) p->output=ONE;
			else if(p->output==DBAR) p->output=ZERO;

			if(!(p->fn==PI || p->output==X))
			{
				currObj.clear();
				currObj.push(p);
				while(!currObj.isEmpty())
				{
					p=currObj.pop();
					switch(p->fn)
					{
					case PI: break;
					case XOR:
						p->inlis[0]->output=ZERO;
						currObj.push(p->inlis[0]);
						for(j=1;j<p->ninput;j++)
						{
							p->inlis[j]->output=p->output;
							currObj.push(p->inlis[j]);
						}
						break;
					case XNOR:
						p->inlis[0]->output=ONE;
						currObj.push(p->inlis[0]);
						for(j=1;j<p->ninput;j++)
						{
							p->inlis[j]->output=p->output;
							currObj.push(p->inlis[j]);
						}
						break;
					case PO:
					case BUFF:
					case NOT:
						p->inlis[0]->output=a_truthtbl1[p->fn][p->output];
						currObj.push(p->inlis[0]);
						break;
					default: // and,or,nor,nand
						value=a_truthtbl1[p->fn][p->output];
						gtype=(p->fn==AND || p->fn==NAND) ? ONE : ZERO;
						if(value==gtype)
							for(j=0;j<p->ninput;j++)
							{
								p->inlis[j]->output=value;
								currObj.push(p->inlis[j]);
							}
						else
						{
							k=0;
							for(j=1;j<p->ninput;j++)
								if(p->inlis[j]->dpi<p->inlis[k]->dpi) k=j;
							p->inlis[k]->output=value;
							currObj.push(p->inlis[k]);
						}
					}
				}
			}
		}
	}

	void FanNet::justifyFreeLines(Fault *of,Fault* cf)
	{
		int i,j,k;
		Gate *p;
		level value,gtype;

		for(i=0;i<numberOfPrimaryInputs;i++)
		{
			if(headlines[i]<0) break;
			p=net[headlines[i]];
			if(p==cf->gate && of!=0)
			{
				restoreFaults(of);
				continue;
			}

			if(p->output==D) p->output=ONE;
			else if(p->output==DBAR) p->output=ZERO;
			if(!(p->fn==PI) || p->output==X)
			{
				currObj.clear();
				currObj.push(p);
				while(!currObj.isEmpty())
				{
					p=currObj.pop();
					switch(p->fn)
					{
					case PI: break;
					case XOR:
						p->inlis[0]->output=ZERO;
						currObj.push(p->inlis[0]);
						for(j=1;j<p->ninput;j++)
						{
							p->inlis[j]->output=p->output;
							currObj.push(p->inlis[j]);
						}
						break;
					case XNOR:
						p->inlis[0]->output=ONE;
						currObj.push(p->inlis[0]);
						for(j=1;j<p->ninput;j++)
						{
							p->inlis[j]->output=p->output;
							currObj.push(p->inlis[j]);
						}
						break;
					case PO:
					case BUFF:
					case NOT:
						p->inlis[0]->output=a_truthtbl1[p->fn][p->output];
						currObj.push(p->inlis[0]);
						break;
					default:	// and,or,nor,nand 
						value=a_truthtbl1[p->fn][p->output];
						gtype= (p->fn==AND || p->fn==NAND) ? ONE : ZERO;
						if(value==gtype)
							for(j=0;j<p->ninput;j++)
							{
								p->inlis[j]->output=value;
								currObj.push(p->inlis[j]);
							}
						else
						{
							k=0;
							for(j=1;j<p->ninput;j++)
								if(p->inlis[j]->dpi<p->inlis[k]->dpi) k=j;
							p->inlis[k]->output=value;
							currObj.push(p->inlis[k]);
						}
					}
				}
			}
		}
	}

	int FanNet::dynamicUniqueSensitize(Stack *dominators,int maxdpi,Gate* faultyGate)
	{
		int nDom=0,nGate;
		int dyID2;
		int i,j,k;
		Gate *gut,*next;	   
		int nDominator=0,noDom=0;
		bool flag=false;
		bool debud=true;
		int v1,send=01;
		vector<Gate *> xPo;//Gate* xPo[MAXPO];
		int nxPo;
		vector<Gate *>domArray;//Gate *domArray[MAXGATE];

		// pass 1: D-frontier propagation
		dyID++;

		for(i=nxPo=0;i<=dominators->getCount()-1;i++)
		{
			gut=(*dominators)[i];
			if(gut->freach1<dyID)
			{
				eventList[gut->dpi]->push(gut);
#ifdef _ALG_DEBUG
				dbgFile << "dynuniquesensitize event push index:" << gut->index << "\n";
#endif

				gut->freach1=dyID;
			}
		}

		for(i=0;i<maxdpi;i++)
		{
			while(!eventList[i]->isEmpty())
			{
				gut=eventList[i]->pop();
				if(gut->fn==PO) {
					if(nxPo >= xPo.size()) {
						xPo.resize(nxPo + 1, NULL);
					}
					xPo[nxPo++]=gut;
				}
				for(j=0;j<gut->noutput;j++)
				{
					next=gut->outlis[j];
					if(next->output==X && next->freach1<dyID)
					{
						eventList[next->dpi]->push(next);
#ifdef _ALG_DEBUG
				dbgFile << "dynuniquesensitize(1) event push index:" << next->index << "\n";
#endif
						next->freach1=dyID;
					}
				}
			}
		}

		// pass 2: Backward propagation --- X-path
		dyID2=dyID+1;
		for(i=0;i,nxPo;i++)
		{
			gut=xPo[i];
			gut->freach1=dyID2;
			for(j=0;j<gut->ninput;j++)
			{
				next=gut->inlis[j];
				if(next->freach1==dyID)
				{
					eventList[next->dpi]->push(next);
#ifdef _ALG_DEBUG
				dbgFile << "dynuniquesensitize(2) event push index:" << next->index << "\n";
#endif
					next->freach1=dyID2;
				}
			}
		}

		for(i=maxdpi-1; i>=0; i--)
			while(!eventList[i]->isEmpty())
			{
				gut=eventList[i]->pop();
				for(j=0;j<gut->ninput;j++)
				{
					next=gut->inlis[j];
					if(next->freach1==dyID)
					{
						eventList[next->dpi]->push(next);
#ifdef _ALG_DEBUG
				dbgFile << "dynuniquesensitize(3) event push index:" << next->index << "\n";
#endif
						next->freach1=dyID2;
					}
				}
			}

			// pass 3: Compute dominators
			dyID=dyID2+1;
			for(i=nGate=0;i<=dominators->getCount()-1;i++)
			{
				gut=(*dominators)[i];
				eventList[gut->dpi]->push(gut);
#ifdef _ALG_DEBUG
				dbgFile << "dynuniquesensitize(4) event push index:" << gut->index << "\n";
#endif
				nGate++;
				gut->freach1=dyID;
			}

			for(i=k=0;i<maxdpi;i++)
			{
				if(nGate==1 && eventList[i]->getCount()==1) {
					//domArray[k++] = (*eventList[i])[0];
					domArray.push_back((*eventList[i])[0]);
					k++;
				}
				while(!eventList[i]->isEmpty())
				{
					gut=eventList[i]->pop();
					nGate--;
					if(gut->fn==PO) {nGate=INFINITY; break;}
					for(j=0; j<gut->noutput; j++)
					{
						next=gut->outlis[j];
						if(next->freach1==dyID2)
						{
							eventList[next->dpi]->push(next);
#ifdef _ALG_DEBUG
				dbgFile << "dynuniquesensitize(5) event push index:" << next->index << "\n";
#endif
							next->freach1=dyID;
							nGate++;
						}
					}
				}
				if(nGate==INFINITY) break;
			}

			// Assign non-controlling values to dominators
			send=-1;
			while(--k >= 0)
			{
				gut=domArray[k];
				//printf("dominator: gut=%d #Dfrontier=%d\n",gut->index,nod+1);
				if(gut==faultyGate) continue;
				v1=(gut->fn==AND || gut->fn==NAND) ? ONE : (gut->fn==OR || gut->fn==NOR) ? ZERO : X;
				if(v1 != X)
				{
					for(i=0; i<gut->ninput; i++)
					{
						next = gut->inlis[i];
						if(next->freach1<dyID && next->output==X)
						{
							next->output=v1;
							//printf("\tmandatory signal assignment: gut=%d val=%d\n", next->index, v1);
							send=MAX(next->dpi,send);
							stack->push(next);
							scheduleInput(gut,i);
						}
					}
				}
			}

			return send;
	}

	status FanNet::fan(int maxdpi,Fault* cf, int maxbacktrack, int *nbacktrack)
	{
		int i;
		Gate *gut,*g;
		int lastDpi;
		Gate *lastDFrontier;
		bool backwardFlag,backtraceFlag,faultPropagateToPo;
		bool dFrontierChanged,done;
		status state;
		Fault *original=0;

#ifdef _ALG_DEBUG
	dbgFile << "fan begin, fault index:" << cf->index << "\n";
#endif
		*nbacktrack=0;
		done=false;
		backwardFlag=false;
		backtraceFlag=true;
		faultPropagateToPo=false;
		lastDFrontier=0;

		gut=cf->gate;

		initNet(gut,maxdpi);		// initializaiton 
		if(cf->line!=OUTFAULT) gut=gut->inlis[cf->line];
		if(gut->isFree())	//box 1
		{
			original=new Fault();
			original->gate=cf->gate;
			original->line=cf->line;
			original->type=cf->type;
			faultyLineIsFree(cf);
			lastDpi=0;
		} else
			lastDpi=setFaultyGate(cf);

		if(lastDpi==-1) return NO_TEST;

		gut=cf->gate;

#ifdef LEARNFLG ///////////////////////////
		if(!gut->pLearn.empty() && gut->output!=X)
			switch(implyLearn1(gut,gut->output))
		{
			case CONFLICT: return(NO_TEST); break;
			case BACKWARD: lastDpi=gut->dpi; break;
		}
#endif

		dFrontier.push(gut);
		i=uniqueSensitize(gut,gut);

		if((lastDpi=MAX(i,lastDpi))>0) backwardFlag=true;
		state=93;

		// main loop of fan algorithm
		while(done==false)
		{
			switch(state)
			{
			case 93: //box 3,4,5,6
#ifdef _ALG_DEBUG
				dbgFile << "fan case 93, lastDpi:" << lastDpi << ", ninput:" << gut->ninput << ", index:" << 
					gut->index << ", maxdpi:" << maxdpi << ", backward:" << backwardFlag << "\n" ;
#endif
				if(!imply(maxdpi,backwardFlag,lastDpi,cf))
				{
					//box 3
					state=98;
					break;
				}
				if(gut->output==ZERO || gut->output==ONE) {state=98; break;}

				// update unjustified lines and delete duplicated lines 
				// final_obj should be empty 
				for(i=unjustified.getCount()-1;i>=0;i--)  //-1 je navic
				{
					g=unjustified[i];
					if(g->isJustified()) 
						unjustified.deleteItem(i);
					else
					{
						g->changed=true;
						finalObj.push(g);
					}
				}
				while(!finalObj.isEmpty()) finalObj.pop()->changed=false;

				// check for backtrace 
				for(i=initObj.getCount()-1;i>=0;i--)   //-1 je navic
					if(initObj[i]->isJustified()) initObj.deleteItem(i);

				faultPropagateToPo=false;
				for(i=0;i<numberOfPrimaryOutputs;i++)
					if(net[primaryOut[i]]->output==D || net[primaryOut[i]]->output==DBAR)
					{
						faultPropagateToPo=true;
						break;
					}

					if(lastDFrontier!=0)
						if(lastDFrontier->output==X)
							dFrontierChanged=false;
						else
							dFrontierChanged=true;
					else
						dFrontierChanged=true;

					if(initObj.isEmpty() && dFrontierChanged)  backtraceFlag=true;//box 4, 4-1

					if(faultPropagateToPo)
					{
						state=99;	//box 4-3
#ifdef _ALG_DEBUG
						for(i=unjustified.getCount()-1;i>=0;i--) { 
								dbgFile << "fan unjustified(" << i << "): isunjust. " << unjustified[i]->isUnJustified() << 
								", isbound: " << unjustified[i]->isBound() << "\n";
						}
#endif
						for(i=unjustified.getCount()-1;i>=0;i--) 
							if(unjustified[i]->isUnJustified() && unjustified[i]->isBound())
							{
								state=97;
#ifdef _ALG_DEBUG
								dbgFile << "fan case 93: jump to 97(1)\n" ;
#endif
								break;
							}
					} else
					{		//box 5
						//update dFrontier
						if(!dFrontier.isEmpty()) updateDFrontier();
						for(i=gut->index;i<numberOfGates;i++) {
							if(net[i]->xpath==2) 
								net[i]->xpath=1;
						}
						for(i=dFrontier.getCount()-1;i>=0;i--)
							if(!xPath(dFrontier[i])) dFrontier.deleteItem(i);

						if(dFrontier.isEmpty()) state=98;
						else if(dFrontier.getCount()==1)
						{	//box 9
							if((lastDpi=uniqueSensitize(dFrontier[0],gut))>0)
							{
								backwardFlag=true;
								state=93;
#ifdef _ALG_DEBUG
								dbgFile << "fan case 93: jump to 93(1)\n" ;
#endif
							}
							else if(lastDpi==0) {
								state=93;
#ifdef _ALG_DEBUG
								dbgFile << "fan case 93: jump to 93(2)\n" ;
#endif
							}
							else {
								state=97;
#ifdef _ALG_DEBUG
								dbgFile << "fan case 93: jump to 97(2)\n" ;
#endif
							}
						}
						else {
							state=97;
#ifdef _ALG_DEBUG
								dbgFile << "fan case 93: jump to 97(3)\n" ;
#endif
						}
					}
					break;
			case 97:	//box 7
#ifdef _ALG_DEBUG
				dbgFile << "fan case 97\n" ;
#endif

				findFinalOjective(&backtraceFlag,faultPropagateToPo,&lastDFrontier);

				while(!finalObj.isEmpty())
				{
					g=finalObj.pop();
					if(g->numzero>g->numone) 
						g->output=ZERO;
					else
						g->output=ONE;
					tree.push_back(TreeNode(g,false));

					stack->push(g);
					if(g->isHead()) 
						g->changed=true;
					else {
						pushEvent(g);		/**** study ****/
#ifdef _ALG_DEBUG
						dbgFile << "fan case 97 pushevent index:" << g->index << "\n" ;
#endif
					}
					scheduleOutput(g);
					tree.back().pstack=stack->getCount()-1;
				}

				backwardFlag=false;
				state=93;
				break;

			case 98:	//box 8
#ifdef _ALG_DEBUG
				dbgFile << "fan case 98 index:" << gut->index << "\n" ;
			for(int qwer = 0; qwer < net[172]->noutput; qwer++) {
				dbgFile << "fan case 98, outlist " << qwer << ", index:" << net[172]->outlis[qwer]->index << "\n";
			}
#endif
				if(tree.back().isFlagged()) (*nbacktrack)++;

				for(i=0;i<maxdpi;i++)
					while(!eventList[i]->isEmpty()) eventList[i]->pop()->changed=false;
				if(*nbacktrack>maxbacktrack) state=OVER_BACKTRACK;
				else if(backTrack(gut,&lastDpi))
				{
					if(lastDpi>0) 
						backwardFlag=true;
					else
						backwardFlag=false;
					backtraceFlag=true;
					lastDFrontier=0;
					state=93;
				}
				else state=NO_TEST;
				break;

			case 99:
#ifdef _ALG_DEBUG
				dbgFile << "fan case 99\n" ;
#endif
				justifyFreeLines(original,cf);
				state=TEST_FOUND;
				if(noFaultSim=='y') 
				{
					myDPrintIO(++nTestEach);							

					if(genAllPat=='y' && (nTestEachLimit<=0 || nTestEach<nTestEachLimit))
					{
						*nbacktrack=0;
						state=98;
					}
				}
				break;
			default:
#ifdef _ALG_DEBUG
				dbgFile << "fan case default: " << state << "\n";
#endif
				done=true;

			}
		}

		if(original!=0)
		{
			cf->gate=original->gate;
			cf->line=original->line;
			cf->type=original->type;
			delete original;
		}

		return state;
	}

	status FanNet::fan1(int maxdpi,Fault* cf,int maxbacktrack, int *nbacktrack)
	{
		int i;
		Gate *gut,*g;
		int lastDpi;
		Gate *lastDfrontier;
		bool backwardFlag,backtraceFlag,faultPropagateToPo;
		bool dFrontierChanged,done;
		status state;
		Fault *original=0;

		*nbacktrack=0;
		done=false;
		backwardFlag=false;
		backtraceFlag=true;
		faultPropagateToPo=false;
		lastDfrontier=0;

		gut=cf->gate;

		initNet(gut,maxdpi);	//TODO: initialization

		if(cf->line!=OUTFAULT) gut=gut->inlis[cf->line];
		if(gut->isFree())
		{	//box 1
			original=new Fault();
			original->gate=cf->gate;
			original->line=cf->line;
			original->type=cf->type;
			faultyLineIsFree(cf);
			lastDpi=0;
		} else
			lastDpi=setFaultyGate(cf);

		if(lastDpi==-1) return NO_TEST;

		gut=cf->gate;
#ifdef LEARNFLG
		if(!gut->pLearn.empty() && gut->output!=X)
			switch(implyLearn1(gut,gut->output)) {
		case CONFLICT: return(NO_TEST); break;
		case BACKWARD: lastDpi=gut->dpi; break;
			}
#endif

			dFrontier.push(gut);
			i=uniqueSensitize(gut,gut);
			if((lastDpi=MAX(i,lastDpi))>0) backwardFlag=true;
			state=93;

			// main loop of fan algorithm

			while(done==false)
			{
				switch(state)
				{
				case 93:		//box 3,4,5,6
					if(!imply(maxdpi,backwardFlag,lastDpi,cf))
					{	//box 3
						state=98;
						break;
					}

					if(gut->output==ZERO || gut->output==ONE) {state=98;break;};

					// update unjustified lines and delete duplicated lines 
					//final_obj should be empty
					for(i=unjustified.getCount()-1;i>=0;i--)
					{
						g=unjustified[i];
						if(g->isUnJustified()) 
							unjustified.deleteItem(i);
						else
						{
							g->changed=true;
							finalObj.push(g);
						}
					}

					while(!finalObj.isEmpty()) finalObj.pop()->changed=false;

					//check for backtrace
					for(i=initObj.getCount()-1;i>=0;i--)
						if(initObj[i]->isJustified()) initObj.deleteItem(i);

					faultPropagateToPo=false;
					for(i=0;i<numberOfPrimaryOutputs;i++)
						if(net[primaryOut[i]]->output==D || net[primaryOut[i]]->output==DBAR)
						{
							faultPropagateToPo=true;
							break;
						}

						if(lastDfrontier!=0)
							if(lastDfrontier->output==X)
								dFrontierChanged=false;
							else
								dFrontierChanged=true;
						else
							dFrontierChanged=true;

						if(initObj.isEmpty() && dFrontierChanged) //box 4, 4-1
							backtraceFlag=true;

						if(faultPropagateToPo)
						{	//box 4-3
							state=99;
							for(i=unjustified.getCount()-1;i>=0;i--)
								if(unjustified[i]->isUnJustified() && unjustified[i]->isBound())
								{
									state=97;
									break;
								}

						} else
						{	//box 5
							//update dFrontier
							if(!dFrontier.isEmpty()) updateDFrontier();
							for(i=gut->index;i<numberOfGates;i++) if(net[i]->xpath==2) net[i]->xpath=1;
							for(i=dFrontier.getCount()-1;i>=0;i--)
								if(!xPath(dFrontier[i])) dFrontier.deleteItem(i);

							if(dFrontier.isEmpty()) 
								state=98;	//when dfrontier is not zero
							else
							{	// box 6
								if(dyID>=INFINITY-3)
								{
									for(i=0;i<numberOfGates;i++) net[i]->freach1=0;
									dyID=0;
								}

								if(lastDpi==dynamicUniqueSensitize(&dFrontier,maxdpi,gut)>0)
								{
									backwardFlag=true;
									state=93;
								}
								else if(lastDpi==0) state=93;
								else state=97;
							}
						}
						break;

				case 97:		//box 7
					findFinalOjective(&backtraceFlag,faultPropagateToPo,&lastDfrontier);

					while(!finalObj.isEmpty())
					{
						g=finalObj.pop();
						if(g->numzero > g->numone) 
							g->output=ZERO;
						else
							g->output=ONE;

						tree.push_back(TreeNode(g,false));
						stack->push(g);

						if(g->isHead()) 
							g->changed=true;
						else
							pushEvent(g);	/**** study ****/
						scheduleOutput(g);
						tree.back().pstack=stack->getCount()-1;
					}

					backwardFlag=false;
					state=93;
					break;
				case 98:		//box 8
					if(tree.back().isFlagged()) (*nbacktrack)++;

					for(i=0;i<maxdpi;i++)
						while(!eventList[i]->isEmpty()) eventList[i]->pop()->changed=false;
					if(*nbacktrack>maxbacktrack) state=OVER_BACKTRACK;
					else if (backTrack(gut,&lastDpi))
					{
						if(lastDpi>0) 
							backwardFlag=true;
						else
							backwardFlag=false;
						backtraceFlag=true;
						lastDfrontier=0;
						state=93;
					}
					else state=NO_TEST;
					break;

				case 99:
					justifyFreeLines(original,cf);
					state=TEST_FOUND;
				default:
					done=true;
				}
			}


			if(original!=0)
			{
				cf->gate=original->gate;
				cf->line=original->line;
				cf->type=original->type;
				delete original;
			}
			return state;
	}

	status FanNet::implyLearn(Gate *gut, level val)
	{
		Gate *tg;
		int state;
		list<Learn>::iterator i;

#ifdef _ALG_DEBUG
		dbgFile << "implyLearn begin, index:" << gut->index << "\n";
#endif
		switch(gut->fn) {
		case AND: case NOR: if(val==ONE) return(FORWARD); break;
		case OR: case NAND: if(val==ZERO) return(FORWARD); break;
		}

		state=FORWARD;
		for(i = gut->pLearn.begin(); i != gut->pLearn.end(); ++i) {
			if(i->sval==val) {
				tg=net[i->node];
				switch(conflictTbl[tg->output][i->tval]) {
				case PASS: break;
				case FAIL: 
#ifdef DEBUGLEARN
					printf("Learned: conflict at node=%d old=%s new=%s from node=%d val=%s\n",
						tg->index, level2str[tg->output], level2str[tmp->tval],
						gut->index, level2str[val]);
#endif
					return(CONFLICT);
				case PASS1:
					tg->output=i->tval;
					stack->push(tg);
					pushEvent(tg);
					scheduleOutput(tg);
#ifdef DEBUGLEARN
					printf("Learned: node=%d val=%s from node=%d val=%s\n",
						tg->index, level2str[tg->output], gut->index, level2str[val]);
#endif
					state=BACKWARD;
					break;
				}
			}

		}
		return(state);
	}

	status FanNet::implyLearn1(Gate *gut, level val)
	{
		Gate *tg;
		int state;
		list<Learn>::iterator i;

#ifdef _ALG_DEBUG
		dbgFile << "implyLearn1 begin, index:" << gut->index << "\n";
#endif
		if(val==D) val=1;
		else if(val==DBAR) val=0;

		switch(gut->fn) {
		case AND: case NOR: if(val==ONE) return(FORWARD); break;
		case OR: case NAND: if(val==ZERO) return(FORWARD); break;
		}

		state=FORWARD;
		for(i=gut->pLearn.begin(); i != gut->pLearn.end(); ++i)
			if(i->sval==val) {
				tg=net[i->node];
				if(tg->freach) continue;
				switch(conflictTbl[tg->output][i->tval]) {
				case PASS: break;
				case FAIL:
#ifdef DEBUGLEARN
					printf("Learned: conflict at node=%d old=%s new=%s from node=%d val=%s\n",
						tg->index, level2str[tg->output], level2str[tmp->tval],
						gut->index, level2str[val]);
#endif
					return(CONFLICT);
				case PASS1:
					tg->output=i->tval;
					stack->push(tg);
					pushEvent(tg);
					scheduleOutput(tg);
#ifdef DEBUGLEARN
					printf("Learned: node=%d val=%s from node=%d val=%s\n",
						tg->index, level2str[tg->output], gut->index, level2str[val]);
#endif
					state=BACKWARD;
					break;
				}
			}

			return(state);
	}
	//#endif

	/*FanNet::~FanNet()
	{

	}*/
}
