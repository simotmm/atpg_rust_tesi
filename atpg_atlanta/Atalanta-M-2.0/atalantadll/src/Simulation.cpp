// Simulation.cpp: implementation of the Simulation class.
//
//////////////////////////////////////////////////////////////////////

#include <string>
#include <list>
#include <iostream>
#include <fstream>
#include <sstream>

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
#include "Random.h"

#include "Simulation.h"

namespace atalantadll {
	//////////////////////////////////////////////////////////////////////
	// Construction/Destruction
	//////////////////////////////////////////////////////////////////////

	/*Simulation::Simulation()
	{

	}*/

	void Simulation::setBit(int *word, int nth) {*word |= BITMASK[nth];}
	void Simulation::resetBit(int *word,int nth) {*word &= ~BITMASK[nth];}
	void Simulation::setb0(int *word0, int *word1, int nth) { *word0 |= BITMASK[nth]; *word1 &= (~BITMASK[nth]); }
	void Simulation::setb1(int *word0, int *word1, int nth) { *word0 &= (~BITMASK[nth]); *word1 |= BITMASK[nth]; }
	void Simulation::setbx(int *word0, int *word1, int nth) { *word0 &= (~BITMASK[nth]); *word1 &= (~BITMASK[nth]); }

	int Simulation::randomFsim(int levels,int nStem,Gate **stem,level *lfsr,int limit,int maxBit,int maxDetect,int *nTest,int *nPacket,int *nBit)
	{
		int iteration=0;
		int i,j;
		int profile[BITSIZE];
		int nDetect=0;
		level value;

		while(iteration<limit)
		{
			Random::getPRandompattern(numberOfPrimaryInputs,lfsr);



			for(i=0;i<numberOfPrimaryInputs;i++) {
				net[i]->output1=net[i]->output=lfsr[i];
			}
			pFaultFreeSimulation();
			for(i=0;i<maxBit;i++) profile[i]=0;
			if(fault1Simulation(levels,nStem,stem,maxBit,profile)>0)
			{
				iteration=0;
				for(i=maxBit-1;i>=0;i--)
					if(profile[i]>0)
					{
						(*nTest)++;
						nDetect+=profile[i];
						for(j=0;j<numberOfPrimaryInputs;j++) {
							if((net[j]->output1 & BITMASK[i]) != ALL0) {
								setBit(&testVectors[*nPacket][j],*nBit);
							}
							else
								resetBit(&testVectors[*nPacket][j],*nBit);
						}

						if(++(*nBit)==maxBit) {
							*nBit=0; 
							(*nPacket)++;
						}
						if(compact=='n') printIO(i,*nTest);
					}
					if(nDetect>=maxDetect) break;
			}
			else iteration++;

			for(i=0;i<=nsStack;i++) (*dynamicStack)[i]->cobserve=ALL0;
			if(updateFlag)
			{
				updateAll1();
				updateFlag=false;
			}
			else for(i=ndStack;i>nsStack;i--) (*dynamicStack)[i]->freach=0;
			ndStack=nsStack;
		}

		return nDetect;
	}

	int Simulation::randomHope(level *lfsr,int limit,int maxBit,int maxDetect,int *nTest,int *nPacket,int *nBit)
	{
		int iteration=0;
		int i,j,n,n1;
		int nDetect=0;
		int ranTest=0;

		while(iteration<limit)
		{
			Random::getPRandompattern(numberOfPrimaryInputs,lfsr);
			for(n=0, i=maxBit-1; i>=0; i--)
			{
				for(j=0;j<numberOfPrimaryInputs;j++) {
					inVal[j]=((lfsr[j]&BITMASK[i])==ALL0)?ZERO:ONE;
				}
				goodSim(++ranTest);
				if((n1=simulation())>0)
				{
					n+=n1;
					(*nTest)++;
					nDetect+=n1;
					for(j=0;j<numberOfPrimaryInputs;j++)
						if(inVal[j]==ONE)
						{
							setBit(&testVectors[*nPacket][j],*nBit);
							resetBit(&testVectors1[*nPacket][j],*nBit);
						} else
						{
							resetBit(&testVectors[*nPacket][j],*nBit);
							setBit(&testVectors1[*nPacket][j],*nBit);
						}
						if(++(*nBit)==maxBit) {*nBit=0; (*nPacket)++;}
						if(compact=='n') printIOValues(primaryIn,primaryOut);

						if(nDetect>=maxDetect) break;
				}
			}
			iteration=(n>0) ? 0 : iteration+1;
			if(nDetect>=maxDetect) break;
		}

		return nDetect;
	}

	int Simulation::randomSim(int levels,int nStem,Gate **stem,level *lfsr,int limit,int maxBit,int maxDetect,int *nTest,int *nPacket,int *nBit)
	{
		if(simMode=='f')
			return randomFsim(levels,nStem,stem,lfsr,limit,maxBit,maxDetect,nTest,nPacket,nBit);
		else
			return randomHope(lfsr,limit,maxBit,maxDetect,nTest,nPacket,nBit);
	}

	int Simulation::simulateHope(int *nPacket,int *nBit)
	{
		int ranTest=0;
		int j,n1;

		goodSim(++ranTest);
		n1=simulation();

		for(j=0;j<numberOfPrimaryInputs;j++)                // ????
			if(inVal[j]==ONE)
			{
				setBit(&testVectors[*nPacket][j],*nBit);
				resetBit(&testVectors1[*nPacket][j],*nBit);
			} else
			{
				resetBit(&testVectors[*nPacket][j],*nBit);
				setBit(&testVectors1[*nPacket][j],*nBit);
			}
			return n1;
	}

	int Simulation::tGenSim(int levels,int nStem,Gate **stem,int nTest,int *profile)
	{
		if(simMode=='f')
			return fault0Simulation(levels,1,profile);
		else
		{
			goodSim(nTest);
			return simulation();
		}
	}

	void Simulation::fillPatternsFsim(char mode,int nPacket,int nBit)
	{	
		int j,ran;

		switch(mode)
		{
		case '0':
			for(j=0;j<numberOfPrimaryInputs;j++)
				switch(net[j]->output)
			{
				case ONE:
					setBit(&testVectors[nPacket][j],nBit);
					net[j]->output1=ALL1;
					break;
				default:
					resetBit(&testVectors[nPacket][j],nBit);
					net[j]->output1=ALL0;
			}
			break;
		case '1':
			for(j=0;j<numberOfPrimaryInputs;j++)
				switch(net[j]->output)
			{
				case ZERO:	
					resetBit(&testVectors[nPacket][j],nBit);
					net[j]->output1=ALL0;
					break;
				default:
					resetBit(&testVectors1[nPacket][j],nBit);
					net[j]->output1=ALL1;
			}
			break;
		case 'r':
		case 'x':
			for(j=0;j<numberOfPrimaryInputs;j++)
				switch(net[j]->output)
			{
				case ZERO:
					resetBit(&testVectors[nPacket][j],nBit);
					net[j]->output1=ALL0;
					break;
				case ONE:
					setBit(&testVectors[nPacket][j],nBit);
					net[j]->output1=ALL1;
					break;		
				default:
					ran=(int)rand()&01;
					if(ran!=0)
						setBit(&testVectors[nPacket][j],nBit);
					else
						resetBit(&testVectors[nPacket][j],nBit);
					net[j]->output1=ran;
			}
		}
	}

	void Simulation::fillPatternsHope(char mode,int nPacket,int nBit)
	{
		int j;

		switch(mode)
		{
		case '0':
			for(j=0;j<numberOfPrimaryInputs;j++)
				switch(net[j]->output)
			{
				case ONE: 
					setb1(&testVectors[nPacket][j], &testVectors1[nPacket][j],nBit);
					inVal[j]=ONE;
					break;
				default: 
					setb0(&testVectors[nPacket][j], &testVectors1[nPacket][j],nBit);
					inVal[j]=ZERO;
					break;
			}
			break;
		case '1':
			for(j=0;j<numberOfPrimaryInputs;j++)
				switch(net[j]->output)
			{
				case ZERO: 
					setb0(&testVectors[nPacket][j], &testVectors1[nPacket][j],nBit);
					inVal[j]=ZERO;
					break;
				default:
					setb1(&testVectors[nPacket][j], &testVectors1[nPacket][j],nBit);
					inVal[j]=ONE;
					break;
			}
			break;
		case 'r':
			for(j=0;j<numberOfPrimaryInputs;j++)
				switch(net[j]->output)
			{
				case ZERO:
					setb0(&testVectors[nPacket][j], &testVectors1[nPacket][j],nBit);
					inVal[j]=ZERO;
					break;
				case ONE:
					setb1(&testVectors[nPacket][j], &testVectors1[nPacket][j],nBit);
					inVal[j]=ONE;
					break;
				default:
					if((inVal[j]=(int)rand()&01) != 0)
					{
						setb1(&testVectors[nPacket][j], &testVectors1[nPacket][j],nBit);
					} else
					{
						setb0(&testVectors[nPacket][j], &testVectors1[nPacket][j],nBit);
					}
			}
			break;
		case 'x':
			for(j=0;j<numberOfPrimaryInputs;j++)
				switch(net[j]->output)
			{
				case ZERO: 
					setb0(&testVectors[nPacket][j], &testVectors1[nPacket][j],nBit);
					inVal[j]=ZERO;
					break;
				case ONE:
					setb1(&testVectors[nPacket][j], &testVectors1[nPacket][j],nBit);
					inVal[j]=ONE;
					break;
				default:
					inVal[j]=X;
					setbx(&testVectors[nPacket][j], &testVectors1[nPacket][j],nBit);
					break;
			}
		}
	}

	void Simulation::fillPatterns(int mode,int nPacket,int nBit)
	{
		if(simMode=='f')
			fillPatternsFsim(mode,nPacket,nBit);
		else
			fillPatternsHope(mode,nPacket,nBit);
	}

	int Simulation::testGen(int levels,int maxBits,int nStem,Gate **stem,int maxBackTrack,int phase,int *nRedundant,int *nOverBackTrack,int *nBackTrack,int *nTest,int *nPacket,int *nBit,double *fanTime)
	{
		int j,nBack;
		status faultSelectionMode;
		int lastFault;
		int state;
		int nDetect=0;
		int profile[BITSIZE];
		bool done;
		Fault *pCurrentFault;
		Gate *gut;
		double seconds,minutes,runtime1,runtime2;

		faultSelectionMode=DEFAULTMODE;
		lastFault=numberOfFaults;
		allOne=~(ALL1<<1);
		done=false;

		while(!done)
		{
			if(maxBackTrack==0) break;

			// select any undetected and untried fault
			pCurrentFault=0;
			switch(faultSelectionMode)
			{
			case CHECKPOINTMODE:
				while(--lastFault>=0)
					if(faultList[lastFault]->detected==UNDETECTED)
					{
						pCurrentFault=faultList[lastFault];
						gut=pCurrentFault->gate;
						if(pCurrentFault->line!=OUTFAULT)
							gut=gut->inlis[pCurrentFault->line];
						if(gut->isCheckPoint()) break;
						pCurrentFault=0;
					}
					if(pCurrentFault==0)
					{
						faultSelectionMode=DEFAULTMODE;
						lastFault=numberOfFaults;
					}
					break;

			default:
				while(--lastFault>=0)
					if(faultList[lastFault]->detected==UNDETECTED)
					{
						pCurrentFault=faultList[lastFault];
						//pCurrentFault=faultList[485];
						break;
					}
					if(pCurrentFault==0) done=true;;
			}

			//printf("%d\n", pCurrentFault->index);
			if(pCurrentFault==0) continue;
			gut=pCurrentFault->gate;

			nTestEach=0;
			myCurrFault=pCurrentFault; //added by me	

			/*      if(no_faultsim=='y') {
			printfault(test,pcurrentfault,0);
			if(logmode=='y') printfault(logfile,pcurrentfault,0);
			}
			Removed my me! */

			getTime(&minutes,&seconds,&runtime1);

			// test pattern generation using fan
			state = (phase==false) ? 
				fan(levels,pCurrentFault,maxBackTrack,&nBack) :
			fan1(levels,pCurrentFault,maxBackTrack,&nBack);
			(*nBackTrack)+=nBack;

			getTime(&minutes,&seconds,&runtime2);
			(*fanTime)+=(runtime2-runtime1);

			if(noFaultSim=='y')
			{
				(*nTest)+=nTestEach;
				if(nTestEach > 0)
				{
					pCurrentFault->detected=DETECTED;
					nDetect++;
				}
				else if(state==NO_TEST)
				{	// redundant faults
					pCurrentFault->detected=REDUNDANT;
					(*nRedundant)++;
				} else
				{	// over backtracking
					(*nOverBackTrack)++;
					pCurrentFault->detected=PROCESSED;
				}
			} 
			else if(state==TEST_FOUND)
			{	// fault is detected, delete the detected fault from fault list
				pCurrentFault->detected=PROCESSED;

				/*			pcurrentfault->detected=DETECTED;
				gut->nfault--;
				ndetect++;*/

				// assign random zero and ones to the unassigned bits
				(*nTest)++;
				fillPatterns(fillMode,*nPacket,*nBit);
				if(simMode=='f')
					for(j=0;j<numberOfPrimaryInputs;j++)
					{
						net[j]->changed=false;
						net[j]->freach=false;
						net[j]->cobserve=ALL0;
						net[j]->output=net[j]->output1;
					}
				else
					for(j=0;j<numberOfGates;j++)
					{
						net[j]->changed=false;
						net[j]->freach=false;
					}
					if(++(*nBit)==maxBits) {*nBit=0; (*nPacket)++;}
					stack->clear();

					// fault simulation 
					profile[0]=0;
					profile[0] =
						tGenSim(levels,nStem,stem,*nTest,profile);
					nDetect += profile[0];

					if(compact=='n')
					{
						if(simMode=='f')
						{
							/*printio(test,nopi,nopo,j,*ntest);KHB*/      // ????
							printIO(0,*nTest);
							/*   if(logmode=='y') {
							fprintf(logfile,"test %4d: ",*ntest);
							printinputs(logfile,nopi,0);
							fprintf(logfile," ");
							printoutputs(logfile,nopo,0);
							fprintf(logfile," %4d faults detected\n",profile[0]);
							}     */
						} else
						{
							//   fprintf(test,"test %4d: ",*ntest);
							printIOValues(primaryIn,primaryOut);
							//   fprintf(test," ");
							//   my_printiovalues(primaryout,nopo,'o','g',0);
							//   fprintf(test,"\n");
							/*   if(logmode=='y') {
							fprintf(logfile,"test %4d: ",*ntest);
							my_printiovalues(logfile,primaryin,nopi,'o','g',0);
							fprintf(logfile," ");
							my_printiovalues(logfile,primaryout,nopo,'o','g',0);
							fprintf(logfile," %4d faults detected",profile[0]);
							fprintf(logfile,"\n");
							}  */
						}
					}

					if(pCurrentFault->detected!=DETECTED)
					{
						cout<<"Error in test generation:" << pCurrentFault->index;
						cout<<endl;
					}
			}
			else if(state==NO_TEST)
			{	// redundant faults
				pCurrentFault->detected=REDUNDANT;
				(*nRedundant)++;
				if(simMode=='f')
				{
					gut->pfault.remove(pCurrentFault);
					if(gut->pfault.empty()) updateFlag=true;
				}
			}
			else
			{		 // over backtracking
				(*nOverBackTrack)++;
				pCurrentFault->detected=PROCESSED;
			}
		}

		return nDetect;
	}

	void Simulation::randomTestFsim(TestVectorsData *testSt, TestVectorsData *testVect,
		int pack,int noBit)
	{
		int array[400*BITSIZE];
		int array1[400*BITSIZE];
		int i,j,x,bits,k,B,nbit=0,npacket=0;
		int maxbits=BITSIZE;

		bits=32*pack+noBit;
		for(i=0;i<bits;i++)array[i]=i;

		j=0;
		for(i=bits-1;i>=0;i--)
		{
			x=rand()%(i+1);
			array1[j]=array[x];
			array[x]=array[i];
			j++;
		}
		for(i=0;i<bits;i++)
		{
			x=array1[i];
			k=x/32;
			B=x%32;
			for(j=0;j<numberOfPrimaryInputs;j++)
				if(((*testSt)[k][j]&BITMASK[B])!=ALL0)
					setBit(&(*testVect)[npacket][j],nbit);
				else
					resetBit(&(*testVect)[npacket][j],nbit);
			if(++nbit==maxbits) {nbit=0;npacket++;}
		}
	}

	int Simulation::reverseFsim(int levels,int nStem,Gate **stem,int *nDet,int nPacket,int nBit,int maxBits)
	{
		int i, j, k, n;
		int nRestoredFault;
		int nDetect=0;
		int noTest=0;
		int profile[BITSIZE];

		for(i=0;i<numberOfGates;i++) net[i]->pfault.clear();

		if((nRestoredFault=restoreDetectedFaultList())<0)
		{
			/*cout<<"error occurred in restoration of fault list";
			cout<<endl;
			exit(0);*/
			stringstream ss;
			ss << "error occurred in restoration of fault list";
			throw ss.str();
		}

		pInitSimulation(levels);

		updateFlag=false;
		if(nBit==0) {--nPacket; nBit=maxBits;}

		// reverse fault simulation
		k=nPacket+1;
		while(--k>=0)
		{
			if(nDetect>=nRestoredFault) break;
			if(k<nPacket) nBit=maxBits;

			allOne= (nBit==BITSIZE) ? ALL1 : ~(ALL1<<nBit);

			for(j=0;j<numberOfPrimaryInputs;j++)
				net[j]->output1=net[j]->output=testVectors[k][j];
			pFaultFreeSimulation();

			for(i=0;i<nBit;i++) profile[i]=0;
			if((n = fault1Simulation(levels,nStem,stem,nBit,profile))>0)
			{
				nDetect+=n;

				// print out test files
				for(i=nBit-1;i>=0;i--)
					if(profile[i]>0)
					{
						noTest++;
						if(compact=='r')
						{
							printIO(i,noTest );
							/*						if(logmode=='y')
							{
							fprintf(logfile,"test %4d: ",no_test);
							printinputs(logfile,nopi,i);
							fprintf(logfile," ");
							printoutputs(logfile,nopo,i);
							fprintf(logfile," %4d faults detected\n",profile[i]);
							}*/
						}
					}
			}

			for(i=0;i<=nsStack;i++) (*dynamicStack)[i]->cobserve=ALL0;
			if(updateFlag)
			{
				updateAll1();
				updateFlag=false;
			}
			else
				for(i=ndStack;i>nsStack;i--) (*dynamicStack)[i]->freach=0;

			ndStack=nsStack;
		}

		*nDet=nDetect;
		return(noTest);
	}

	int Simulation::shuffleFsim(int levels,int nStem,Gate **stem,int *nShuf,int *nDet,int nPacket,int nBit,int maxBits)
	{
		int i, j, k, n;
		int nRestoredFault;
		int nDetect=0;
		int noTest=0;
		int nComp=INFINITY, stop=ONE;
		int bit=0, packet=0;
		int profile[BITSIZE];
		int nArray[MAXTEST], store=0;
		bool done, flagBit;


		for(i=0;i<=maxCompact;i++) nArray[i]=0;
		done=false;
		flagBit=false;
		*nShuf=0;

		// shufle fault simulation
		if(compact=='s')
		{
			while((!done))
			{
				(*nShuf)++;
				for(i=0;i<numberOfGates;i++) net[i]->pfault.clear();

				if((nRestoredFault=restoreDetectedFaultList())<0)
				{
					/*cout<<"error occurred in restoration of fault list\n";
					cout<<endl;
					exit(0);*/
					stringstream ss;
					ss << "error occurred in restoration of fault list";
					throw ss.str();
				}

				pInitSimulation(levels);

				updateFlag=false;
				if(flagBit)
				{
					nBit=bit;   
					nPacket=packet;
					//shuffles the test patterns and stores it back in the random fashion
					randomTestFsim(&testStore, &testVectors, packet, bit);
					bit=packet=0;
					for(nComp=0;nComp<=maxCompact-1;nComp++)
					{
						stop=STOP;
						if(nArray[nComp]!=nArray[nComp+1])
						{
							stop=TWO;
							break;
						}
					}
				}

				noTest=0;
				nDetect=0;

				if(nBit==0) {--nPacket; nBit=maxBits;}
				k=nPacket+1;
				while(--k>=0)
				{
					if(nDetect>=nRestoredFault) break;
					if(k<nPacket) nBit=maxBits;
					allOne= nBit==BITSIZE ? ALL1 : ~(ALL1<<nBit);

					for(j=0;j<numberOfPrimaryInputs;j++)
						net[j]->output1=net[j]->output=testVectors[k][j];
					pFaultFreeSimulation();

					for(i=0;i<nBit;i++) profile[i]=0;
					if((n = fault1Simulation(levels,nStem,stem,nBit,profile))>0)
					{
						nDetect+=n;

						// print out test files
						for(i=nBit-1;i>=0;i--)
							if(profile[i]>0)
							{
								noTest++;
								if(stop==STOP)
								{
									printIO(i,noTest );
									/*							    if(logmode=='y')
									{
									fprintf(logfile,"test %4d: ",no_test);
									printinputs(logfile,nopi,i);
									fprintf(logfile," ");
									printoutputs(logfile,nopo,i);
									fprintf(logfile," %4d faults detected\n",profile[i]);
									}*/
									done=true;
								}
								flagBit=true;
								for(j=0;j<numberOfPrimaryInputs;j++)
									if((net[j]->output1&BITMASK[i])!=ALL0)
										setBit(&testStore[packet][j],bit);
									else
										resetBit(&testStore[packet][j],bit);
								if(++bit==maxBits) {bit=0; packet++;}
							}
					}


					for(i=0;i<=nsStack;i++) (*dynamicStack)[i]->cobserve=ALL0;
					if(updateFlag)
					{
						updateAll1();
						updateFlag=false;
					}
					else
						for(i=ndStack;i>nsStack;i--) (*dynamicStack)[i]->freach=0;

					ndStack=nsStack;
				}

				if(store==maxCompact+1) store=0;
				nArray[store]=noTest;
				store++;
			}
		}

		*nDet=nDetect;
		return noTest;
	}

	void Simulation::randomTestHope(TestVectorsData *testSt0, TestVectorsData *testSt1,
		TestVectorsData *testVect0,TestVectorsData *testVect1,int pack,int noBit)
	{
		int array[40*BITSIZE];
		int array1[40*BITSIZE];
		int i,j,x,bits,k,B,nBit=0,nPacket=0;
		int maxbits=BITSIZE;

		bits=32*pack+noBit;
		for(i=0;i<bits;i++) array[i]=i;

		j=0;
		for(i=bits-1;i>=0;i--)
		{
			x=rand()%(i+1);
			array1[j]=array[x];
			array[x]=array[i];
			j++;	
		}

		for(i=0;i<bits;i++)
		{
			x=array1[i];
			k=x/32;
			B=x%32;
			for(j=0;j<numberOfPrimaryInputs;j++)
			{
				if((((*testSt0)[k][j])&BITMASK[B])!=ALL0)
				{ setBit(&(*testVect0)[nPacket][j],nBit); }
				else
				{ resetBit(&(*testVect0)[nPacket][j],nBit); }

				if((((*testSt1)[k][j])&BITMASK[B])!=ALL0)
				{ setBit(&(*testVect1)[nPacket][j],nBit); }
				else
				{ resetBit(&(*testVect1)[nPacket][j],nBit); }
			}
			if(++nBit==maxbits){nBit=0;nPacket++;}
		}
	}

	int Simulation::reverseHope(int *nDet,int nPacket,int nBit,int maxBits)
	{
		int i, j, k, n;
		level v1, v2;
		int nRestoredFault;
		int nDetect=0;
		int noTest=0;
		int nComp=INFINITY, stop=ONE;
		int bit=0, packet=0;

		if((nRestoredFault=restoreHopeFaultList())<0)
		{
			/*printf("error occurred in restoration of fault list\n");
			exit(0);*/
			stringstream ss;
			ss << "error occurred in restoration of fault list";
			throw ss.str();
		}

		potentialFault=hopeFaultList.back();

		for(i=0; i<numberOfGates; i++) { net[i]->changed=false;}

		if(nBit==0) {--nPacket; nBit=maxBits;}

		// reverse fault simulation
		k=nPacket+1;
		while(--k>=0)
		{
			if(nDetect>=nRestoredFault) break;
			if(k<nPacket) nBit=maxBits;
			for(i=nBit-1; i>=0; i--)
			{
				for(j=0;j<numberOfPrimaryInputs;j++)
				{
					v1=(((testVectors[k][j])&BITMASK[i])==ALL0)?ZERO:ONE;
					v2=(((testVectors1[k][j])&BITMASK[i])==ALL0)?ZERO:ONE;
					inVal[j]=(v1==ONE)?ZERO:(v2==ONE)?ONE:X;
				}
				goodSim(2);
				if((n=simulation()) > 0)
				{
					nDetect+=n;
					noTest++;
					if(compact=='r')
					{
						//  fprintf(test,"test %4d: ",no_test);
						printIOValues(primaryIn, primaryOut);
						//   fprintf(test," ");
						//   my_printiovalues(primaryout,nopo,'o','g',0);
						//   fprintf(test,"\n");
						/*					if(logmode=='y')
						{				
						fprintf(logfile,"test %4d: ",no_test);
						my_printiovalues(logfile,primaryin,nopi,'o','g',0);
						fprintf(logfile," ");
						my_printiovalues(logfile,primaryout,nopo,'o','g',0);
						fprintf(logfile," %4d faults detected",n);
						fprintf(logfile,"\n");
						}*/
					}
					if(nDetect>=nRestoredFault) break;
				}
			}
		}

		*nDet=nDetect;
		return noTest;
	}

	int Simulation::shuffleHope(int *nShuf,int *nDet,int nPacket,int nBit,int maxBits)
	{
		int i, j, k, n;
		level v1, v2;
		int nRestoredFault;
		int nDetect=0;
		int noTest=0;
		int nComp=INFINITY, stop=ONE;
		int bit=0, packet=0;
		int nArray[MAXTEST], store=0;
		bool done, flagBit;

		for(i=0;i<=maxCompact;i++) nArray[i]=0;
		done=false;
		flagBit=false;
		*nShuf=0;

		// shufle fault simulation
		if(compact=='s')
		{
			while((!done))
			{
				(*nShuf)++;
				if((nRestoredFault=restoreHopeFaultList())<0)
				{
					/*cout<<"error occurred in restoration of fault list";
					cout<<endl;
					exit(0);*/
					stringstream ss;
					ss << "error occurred in restoration of fault list";
					throw ss.str();
				}

				potentialFault=hopeFaultList.back();
				for(i=0; i<numberOfGates; i++) net[i]->changed=false;

				if(flagBit)
				{
					nBit=bit;   
					nPacket=packet;
					//shuffles the test patterns and stores it back in the random fashion
					randomTestHope(&testStore, &testStore1, &testVectors, &testVectors1,packet,bit);
					bit=packet=0;
					for(nComp=0;nComp<=maxCompact-1;nComp++)
					{
						stop=STOP;
						if(nArray[nComp]!=nArray[nComp+1])
						{
							stop=TWO;
							break;
						}
					}
				}

				noTest=0;
				nDetect=0;

				if(nBit==0) {--nPacket; nBit=maxBits;}
				k=nPacket+1;
				while(--k>=0)
				{
					if(nDetect>=nRestoredFault) break;
					if(k<nPacket) nBit=maxBits;
					for(i=nBit-1; i>=0; i--)
					{
						for(j=0;j<numberOfPrimaryInputs;j++)
						{
							v1=(((testVectors[k][j])&BITMASK[i])==ALL0)?ZERO:ONE;
							v2=(((testVectors1[k][j])&BITMASK[i])==ALL0)?ZERO:ONE;
							inVal[j]=(v1==ONE)?ZERO:(v2==ONE)?ONE:X;
						}

						goodSim(2);
						if((n=simulation()) > 0)
						{
							nDetect+=n;
							noTest++;
							if(stop==STOP)
							{
								//fprintf(test,"test %4d: ",no_test);
								printIOValues(primaryIn, primaryOut);
								//fprintf(test," ");
								//my_printiovalues(primaryout,nopo,'o','g',0);
								//fprintf(test,"\n");
								/*							if(logmode=='y')
								{
								fprintf(logfile,"test %4d: ",no_test);
								my_printiovalues(logfile,primaryin,nopi,'o','g',0);
								fprintf(logfile," ");
								my_printiovalues(logfile,primaryout,nopo,'o','g',0);
								fprintf(logfile," %4d faults detected",n);
								fprintf(logfile,"\n");
								}*/
								done=true;
							}

							flagBit=true;
							for(j=0;j<numberOfPrimaryInputs;j++)
							{
								switch(inVal[j])
								{
								case ONE:
									resetBit(&testStore[packet][j],bit);
									setBit(&testStore1[packet][j],bit);
									break;
								case ZERO:
									setBit(&testStore[packet][j],bit);
									resetBit(&testStore1[packet][j],bit);
									break;
								default:
									resetBit(&testStore[packet][j],bit);
									resetBit(&testStore1[packet][j],bit);
									break;
								}
							}
							if(++bit==maxBits) {bit=0; packet++;}
						}
						if(nDetect>=nRestoredFault) break;
					}
				}
				if(store==maxCompact+1) store=0;
				nArray[store]=noTest;
				store++;
			}
		}

		*nDet=nDetect;
		return noTest;
	}

	int Simulation::compactTest(int levels,int nStem,Gate **stem,int *nShuf,int *nDet,int nPacket,int nBit,int maxBits)
	{
		*nShuf=0;
		if(simMode=='f')
		{
			if(compact=='s')
				return(shuffleFsim(levels,nStem,stem,nShuf,nDet,nPacket,nBit,maxBits));
			else
				return(reverseFsim(levels,nStem,stem,nDet,nPacket,nBit,maxBits));
		} else
		{
			if(compact=='s')
				return(shuffleHope(nShuf,nDet,nPacket,nBit,maxBits));
			else
				return(reverseHope(nDet,nPacket,nBit,maxBits));
		}
	}

	/*Simulation::~Simulation()
	{

	}*/
}
