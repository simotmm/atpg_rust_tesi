#include "Atpg.h"

#include <string>
#include <sstream>
#include <list>
#include <iostream>
#include <fstream>
#include <memory>
#include <time.h>

#include "Defines.h"
#include "Truthtable.h"
#include "Parameters.h"
#include "Error.h"

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
#include "Random.h"

#include "Simulation.h"

//#include "windows.h"

using namespace atalantadll;


namespace atalantadll {
	myFaultList::myFaultList(int fault,Fault **faultList):fault(fault),faultList(faultList)
	{
		mask=new char[fault];
		memset(mask,0,fault);
		names=new string[fault];

		for (int  i = 0; i < fault; i++ )
		{
			if(faultList[i]->line>=0)
			{
				names[i]=faultList[i]->gate->inlis[faultList[i]->line]->symbol->symbol;
				names[i]+="->";
			}
			/*names[i]+="i:";
			names[i]+=faultList[i]->gate->index+'0';
			names[i]+=";";*/
			names[i]+=faultList[i]->gate->symbol->symbol;
			names[i]+=fault2str[faultList[i]->type];
		}
	}

	void myFaultList::printList(std::streambuf *fn)
	{

		if (fn != NULL)
		{
			ostream f(fn);
			for (int i=0; i<fault; i++)	f<<names[i]<<endl;
		}
	}

	void myFaultList::updateFaultList()
	{
		for(int i=0;i<fault;i++) mask[i]=faultList[i]->detected;
	}

	void myFaultList::writeFaultMask(std::streambuf *fn)
	{
		//  Writes the faltlist mask, according the order in faultlist
		//  0 = not detected
		//  1 = detected
		//  3 = redundant
		//  4 = aborted


		if(fn != NULL)
		{
			ostream file(fn);

			for(int i=0;i<fault;i++) file<<(char)(mask[i]+'0');
		}
	}

	void myFaultList::writeABFaults(std::streambuf *fn)
	{

		if(fn != NULL)
		{
			ostream file(fn);

			for(int i=0;i<fault;i++)
				if(mask[i]==4) file<<names[i]<<endl;
		}
	}

	void myFaultList::writeUDFaults(std::streambuf *fn)
	{

		if(fn != NULL)
		{
			ostream file(fn);

			for(int i=0;i<fault;i++)
				if(mask[i]!=1) file<<names[i]<<endl;
		}
	}

	Atalanta::Atalanta() 
		{		
			inputMode='d';
			iseed=0;
			faultMode='d';
			maxBackTrack=10;
			maxBackTrack1=0;
			randomLimit=16;
			rptMode='y';

			wFaults=0;
			wTestMode=0;
			uFaultMode=0;
			simulationMode=0;
			lfsrSimMode=0;

			nTest2=0;
			nTest3=0;

			mnBit=0;
			mnPacket=0;
			mnTest=0;
			mnDetect=0;

			nRedundant=0;

			fantime=0;

			lid=0;

			myCurrFault=NULL;

			benchStream = NULL;
			faultStream = NULL;
			patternStream = NULL;
			udFaultsStream = NULL;
			wFaultStream = NULL;
			maskStream = NULL;
			reportStream = NULL;
			genResStream = NULL;
		}

	void Atalanta::setFaults()
	{
		if (faultMode == 'f')
			readFaults(faultStream);
		else
		{

			if(simMode=='f')
			{ // FSIM
				numberOfFaults = setAllFaultList(myNumberOfStems,myStem);
			}
#ifdef INCLUDE_HOPE
			else FWDfaults();
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

	void Atalanta::storeLearn(Gate *gut,level val)
	{
		//struct LEARN *tmp;

		if(gut->ninput < 2) return;
		if(gut->numzero != lid) return;

		switch(gut->fn)
		{
		case AND: case NOR: if(val==ZERO) return; break;
		case OR: case NAND: if(val==ONE) return; break;
		case XOR: case XNOR: break;
		default: return;
		}

		gut->pLearn.push_front(Learn(snode,sval,aNot(val)));

#ifdef DEBUGLEARN
		norecord++;
		learnmemory+=sizeof(struct LEARN);
		printf("learn: (node %d = %s) from ",tmp->node,level2str[tmp->tval]);
		printf("(node %d = %s)\n",gut->index,level2str[tmp->sval]);
#endif
	}

	status Atalanta::leval(Gate* gate)
	{
		int i,j;
		level val, v1;
		int numX;
		logic f;
		Gate **p;

		// forward gate evaluation
		gate->changed=false;
		p=gate->inlis;

		j = 0;

		// if a line is a head line, stop
		if(gate->isFree()) return FORWARD;

		// fault free gate evaluation
		for(i=0; i<gate->ninput; i++)
			if(gate->inlis[i]->numzero==lid) { gate->numzero=lid; break; }

			gateEval1(gate,&val,&f);

			if(val==gate->output)
			{	// no event 
				if(val!=X) gate->changed=true;
				return FORWARD;
			}

			if(gate->output==X)
			{	// forward evaluation
				gate->output=val;				// update gate output
				stack->push(gate);
				gate->changed=true;
				scheduleOutput(gate);

				storeLearn(gate,val);
				return FORWARD;
			}

			if(val!=X) return(CONFLICT);		// report conflict

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

						return BACKWARD;
				} else
				{
					for(i=numX=0; i<gate->ninput; i++)
						if(p[i]->output==X) { numX++; j=i; }

						if(numX==1)
						{
							p[j]->output = a_truthtbl1[gate->fn][gate->output];
							gate->changed=true;
							stack->push(p[j]);
							scheduleInput(gate,j);
							return BACKWARD;
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
				return BACKWARD;
				break;

			case XOR:
			case XNOR:
				for(i=numX=0;i<gate->ninput;i++)
					if(p[i]->output==X) { numX++; j=i; }

					if(numX==1)
					{
						v1=(j==0) ? p[1]->output : p[0]->output;
						val=a_truthtbl1[gate->fn][gate->output];
						if(v1==ONE) val=a_truthtbl1[NOT][val];
						p[j]->output=val;
						gate->changed=true;
						stack->push(p[j]);
						scheduleInput(gate,j);
						return BACKWARD;
					}
			}
			return FORWARD;
	}

	bool Atalanta::impval(int maxDpi,bool backward,int last)
	{
		int i, start;
		status st;
		Gate *g;

		start= backward ? last : 0;

		while(true)
		{   // backward implication
			if(backward)
				for(i=start;i>=0;i--)
					while(!eventList[i]->isEmpty())
					{
						g=eventList[i]->pop();
						if((st=leval(g))==CONFLICT) return false;
					}

					// forward implication
					backward=false;
					for(i=0;i<maxDpi;i++)
					{
						while(!eventList[i]->isEmpty())
						{
							if((st=leval(eventList[i]->pop()))==CONFLICT) return false;
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

	void Atalanta::learnNode(int maxDpi,int node,level val)
	{
		int ix;
		Gate *gut=net[node];

		snode=node;
		gut->output=val;
		stack->push(gut);
		sval=aNot(val);

		pushEvent(gut);
		gut->numzero=++lid;
		scheduleOutput(gut);

		if(!impval(maxDpi,FORWARD,0))
		{
			impo.push_front(Eden(node,aNot(val)));

#ifdef DEBUGLEARN
			noimpo++;
			learnmemory+=sizeof(struct EDEN);
			printf("learn: impo: node %d = %s\n",tmp->node,level2str[tmp->val]);
#endif
			for(ix=0; ix<maxDpi; ix++)
				while(!eventList[ix]->isEmpty())
				{
					gut=eventList[ix]->pop();
					gut->changed=false;
				}
		}

		// restore good values
		for(ix=stack->getCount()-1; ix>=0; ix--)
		{
			(*stack)[ix]->output=X;
			(*stack)[ix]->changed=false;
		}

		stack->clear();
	}

	void Atalanta::learn(int maxDpi)
	{
		int ix;
		Gate *gut;
		//struct EDEN *temp;
		//struct LEARN *record;

#ifdef DEBUGLEARN
		gettime(&slearntime_beg,&ulearntime_beg,&learntime_beg);
		printf("DEBUG: Start learn(nog,maxdpi), nog=%d, maxdpi=%d\n",nog,maxdpi);
#endif

		impo.clear();

		for(ix=0; ix<numberOfGates;ix++)
		{
			gut=net[ix];
			gut->changed=false;
			gut->numzero=-1;
			gut->output=X;

			gut->pLearn.clear();
		}

		for(ix=0; ix<numberOfGates; ix++)
		{
			gut=net[ix];
			if(gut->isFree()) continue;
			if(gut->ninput==1) continue;

#ifdef DEBUGLEARN
			printf("** Learn node %d: symbol=%s fi=%d fo=%d\n",
				gut->index, gut->symbol->symbol, gut->ninput, gut->noutput);
#endif

			switch(gut->fn)
			{
			case AND:
			case NOR:
				learnNode(maxDpi,ix,ONE);
				if(gut->noutput>1) learnNode(maxDpi,ix,ZERO);
				break;
			case OR:
			case NAND:
				learnNode(maxDpi,ix,ZERO);
				if(gut->noutput>1) learnNode(maxDpi,ix,ONE);
				break;
			default:
				if(gut->noutput > 1)
				{
					learnNode(maxDpi,ix,ZERO);
					learnNode(maxDpi,ix,ONE);
				}
			}
		}

		stack->clear();

#ifdef DEBUGLEARN
		gettime(&slearntime_end,&ulearntime_end,&learntime_end);
		printf("\n*** End of learning\n");
		printf("*** Number of redundant nodes = %d\n",noimpo);
		printf("*** Number of learning records = %d\n",norecord);
		printf("*** Memory required = %d bytes\n",learnmemory);
		printf("*** CPU time spent = %.2f seconds\n",learntime_end-learntime_beg);
#endif
	}

	void Atalanta::initFS()
	{
		int i;

		setTestAbility();

		for(i=0;i<numberOfGates;i++)
		{
			net[i]->changed=false;
			net[i]->freach=numberOfGates;
			if(net[i]->dpi>=PPOlevel)
				cout<<"Error: gut="<<net[i]->symbol->symbol<<" dpi="<<net[i]->dpi<<endl;
		}

		FanNet::setDominator(levels);
		setUniquePath(levels);

		//print_test_topic(test,nopi,nopo,name1);
		if(learnMode=='y') learn(levels);

		for(i=0;i<numberOfFaults;i++)
		{
			faultList[i]->detected=UNDETECTED;
			faultList[i]->observe=ALL0;
		}

		if(simMode=='f')
		{
			nRedundant=checkRedundantFaults();
			pInitSimulation(levels);
		}
		else
			initFaultSim();

		maxDetect=numberOfFaults;
		testVectors.clear();
		testVectors1.clear();
		testStore.clear();
		testStore1.clear();
		testVectors.setSecondSize(numberOfPrimaryInputs);
		testVectors1.setSecondSize(numberOfPrimaryInputs);
		testStore.setSecondSize(numberOfPrimaryInputs);
		testStore1.setSecondSize(numberOfPrimaryInputs);

		allOne=ALL1;
	}

	// This is used to read options from command line
	int Atalanta::readOption(char option,char **array,int i,int n)
	{
		// Every switch has 1 parameter
		if(i+1 >= n) return((-1));
		int temp;

		switch(option)
		{
		case 'd': inputMode='d'; break;
#ifdef ISCAS85_NETLIST_MODE
		case 'I': cctMode=ISCAS85; break;
#endif
		case 'r':
			sscanf(array[++i],"%d",&randomLimit);           // RPT limit
			if(randomLimit==0) rptMode='n'; break;
		case 's': sscanf(array[++i],"%d",&iseed); break;          // initial seed
		case 'N': compact='n'; maxCompact=0; break;		// no test compaction
		case 'c': sscanf(array[++i],"%d",&maxCompact); break;		// limit of shuffeling compaction
		case 'b': sscanf(array[++i],"%d",&maxBackTrack); break;				// max. backtracks for FAN
		case 'B': sscanf(array[++i],"%d",&maxBackTrack1); break;			// max. backtracks (2 phases)
		case 't': 
			sPatternFile = array[++i];
			break;					// test pattern file
			//   case 'l': logmode='y'; strcpy(name3,array[++i]); break;
		case 'n': 
			benchFile.open(array[++i], ios::in);
			if(!benchFile.is_open()) {
				stringstream ss;
				ss << "Fatal error: Cannot open bench file: " << array[i];
				throw ss.str();
			}
			else {
				benchStream = benchFile.rdbuf();
			}
			break;					// .bench file
		case 'L': learnMode='y'; break;							// static learning
		case 'f': 
			faultMode='f'; 
			faultFile.open(array[++i], ios::in | ios::binary);
			if(!faultFile.is_open()) {
				stringstream ss;
				ss << "Fatal error: Cannot open fault file: " << array[i];
				throw ss.str();
			}
			else {
				faultStream = faultFile.rdbuf();
			}
			break;		// source fault file
		case 'F': 
			wFaults = 1; 
			wFaultFile.open(array[++i], ios::out); 
			if(!wFaultFile.is_open()) {
				stringstream ss;
				ss << "Fatal error: Cannot open processed faults file: " << array[i];
				throw ss.str();
			}
			else {
				wFaultStream = wFaultFile.rdbuf();
			}
			break;		// destination fault file (complete fault list)
		case 'H': simMode='h'; break;								// HOPE simulation
		case '0': fillMode='0'; break;								// fill X's with 0s
		case '1': fillMode='1'; break;								// fill X's with 1s
		case 'X': fillMode='x'; break;								// don't fill X's
		case 'R': fillMode='r'; break;								// randomly fill X's
		case 'A': genAllPat='y'; break;							// generate all patterns for each fault
		case 'D': sscanf(array[++i],"%d",&temp);							// debug mode limit
			setEachLimit(temp);
			genAllPat='y'; break;
		case 'Z': noFaultSim='y'; break;							// one test pattern for each fault
			//    case 'u': ufaultmode='y'; break;
		case 'U': 
			uFaultMode = 1; 
			udFaultsFile.open(array[++i], ios::out); 
			if(!udFaultsFile.is_open()) {
				stringstream ss;
				ss << "Fatal error: Cannot open aborted faults file: " << array[i];
				throw ss.str();
			}
			else {
				udFaultsStream = udFaultsFile.rdbuf();
			}
			break; // aborted faults file name
		case 'v': uFaultMode = 2; break;									// undetected faults are printed to a file as well (with -U)
		case 'S': simulationMode = 1; simMode='h'; break;			// perform pattern simulation, not TPG
		case 'm': 
			maskFile.open(array[++i], ios::out); 
			if(!maskFile.is_open()) {
				stringstream ss;
				ss << "Fatal error: Cannot open mask file: " << array[i];
				throw ss.str();
			}
			else {
				maskStream = maskFile.rdbuf();
			}
			break;						// mask file name
		case 'P': 
			reportFile.open(array[++i], ios::out);
			if(!reportFile.is_open()) {
				stringstream ss;
				ss << "Fatal error: Cannot open report file.";
				throw ss.str();
			}
			else {
				reportStream = reportFile.rdbuf();
			}
			break;					// report file name
		case 'W': wTestMode = array[++i][0] - '0'; break;					// test output file
			//        0 - no test
			//        1 - PAT file
			//        2 - input + output
			//        3 - more test vectors for each fault, output
			//        4 - more test vectors for each fault, output, with fault mask
		case 'l': lfsrSimMode = 1;
			//sscanf(array[++i],"%s",&lfsrPoly);             // LFSR simulation
			//sscanf(array[++i],"%s",&lfsrSeed);
			lfsrPoly = array[++i];
			lfsrSeed = array[++i];
			sscanf(array[++i],"%i",&lfsrNum);
			simulationMode = 1; simMode='h';
			break;
		case 'g':
			lfsrSimMode = 2;											// LFSR simulation with generating poly and seed
			sscanf(array[++i], "%i", &lfsrNum);
			simulationMode = 1; simMode='h';
			genResFile.open(array[++i], fstream::out);
			if(!genResFile.is_open()) {
				stringstream ss;
				ss << "Fatal error: Cannot open generator file.";
				throw ss.str();
			}
			else {
				genResStream = genResFile.rdbuf();
			}
			break;
		default:  i=-1;
		}
		return i;
	}

	void Atalanta::help()
	{
		stringstream ss;

		ss<< "Atalanta-M 1.1b\n\n ";

		ss<<"SYNOPSIS: atalanta-M [options] circuit_file\n\n";
		ss<<"OPTIONS:\n\n";
		ss<<"   File specification:\n";
		ss<<"      -n fn      Name of the .bench file. The -n statement is optional, the benchmark name can be specified without any prequisite parameter.\n";
		ss<<"      -f fn      The fault file is read from fn. If not specified, all the s-a-faults are set as default.\n";
		ss<<"      -F fn      The processed fault list is written to a file fn.\n";
		ss<<"      -t fn      Test patterns are are written or read from the file fn (written in TPG mode, read in simulation mode).\n";
		ss<<"      -U fn      Atalanta writes aborted faults to the given file name.\n";
		ss<<"      -v         Atalanta prints out all identified redundant faults as well as aborted fauts in a file. The -U option has to be specified.\n";
		ss<<"      -m fn      The fault mask is written to the file fn. The order of faults corresponds to the fault list.\n";
		ss<<"      -P fn      The ATPG/FS report is written to a file fn.\n";
		ss<<"      -W n       The output test file format (for TPG only).\n";
		ss<<"                   n = 0 - no test pattern output (default).\n";
		ss<<"                   n = 1 - PAT file (test vectors only). Do not use together with -D n option, since all the test vectors are put together.\n";
		ss<<"                   n = 2 - input + output. Do not use together with -D n option, since all the test vectors are put together.\n";
		ss<<"                   n = 3 - more test vectors for each fault, output.\n";
		ss<<"                   n = 4 - more test vectors for each fault, output, with fault mask. HOPE simulator is employed.\n";
		ss<<"\n";
		ss<<"   ATPG Options. These options have no sense in the simulation mode (-S).\n";
		ss<<"      -A         Atalanta derives all test patterns for each fault. In this option, all unspecified inputs are left unknown, and fault simulation is not performed. HOPE fault simulator is employed. Note: it does not work properly. Not all existing test patterns are produced.\n";
		ss<<"      -D n       Atalanta derives n test patterns for each fault. In this option, all unspecified inputs are left unknown, and fault simulation is not performed. If both -A and -D option are specified, -D option is applied. HOPE fault simulator is employed. Note: it does not work properly. Not all existing test patterns are produced.\n";
		ss<<"      -b n        	The number of maximum backtracks for the FAN algorithm phase 1. (default: -b 10)\n";
		ss<<"      -B n       If -B n (n > 0) option is specified, atalanta generates test patterns in two phases. In phase 1, static unique path sensitization is employed. If the test generation for a target fault is aborted in phase 1, the test generation is tried in phase 2. In phase 2, dynamic unique path sensitization is employed. If n=0, phase 2 is not performed. If n > 0, phase 2 test generation is performed with the backtrack limit of n. (default: -B 0, i.e., phase 2 is not performed.)\n";
		ss<<"      -H         HOPE is employed for fault simulation. In this option, three logic values (0, 1 and X), instead of two logic values (0 and 1), are employed. Due to the embedding of the unknown logic value and the parallel fault fault simulation algorithm, the test generation time is slower than the default mode.) (default: FSIM, which is a parallel pattern fault simulator, is employed, and two logic values are used.)\n";
		ss<<"      -L         Static learning is performed. (default: no learning)\n";
		ss<<"      -c n       Atalanta compacts test patterns using two different methods: reverse order compaction and shuffling compaction. First, test patterns are applied in the reverse order and fault simulated (reverse order compaction). Second, test patterns are shuffled randomly and fault simulated (shuffling compaction). During the fault simulations, all the test patterns which do not detect a new fault are eliminated. The option -c n specifies the limit of shuffling compaction. If n>0, shuffling compaction is terminated if n consecutive shuffles do not drop a test pattern. If n=0, shuffling compaction is not included and compaction is done only by the reverse order fault simulation. (default: -c 2)\n";
		ss<<"      -N         Test compaction is not performed.\n";
		ss<<"      -r n       Random Pattern Testing (RPT) Session is included before deterministic test pattern generation session. The RPT session stops if any n consecutive packets of 32 random patterns do not detect any new fault. If n=0, the RPT session is not included. (default: -r 16)\n";
		ss<<"      -s n       Initial seed for the random number generator (random()). If n=0, the initial seed is the current time. (default: -s 0)\n";
		ss<<"      -Z         Atalanta derives one test pattern for each fault. In this option, no fault simulation is performed during the entire test generation (including random pattern test generation session, deterministic test generation session and test compaction session). All unspecified inputs are left unknown.\n";
		ss<<"      -0, -1, -X, -R    During test generation, some inputs can be unspecified. Atalanta provides various options to set these unspecified inputs into a certain value. (default: -R)\n";
		ss<<"   Fault Simulation Options:\n";
		ss<<"      -S          	Simulation mode is performed, instead of TPG. The pattern file has to be specified (-t option), if the -l option is not present.\n";
		ss<<"      -l poly seed num    	Simulates num LFSR patterns. The LFSR polynomial and seed are specified in octal form (poly, seed).\n";

		throw ss.str();
	}

	int Atalanta::optionSet(int argc,char **argv)
	{
		int i;

		if(argc==1)
		{
			// Display help when no parameter has been passed
			// First param is program name
			help();
		} else
			for(i=1;i<argc;i++)
			{
				if(argv[i][0]=='-')
				{
					if((i=readOption(argv[i][1],argv,i,argc))<0)
					{ 
						help();
						break;
					}
				}
				else {
					benchFile.open(argv[i], ios::in);
					if(!benchFile.is_open()) {
						stringstream ss;
						ss << "Cannot open bench file: " << argv[i];
						throw ss.str();
					}
					else {
						benchStream = benchFile.rdbuf();
					}
				}
			}

			if(fillMode=='x') simMode='h';
			if(genAllPat=='y')
			{
				randomLimit=0;
				rptMode='n';
				maxBackTrack1=0;
				fillMode='x';
				compact='n';
				maxCompact=0;
				simMode='h';
				noFaultSim='y';
			}

			if(noFaultSim=='y')
			{
				randomLimit=0;
				rptMode='n';
				fillMode='x';
				compact='n';
				maxCompact=0;
				simMode='h';
			}
			if ( wTestMode == 4 ) simMode = 'h';
			return 0;
	}

	void Atalanta::readTestFile()
	{
		string s;

		if(patternStream != NULL)
		{
			istream f(patternStream);
			tv.num = 0;
			tv.inpVars=numberOfPrimaryInputs;
			tv.outVars=numberOfPrimaryOutputs;

			while(f.peek() > 0)
			{
				//f.getline(s, MAXPI);
				//f.get(s, MAXPI);
				f >> s;
				myCurrFault=0;
				if(s.length() != numberOfPrimaryInputs)
				{
					/*cerr<<"Fatal error: Incorrect number of test vector inputs"<<endl;
					exit(0);*/
					stringstream ss;
					ss << "Fatal error: Incorrect number of test vector inputs";
					throw ss.str();
				}
				addTestVector(&s, NULL,  -1);
			}
		}
	}

	int Atalanta::simulateVector(string vct)
	{
		int i;

		inVal.clear();
		inVal.resize(numberOfPrimaryInputs);
		for(i=0;i<numberOfPrimaryInputs;i++)
			switch(vct[i])
		{
			case '0': inVal[i] = ZERO; 
				break;
			case '1': inVal[i] = ONE; 
				break;
			case 'x':
			case 'X':
			case '-':
			case '2': inVal[i] = X; 
				break;
			default:  inVal[i] = X; 
				break;
		}
		return simulateHope(&mnPacket, &mnBit);
	}

	string Atalanta::octToBin(string *c)
	{

		if(numberOfPrimaryInputs / 3 + (numberOfPrimaryInputs % 3 ? 1 : 0)!=c->length()) {
			stringstream ss;
			throw "Can't use it.";
		}

		string num;

		unsigned int i, p, n, lead;

		num.clear();
		num.append(numberOfPrimaryInputs, 'x');
		p = 0;
		lead = 1;	
		for (i = 0; i < c->length() && p < numberOfPrimaryInputs; i++) {
			n = (*c)[i]-'0';
			if (n > 3) {
				num[p++] = '1';
				lead = 0;
				n -= 4;
			} else if (lead == 0) num[p++] = '0';
			if (n > 1) {
				num[p++] = '1';
				lead = 0;
				n -= 2;
			} else if (lead == 0) num[p++] = '0';
			if (n > 0) {
				num[p++] = '1';
				lead = 0;
			} else if (lead == 0) num[p++] = '0';
		}
		return num;
	}

	int Atalanta::simulateTest()
	{
		list<TestVector*>::iterator current,final;
		int det=0;

		current=tv.vectors.begin();
		final=tv.vectors.end();

		while(current!=final)
		{
			det+=simulateVector((*current)->ivct);
			current++;
		}
		mnDetect = det;
		return det;
	}

	int Atalanta::generatePolyAndSeed(void)
	{
		long tmp;
		string state, newState;
		string poly;
		int i, j, t, order;
		bool end = false;
		// check if number of lsfrnum is less than number of combination of primary inp.
		if(numberOfPrimaryInputs <= sizeof(int) * 8) {
			tmp = (1 << (numberOfPrimaryInputs + 1)) - 1;
			if(tmp < lfsrNum) {
				stringstream ss;
				ss << "Fatal error: Number of lsfr cycles is too big.";
				throw ss.str();
			}
		}

		while(!end) {
			end = true;
			Random::getRandomOctVector(numberOfPrimaryInputs, &lfsrSeed);
			//strcpy(newState, lfsrSeed);
			newState = octToBin(&lfsrSeed);
			state = newState;
			Random::getRandomOctVector(numberOfPrimaryInputs, &lfsrPoly);
			poly = octToBin(&lfsrPoly);
			order = numberOfPrimaryInputs - 1;
			for(i = 1; i < lfsrNum; i++) {

				t=newState[order-1]-'0';
				for(j=order-1;j>0;j--)
					newState[j]=(newState[j-1]-'0' != t*(lfsrPoly[j]-'0') ? 1 : 0)+'0';
				newState[0]=t+'0';
				//				if(!strcmp(newState, lfsrSeed)) {
				if(!newState.compare(state)) {
					end = false;
					break;
				}
			}
		}
		return order;
	}

	int Atalanta::simulateLFSR()
	{
		int i, j, t;
		int det;
		string poly;
		string state;
		int order;

		if(lfsrSimMode == 2) {
			generatePolyAndSeed();
		}
		det = 0;
		try {
			poly=octToBin(&lfsrPoly);
			state=octToBin(&lfsrSeed);
		}
		catch (...) {
			stringstream ss;
			ss << "FatalError: poly and state parameters have to be equal to number of inputs (" 
				<< numberOfPrimaryInputs << ")";
			throw ss.str();
		}
		order = poly.length() - 1;

		for ( i = 0; i < lfsrNum; i++ )
		{
			det+=simulateVector(state);

			t=state[order-1]-'0';
			for(j = order-1;j>0;j--) {
				state[j]=(state[j-1]-'0' != t*(poly[j]-'0') ? 1 : 0)+'0';
			}
			state[0]=t+'0';

			//printf("%s\n", state);
		}
		mnDetect = det;
		return det;
	}

	atpgResults Atalanta::getResults()
	{
		atpgResults ar;

		ar.circuitName=circuitName;
		ar.gates=numberOfGates - numberOfPrimaryInputs - numberOfPrimaryOutputs;
		ar.iv = numberOfPrimaryInputs;
		ar.ov = numberOfPrimaryOutputs;
		ar.iPatterns= nTest2;
		ar.patterns= nTest3;
		ar.faults= numberOfFaults;
		ar.detectedFaults= mnDetect;
		ar.redundantFaults= nRedundant;
		return ar;
	}

	void Atalanta::writeResults(atpgResults ar)
	{

		if(reportStream != NULL) {
			ostream file(reportStream);

			file.precision(3);

			file<<"gates: "<<ar.gates<<endl;
			file<<"iv: "<<ar.iv<<endl;
			file<<"ov: "<<ar.ov<<endl;
			file<<"i_patterns: "<<ar.iPatterns<<endl;
			file<<"patterns: "<<ar.patterns<<endl;
			file<<"faults: "<<ar.faults<<endl;
			file<<"d_faults: "<<ar.detectedFaults<<endl;
			file<<"r_faults: "<<ar.redundantFaults<<endl;
			file<<"time: "<<ar.time<<endl;
			file.flush();
		}
	}

	void Atalanta::simulateAllVectors()
	{
		list<TestVector*>::iterator current,end;

		int i;
		int det=0;
		TestVector *temp;

		current=tv.vectors.begin();
		end=tv.vectors.end();

		setFaults();
		initFS();

		while(current!=end)
		{
			temp=*current;

			for(i=0;i<numberOfFaults;i++)
			{
				faultList[i]->detected=UNDETECTED;
				faultList[i]->observe=ALL0;
			}

			det=simulateVector(temp->ivct);
			delete temp->mask;
			temp->mask=new char[numberOfFaults];

			for(i=0;i<numberOfFaults;i++) temp->mask[i]=faultList[i]->detected;

			current++;
		}
	}

	void Atalanta::generateTest()
	{
		int i;
		int nDetect3=0;
		level *LFSR = new level[numberOfPrimaryInputs];//level LFSR[MAXPI];
		status state;
		Fault *f;
		int nOverBackTrack = 0;
		int tBackTrack = 0;
		double fan1Time;
		int shuf = 0;

		tv.num = 0;
		tv.inpVars = numberOfPrimaryInputs;
		tv.outVars = numberOfPrimaryOutputs;

		/*****************************************************************
		*                                                               *
		*         step 2: Random pattern testing session                *
		*              1. generate 32 random patterns                   *
		*              2. fault free simulation                         *
		*              3. fault simulation                              *
		*              4. fault dropping                                *
		*                                                               *
		*****************************************************************/

		if(rptMode=='y')
			mnDetect=randomSim(levels,myNumberOfStems,myStem,LFSR,randomLimit,BITSIZE,maxDetect,&mnTest,&mnPacket,&mnBit);

		/******************************************************************
		*                                                                *
		*    step 3: Deterministic Test Pattern Generation Session       *
		*            (fan with unique path sensitization                 *
		*                                                                *
		******************************************************************/
		fantime=0;

		mnDetect+=testGen(levels,BITSIZE,myNumberOfStems,myStem,maxBackTrack,false,&nRedundant,&nOverBackTrack,&tBackTrack,&mnTest,&mnPacket,&mnBit,&fantime);
		nTest2=mnTest;

		/******************************************************************
		*                                                                *
		*    step 4: Deterministic Test Pattern Generation Session       *
		*            Phase 2: Employs dynamic unique path sensitization  *
		*                                                                *
		******************************************************************/

		state=NO_TEST;
		if(maxBackTrack1>0 && numberOfFaults-mnDetect-nRedundant>0)
		{
			for(i=0;i<numberOfFaults;i++)
			{
				f=faultList[i];
				if(f->detected==PROCESSED) f->detected=UNDETECTED;
			}

			fan1Time=0;
			mnDetect+=testGen(levels,BITSIZE,myNumberOfStems,myStem,maxBackTrack1,true,&nRedundant,&nOverBackTrack,&tBackTrack,&mnTest,&mnPacket,&mnBit,&fan1Time);
			fantime+=fan1Time;
		}

		nTest2=mnTest;

		/********************************************************************
		*                                                                  *
		*       step 5: Test compaction session                            *
		*               32-bit reverse fault simulation                    *
		*               + shuffling compaction   	                       *
		*                                                                  *
		********************************************************************/

		if(mnTest==0)
		{
			nTest3=0;
			nDetect3=0;
		} else if(compact=='n')
		{
			nTest3=mnTest;
			nDetect3=mnDetect;
		} else 
		{
			if(maxCompact==0) {
				compact='r';
			}
			nTest3= compactTest(levels,myNumberOfStems,myStem,&shuf,&nDetect3,mnPacket,mnBit,BITSIZE);
			if(nDetect3 != mnDetect)
			{
				/*cout<<"Error in test compaction: m_ndetect="<<mnDetect<<", ndetect3="<<nDetect3<<endl;
				exit(0);*/
				stringstream ss;
				ss << "Error in test compaction: m_ndetect="<<mnDetect<<", ndetect3="<<nDetect3;
				throw ss.str();
			}
		}
	}

	void Atalanta::writeTestFile()
	{
		//    Writes a test file. Only test vectors (pat format).
		//    For -D n does not distinguish between vectors for the same fault - do not use here!

		list<TestVector*>::iterator current,final;

		if(patternStream != NULL)
		{
			ostream file(patternStream);
			file.clear();

			current=tv.vectors.begin();
			final=tv.vectors.end();

			while(current!=final)
			{
				file<<(*current)->ivct<<endl;
				current++;
			}

		}
	}

	void Atalanta::writeTestFileOut()
	{
		//    Writes a test file. Only test vectors (pat format).
		//    For -D n does not distinguish between vectors for the same fault - do not use here!


		list<TestVector*>::iterator current,final;

		if(patternStream != NULL)
		{
			ostream file(patternStream);
			file.clear();

			current=tv.vectors.begin();
			final=tv.vectors.end();

			while(current!=final)
			{
				file<<(*current)->ivct<<" "<<(*current)->ovct<<endl;
				current++;
			}

		}
	}

	void Atalanta::writeMultiTestFile()
	{
		//  Writes a test file with outputs. Numbers the vectors for one fault (for -D)



		list<TestVector*>::iterator current,final;

		if(patternStream != NULL)
		{
			ostream file(patternStream);
			file.clear();

			current=tv.vectors.begin();
			final=tv.vectors.end();

			while(current!=final)
			{
				file<<(*current)->no<<": "<<(*current)->ivct<<" "<<(*current)->ovct<<endl;
				current++;
			}

		}
	}

	void Atalanta::writeMultiTestFileMask()
	{
		//  Writes a test file with outputs. Numbers the vectors for one fault (for -D)



		list<TestVector*>::iterator current,final;
		TestVector *temp;

		if(patternStream != NULL)
		{
			ostream file(patternStream);
			file.clear();

			current=tv.vectors.begin();
			final=tv.vectors.end();

			while(current!=final)
			{
				temp=*current;

				file<<temp->no<<": "<<temp->ivct<<" "<<temp->ovct<<" ";

				if(temp->mask)
					for(int i=0;i<numberOfFaults;i++)
						file<<temp->mask[i]+'0';

				file<<endl;

				current++;
			}

		}
	}

	int Atalanta::run()
	{
		int n;
		int ndetect3=0;
		int store=0;
		int ncomp=INFINITY,stop=ONE;
		atpgResults ar;
		myFaultList * fl;
		int bit=0,packet=0;
		clock_t start, end;

#ifdef _ALG_DEBUG
		dbgFile.open("dbg.txt", ios::out);
#endif

		levels=setBenchStream(benchStream);

		setFaults();
		indexFaults();
		fl=new myFaultList(numberOfFaults,faultList);

		if(wFaults) fl->printList(wFaultStream);

		iseed=Random::seed(iseed);
		//iseed=Random::seed(100);

		start = clock();

		initFS();

		if(simulationMode)
		{
			// Open pattern File for reading
			if(sPatternFile.length() > 0) {
				patternFile.open(sPatternFile.c_str(), fstream::in);
				if(!patternFile.is_open()) {
					stringstream ss;
					ss << "Fatal error: Cannot open pattern file " << sPatternFile;
					throw ss.str();
				}
				else {
					patternStream = patternFile.rdbuf();
				}
			}
			if(!lfsrSimMode)
			{
				readTestFile();
				n=simulateTest();
			} else
			{
				n=simulateLFSR();
				if(lfsrSimMode == 2) {
					ostream f(genResStream);
					f.clear();
					f << "poly: " << lfsrPoly << ", seed: " << lfsrSeed << endl;
					f.flush();
				}
				nTest2=lfsrNum;
				nTest3=lfsrNum;
			}
			end = clock();
			ar = getResults();
			ar.time = (end-start)/(double)CLOCKS_PER_SEC;
			fl->updateFaultList();
			fl->writeFaultMask(maskStream);
			writeResults(ar);

			if (uFaultMode == 1) fl->writeABFaults(udFaultsStream);
			else if (uFaultMode == 2) fl->writeUDFaults(udFaultsStream);
		} else
		{
			// Open pattern File for writing
			if(sPatternFile.length() > 0) {
				patternFile.open(sPatternFile.c_str(), fstream::out);
				if(!patternFile.is_open()) {
					stringstream ss;
					ss << "Fatal error: Cannot open pattern file " << sPatternFile;
					throw ss.str();
				}
				else {
					patternStream = patternFile.rdbuf();
				}
			}

			generateTest();
			ar = getResults();
			fl->updateFaultList();

			if(wTestMode==4) simulateAllVectors();
			end = clock();
			//			Sleep(5000);
			ar.time = (end-start)/(double)CLOCKS_PER_SEC;
			writeResults(ar);
			switch (wTestMode)
			{
			case 0: break;
			case 1: writeTestFile(); break;
			case 2: writeTestFileOut(); break;
			case 3: writeMultiTestFile(); break;
			case 4: writeMultiTestFileMask(); break;
			}

			fl->writeFaultMask(maskStream);
			if(uFaultMode == 1 ) fl->writeABFaults(udFaultsStream);
			else if ( uFaultMode == 2 ) fl->writeUDFaults(udFaultsStream);
		}
		printf("\n Computing time: %.2fs\n", (end-start)/(double)CLOCKS_PER_SEC);

		//Close opened files
		if(benchFile.is_open()) { benchFile.close(); benchStream = NULL; };
		if(faultFile.is_open()) { faultFile.close(); faultStream = NULL; };
		if(patternFile.is_open()) { patternFile.close(); patternStream = NULL; };
		if(udFaultsFile.is_open()) { udFaultsFile.close(); udFaultsStream = NULL; };
		if(wFaultFile.is_open()) { wFaultFile.close(); wFaultStream = NULL; };
		if(maskFile.is_open()) { maskFile.close(); maskStream = NULL; };
		if(reportFile.is_open()) { reportFile.close(); reportStream = NULL; };
		if(genResFile.is_open()) { genResFile.close(); genResStream = NULL; };
#ifdef _ALG_DEBUG
		if(dbgFile.is_open()) { dbgFile.close(); };
#endif
		return 0;
	}

	int Atalanta::run(int argc, char **argv)
	{

		optionSet(argc,argv);
		return run();
	}
	void Atalanta::setParams(Params *p) {
		cctMode = p->getCctMode();
		randomLimit = p->getRandomLimit();
		iseed = p->getIseed();
		maxCompact = p->getMaxCompact();
		compact = p->getCompact();
		maxBackTrack = p->getMaxBackTrack();
		maxBackTrack1 = p->getMaxBackTrack1();
		if(p->getSPatternStream() != NULL) {
			patternStream = p->getSPatternStream();
		}
		else {
			sPatternFile = p->getSPatternFile();
		}
		if(p->getBenchStream() != NULL) {
			benchStream = p->getBenchStream();
		}
		else {
			if(p->getBenchFile().length()) {
				OpenFile(&benchFile, &benchStream, p->getBenchFile(), ios::in);
			}
		}
		learnMode = p->getLearnMode();
		faultMode = p->getFaultMode();
		if(p->getFaultStream() != NULL) {
			faultStream = p->getFaultStream();
		}
		else {
			if(p->getFaultFile().length()) {
				OpenFile(&faultFile, &faultStream, p->getFaultFile(), ios::in);
			}}
		//faultFile = p->getFaultFile();
		if(p->getFaultStream() != NULL) {
			faultStream = p->getFaultStream();
		}
		else {
			if(p->getWFaultFile().length()) {
				OpenFile(&wFaultFile, &wFaultStream, p->getWFaultFile(), ios::out);
			}
		}
		//wFaultFile = p->getWFaultFile();
		simMode = p->getSimMode();
		fillMode = p->getFillMode();
		genAllPat = p->getGenAllPat();
		setEachLimit(p->getEachLimit());
		noFaultSim = p->getNoFaultSim();
		uFaultMode = p->getUFaultMode();
		if(p->getUdFaultsStream() != NULL) {
			udFaultsStream = p->getUdFaultsStream();
		}
		else {
			if(p->getUdFaultsFile().length()) {
				OpenFile(&udFaultsFile, &udFaultsStream, p->getUdFaultsFile(), ios::out);
			}
		}
		//udFaultsFile = p->getUdFaultsFile();
		simulationMode = p->getSimulationMode();
		if(p->getMaskStream() != NULL) {
			maskStream = p->getMaskStream();
		}
		else {
			if(p->getMaskFile().length()) {
				OpenFile(&maskFile, &maskStream, p->getMaskFile(), ios::out);
			}
		}
		//maskFile = p->getMaskFile();
		if(p->getReportStream() != NULL) {
			reportStream = p->getReportStream();
		}
		else {
			if(p->getReportFile().length()) {
				OpenFile(&reportFile, &reportStream, p->getReportFile(), ios::out);
			}
		}
		//reportFile = p->getReportFile();
		wTestMode = p->getWTestMode();
		lfsrSimMode = p->getLfsrSimMode();
		lfsrPoly = p->getLfsrPoly();
		lfsrSeed = p->getLfsrSeed();
		lfsrNum = p->getLfsrNum();	
	}

	void Atalanta::OpenFile(fstream *file, streambuf **buf, string filename, ios_base::open_mode mode) {
		file->open(filename.data(), mode); 
		if(!file->is_open()) {
			stringstream ss;
			ss << "Fatal error: Cannot open processed faults file: " << filename;
			throw ss.str();
		}
		else {
			if(buf != NULL) {
				(*buf) = file->rdbuf();
			}
			//wFaultStream = wFaultFile.rdbuf();
		}
	}
}
