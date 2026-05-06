// Fault.cpp: implementation of the Fault class.
//
//////////////////////////////////////////////////////////////////////
#include <list>
#include <string>
#include <iostream>
#include <sstream>
#include <fstream>

#include "Error.h"

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

namespace atalantadll {

	//////////////////////////////////////////////////////////////////////
	// Construction/Destruction
	//////////////////////////////////////////////////////////////////////

	Fault::Fault()
	{

	}

	void Fault::printFault(fstream *fp,bool mode, char cctMode)
	{
		Gate* gut;

#ifdef ISCAS85_NETLIST_MODE
		if(cctMode==ISCAS89)
		{
			if(line >= 0)
			{
				gut = gate->inlis[line];
				*fp<<gut->symbol->symbol.c_str()<<"->";
			}

			*fp<<gate->symbol->symbol.c_str();
			*fp<<fault2str[type];
		} else
		{
			if(line<0)
				*fp<<"Output line s-a-"<<type;
			else
				*fp<<"Input line "<<(line+1)<<"s-a-"<<type;
			*fp<<"of gate "<<gate->gid;
		}
#else
		if(line >= 0)
		{
			gut = gate->inlis[line];
			*fp<<gut->symbol->symbol.c_str()<<"->";
		}
		*fp<<gate->symbol->symbol.c_str();
		*fp<<" ";
		*fp<<fault2str[type];
#endif
		if(mode)
			switch(detected)
		{
			case DETECTED: *fp<<" detected"; break;
			case UNDETECTED: *fp<<" undetected"; break;
			case PROCESSED: *fp<<" aborted"; break;
			case REDUNDANT: *fp<<" redundant";
		}
		*fp<<endl;
	}

	Fault::~Fault()
	{

	}

	//	addFault
	//	Add a fault to the linked list of each gate.

	void Fault::addFault()
	{
		gate->pfault.push_back(this);
	}

	//	restore_detected_fault_list
	//	Restores the fault list for test compaction.
	//	Does not restore redundant faults.

	int FaultList::restoreDetectedFaultList()
	{
		Fault *f;
		int n=0;

		for(int i=0;i<numberOfFaults;i++)
		{
			f=faultList[i];
			if(f->detected==DETECTED)
			{
				f->detected=UNDETECTED;
				f->addFault();
				n++;
			}
		}

		return n;
	}

	int FaultList::checkRedundantFaults()
	{
		Gate *gut;
		Fault *f;

		int n=0, j;

		for(int i=0;i<numberOfGates;i++)
			if(net[i]->noutput>1)
			{
				gut=net[i];
				for(j=0;j<gut->noutput;j++) gut->outlis[j]->changed++;
				for(j=0;j<gut->noutput;j++)
				{
					if(gut->outlis[j]->changed>1)
					{
						list<Fault*>::iterator current,final;

						current=gut->outlis[j]->pfault.begin();
						final=gut->outlis[j]->pfault.end();

						while(current!=final)
						{
							f=*current;
							if(f->line>=0)
							{
								if(f->gate->inlis[f->line]==gut)
								{
									f->detected=REDUNDANT;
									current=f->gate->pfault.erase(current);		
									//current--;
									n++;
								} else current++;
							} else current++;
						}
					}
					gut->outlis[j]->changed=0;
				}
			}
			return n;
	}

	int FaultList::setAllFaultList(int noStem,Gate **stem)
	{
		Gate *g;
		Fault* p;

		Fault *current;
		Fault *curr;
		fault_type f;
		int nfault,n,nof,i;
		int *test,size;

		test=new int;
		size=sizeof(Fault);
		nfault=0;
		curr=new Fault;
		current=new Fault;

		for(i=0;i<numberOfGates;i++)
		{
			g=net[i];

			/* if the input of the gate has more than one fanouts, 
			add a s-a-1 for each AND/NAND,
			a s-a-0 for each OR/NOR and
			a s-a-0 and s-a-1 for other gates. */

			if(g->ninput>1)
			{
				f=(g->fn==AND || g->fn==NAND) ? SA1 : SA0;
				for(int j=0;j<g->ninput;j++)
				{
					if(g->inlis[j]->noutput>1)
					{
						p=new Fault;
						p->gate=g;
						p->type=f;
						p->line=j;
						nfault++;
						g->pfault.push_back(p);

						/* case of high level gates */
						if(g->fn>PI)
						{
							p=new Fault();
							p->gate=g;
							p->type=(f==SA1) ? SA0 : SA1;
							p->line=j;
							nfault++;

							g->pfault.push_back(p);
						}
					}
				}
			}

			if(	(g->noutput==1) &&
				(g->outlis[0]->ninput>1 || g->outlis[0]->fn==PO))
			{
				f=(g->outlis[0]->fn==OR || g->outlis[0]->fn==NOR) ? SA0 : SA1;
				p=new Fault;
				p->gate=g;
				p->type=f;
				p->line=OUTFAULT;
				nfault++;
				g->pfault.push_back(p);

				// case of high level gates 
				if(g->outlis[0]->fn>PI)
				{
					p=new Fault;
					p->gate=g;
					p->type=(f==SA1) ? SA0 : SA1;
					p->line=OUTFAULT;
					nfault++;
					g->pfault.push_back(p);
				}
			} else if(g->noutput>1)
			{
				p=new Fault();
				p->gate=g;
				p->type=SA1;
				p->line=OUTFAULT;
				nfault++;
				g->pfault.push_back(p);

				p=new Fault();
				p->gate=g;
				p->type=SA0;
				p->line=OUTFAULT;
				nfault++;
				g->pfault.push_back(p);
			} else if( g->fn==PO && g->inlis[0]->noutput>1)
			{
				p=new Fault();
				p->gate=g;
				p->type=SA1;
				p->line=0;
				nfault++;
				g->pfault.push_back(p);

				p=new Fault();
				p->gate=g;
				p->type=SA0;
				p->line=0;
				nfault++;
				g->pfault.push_back(p);
			}
		}

		// create the fault_list and
		// enumerate faults in each fanout free region 

		faultList=new Fault*[nfault];
		stack->clear();

		nof=0;
		for(i=noStem-1;i>=0;i--)
		{
			stack->push(stem[i]);
			n=1;
			while(!stack->isEmpty())
			{
				g=stack->pop();

				list<Fault*>::iterator current,final;

				current=g->pfault.begin();
				final=g->pfault.end();

				while(current!=final)
				{
					faultList[nof++]=*current;
					current++;
					n++;
				}
				for(int j=0;j<g->ninput;j++)
					if(g->inlis[j]->noutput==1) stack->push(g->inlis[j]);
			}
			stem[i]->dfault=new Fault*[n];
		}

		if(nfault==nof) return nfault; else return -1;
	}

#ifdef INCLUDE_HOPE

	void FaultList::setParity(Gate *gut,int par) {gut->changed=inverseParity[parityOfGate[gut->fn]][par];}
	void FaultList::mark(Gate *gut) {gut->changed+=2;}
	bool FaultList::isStem(Gate *gut) {return ((gut->noutput != 1) || (gut->outlis[0]->fn==DFF));}
	bool FaultList::isNotMarked(Gate *gut) {return gut->changed<2;}

	void FaultList::insertFault(Gate *gut,int line,fault_type type)
	{
		int parity;
		Fault *f;

		parity = (gut->changed>=2) ? gut->changed-2 : gut->changed;
		if(line<0) parity = inverseParity[parityOfGate[gut->fn]][parity];

		f=new Fault();
		f->gate=gut;
		f->line=line;
		f->type=type;
		f->npot=0;

		numberOfFaults++;

		if((parity==0 && type==SA0) || (parity==1 && type==SA1))
			evenList->push_back(f);
		else
			oddList->push_back(f);
	}

	void FaultList::defaultLineFault(Gate *gut,int line)
	{
		Gate *from,*to;

		if(line<0)
		{
			//output line fault
			if(gut->fn==DUMMY || gut->fn==PO) return;
			if(gut->noutput!=1)
			{
				insertFault(gut,OUTFAULT,SA0);
				insertFault(gut,OUTFAULT,SA1);
			} else
			{
				to=gut->outlis[0];
				if(to->fn==DUMMY) to=to->outlis[0];
				switch(to->fn)
				{
				case AND:
				case NAND: 
					if(to->ninput>1) insertFault(gut,OUTFAULT,SA1); break;
				case OR:
				case NOR: 
					if(to->ninput>1) insertFault(gut,OUTFAULT,SA0); break;
				case XOR:
				case XNOR:
				case DFF:
				case PO:
					insertFault(gut,OUTFAULT,SA0);
					insertFault(gut,OUTFAULT,SA1);
					break;
				}
			}
		} else 
		{
			from=gut->inlis[line];
			if(from->fn==DUMMY || from->fn==PO) from=from->inlis[0];
			if(from->noutput>1)
				switch(gut->fn)
			{
				case AND:
				case NAND:
					if(gut->ninput>1) insertFault(gut,line,SA1); break;
				case OR:
				case NOR:
					if(gut->ninput>1) insertFault(gut,line,SA0); break;
				case XOR:
				case XNOR:
				case DFF:
				case PO:
					insertFault(gut,line,SA0);
					insertFault(gut,line,SA1);
					break;
			}
		}
	}

	void FaultList::FFRfault(Gate *gut)
	{
		Gate* temp;
		oddList=new list<Fault*>;
		evenList=new list<Fault*>;

		stack1->clear();
		stack1->push(gut);

		while(!stack1->isEmpty())
		{
			gut=stack1->pop();
			defaultLineFault(gut,OUTFAULT);
			for(int ix=0;ix<gut->ninput;ix++)
			{
				temp=gut->inlis[ix];
				if(isStem(temp))
					defaultLineFault(gut,ix);
				else
					stack1->push(temp);
			}
		}

		if(evenList->size() > 0) {
		  hopeFaultList.insert(hopeFaultList.end(),evenList->begin(),evenList->end());
		}
		if(oddList->size() > 0) {
			hopeFaultList.insert(hopeFaultList.end(),oddList->begin(),oddList->end());
		}

		delete oddList;
		delete evenList;
	}

	void FaultList::DFSpo(Gate *parent,Gate *child)
	{
		//preWORK
		setParity(child,(parent==0 ? 0 : parent->changed-2));
		mark(child);

		if(isStem(child)) FFRfault(child);

		// Go into children 
		for(int i=0;i<child->ninput;i++)
		{
			// preWORK for input lines
			if(isNotMarked(child->inlis[i]))
				DFSpo(child,child->inlis[i]);
		}
	}

	void FaultList::FWDfaults()
	{
		Gate *gut;
		int i;

		for(i=0;i<numberOfGates;i++) net[i]->changed=0;

		//   init_fault_list();

		// Primary Outputs
		for(i=0;i<numberOfPrimaryOutputs;i++)
		{
			gut=net[primaryOut[i]];
			DFSpo(0,gut);
		}

		// count faults and copy
		numberOfFaults=hopeFaultList.size();

		faultList=new Fault*[numberOfFaults];

		list<Fault*>::iterator current,final;

		current=hopeFaultList.begin();
		final=hopeFaultList.end();

		i=0;
		while(current!=final)
		{
			faultList[i]=*current;
			i++;
			current++;
		}
	}

	int FaultList::restoreHopeFaultList()
	{
		Fault *f;
		int i,n;

		n=0;

		hopeFaultList.clear();

		for(i=0;i<numberOfFaults;i++)
		{
			f=faultList[i];
			if(f->detected==DETECTED)
			{
				f->detected=UNDETECTED;
				hopeFaultList.push_back(f);
				n++;
			}
			if(!f->event.empty())
			{
				hopeEventList.insert(hopeEventList.begin(),f->event.begin(),f->event.end());
				f->event.clear();
			}
		}
		return n;
	}

#endif

	char ReadableFaultList::getFaultSymbol(string *s)
	{
		char c = 0;
		int n=0;
		status valid=false;

		s->clear();
		while(!inputf->eof())
		{
			c = inputf->get();
			if(c == -1) continue;
			
			if(isWhitespace(c)) {if(valid) break; else continue;}
			if(isHeadSymbol(c)) {s->append(1, c);continue;}
			if(isValid(c)) {s->append(1, c); valid=true;}
			else Error::fatalerror(FAULTERROR);
		};
		if(inputf->eof()) {
			c=EOF;
		}
		//s[n]=EOS;
		return c;
	}

	void ReadableFaultList::readFaults(std::streambuf *fn)
	{
		if (fn != NULL)
		{
			istream fault(fn);

			if(simMode=='f')
				numberOfFaults = readFaultsFsim(&fault,myNumberOfStems,myStem);
#ifdef INCLUDE_HOPE
			else readFaultsHope(&fault);
#endif

			if(numberOfFaults<0)
			{
				/*cerr<<"Fatal error: error in setting fault list"<<endl;
				exit(0);*/
				stringstream ss;
				ss << "Fatal error: error in setting fault list";
				throw ss.str();
			}
		}
	}

	int ReadableFaultList::readFaultsFsim(istream *file,int noStem,Gate **stem)
	{
		inputf=file;

		Gate *gut;
		Fault *f;
		HashData *h;
		int from,to,line,type;
		string s; //char s[MAXSTRING];
		int nfault,n,nof;

		nfault=0;

		while(getFaultSymbol(&s)!=EOF)
		{
			if(isValid(s[0]))
			{
				if((h=hashTable.findHash(&s,0)) ==0)
				{
					cout<<"Error in fault file:";
					cout<<s;
					cout<<" is not defined\n";
					Error::fatalerror(FAULTERROR);
				}
				if((to=h->pnode->index) < 0) Error::fatalerror(FAULTERROR);
				gut=net[to];
				line=-1;
			} else if(s[0]=='>')
			{
				from=to;
				if((h=hashTable.findHash(&string(&s[1]),0)) ==0)
				{
					cout<<"Error in fault file:";
					cout<<s;
					cout<<" is not defined\n";
					Error::fatalerror(FAULTERROR);
				}
				if((to=h->pnode->index)<0) Error::fatalerror(FAULTERROR);
				gut=net[to];
				for(int i=0;i<gut->ninput;i++)
					if(gut->inlis[i]->index==from) {line=i; break;};
			} else if(s[0]=='/')
			{
				if(s[1]=='1') type=SA1; else type=SA0;
				if(line>=0)
					type=(type==SA1) ? SA1 : SA0;

				f=new Fault;
				f->gate=gut;
				f->line=line;
				f->type=type;
				gut->pfault.push_front(f);
				nfault++;
			} else Error::fatalerror(FAULTERROR);
		}

		// create the fault_list and
		// enumerate faults in each fanout free region
		faultList=new Fault*[nfault];
		stack->clear();

		nof=0;
		for(int i=noStem-1;i>=0;i--)
		{
			stack->push(stem[i]);
			n=1;
			while(!stack->isEmpty())
			{
				gut=stack->pop();

				list<Fault*>::iterator current,final;

				current=gut->pfault.begin();
				final=gut->pfault.end();

				while(current!=final)
				{
					faultList[nof++]=*current;
					n++;
					current++;
				}
				for(int j=0;j<gut->ninput;j++)
					if(gut->inlis[j]->noutput==1) stack->push(gut->inlis[j]);
			}
			stem[i]->dfault=new Fault*[n];
		}	

		if(nfault==nof) return(nfault);

		return -1;
	}

#ifdef INCLUDE_HOPE
	void ReadableFaultList::readFaultsHope(istream *file)
	{
		Gate*	gut;
		Fault*	f;
		int from,to,line,type;
		HashData*	h;
		string s; //char s[MAXSTRING];
		int i;

		inputf=file;

		//init_fault_list();
		while(getFaultSymbol(&s)!=EOF)
		{
			if(isValid(s[0]))
			{
				if((h=hashTable.findHash(&s,0))==0)
				{
					cout<<"Error in fault file:";
					cout<<s;
					cout<<" is not defined\n";
					Error::fatalerror(FAULTERROR);
				}
				if((to=h->pnode->index)<0) Error::fatalerror(FAULTERROR);
				gut=net[to];
				line=OUTFAULT;
			} else if(s[0]=='>')
			{
				from=to;
				if((h=hashTable.insertHash(&string(&s[1]),0))==0)
				{
					cout<<"Error in fault file:";
					cout<<s;
					cout<<" is not defined\n";
					Error::fatalerror(FAULTERROR);
				}

				if((to=h->pnode->index)<0) Error::fatalerror(FAULTERROR);
				gut=net[to];
				for(i=0;i<gut->ninput;i++)
					if(gut->inlis[i]->index==from) { line=i; break; }
			} else if(s[0]=='/')
			{
				if(s[1]=='1') type=SA1; else type=SA0;
				f=new Fault;
				f->gate=gut;
				f->line=line;
				f->type=type;

				hopeFaultList.push_front(f);
			} else Error::fatalerror(FAULTERROR);
		}

		// count faults and copy 
		numberOfFaults=hopeFaultList.size();

		faultList=new Fault*[numberOfFaults];

		list<Fault*>::iterator current,final;

		current=hopeFaultList.begin();
		final=hopeFaultList.end();

		i=0;
		while(current!=final)
		{
			faultList[i]=*current;
			i++;
			current++;
		}

	}
}
#endif