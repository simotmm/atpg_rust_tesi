// Simulation.h: interface for the Simulation class.
//
//////////////////////////////////////////////////////////////////////

#ifndef __ATALANTA_SIMULATION_H__
#define __ATALANTA_SIMULATION_H__

#include "ReadableNet.h"
//#define TestVectorsData vector< vector<level> >


namespace atalantadll {
	
	class TestVectorsData {
		int secondSize;

	public:
		vector<level *> testData;

		TestVectorsData(int n_secondSize) {
			secondSize = n_secondSize;
		};

		level * operator [] (int i) { 
			int j, k, l;
			if(i >= testData.size()) {
				j = testData.size();
				testData.resize(i + 1);
				for(k = j; k <= i; k++) {
					testData[k] = new level[secondSize]; 
					for(l = 0; l < secondSize; l++) {
						testData[k][l] = 0;
					}
				}
			}
			return testData[i];
		};

		void clear(void) {
			testData.clear();
		};

		void setSecondSize(int i) {
			secondSize = i;
		};

		unsigned int size() { return testData.size(); };
	};

	class SHARE_EXPORT Simulation:public ReadableNet
	{
	protected:
		char compact;
		int maxCompact;
		char fillMode;
		TestVectorsData testVectors;//level testVectors[MAXTEST/10][MAXPI+1];
		TestVectorsData testVectors1;//level testVectors1[MAXTEST/10][MAXPI+1];
		TestVectorsData testStore;//level testStore[MAXTEST/10][MAXPI+1];
		TestVectorsData testStore1;//level testStore1[MAXTEST/10][MAXPI+1];

		void setBit(int *word,int nth);
		void resetBit(int *word,int nth);
		void setb0(int *word0, int *word1, int nth) ;
		void setb1(int *word0, int *word1, int nth) ; 
		void setbx(int *word0, int *word1, int nth) ;

		int		randomFsim(int levels,int nStem,Gate **stem,level *lfsr,int limit,int maxBit,int maxDetect,int *nTest,int *nPacket,int *nBit);
		int		randomHope(level *lfsr,int limit,int maxBit,int maxDetect,int *nTest,int *nPacket,int *nBit);
		void	randomTestFsim(TestVectorsData *testSt, TestVectorsData *testVect,int pack,int noBit);
		void	randomTestHope(TestVectorsData *testSt0, TestVectorsData *testSt1, TestVectorsData *testVect0, TestVectorsData *testVect1, int pack, int noBit);
		int		reverseFsim(int levels,int nStem,Gate **stem,int *nDet,int nPacket,int nBit,int maxBits);
		int		reverseHope(int *nDet,int nPacket,int nBit,int maxBits);
		int		shuffleFsim(int levels,int nStem,Gate **stem,int *nShuf,int *nDet,int nPacket,int nBit,int maxBits);
		int		shuffleHope(int *nShuf,int *nDet,int nPacket,int nBit,int maxBits);
		void	fillPatternsFsim(char mode,int nPacket,int nBit);
		void	fillPatternsHope(char mode,int nPacket,int nBit);

	public:
		Simulation(): testVectors(0), testVectors1(0), testStore(0), testStore1(0) {
			compact = 's';
			maxCompact = 2;
			fillMode = 'r';
		};

		int compactTest(int levels,int nStem,Gate **stem,int *nShuf,int *nDet,int nPacket,int nBit,int maxBits);
		void fillPatterns(int mode,int nPacket,int nBit);
		int randomSim(int levels,int nStem,Gate **stem,level *lfsr,int limit,int maxBit,int maxDetect,int *nTest,int *nPacket,int *nBit);
		int simulateHope(int *nPacket,int *nBit);
		int tGenSim(int levels,int nStem,Gate **stem,int nTest,int *profile);
		int testGen(int levels,int maxBits,int nStem,Gate **stem,int maxBackTrack,int phase,int *nRedundant,int *nOverBackTrack,int *nBackTrack,int *nTest,int *nPacket,int *nBit,double *fanTime);

		virtual ~Simulation(){};
	};

}
#endif // __ATALANTA_SIMULATION_H__
