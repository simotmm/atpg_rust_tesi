// GateNet.h: interface for the GateNet class.
//
//////////////////////////////////////////////////////////////////////

#ifndef __ATALANTA_GATENET_H__
#define __ATALANTA_GATENET_H__

#include "Stack.h"
#include "Hash.h"
#include "Parameters.h"
#include "Globals.h"

namespace atalantadll {
	class GateNet
	{
#ifdef _ALG_DEBUG
	public:
		fstream dbgFile;
#endif
	protected:
		unsigned int numberOfGates;
		unsigned int numberOfFlipFlops;
		unsigned int numberOfPrimaryInputs;
		unsigned int numberOfPrimaryOutputs;
		struct TestVectors tv;

		int PPOlevel;
		int POlevel;
		int lastGate;

		vector<int> primaryIn;
		vector<int> headlines;
		vector<int> primaryOut;
		int *flipFlops;

		vector<Gate *> net; //Gate **net;

		Stack *freeGates, *faultyGates,*evalGates,*activeStems;
		Stack *dynamicStack;
		Stack *stack;
		Stack *stack1,*stack2;

		int nsStack,ndStack;

		Hash	hashTable;

		char initialMode;

		Fault *myCurrFault;

		bool checkBit(level word,int n) {return (word & (1<<n)) != 0;};
		void addTestVector(string *ivct,string *ovct,int no);
		level logiclevel(level V0,level V1,int n);

		string *printInputs(int nth_bit );
		string *printOutputs(int nth_bit);

	public:
		GateNet():hashTable(HASHSIZE),stack(0)
		{
			numberOfGates=numberOfFlipFlops=numberOfPrimaryInputs=numberOfPrimaryOutputs=0;
			PPOlevel=POlevel=lastGate=0;
			flipFlops=NULL;
			primaryOut.clear();//primaryOut=0;
			headlines.clear();  //headlines=0;

			primaryIn.clear(); //primaryIn = 0;
			net.clear(); //net=0;

			faultyGates=evalGates=activeStems=0;
			dynamicStack=stack=stack1=stack2=0;

			nsStack=ndStack=0;

			initialMode='x';
		}

		void setTestAbility();
		void myDPrintIO(int no);

		void allocateDynamicBuffers();
		void allocateStacks();

		void checkParameters();

#ifdef INCLUDE_HOPE
		void printIOValues( vector<int> iarray, vector<int> oarray);
#endif
		void printIO(int nth_bit,int start);

	};
}
#endif // __ATALANTA_GATENET_H__
