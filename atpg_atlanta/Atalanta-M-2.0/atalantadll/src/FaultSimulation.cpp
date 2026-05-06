// FaultSimulation.cpp: implementation of the FaultSimulation class.
//
//////////////////////////////////////////////////////////////////////
#include <string>
#include <list>
#include <fstream>

#include "Defines.h"
#include "Truthtable.h"
#include "Parameters.h"

#include "Hash.h"

#include "Gate.h"
#include "Stack.h"
#include "GateNet.h"
#include "FanNet.h"
#include "ParralelPattern.h"

#include "Fault.h"

#include "Globals.h"

#include "FaultSimulation.h"

namespace atalantadll {
	//////////////////////////////////////////////////////////////////////
	// Construction/Destruction
	//////////////////////////////////////////////////////////////////////
#ifdef INCLUDE_HOPE
	int FaultSimulation::setFFR()
	{
		int i,j;
		Gate *gut;

		for(i=0;i<numberOfGates;i++)
		{
			gut=net[i];
			gut->changed=false;
			if(gut->noutput!=1) numberOfStems++;
			else if(gut->outlis[0]->fn==DFF) numberOfStems++;
			gut->stem=0;
		}

		stems=new Stem[numberOfStems+1];
		numberOfStems=0;

		for(i=0;i<numberOfFlipFlops;i++)
		{	// PPI
			gut=net[flipFlops[i]];
			if(gut->noutput!=1)
			{
				gut->stem=++numberOfStems;
				stems[numberOfStems].gate=gut->index;
				stems[numberOfStems].dominator=-1;
			}
		}

		for(i=0;i<numberOfGates;i++)
		{
			gut=net[i];
			if(gut->fn==DFF || gut->fn==PO) continue;
			if(gut->noutput!=1)
			{
				gut->stem=++numberOfStems;
				stems[numberOfStems].gate=gut->index;
				stems[numberOfStems].dominator=-1;
			}
		}

		rstem=numberOfStems;
		for(i=0; i<numberOfPrimaryOutputs; i++)
		{	// PO
			gut=net[primaryOut[i]];
			gut->stem=++numberOfStems;
			stems[numberOfStems].gate=gut->index;
			stems[numberOfStems].dominator=-1;
		}

		for(i=0; i<numberOfFlipFlops; i++)
		{	// PPO
			gut=net[flipFlops[i]]->inlis[0];
			if(gut->noutput!=1) continue;
			gut->stem=++numberOfStems;
			stems[numberOfStems].gate=gut->index;
			stems[numberOfStems].dominator=-1;
		}

		stack1->clear();

		// FFR region analysis 
		for(i=numberOfStems;i>0;i--)
		{
			gut=net[stems[i].gate];
			for(j=0;j<gut->ninput;j++)
				if(gut->inlis[j]->stem<=0) stack1->push(gut->inlis[j]);
			while(!stack1->isEmpty()) 
			{
				gut=stack1->pop();
				gut->stem=(-1)*i;
				for(j=0;j<gut->ninput;j++)
					if(gut->inlis[j]->stem<=0) stack1->push(gut->inlis[j]);
			}
		}

		return numberOfStems;
	}

	int FaultSimulation::setDominator()
	{
		int i,j,gCount,nDom=0;
		Gate *gut,*dominator;

		for(i=numberOfStems;i>0;i--)
		{
			gut=net[stems[i].gate];
			stems[i].fault[0]=stems[i].fault[1]=stems[i].fault[2]=0;
			if(i>rstem) continue;

			gCount=0;
			for(j=0;j<gut->noutput;j++)
			{
				pushGate(gut->outlis[j]);
				gCount++;
				gut->outlis[j]->changed=true;
			}

			for(j=0;j<=PPOlevel;j++)
			{
				while(!eventList[j]->isEmpty())
				{
					gut=eventList[j]->pop();
					gut->changed=false;
					if(gCount<=0) continue;
					if(--gCount==0) { stems[i].dominator=gut->index; break; }

					if(gut->noutput==0 || gut->fn==DFF) gCount=0;
					else if(gut->noutput>1)
					{
						if(stems[gut->stem].dominator==-1) gCount=0;
						else
						{
							dominator=net[stems[gut->stem].dominator];
							if(!dominator->changed)
							{
								pushGate(dominator);
								dominator->changed=true;
								gCount++;
							}
						}
					}
					else if(!gut->outlis[0]->changed)
					{
						pushGate(gut->outlis[0]);
						gut->outlis[0]->changed=true;
						gCount++;
					}
				}
			}
		}

		for(i=numberOfStems;i>0;i--)
		{
			if(stems[i].dominator>=0) nDom++;
			stems[i].checkup=0;
		}

		// Check-up criteria
		for(i=numberOfStems;i>0;i--)
		{
			gut=net[stems[i].gate];
			if(gut->noutput<=1) continue;
			if(stems[i].dominator>0)
			{
				gut=net[stems[i].gate];
				stems[i].dominator=stems[abs(net[stems[i].dominator]->stem)].gate;
				//			if((j=sizeSR(gut))<100)
				stems[i].checkup=1;
				continue;
			}

			gCount=0;

			for(j=0;j<gut->noutput;j++)
			{
				if(gut->outlis[j]->ninput==1)
				{
					gCount=1;
					if(gut->outlis[j]->fn==DFF) gCount=1;

					/*				for(k=0;k<gut->outlis[j]->noutput;k++)
					{
					temp=gut->outlis[j]->outlis[k];
					if(temp->ninput==1) gcount=1;
					if(temp->fn==DFF) gcount=1;
					if(temp->ninput==1 && temp->stem>0 ) gcount=1;
					}*/
				}
			}

			if(gCount==0) stems[i].checkup=1;

			/*		for(j=0;j<gut->noutput;j++)
			if(gut->outlis[j]->ninput==1)

			for(k=0;k<gut->outlis[j]->noutput;k++)
			{
			temp=gut->outlis[j]->outlis[k];
			if(temp->ninput==1 && temp->noutput>0) gcount=1;
			}
			if(gcount==0) stems[i].checkup=1; */
		}

		return nDom;
	}

	void FaultSimulation::initFaultSim()
	{
		int i;
		Gate *gut;
		Fault *cf;
		Event *e1;

		for(i=0;i<numberOfGates;i++)
		{
			gut=net[i];
			gut->SGV=X;
			setx(&gut->GV[0],&gut->GV[1]);
			setx(&gut->FV[0],&gut->FV[1]);

			gut->gid=groupID;
			gut->changed=false;
		}

		FaultIter current,final;

		current=hopeFaultList.begin();
		final=hopeFaultList.end();

		while(current!=final)
		{
			cf=*current;
			if(cf->detected!=REDUNDANT) cf->detected=UNDETECTED;
			while(cf->event.size()!=0)
			{
				e1=cf->event.front();
				delete e1;
				cf->event.pop_front();
			}
			current++;
		}
		potentialFault=hopeFaultList.back();

		for(i=1;i<=numberOfStems;i++)
		{
			stems[i].fault[ZERO]=stems[i].fault[X]=stems[i].fault[ONE]=NULL;
			stems[i].flag[ZERO]=stems[i].flag[X]=stems[i].flag[ONE]=UNSIMULATED;
		}

		undfault=new Fault();
		undfault->gate=0;
		undfault->line=0;
		undfault->type=0;
		undfault->detected=UNDETECTED;
		undfault->event.clear();

		stack1->clear();
		stack2->clear();
	}

	Gate* FaultSimulation::injectFault(Gate *gate,int ftype,int fline,int bit)
	{
		int i;
		Gate* gut,*temp;
		Event* event;	
		int val[2];

		if(gate->fn < FAULTY)
		{
			sut[bit].fn=gate->fn;
			sut[bit].papa = -1;
		} else
		{
			sut[bit].fn=sut[gate->fn-FAULTY].fn;
			sut[bit].papa=gate->fn-FAULTY;
		}
		gate->fn=bit+FAULTY;

		// Flip-Flops 
		list<Event*>::iterator current,final;

		current=sut[bit].event.begin();
		final=sut[bit].event.end();

		while(current!=final)
		{
			hopeEventList.push_back(*current);
			event=*current;

			gut=net[event->node];
			if(gut->gid!=groupID)
			{
				gut->gid=groupID;
				copyLevel(gut->FV,gut->GV);
			}

			if(bitb(event->value,0)) 
				gut->FV[0]=resetb(gut->FV[0],bit);
			else
				gut->FV[0]=setb(gut->FV[0],bit);
			if(bitb(event->value,1))
				gut->FV[1]=resetb(gut->FV[1],bit);
			else
				gut->FV[1]=setb(gut->FV[1],bit);

			for(i=0;i<gut->noutput;i++)
			{
				temp=gut->outlis[i];
				if(!temp->changed)
				{
					pushGate(temp);
					temp->changed=true;
				}
			}
			current++;
		}

		// Faulty gate
		if(fline<0)
		{	 // output line fault
			if(gate->gid!=groupID)
			{
				gate->gid=groupID;
				copyLevel(gate->FV,gate->GV);

			}
			copyLevel(val,gate->FV);

			switch(ftype)
			{
			case SA0: 
				val[0] |=  BITMASK[bit];
				val[1] &= ~BITMASK[bit];
				break;
			case SA1:
				val[0] &= ~BITMASK[bit];
				val[1] |=  BITMASK[bit];
				break;
			default:
				val[0] &= ~BITMASK[bit];
				val[1] &= ~BITMASK[bit];
			}

			if(cmpLevel(val,gate->FV))
			{
				copyLevel(gate->FV,val);
				for(i=0;i<gate->noutput;i++)
				{
					temp=gate->outlis[i];
					if(!temp->changed)
					{
						pushGate(temp);
						temp->changed=true;
					}
				}
			}
		} else
		{	// input line fault
			if(gate->changed) return 0;
			temp=gate->inlis[fline];
			if(temp->gid==groupID)
			{
				val[0]=temp->FV[0] & BITMASK[bit];
				val[1]=temp->FV[1] & BITMASK[bit];
			} else
			{
				val[0]=temp->GV[0] & BITMASK[bit];
				val[1]=temp->GV[1] & BITMASK[bit];
			}

			val[0]= val[0]!=ALL0 ? ZERO : val[1]!=ALL0 ? ONE : X;
			if(val[0]!=ftype) {pushGate(gate); gate->changed=true;};
		}

		return 0;
	}

	void FaultSimulation::faultyGateEval(Gate *gut,level *val)
	{
		int bit,fn;
		level v;
		Gate *temp;

		fn=gut->fn;
		for(bit=fn-FAULTY,gut->fn=sut[bit].fn; bit>=0; bit=sut[bit].papa)
		{
			if(sut[bit].line>=0)
			{
				temp=gut->inlis[sut[bit].line];
				if(temp->gid!=groupID)
				{
					copyLevel(temp->FV,temp->GV);
					temp->gid=groupID;
				}

				if(temp->changed<3)
				{
					copyLevel(sut[bit].Val,temp->FV);
					temp->changed+=3;
				}
				switch(sut[bit].type)
				{
				case SA0:
					temp->FV[0] |=  BITMASK[bit];
					temp->FV[1] &= ~BITMASK[bit];
					break;
				case SA1:
					temp->FV[0] &= ~BITMASK[bit];
					temp->FV[1] |=  BITMASK[bit];
					break;
				default:
					temp->FV[0] &= ~BITMASK[bit];
					temp->FV[1] &= ~BITMASK[bit];
					break;
				}
			}
		}

		feval(gut,val,&v,groupID);

		for(bit=fn-FAULTY,gut->fn=fn; bit>=0; bit=sut[bit].papa)
		{
			if(sut[bit].line>=0)
			{
				temp=gut->inlis[sut[bit].line];
				if(temp->changed>=3)
				{
					copyLevel(temp->FV,sut[bit].Val);
					temp->changed-=3;
				}
			} else
			{
				switch(sut[bit].type)
				{
				case SA0:
					val[0] |= BITMASK[bit];
					val[1] &=~BITMASK[bit];
					break;
				case SA1:
					val[0] &=~BITMASK[bit];
					val[1] |= BITMASK[bit];
					break;
				default:
					val[0] &=~BITMASK[bit];
					val[1] &=~BITMASK[bit];
				}
			}
		}
	}

	void FaultSimulation::feval(Gate *gut,int *val,int *v,int ggid)
	{
		Gate *temp;
		int j;
		switch(gut->fn)
		{
		case NOT:
			if(gut->inlis[0]->gid==ggid)
			{
				val[0]=gut->inlis[0]->FV[1];
				val[1]=gut->inlis[0]->FV[0];
			} else
			{
				val[0]=gut->inlis[0]->GV[1];
				val[1]=gut->inlis[0]->GV[0];
			}
			break;
		case AND:
			if(gut->inlis[0]->gid==ggid)
			{copyLevel(val,gut->inlis[0]->FV);}
			else
			{copyLevel(val,gut->inlis[0]->GV);}

			for(j=1;j<gut->ninput;j++)
				if(gut->inlis[j]->gid==ggid) 
				{ 
					val[0] |= gut->inlis[j]->FV[0];
					val[1] &= gut->inlis[j]->FV[1];
				} else
				{
					val[0] |= gut->inlis[j]->GV[0];
					val[1] &= gut->inlis[j]->GV[1];
				}
				break;
		case NAND:
			if(gut->inlis[0]->gid==ggid)
			{copyLevel(val,gut->inlis[0]->FV);}
			else 
			{copyLevel(val,gut->inlis[0]->GV);}

			for(j=1;j<gut->ninput;j++)
				if(gut->inlis[j]->gid==ggid)
				{
					val[0] |= gut->inlis[j]->FV[0];
					val[1] &= gut->inlis[j]->FV[1];
				} else
				{					
					val[0] |= gut->inlis[j]->GV[0];
					val[1] &= gut->inlis[j]->GV[1];
				}
				*v=val[0]; val[0]=val[1]; val[1]=*v;
				break;
		case OR:
			if(gut->inlis[0]->gid==ggid)
			{copyLevel(val,gut->inlis[0]->FV);}
			else
			{copyLevel(val,gut->inlis[0]->GV);}

			for(j=1;j<gut->ninput;j++)
				if(gut->inlis[j]->gid==ggid) 
				{
					val[0] &= gut->inlis[j]->FV[0];
					val[1] |= gut->inlis[j]->FV[1];
				} else
				{
					val[0] &= gut->inlis[j]->GV[0];
					val[1] |= gut->inlis[j]->GV[1];
				}
				break;
		case NOR:
			if(gut->inlis[0]->gid==ggid)
			{copyLevel(val,gut->inlis[0]->FV);}
			else
			{copyLevel(val,gut->inlis[0]->GV);}

			for(j=1;j<gut->ninput;j++)
				if(gut->inlis[j]->gid==ggid)
				{
					val[0] &= gut->inlis[j]->FV[0];
					val[1] |= gut->inlis[j]->FV[1];
				} else 
				{ 
					val[0] &= gut->inlis[j]->GV[0];
					val[1] |= gut->inlis[j]->GV[1];
				}
				*v=val[0]; val[0]=val[1]; val[1]=*v;
				break;
		case XOR:
			if(gut->inlis[0]->gid==ggid)
			{copyLevel(val,gut->inlis[0]->FV);}
			else
			{copyLevel(val,gut->inlis[0]->GV);}

			for(j=1;j<gut->ninput;j++)
			{
				temp=gut->inlis[j];
				*v=val[0];
				if(temp->gid==ggid)
				{
					val[0] = (*v&temp->FV[0])|(val[1]&temp->FV[1]);
					val[1] = (val[1]&temp->FV[0])|(*v&temp->FV[1]);
				} else 
				{
					val[0] = (*v&temp->GV[0])|(val[1]&temp->GV[1]);
					val[1] = (val[1]&temp->GV[0])|(*v&temp->GV[1]);
				}
			}
			break;
		case XNOR:
			if(gut->inlis[0]->gid==ggid)
			{copyLevel(val,gut->inlis[0]->FV);}
			else
			{copyLevel(val,gut->inlis[0]->GV);}

			for(j=1;j<gut->ninput;j++)
			{
				temp=gut->inlis[j];
				*v=val[0];
				if(temp->gid==ggid)
				{
					val[0] = (*v&temp->FV[0])|(val[1]&temp->FV[1]);
					val[1] = (val[1]&temp->FV[0])|(*v&temp->FV[1]);
				} else
				{
					val[0] = (*v&temp->GV[0])|(val[1]&temp->GV[1]);
					val[1] = (val[1]&temp->GV[0])|(*v&temp->GV[1]);
				}
			}
			*v=val[0]; val[0]=val[1]; val[1]=*v;
			break;
		case DUMMY:
		case PO:
		case BUFF:
			if(gut->inlis[0]->gid==ggid)
			{copyLevel(val,gut->inlis[0]->FV);}
			else
			{copyLevel(val,gut->inlis[0]->GV);}
			break;
		default:
			faultyGateEval(gut,val);
			break;
		}
	}

	void FaultSimulation::faultSim(int start,int stop,int gid)
	{
		int i,j;
		Gate *gut,*temp;
		level val[2],v;

		for(i=start;i<stop;i++)
			while(!eventList[i]->isEmpty())
			{
				gut=eventList[i]->pop();
				gut->changed=false;
				feval(gut,val,&v,gid);

				if(cmpLevel(gut->GV,val))
				{
					copyLevel(gut->FV,val);
					gut->gid=gid;
					for(j=gut->noutput-1;j>=0;j--)
					{
						temp=gut->outlis[j];
						if(!temp->changed)
						{
							pushGate(temp);
							temp->changed=true;
						}
					}
				}
			}
	}

	int FaultSimulation::dropDetectedFaults()
	{
		int i;
		level s=ALL0; // signature for detection
		level sx=ALL0;	// signature for potential detection
		Gate *gut;
		Fault *f;

		while(!eventList[POlevel]->isEmpty())
		{
			gut=eventList[POlevel]->pop();
			gut->changed=false;
			if(gut->inlis[0]->gid==groupID)
			{copyLevel(gut->FV,gut->inlis[0]->FV);}
			else
			{copyLevel(gut->FV,gut->inlis[0]->GV);};
			gut->gid=groupID;

			if(gut->fn >= FAULTY)
			{
				for(i=gut->fn-FAULTY; i>=0; i=sut[i].papa)
				{
					switch(sut[i].type)
					{
					case SA0:
						gut->FV[0] |= BITMASK[i];
						gut->FV[1] &=~BITMASK[i];
						break;
					case SA1:
						gut->FV[0] &=~BITMASK[i];
						gut->FV[1] &= BITMASK[i];
						break;
					default:
						gut->FV[0] &=~BITMASK[i];
						gut->FV[1] &=~BITMASK[i];
					}
				}
				if(!cmpLevel(gut->FV,gut->GV)) continue;
			}

			s|=(gut->GV[0] & ~gut->GV[1] & ~gut->FV[0] & gut->FV[1]) |
				(~gut->GV[0] & gut->GV[1] & gut->FV[0] & ~gut->FV[1]);	
#ifndef ATALANTA
			sx|=(gut->GV[0] ^ gut->GV[1]) & (~gut->FV[0] & ~gut->FV[1]);
#endif		
		}
		if(s!=ALL0)
			for(i=0;i<nFut;i++)	//detected
				if((s & BITMASK[i])!=ALL0)
				{
					f=fut[i];
					f->detected=DETECTED;
				nDetected++;
#ifdef DIAGNOSIS
					if(dropmode=='y') 
					{
						hopeEventList.insert(hopeEventList.end(),f->event.begin(),f->event.end());
						f->event.clear();
					}
#else
					hopeEventList.insert(hopeEventList.end(),f->event.begin(),f->event.end());
					f->event.clear();
#endif
				}

#ifndef ATALANTA
				if(sx!=ALL0)
					for(i=0;i<nFut;i++)		//potentially detected
						if((sx & BITMASK[i])!=ALL0)
						{
							f=fut[i];
							f->npot+=1;
							if(f->detected==UNDETECTED)
							{
								f->detected=XDETECTED;
								if(xdetectmode=='y')
								{
									nDetected++;
#ifdef DIAGNOSIS
									if(dropmode=='y') 
									{
										hopeEventList.insert(hopeEventList.end(),f->event.begin(),f->event.end());
										f->event.clear();
									}
#else
									hopeEventList.insert(hopeEventList.end(),f->event.begin(),f->event.end());
									f->event.clear();
#endif
								}
							}
						}

#endif 


						return nDetected;
	}

	void FaultSimulation::storeFaultyStatus()
	{
		int i,node;
		Gate *gut;
		Event*	temp;
		Fault*	f;
		level s;
		level fVal[2];

		while(!eventList[PPOlevel]->isEmpty())
		{
			gut=eventList[PPOlevel]->pop();
			node=gut->index;
			gut->changed=false;

			if(gut->inlis[0]->gid==groupID)
			{copyLevel(fVal,gut->inlis[0]->FV);}
			else
			{copyLevel(fVal,gut->inlis[0]->GV);}

			if(gut->fn >= FAULTY)
			{
				for(i=gut->fn-FAULTY; i>=0; i=sut[i].papa)
				{
					if(sut[i].line<0) continue;
					switch(sut[i].type)
					{
					case SA0:
						fVal[0] |= BITMASK[i];
						fVal[1] &=~BITMASK[i];
						break;
					case SA1:
						fVal[0] &=~BITMASK[i];
						fVal[1] |= BITMASK[i];
						break;
					default:
						fVal[0] &=~BITMASK[i];
						fVal[1] &=~BITMASK[i];
						break;
					}
				}
			}

			gut=gut->inlis[0];

			if((s=(gut->GV[0]^fVal[0]) | (gut->GV[1]^fVal[1]))==ALL0) continue;

			for(i=0;i<nFut;i++)
			{
				f=fut[i];
				if((s & BITMASK[i]) == ALL0) continue;
#ifdef DIAGNOSIS
				if(dropmode=='n' || f->detected==UNDETECTED || 
#else
				if(f->detected==UNDETECTED || 
#endif
#ifdef ATALANTA
					f->detected==PROCESSED ||
#endif

					(xdetectmode=='n' && f->detected==XDETECTED))
				{
					if(hopeEventList.size()==0)
						temp=new Event;
					else
					{
						temp=hopeEventList.front();
						hopeEventList.pop_front();
					}
					temp->node=node;
					temp->value=ALL0;
					if(!bitb(fVal[0],i)) temp->value=setb(temp->value,0);
					if(!bitb(fVal[1],i)) temp->value=setb(temp->value,1);
					f->event.push_front(temp);
					if((s=resetb(s,i))==ALL0) break;
				}
			}
		}
	}

	Gate* FaultSimulation::sSimToDominator(Gate* gut,Gate *dom,int gid)
	{
		int i,j;
		Gate *temp;
		level val;
		int end;

		end=(dom==NULL) ? PPOlevel : dom->dpi+1;

		for(i=0;i<gut->noutput;i++)
		{
			temp=gut->outlis[i];
			if(!temp->changed) { pushGate(temp); temp->changed=true; }
		}

		for(i=(gut->dpi>=PPOlevel) ? 1 : gut->dpi;i<end;i++)
			while(!eventList[i]->isEmpty())
			{
				gut=eventList[i]->pop();
				gut->changed=false;
				val= gut->inlis[0]->gid==gid ?
					truthtbl1[gut->fn][gut->inlis[0]->FV[0]]:
				truthtbl1[gut->fn][gut->inlis[0]->SGV];

				if(gut->ninput>1)
					for(j=1;j<gut->ninput;j++)
						val= gut->inlis[j]->gid==gid ?
						truthtbl2[gut->fn][val][gut->inlis[j]->FV[0]] :
				truthtbl2[gut->fn][val][gut->inlis[j]->SGV];

				if(val!=gut->SGV)
				{
					gut->FV[0]=val;
					gut->gid=gid;
					if(gut==dom) return gut;
					for(j=0;j<gut->noutput;j++)
					{
						temp=gut->outlis[j];
						if(!temp->changed) {pushGate(temp); temp->changed=true;}
					}
				}
			}

			return 0;	
	}

	Gate* FaultSimulation::checkStem(Gate* gut,int gid)
	{
		int i,j;
		level val;
		Gate *temp;

		for(i=0;i<gut->noutput;i++)
		{
			temp=gut->outlis[i];
			val = (temp->inlis[0]->gid==gid) ?
				truthtbl1[temp->fn][temp->inlis[0]->FV[0]] :
			truthtbl1[temp->fn][temp->inlis[0]->SGV];
			for(j=1;j<temp->ninput;j++)
				val = (temp->inlis[j]->gid==gid) ? 
				truthtbl2[temp->fn][val][temp->inlis[j]->FV[0]] :
			truthtbl2[temp->fn][val][temp->inlis[j]->SGV];
			if(val != temp->SGV) return gut;
		}
		return 0;
	}

	Gate* FaultSimulation::checkSingleEvent(Fault *f,Gate *gut,int gid)
	{
		int i,j,k;
		level val;
		Event *event,*e1;
		Fault *g;
		Gate *temp;

		while(gut->stem<=0)
		{
			gut=gut->outlis[0];
			val= gut->inlis[0]->gid==gid ? 
				truthtbl1[gut->fn][gut->inlis[0]->FV[0]] :
			truthtbl1[gut->fn][gut->inlis[0]->SGV];
			if(gut->ninput>1)
				for(i=1;i<gut->ninput;i++)
					val=gut->inlis[i]->gid==gid ?
					truthtbl2[gut->fn][val][gut->inlis[i]->FV[0]] :
			truthtbl2[gut->fn][val][gut->inlis[i]->SGV];

			if(val==gut->SGV) return 0;
			gut->FV[0]=val ;
			gut->gid=gid;
		}

		if( (stems[gut->stem].checkup >= 1 ) && stems[gut->stem].flag[gut->FV[0]]==UNSIMULATED)
		{
			if((j=stems[gut->stem].dominator)>0)
			{	//dominator
				if((temp=sSimToDominator(gut,net[j],gid))!=0)
				{
					sStem=gut->stem;
					sSval=gut->FV[0];
					gut=temp;
				} else
				{
					stems[gut->stem].flag[gut->FV[0]]=SIMULATED;
					stems[gut->stem].fault[gut->FV[0]]=undfault;
					stack1->push(gut);
					return 0;
				}
			} 
			else if(checkStem(gut,gid)==0)
			{	//no dominator
				stems[gut->stem].flag[gut->FV[0]]=SIMULATED;
				stems[gut->stem].fault[gut->FV[0]]=undfault;
				stack1->push(gut);
				return 0;
			}
		}

		if(gut!=0)
		{
			j=gut->FV[0];
			if(gut->fn==PO)
			{	//PO
				if((k=whatIs(gut->GV[0],gut->GV[1])) != X)
				{
					if(j==X)
					{
#ifndef ATALANTA
						f->detected=XDETECTED;
						if(xdetectmode=='y') {
							nDetected++;
							
						}
#endif
					} else
					{
						f->detected=DETECTED;
						nDetected++;
					}
				}

				if(sStem>0)
				{
					stems[sStem].flag[sSval]=SIMULATED;
					stems[sStem].fault[sSval]=f;
					stack1->push(net[stems[sStem].gate]);
				}

				gut=0;
			} 
			else if(gut->noutput==1)
			{	//PPO
				if(hopeEventList.size()==0)
					event=new Event;
				else
				{
					event=hopeEventList.front();
					hopeEventList.pop_front();
				}

				event->node=gut->outlis[0]->index;
				event->value=ALL0;
				if(!bitb(PLEVELTBL[j][0],0)) event->value=setb(event->value,0);
				if(!bitb(PLEVELTBL[j][1],0)) event->value=setb(event->value,1);	

				f->event.clear();
				f->event.push_front(event);

				if(sStem>0)
				{
					stems[sStem].flag[sSval]=SIMULATED;
					stems[sStem].fault[sSval]=f;
					stack1->push(net[stems[sStem].gate]);
				}
				gut=0;

			} else
			{ // Check the stem is already simulated
				switch(k=stems[gut->stem].flag[j])
				{
				case UNSIMULATED: break;
				case SIMULATED:
					{
						g=stems[gut->stem].fault[j];

						list<Event*>::iterator current,final;
						current=g->event.begin();
						final=g->event.end();

						while(current!=final)
						{
							if(hopeEventList.size()==0)
								e1=new Event;
							else
							{
								e1=hopeEventList.front();
								hopeEventList.pop_front();
							}
							e1->node=(*current)->node;
							e1->value=(*current)->value;
							f->event.push_front(e1);
							current++;
						}

						switch(g->detected)
						{
						case UNDETECTED: break;
						case DETECTED:
							f->detected=DETECTED;
							nDetected++;
							break;
#ifndef ATALANTA	
						default:
							f->detected=XDETECTED;
							if(xdetectmode=='y') {
								nDetected++;
							}
#endif
						}

						if(sStem>0)
						{
							stems[sStem].flag[sSval]=SIMULATED;
							stems[sStem].fault[sSval]=g;
							stack1->push(net[stems[sStem].gate]);
						}
						gut=0;
						break;
					}
				default:
					sut[k].extra.push_front(f);
					if(sStem>0)
					{
						stems[sStem].flag[sSval]=k;
						stems[sStem].fault[sSval]=f;
						sStems[++nsStems].stem=sStem;
						sStems[nsStems].val=sSval;
						stack1->push(net[stems[sStem].gate]);
					}
					gut=0;
				}
			}
		}

		return gut ;
	}

	FaultIter FaultSimulation::selectOneFault(FaultIter current)
	{
		int i;
		Fault *cf,*f;

		// Select upto 32 faults to be simulated and inject them
		i=0;

		while(i<SIZEOFFUT)
		{
			cf=*current;
			current++;
			if(current==hopeFaultList.end()) {current--;break;}
			f=*current;

			if( f->detected==DETECTED || f->detected==REDUNDANT || (f->detected=='y' && f->detected==XDETECTED))
			{
				current = hopeFaultList.erase(current);			
				//current--;
				if(f==potentialFault) potentialFault=*current;
				continue;
			}

			fut[i]=f;
			sut[i].gate=f->gate;
			sut[i].type=f->type;
			sut[i].line=f->line;
			sut[i].event=f->event;

			f->event.clear();

			i++;
		}

		if((nFut=i)==0) return current;	// end of fault simulation

		// Schedule faulty events 
		groupID++;

		for(i=0;i<nFut;i++)
		{
			injectFault(sut[i].gate,sut[i].type,sut[i].line,i);
		}

		return current;
	}


	FaultIter FaultSimulation::selectNextFaults(FaultIter current)
	{
		int i,j,k;
		Gate *gut;
		Fault *cf,*f;
		Event *event;


		// Select upto 32 faults to be simulated and inject them
		i=0;


		while(i<SIZEOFFUT)
		{
			cf=*current;
			current++;
			if(current==hopeFaultList.end()) {
				current--;
				break;
			}
			f=*current;

			if( f->detected==DETECTED || f->detected==REDUNDANT || (f->detected=='y' && f->detected==XDETECTED))
			{
				current = hopeFaultList.erase(current);			
				current--;
				if(f==potentialFault) potentialFault=cf;
				continue;
			}

			// Dynamic fault ordering
			if(dynamicOrderFlag)
			{
				if(cf==potentialFault || f==potentialFault) 
					dynamicOrderFlag=false;
				else {
					if(f->npot>0)
					{
						current = hopeFaultList.erase(current);
						//current--;
						hopeFaultList.push_back(f);
						continue;
					}
				}
			}

			if(f->event.empty())
			{	//single event fault
				// Check 1: fulty gate output
				gut=f->gate;
				k=FAULTYVALUE[f->type];

				if(f->line<0)
				{
					if(gut->SGV==k) continue;
				}
				else
				{
					if(gut->inlis[f->line]->SGV==k) continue;

					if(gut->fn==DFF)
					{
						if(hopeEventList.size()==0)
							event=new Event;
						else
						{	
							event=hopeEventList.front();
							hopeEventList.pop_front();
						}
						event->node=gut->index;
						event->value=LEVELTOEVENT[k];
						f->event.push_front(event);
						continue;
					}

					//try {
					k=truthtbl1[gut->fn][k];
					for(j=0;j<gut->ninput;j++) {
						if(j!=f->line) {
							k=truthtbl2[gut->fn][k][gut->inlis[j]->SGV];
						}
					}
					if(k==gut->SGV)	continue;
					/*}
					catch (...) {
					k = 1;
					}*/
				}

				gut->FV[0]=k;
				gut->gid = ++groupID;
				sStem=-1;
				if((gut=checkSingleEvent(f,gut,groupID))==0) continue;

				fut[i]=f;
				sut[i].gate=gut;
				j=gut->FV[0];
				sut[i].type=j;
				sut[i].line=OUTFAULT;
				sut[i].event.clear();
				stack1->push(gut);

				stems[gut->stem].flag[j]=i;
				stems[gut->stem].fault[j]=f;

				if(sStem>0)
				{
					stems[sStem].flag[sSval]=i;
					stems[sStem].fault[sSval]=f;
					sStems[++nsStems].stem=sStem;
					sStems[nsStems].val=sSval;
					stack1->push(net[stems[sStem].gate]);
				}	
			} else
			{
				fut[i]=f;
				sut[i].gate=f->gate;
				sut[i].type=f->type;
				sut[i].line=f->line;
				sut[i].event=f->event;
				f->event.clear();
			}

			i++;
		}

		if((nFut=i)==0) return current;	// end of fault simulation

		// Schedule faulty events 
		groupID++;

		for(i=0;i<nFut;i++)
		{
			injectFault(sut[i].gate,sut[i].type,sut[i].line,i);
		}

		return current;
	}

	int FaultSimulation::simulation()
	{
		int i;
		Fault *f;
		Event *e,*t;
		Gate *gut;

		nDetected=0;

		list<Fault*>::iterator current;
		current=hopeFaultList.begin();

#ifdef DIAGNOSIS
		if(dictmode=='y') dynamicOrderFlag=false;
		else
#endif
			if(*current==potentialFault) 
				dynamicOrderFlag=false;
			else {
				current++;
				if(*current==potentialFault) dynamicOrderFlag=false; ///////////////////////////
			  else dynamicOrderFlag=true;
				current--;
			}

			while(current!=hopeFaultList.end())
			{
				if(current == hopeFaultList.begin()) {
					current--;
				}
				current = 
#ifdef DIAGNOSIS
					(diagmode=='y') ? selectOneFault(current) :
#endif
				selectNextFaults(current);
				if(nFut==0) break;
				faultSim(0,maxlevel,groupID);		// box 9
				dropDetectedFaults();			// box 10 
				storeFaultyStatus();			// box 11 

				while(nsStems>=0)
				{
					stems[sStems[nsStems].stem].flag[sStems[nsStems].val]=SIMULATED;
					nsStems--;
				}

				for(i=0;i<nFut;i++)
				{
					sut[i].gate->fn=sut[i].fn;

					if(sut[i].event.size()!=0) continue;
					stems[sut[i].gate->stem].flag[sut[i].type]=SIMULATED;

					while(sut[i].extra.size()!=0)
					{
						f=sut[i].extra.front();

						list<Event*>::iterator current,final;

						current=fut[i]->event.begin();
						final=fut[i]->event.end();

						while(current!=final)
						{
							e=*current;
							if(hopeEventList.size()==0)
								t=new Event;
							else
							{
								t=hopeEventList.front();
								hopeEventList.pop_front();
							}
							t->node=e->node;
							t->value=e->value;
							f->event.push_front(t);
						}

						switch(fut[i]->detected)
						{
						case DETECTED:
							f->detected=DETECTED;
							nDetected++;
							break;
#ifndef ATALANTA
						case XDETECTED:
							f->detected=XDETECTED;
							if(xdetectmode=='y') {
								nDetected++;
							}
#endif
						}

						sut[i].extra.pop_front();
					}

				}
			}

			while(!stack1->isEmpty())
			{
				gut=stack1->pop();
				stems[gut->stem].flag[0]=
					stems[gut->stem].flag[1]=
					stems[gut->stem].flag[2]=UNSIMULATED;
			}

			if(groupID>MAXINTEGER)
			{
				groupID=0;
				for(i=0;i<numberOfGates;i++) net[i]->gid=0;
			}

			return nDetected;
	}
#endif

	/*FaultSimulation::~FaultSimulation() 
	{

	}*/

}
