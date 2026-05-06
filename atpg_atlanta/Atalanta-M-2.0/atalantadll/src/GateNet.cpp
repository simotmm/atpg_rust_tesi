// GateNet.cpp: implementation of the GateNet class.
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
#include "Error.h"

#include "Hash.h"
#include "Gate.h"
#include "Stack.h"
#include "GateNet.h"
#include "Atpg.h"
#include "Globals.h"

//#include "fault.h"

namespace atalantadll {
	//////////////////////////////////////////////////////////////////////
	// Construction/Destruction
	//////////////////////////////////////////////////////////////////////
	
	void GateNet::checkParameters()
	{
		if(numberOfGates<=0 || numberOfPrimaryInputs<=0 || numberOfPrimaryOutputs<=0)
		{
			stringstream ss;
			ss <<"Error: #pi="<<numberOfPrimaryInputs<<", #po="<<numberOfPrimaryOutputs<<", #gate="<<numberOfGates;
			throw ss.str();
			//cerr<<"Error: #pi="<<numberOfPrimaryInputs<<", #po="<<numberOfPrimaryOutputs<<", #gate="<<numberOfGates<<endl;
			//Error::fatalerror(CIRCUITERROR);
		}
		
		if(numberOfFlipFlops > 0)
		{
			stringstream ss;
			ss  << "Error: "<<numberOfFlipFlops<<" flip-flop exist in the circuit";
			throw ss.str();
			throw ss.str();
			/*cerr << "Error: "<<numberOfFlipFlops<<" flip-flop exist in the circuit."<<endl;
			Error::fatalerror(CIRCUITERROR);*/
		}
	}
	
	level GateNet::logiclevel(level V0,level V1,int n)
	{
		V0=((V0 & BITMASK[n]) == ALL0) ? ZERO : ONE;
		V1=((V1 & BITMASK[n]) == ALL0) ? ZERO : ONE;
		return(parallelToLevel[V0][V1]);
	}
	
	void GateNet::allocateStacks()
	{
		stack1=new Stack(numberOfGates+numberOfPrimaryOutputs);
		stack2=new Stack(numberOfGates+numberOfPrimaryOutputs);
	}
	
#ifdef INCLUDE_HOPE
	void GateNet::printIOValues(vector<int> iarray, vector<int> oarray)
	{
		int j;
		Gate *gut;
		string iv,ov;
		
		iv.reserve(numberOfPrimaryInputs+1);
		ov.reserve(numberOfPrimaryOutputs+1);
		
		for(j = 0; j < numberOfPrimaryInputs; j++)
		{
			gut=net[iarray[j]];
			iv[j] = levelToString[logiclevel(gut->GV[0],gut->GV[1],0)][0];
		}
		iv[j] = 0;
		for(j = 0; j < numberOfPrimaryOutputs; j++) {
			gut=net[oarray[j]];
			ov[j] = levelToString[logiclevel(gut->GV[0],gut->GV[1],0)][0];
		}
		ov[j] = 0;
		addTestVector( &iv, &ov, 1 );
	}
#endif
	
	void GateNet::addTestVector(string *ivct,string *ovct,int no)
	{
		TestVectorType *testv;
		testv = (TestVectorType *)malloc(sizeof(TestVectorType));
		testv->ivct = (char *)malloc( tv.inpVars + 1 );
		testv->mask = "";
		strcpy(testv->ivct, ivct->c_str());
		if ( ovct != NULL ) {
			testv->ovct = (char *)malloc( tv.outVars + 1 );
			strcpy(testv->ovct, ovct->c_str());
		} else testv->ovct = NULL;
		if ( myCurrFault != NULL ) {
			if(myCurrFault->line >= 0)
				testv->fltLineHash = myCurrFault->gate->inlis[myCurrFault->line]->symbol->key;
			else testv->fltLineHash = -1;
			testv->fltHash = myCurrFault->gate->symbol->key;
			testv->type = myCurrFault->type % 2;
			testv->index = myCurrFault->index;
		} else {
			testv->fltLineHash = -1;
			testv->fltHash = -1;
			testv->type = -1;
			testv->index = -1;
		}
		
		testv->no = no;
		tv.vectors.push_front(testv);
//		testv->next = tv.vcts;
//		tv.vcts = testv;
		tv.num++;
	}
	
	string* GateNet::printInputs(int nth_bit)
	{
		string *s=new string;
		s->resize(numberOfPrimaryInputs, '0');
		
		for(int j=0;j<numberOfPrimaryInputs;j++)
			if(checkBit(net[j]->output1,nth_bit)) 
				(*s)[j] = '1'; 
			
			return s;
	}
	
	string* GateNet::printOutputs(int nth_bit)
	{
		string * s=new string;
		s->resize(numberOfPrimaryOutputs, '0');
		
		for(int j=0;j<numberOfPrimaryOutputs;j++)
			if(checkBit(net[primaryOut[j]]->output1,nth_bit)) 
				(*s)[j] = '1';
			return s;
	}
	
	void GateNet::printIO(int nth_bit, int start)
	{
		string *iv,*ov;
		
		iv = printInputs(nth_bit );
		ov = printOutputs(nth_bit );
		addTestVector( iv, ov, 1 );
		
		delete iv;
		delete ov;
	}
	
	void GateNet::setTestAbility()
	{
		int i,j,depth;
		
		// cont0 and cont1
		for(i=0;i<numberOfGates;i++) 
		{
			if(net[i]->isFree() || net[i]->isHead()) net[i]->cont0=0;
			else
			{
				if(i==200)
				{
					i=i;
				}
				
				
				depth=-1;
				for(j=0;j<net[i]->ninput;j++)
					depth=MAX(depth,net[i]->inlis[j]->cont0);
				net[i]->cont0=depth+1;
			}
			net[i]->cont1=net[i]->cont0;
		}
		
		/* depth from output */
		for(i=numberOfGates-1;i>=0;i--)
		{
			if(net[i]->fn==PO)
				net[i]->dpo=0;
			else
			{
				depth=-1;
				for(j=0;j<net[i]->noutput;j++)
					depth=MAX(depth,net[i]->outlis[j]->dpo);
				net[i]->dpo=depth+1;
			}
		}
	}
	
	void GateNet::myDPrintIO(int no)
	{
		int i;
		
		string iv;
		string ov;
		
		iv.resize(numberOfPrimaryInputs);
		ov.resize(numberOfPrimaryOutputs);
		
		
		
		for(i=0; i<numberOfPrimaryInputs; i++)
			iv[i] = dLevelToString[net[primaryIn[i]]->output][0];
		iv[i] = 0;
		for(i=0; i<numberOfPrimaryOutputs; i++)
			ov[i] = dLevelToString[net[primaryOut[i]]->output][0];
		ov[i] = 0;
		addTestVector( &iv, &ov, no );
		
	}
	
	void GateNet::allocateDynamicBuffers()
	{
		freeGates=new Stack(numberOfGates);
		faultyGates=new Stack(numberOfGates);
		evalGates=new Stack(numberOfGates);
		activeStems=new Stack(numberOfGates);
		dynamicStack=new Stack(numberOfGates);
	}
	
}
