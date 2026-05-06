// ReadableNet.cpp: implementation of the ReadableNet class.
//
//////////////////////////////////////////////////////////////////////
#include <list>
#include <string>
#include <iostream>
#include <fstream>
#include <sstream>

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

#include "FaultSimulation.h"
#include "ReadableNet.h"

namespace atalantadll {
	//////////////////////////////////////////////////////////////////////
	// Construction/Destruction
	//////////////////////////////////////////////////////////////////////

	//ReadableNet::ReadableNet():hashTable(HASHSIZE),maxFout(0)
	int ReadableNet::setBenchStream(streambuf *buf)
	{
		int i, j;

		
		circuit.rdbuf(buf);
		if(readCircuit()<0) {
			Error::fatalerror(CIRCUITERROR);
		}

		checkParameters();
		addPO();

		if(cctMode==ISCAS89)
		{
			stack=new Stack(numberOfGates+10);

#ifdef INCLUDE_HOPE
			allocateStacks();
			maxlevel=computeLevel();
			allocateEventList();
			levelize();
			addSpareGate();

			lastGate=numberOfGates-1;

			i=setFFR();
			j=setDominator();
#else
			if(levelize(numberOfGates,&(*stack)[0]) <0)
			{
				/*cerr<<"Fatal error: Invalid circuit file."<<endl;
				exit(0);*/
				stringstream ss;
				ss <<"Fatal error: Invalid circuit file.";
				throw ss.str();
			}
#endif

			if(numberOfFlipFlops > 0)
			{
				/*cerr<<"Error: Invalid type DFF is defined."<<endl;
				exit(0);*/
				stringstream ss;
				ss<<"Error: Invalid type DFF is defined.";
				throw ss.str();
			}
		}
		else
		{
			delete stack;
			stack=0;
		}

		setCctParameters();

#ifdef INCLUDE_HOPE
		levels=maxlevel+2;
#else
		levels=maxlevel;
#endif

		if(simMode=='f')
		{
			allocateDynamicBuffers();

			myNumberOfStems=0;
			for(i=0;i<numberOfGates;i++)
				if(net[i]->isFanout() || net[i]->fn==PO) myNumberOfStems++;
			myStem=new Gate*[myNumberOfStems];
			setFanoutStemp(myStem,myNumberOfStems);
		}
		return levels;
	}

	/*int ReadableNet::readBenchFile(string benchFile)
	{
		fstream f;
		int lev;

		if (!benchFile.empty())
		{
			f.open(benchFile.c_str(),ios::in);
			if(!f.is_open())
			{
				cerr<<"Fatal error: no such file exists "<<benchFile<<endl;
				exit(0);
			}
			lev = readBenchStream(f.rdbuf());
			f.close();
			return lev;

		}

		return levels;
	}*/

	int ReadableNet::gateType(string *symbol)
	{
		int fn;

		if(symbol->compare("NOT") == 0) fn=NOT;
		else if(symbol->compare("AND") == 0) fn=AND;
		else if(symbol->compare("NAND") == 0) fn=NAND;
		else if(symbol->compare("OR")== 0) fn=OR;
		else if(symbol->compare("NOR") == 0) fn=NOR;
		else if(symbol->compare("DFF") == 0) fn=DFF;
		else if(symbol->compare("XOR") == 0) fn=XOR;
		else if(symbol->compare("XNOR") == 0) fn=XNOR;
		else if(symbol->compare("BUFF") == 0) fn=BUFF;
		else if(symbol->compare("BUF") == 0) fn=BUFF;
		else if(symbol->compare("INPUT") == 0) fn=PI;
		else if(symbol->compare("OUTPUT") == 0) fn=PO;
		else if(symbol->compare("not") == 0) fn=NOT;
		else if(symbol->compare("and") == 0) fn=AND;
		else if(symbol->compare("nand") == 0) fn=NAND;
		else if(symbol->compare("or")== 0) fn=OR;
		else if(symbol->compare("nor") == 0) fn=NOR;
		else if(symbol->compare("dff") == 0) fn=DFF;
		else if(symbol->compare("xor") == 0) fn=XOR;
		else if(symbol->compare("xnor") == 0) fn=XNOR;
		else if(symbol->compare("buff") == 0) fn=BUFF;
		else if(symbol->compare("buf") == 0) fn=BUFF;
		else if(symbol->compare("input") == 0) fn=PI;
		else if(symbol->compare("output") == 0) fn=PO;
		else fn=(-1);

		return(fn);
	}

	char ReadableNet::getSymbol(string *s)
	{
		char c;
		int comm=0;

		s->clear();
		while(!circuit.eof())
		{
			circuit>>c;

			if(c=='#')
			{	
				circuit.ignore(INT_MAX,'\n');
				continue;
			}
			/*		if(comm==1)
			{
			if(c=='\n') comm=0;
			continue;
			}*/
			if(isWhiteSpace(c)) continue;
			if(isDelimiter(c)) break;
			s->append(1, c);//*(s++)=c;
		};
		if(circuit.eof()) c=EOF;
		//*s=EOS;
		return c;
	}

	int ReadableNet::readCircuit()
	{
		char c=0;

		int i, j;

		int numberOfGates=0;
		int numberOfFlipFlops=0;
		int numberOfPrimaryInputs=0;
		int numberOfPrimaryOutputs=0;

		string symbol; //char symbol[MAXSTRING];
		int nofanin=0;
		int fn;
		int netSize;

		int nerrs=0;

		HashData *hashPointer;
		Gate	 *currentGate;
		Gate	 *pg;

		vector<Gate *> poGates; //Gate	 *poGates[MAXPO+100];
		vector<Gate *> pfanin; //Gate	 *pfanin[MAXFIN+100];

		Gate *begnet=0;
		symbol.reserve(200);
		this->numberOfPrimaryInputs = this->numberOfPrimaryOutputs = 
			this->numberOfGates = this->numberOfFlipFlops = 0;
		net.clear();
		tv.vectors.clear();
		hashTable.clear();

		/* Pass 1:
		Adds the gate symbols to symbol_tbl[] and
		counts # of gates, pi's, po's, ff's */

		while((c=getSymbol(&symbol)) != EOF )
		{
			switch(c)
			{
			case '=' :	//a new gate
				hashPointer=hashTable.findAndInsertHash(symbol,0);
				if((currentGate=hashPointer->pnode) == 0)
				{
					currentGate=new Gate();
					hashPointer->pnode=currentGate;
					currentGate->symbol=hashPointer;
					currentGate->next=begnet;
					begnet=currentGate;
				}
				break;
			case '(' :	//gate type
				if((fn=gateType(&symbol)) < 0 )
				{
					fprintf(stderr,"Error: Gate type %s is not valid\n",symbol);
					return -1;
				}
				break;
			case ',' :	//fanin list
				hashPointer=hashTable.findAndInsertHash(symbol,0);
				if((pg=hashPointer->pnode) == 0)
				{
					pg=new Gate();
					hashPointer->pnode=pg;
					pg->symbol=hashPointer;
					pg->index=-1;
					pg->next=begnet;
					begnet=pg;
				}
				if(nofanin >= pfanin.size()) {
					pfanin.resize(nofanin + 1, NULL);
				}
				pfanin[nofanin++]=pg;
				break;
			case ')' : //terminator, fanin list
				hashPointer=hashTable.findAndInsertHash(symbol,0);
				if((pg=hashPointer->pnode) == 0)
				{
					pg=new Gate();
					hashPointer->pnode=pg;
					pg->symbol=hashPointer;
					pg->index=-1;
					pg->next=begnet;
					begnet=pg;
				}
				switch(fn)
				{
				case PI:
					numberOfPrimaryInputs++;
					pg->index=numberOfGates++;
					pg->ninput=0;
					pg->inlis=0;
					pg->fn=PI;
					pg->noutput=0;
					pg->outlis=0;
					break;
				case PO:
					if(numberOfPrimaryOutputs >= poGates.size()) {
						poGates.resize(numberOfPrimaryOutputs + 1, NULL);
					}
					poGates[numberOfPrimaryOutputs++]=pg;
					break;
				default:
					if(nofanin >= pfanin.size()) {
						pfanin.resize(nofanin + 1, NULL);
					}
					pfanin[nofanin++]=pg;
					switch(fn)
					{
					case DFF:
						numberOfFlipFlops++;
						putchar('F');
						break;
					case XOR:
					case XNOR:
						if(nofanin!=2)
						{
							fprintf(stderr,"Error: %d-input %s gate is not supported\n",nofanin, fn_to_string[fn]);
							return -1;
						}
					}
					if(currentGate==0)
					{
						fprintf(stderr,"Error: Syntax error in the circuit file\n");
						return -1;
					}
					currentGate->index=numberOfGates++;
					currentGate->fn=fn;
					if((currentGate->ninput=nofanin) == 0) 
						currentGate->inlis=0;
					else
						currentGate->inlis=new Gate*[currentGate->ninput];
					for(i=0;i<nofanin;i++) {
						currentGate->inlis[i]=pfanin[i];
					}
					currentGate->noutput=0;
					currentGate->outlis=0;

					nofanin=0;
					currentGate=0;
					break;
				}
			}
		}

		/*if(numberOfGates > MAXGATE) {
			//fprintf(stderr,"The number of gates exceeds MAXGATE %d\n",MAXGATE);
			//exit(0);
			stringstream ss;
			ss << "The number of gates exceeds MAXGATE " << MAXGATE;
			throw ss.str();
		}*/
		/*if(numberOfPrimaryInputs > MAXPI) {
			//fprintf(stderr,"The number of primary inputs exceeds MAXPI %d\n",MAXPI);
			//exit(0);
			stringstream ss;
			ss << "The number of primary inputs exceeds MAXPI " << MAXPI;
			throw ss.str();
		}*/
		/*if(numberOfPrimaryOutputs > MAXPO) {
			//fprintf(stderr,"The number of primary outputs exceeds MAXPO %d\n",MAXPO;
			//exit(0);
			stringstream ss;
			ss << "The number of primary outputs exceeds MAXPO " << MAXPO;
			throw ss.str();
		}*/

		// Pass 2: Construct the circuit data structure 

		//netSize=numberOfGates+numberOfPrimaryOutputs+numberOfFlipFlops+SPAREGATES;

		//net=new Gate*[netSize];
		//primaryIn=new int[numberOfPrimaryInputs];
		//primaryOut=new int[numberOfPrimaryOutputs];
		flipFlops=new int[numberOfFlipFlops+1];

#ifdef ATALANTA
		headlines.resize(numberOfPrimaryInputs);//headlines=new int[numberOfPrimaryInputs];
#endif
		// Check floating inputs
		for(currentGate=begnet; currentGate!=0; currentGate=currentGate->next)
		{
#ifdef _ALG_DEBUG
			dbgFile << currentGate->index << "\n";
			dbgFile.flush();
#endif
			if(currentGate->index<0)
			{
				fprintf(stderr,"Error: floating net %s\n",currentGate->symbol->symbol);
				fprintf(stderr, "Workaround. You have to take one of the two actions:\n");
				fprintf(stderr, "   1. Remove all the floating input and associated gates, or\n");
				fprintf(stderr, "   2. Make each floating input a primary output.\n");
				return -1;
			}
			if(currentGate->index >= net.size()) {
				net.resize(currentGate->index + 1, NULL);
			}
			net[currentGate->index]=currentGate;
			this->numberOfGates++;
		}

//		for(i=this->numberOfGates;i<netSize;i++) net[i]=0;

		if(this->numberOfGates!=numberOfGates)
		{
			fprintf(stderr,"Error in read_circuit\n");
			return -1; 
		}

		// Pass 3: Compute fanout list
		for(i=0;i<numberOfGates;i++)
		{
			currentGate=net[i];
#ifdef LEARNFLG
			currentGate->pLearn.clear();
#endif
			for(int j=0;j<currentGate->ninput;j++) currentGate->inlis[j]->noutput++;
			switch(currentGate->fn)
			{
			case PI: 
				primaryIn.push_back(i);
				this->numberOfPrimaryInputs++;
			break;
			case DFF: flipFlops[this->numberOfFlipFlops++]=i;break;
			}
		}
		for(i=0;i<numberOfPrimaryOutputs;i++) {
			if(this->numberOfPrimaryOutputs >= primaryOut.size()) {
				primaryOut.resize(this->numberOfPrimaryOutputs + 1, 0);
			}
			primaryOut[this->numberOfPrimaryOutputs++]=poGates[i]->index;
		}
		for(i=0;i<this->numberOfGates;i++)
		{
			currentGate=net[i];
			if(currentGate->noutput>0)
			{
				currentGate->outlis=new Gate*[currentGate->noutput];
				maxFout=MAX(maxFout,currentGate->noutput);
				currentGate->noutput=0;
			}
		}

		for(i=0;i<this->numberOfGates;i++)
		{
			currentGate=net[i];
			for(int j=0;j<currentGate->ninput;j++)
				currentGate->inlis[j]->outlis[(currentGate->inlis[j]->noutput)++]=currentGate;
		};

		for(i=0; i<this->numberOfGates; i++)
		{
			currentGate=net[i];
			if (currentGate->noutput > 0) continue;
			for(j=0;j<numberOfPrimaryOutputs;j++)
				if (currentGate == poGates[j]) break;
			if (j == numberOfPrimaryOutputs)
			{
				fprintf(stderr, "Error: floating output '%s' detected!\n", currentGate->symbol->symbol);
				nerrs++;
			}
		}

		if (nerrs > 0) {
			fprintf(stderr, "Workaround. You have to take one of the two actions:\n");
			fprintf(stderr, "   1. Remove all the floating output and associated gates, or\n");
			fprintf(stderr, "   2. Make each floating output a primary output.\n");
			return -1;
		}

		if(numberOfGates==this->numberOfGates) 
			return this->numberOfGates;
		else
			return -1;
	}


#ifdef INCLUDE_HOPE
	int ReadableNet::computeLevel()
	{
		int i,j,flag=1;
		Gate *currentGate,*ng;

		for(i=0;i<numberOfGates;i++)
		{
			currentGate=net[i];
			if(currentGate->fn==PI || currentGate->fn==DFF)
			{
				currentGate->dpi=0;
				stack1->push(net[i]);
				currentGate->changed=currentGate->ninput;
			} else
			{
				currentGate->dpi=(-1);
				currentGate->changed=0;
			}
		}

		while(true)
		{
			if(flag==1)
				while(!stack1->isEmpty())
				{
					currentGate=stack1->pop();
					for(i=0;i<currentGate->noutput;i++)
					{
						ng=currentGate->outlis[i];
						if(++ng->changed==ng->ninput)
						{
							ng->dpi=currentGate->dpi+1;
							stack2->push(ng);
						}
					}
				} else
					if(flag==2)
						while(!stack2->isEmpty())
						{
							currentGate=stack2->pop();
							for(i=0;i<currentGate->noutput;i++)
							{
								ng=currentGate->outlis[i];
								if(++ng->changed==ng->ninput)
								{
									ng->dpi=currentGate->dpi+1;
									stack1->push(ng);
								}
							}
						}

						flag = (flag==1) ? 2 : 1;
						if(stack1->isEmpty() && stack2->isEmpty()) break; //exit
		}

		// Compute maxlevel 
		maxlevel=-1;
		for(i=0;i<numberOfPrimaryOutputs;i++)
		{
			currentGate=net[primaryOut[i]];
			if(currentGate->fn==PO)
			{
				if(currentGate->dpi>maxlevel) {maxlevel=currentGate->dpi; flag=1;}
			}
			else 
				if(currentGate->dpi>=maxlevel) {maxlevel=currentGate->dpi; flag=2; }
		}

		for(i=0;i<numberOfFlipFlops;i++)
		{
			currentGate=net[flipFlops[i]];
			for(j=0; j<currentGate->ninput; j++)
				if(currentGate->inlis[j]->dpi>=maxlevel) {maxlevel=currentGate->inlis[j]->dpi; flag=2; }
		}

		// Renumber levels of POs and PPO(DFF)s 
		if(flag==1) maxlevel--;
		POlevel=maxlevel+1;
		PPOlevel=maxlevel+2;
		for(i=0;i<numberOfPrimaryOutputs;i++)
			if(net[primaryOut[i]]->fn==PO) net[primaryOut[i]]->dpi=POlevel;
		for(i=0;i<numberOfFlipFlops;i++) net[flipFlops[i]]->dpi=PPOlevel;

		return(maxlevel+1);
	}

	int ReadableNet::levelize()
	{
		int i,newone=0;
		Gate* cg;

		// re-number gates 
		for(i=0;i<numberOfGates;i++)
		{
			pushGate(net[i]);
		}
		for(i=0;i<maxlevel+2;i++) 
		{
			for(int j=0;j<=eventList[i]->getCount()-1;j++)
			{
				cg=(*eventList[i])[j];
				cg->index=newone++;
			}
			eventList[i]->clear();
		}

		// update gate numbers
		for(i=0;i<numberOfPrimaryInputs;i++)				// primaryin
			primaryIn[i]=net[primaryIn[i]]->index;
		for(i=0;i<numberOfPrimaryOutputs;i++)				/* primaryout */
			primaryOut[i]=net[primaryOut[i]]->index;
		for(i=0;i<numberOfFlipFlops;i++)				/* flip_flops */
			flipFlops[i]=net[flipFlops[i]]->index;

		// sort gates by index 
		i=0;
		while(i<numberOfGates)
		{
			if(i==net[i]->index) 
				i++;
			else 			/* swap net[i] and net[net[i]->gid] */
			{
				cg=net[i];
				net[i]=net[cg->index];
				net[cg->index]=cg;
			}
		}
		return 0;
	}
#else 

	int ReadableNet::levelize(int n,Gate **stack)
	{
		int i, numberOfGates;
		Gate **first,**last;
		Gate *ele,*next;
		int gIndex=0;
		int maxDpi=-1;

		// initialize
		for(i=0; i<n; i++)
			if((ele=net[i]) != 0)
			{
				ele->changed=ele->ninput;
				ele->dpi=-1;
			}

			first=last=stack;	// empty stack 

			// Find gates with indegree=0 
			for(i=0; i<numberOfPrimaryInputs; i++)
			{
				ele=net[primaryIn[i]];
				ele->changed=0;
				*last++=ele;
				primaryIn[i]=gIndex++;
			}
			for(i=0; i<numberOfFlipFlops; i++) {
				ele=net[flipFlops[i]];
				ele->changed=0;
				*last++=ele;
				flipFlops[i]=gIndex++;
			}

			// levelize
			gIndex=0;
			numberOfGates=0;
			while(first != last)
			{
				numberOfGates++;
				ele=*first++;
				ele->index=gIndex++;
				if( ++(ele->dpi) > maxDpi) maxDpi=ele->dpi;
				for(i=0; i<ele->noutput; i++)
					if((next=ele->outlis[i]) != NULL)
						if(next->changed > 0)
						{
							if(--(next->changed) == 0) *last++=next;
							next->dpi = ele->dpi;
						}
			}

			// check for levelization 
			if(numberOfGates != n)
			{
				fprintf(stderr,"Error in circuit file.\n");
				fprintf(stderr,"Some gates are not reachable from PIs.\n");
				for(i=0; i<n; i++)
				{
					ele=net[i];
					if(ele->changed>0 || ele->dpi<0)
					{
						fprintf(stderr,"*** Unreachable gate from an input:");
						fprintf(stderr,"%d'th element %s\n",ele->index,
							ele->symbol->symbol);
					}
				}
				return -1;
			}

			for(i=0; i<numberOfPrimaryOutputs; i++)	// primaryout 
				primaryOut[i]=net[primaryOut[i]]->index;

			// sort gates by index 
			for(i=0; i<n; )
			{
				if(i==net[i]->index) 
					i++;
				else 			// swap net[i] and net[net[i]->index] 
				{
					ele=net[i];
					net[i]=net[ele->index];
					net[ele->index]=ele;
				}
			}

			return(maxDpi+1);
	}

#endif

	int ReadableNet::addPO()
	{
		int i,j;
		Gate *gut,*last,**outlist;
		string name;

		for(i=0;i<numberOfPrimaryOutputs;i++)
		{
			gut=net[primaryOut[i]];
			//if((last=net[numberOfGates])==0) last=new Gate();
			//last = net[numberOfGates]; 
			last = numberOfGates >= net.size() ? new Gate() : net[numberOfGates];

			last->index=numberOfGates;
			last->fn=PO;
			last->ninput=1;
			last->inlis=new Gate*();
			last->inlis[0]=gut;
			last->noutput=0;
			last->outlis=NULL;
#ifdef LEARNFLG
			last->pLearn.clear();
#endif
			name=gut->symbol->symbol;
			name+="_PO";

			while((last->symbol=hashTable.findHash(&name,0)) != 0) name+="_PO";

			if((last->symbol=hashTable.insertHash(&name,0)) == 0) 
				Error::fatalerror(HASHERROR);		
			else
				last->symbol->pnode=last;

			outlist=gut->outlis;
			gut->outlis=new Gate*[gut->noutput+1];

			for(j=0;j<gut->noutput;j++) gut->outlis[j]=outlist[j];
			gut->outlis[gut->noutput]=last;
			gut->noutput+=1;

			primaryOut[i]=numberOfGates;
			if(numberOfGates >= net.size()) {
				net.resize(numberOfGates + 1, NULL);
			}
			net[numberOfGates++]=last;
		}

		return(numberOfPrimaryOutputs);
	}

#ifdef INCLUDE_HOPE
	void ReadableNet::addSpareGate()
	{
		int i;
		Gate* gut;

		// add sparegates 
		for(i=0;i<SIZEOFFUT;i++)
		{
			gut=new Gate();	// CONSTANT Gate 
			gut->index=numberOfGates+2*i;
			gut->fn=DUMMY;
			gut->ninput=0;
			gut->inlis=NULL;
			gut->noutput=1;
			gut->outlis=new Gate*();
			gut->dpi=0;
			gut->changed=false;
			gut->symbol=NULL;
			gut->gid=0;
			gut->stem=0;
			if(gut->index + 1 >= net.size()) {
				net.resize(gut->index + 2); // +2 due to creation of next gate
			}
			net[gut->index]=gut;

			gut=new Gate();   // 2-input AND, OR, XOR 
			gut->index=numberOfGates+2*i+1;
			gut->fn=DUMMY;
			gut->ninput=2;
			gut->inlis=new Gate*[2];
			gut->inlis[0]=net[numberOfGates+2*i];
			gut->noutput=0;
			gut->outlis=new Gate*[maxFout];
			gut->dpi=0;
			gut->changed=false;
			gut->gid=0;
			gut->stem=0;
			gut->symbol=NULL;
			net[gut->index]=gut;    // There is alredy reserved place to this gate. See above.
		}

		// add one memory space for output list of POs 
		for(i=0; i<numberOfPrimaryOutputs; i++)
		{
			gut=net[primaryOut[i]];
			if(gut->noutput==0)
			{
				gut->outlis=new Gate*;
				gut->outlis[0]=0;
			}
		}
	}

#endif

#ifdef ISCAS85_NETLIST_MODE
	bool ReadableNet::circIn(::fstream *circuit)
	{
		int i,j;		
		int currentline=0;
		int lineno,nfout,nfin;
		int inputs[20];
		char name[10],gtype[5],fromLine[10];
		vector<char [10]>nameList;//char nameList[MAXGATE][10];

		int lineindex[MAXLINE];

		numberOfGates=0;
		numberOfPrimaryInputs=0;
		numberOfPrimaryOutputs=0;

		//net=new Gate*[MAXGATE];
		//primaryIn=new int[MAXPI];
		//primaryOut=new int[MAXPO];
		//headlines=new int[MAXPI];


		while(!circuit->eof())
		{
			if(circuit->peek()=='*')
				circuit->ignore(-1,'\n'); //commented line
			else
			{
				// read gate descriptions in the order of
				// line_number, label, gtype, # of fanout, # of fanin
				*circuit>>lineno;
				*circuit>>name;
				*circuit>>gtype;

				if(!strcmp(gtype,"from")) //fan-out branch
					*circuit>>fromLine;
				else
				{
					*circuit>>nfout;
					*circuit>>nfin;
				}
				circuit->ignore(-1,'\n');

				// if gate type is from, search fanin lines
				// and skip fanout lines from the gate list 
				if(!strcmp(gtype,"from"))
				{
					for(i=currentline-1;i>=0;i--)
						if(!strcmp(fromLine,nameList[i]))
						{
							lineindex[lineno]=net[i]->index;
							break;
						}
				} else //else, construct gate structures
				{
					strcpy(nameList[currentline],name); //store label
					if(currentline >= net.size()) {
						net.resize(currentline + 1, NULL);
					}
					net[currentline]=new Gate;
					lineindex[lineno]=currentline;
					net[currentline]->index=currentline; //internal netlist
					net[currentline]->gid=lineno;	//actual netlist
					net[currentline]->ninput=nfin;
					if(nfin!=0)	net[currentline]->inlis=new Gate*[nfin];
					net[currentline]->noutput=nfout;
#ifdef LEARNFLG
					net[currentline]->plearn=0;
#endif
					if(nfout!=0)
					{
						net[currentline]->outlis=new Gate*[nfout];
						memset(net[currentline]->outlis,0,sizeof(Gate*)*nfout);
					}
					if(!strcmp(gtype,"inpt"))
					{
						net[currentline]->fn=PI;
						if(numberOfPrimaryInputs >= primaryIn.size()) {
							primaryIn.resize(numberOfPrimaryInputs + 1, 0);
						}
						primaryIn[numberOfPrimaryInputs]=currentline;
						numberOfPrimaryInputs++;
					} else 
					{
						if(!strcmp(gtype,"and")) net[currentline]->fn=AND;
						else if(!strcmp(gtype,"nand")) net[currentline]->fn=NAND;
						else if(!strcmp(gtype,"or"  )) net[currentline]->fn=OR;
						else if(!strcmp(gtype,"nor"))  net[currentline]->fn=NOR;
						else if(!strcmp(gtype,"not"))  net[currentline]->fn=NAND;
						else if(!strcmp(gtype,"xor"))  net[currentline]->fn=XOR;
						else if(!strcmp(gtype,"buff")) net[currentline]->fn=AND;
						else if(!strcmp(gtype,"buf"))  net[currentline]->fn=AND;
						else return false;

						// get input list
						for(i=0;i<nfin;i++) *circuit>>inputs[i];
						circuit->ignore(-1,'\n');

						// convert input index into internal and check fan-out list
						for(i=0;i<nfin;i++)
						{
							net[currentline]->inlis[i]=net[lineindex[inputs[i]]];
							for(j=0;j<net[lineindex[inputs[i]]]->noutput;j++)
								if(net[lineindex[inputs[i]]]->outlis[j]==0)
								{
									net[lineindex[inputs[i]]]->outlis[j]=net[currentline];
									break;
								}
						}


					}

					if(nfout==0)
					{
						if(numberOfPrimaryOutputs >= primaryOut.size()) {
							primaryOut.resize(numberOfPrimaryOutputs + 1, 0);
						}
						primaryOut[numberOfPrimaryOutputs]=currentline;
						numberOfPrimaryOutputs++;
					}
					currentline++;
				}
			}

		}

		numberOfGates=currentline;
		headlines.resize(numberOfPrimaryInputs);
		return true;
	}
#endif

}
