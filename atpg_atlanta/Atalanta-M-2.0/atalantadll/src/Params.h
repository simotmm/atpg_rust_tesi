#ifndef __ATALANTA_PARAMS_H__
#define __ATALANTA_PARAMS_H__

#include <string>
#include <sstream>
#include "Defines.h"
using namespace std;


namespace atalantadll {
	class SHARE_EXPORT Params
	{
	  protected :
			char cctMode;
			int randomLimit;
			unsigned int iseed;
			int maxCompact;
			char compact;
			int maxBackTrack;
			int maxBackTrack1;
			string sPatternFile;
			streambuf *sPatternStream;
			string benchFile;
			streambuf *benchStream;
			char learnMode;
			char faultMode;
			string faultFile;
			streambuf *faultStream;
			char simMode;
			char fillMode;
			char genAllPat;
			int eachLimit;
			char noFaultSim;
			string udFaultsFile;
			streambuf *udFaultsStream;
			int uFaultMode;
			string wFaultFile;
			streambuf *wFaultStream;
			int simulationMode;
			string maskFile;
			streambuf *maskStream;
			string reportFile;
			streambuf *reportStream;
			int wTestMode;
			int lfsrSimMode;
			string lfsrPoly;
			string lfsrSeed;
			int lfsrNum;
	public:
		Params(void) {
			cctMode = ISCAS89;
			randomLimit = 16;
			iseed = 0;
			maxCompact = 2;
			compact = 's';
			maxBackTrack=10;
			maxBackTrack1=0;
			sPatternFile = "";
			sPatternStream = NULL;
			benchFile = "";
			benchStream = NULL;
			learnMode = 'n';
			faultMode='d';
			faultFile = "";
			faultStream = NULL;
			simMode = 'f';
			fillMode = 'r';
			genAllPat = 'n';
			eachLimit = 0;
			noFaultSim = 'n';
			udFaultsFile = "";
			udFaultsStream = NULL;
			uFaultMode=0;
			wFaultFile = "";
			wFaultStream = NULL;
			simulationMode = 0;
			maskFile = "";
			maskStream = NULL;
			reportFile = "";
			reportStream = NULL;
			wTestMode = 0;
			lfsrSimMode = 0;
			lfsrPoly = "";
			lfsrSeed = "";
			lfsrNum = 1000;
		};
		//cctMode
		char getCctMode(void) {return cctMode;};
		void setCctMode(char p_cctMode) {
			if(p_cctMode == ISCAS89 || p_cctMode == ISCAS85) {
				cctMode = p_cctMode;
			} else {
				stringstream ss;
				ss << "Wrong value. Value must be " << ISCAS85 << " or " << ISCAS89 << ".";
				throw ss.str();
			}
		};
    //randomLimit
		int getRandomLimit (void) { return randomLimit; };
		void setRandomLimit(char p_randomLimit) {
			if(p_randomLimit >= 0 && p_randomLimit <= 32) {
				randomLimit = p_randomLimit;
			} else {
				stringstream ss;
				ss << "Wrong value. Value must be in range 0 to 32.";
				throw ss.str();
			}
		};
		//iseed
		unsigned int getIseed(void) { return iseed; };
		void setIseed(unsigned int p_iseed) { iseed = p_iseed; };
		//maxCompact
		int getMaxCompact(void) { return maxCompact; };
		void setMaxCompact(int p_maxCompact) { maxCompact = p_maxCompact; };
		//compact
		char getCompact(void) { return compact; };
		void setCompact(char p_compact) {
			if(p_compact == 's' || p_compact == 'n') {
				compact = p_compact;
				if(p_compact == 'n') maxCompact = 0;
			} else {
				stringstream ss;
				ss << "Wrong value. Value must be 's' or 'n'.";
				throw ss.str();
			}
		};
		//maxBackTrack
		int getMaxBackTrack(void) { return maxBackTrack; };
		void setMaxBackTrack(int p_maxBackTrack) {
			if(p_maxBackTrack >= 1) {
				maxBackTrack = p_maxBackTrack;
			} else {
				stringstream ss;
				ss << "Wrong value. Value must be greater than 0.";
				throw ss.str();
			}
		};
		//maxBackTrack1
		int getMaxBackTrack1(void) { return maxBackTrack1; };
		void setMaxBackTrack1(int p_maxBackTrack1) {
			if(p_maxBackTrack1 >= 0) {
				maxBackTrack1 = p_maxBackTrack1;
			} else {
				stringstream ss;
				ss << "Wrong value. Value must be greater or equal to 0.";
				throw ss.str();
			}
		};
		//sPatternFile
		string getSPatternFile(void) { return sPatternFile; };
		void setSPatternFile(string p_sPatternFile) { sPatternFile = p_sPatternFile; sPatternStream = NULL; };
		//sPatternStream
		streambuf *getSPatternStream(void) { return sPatternStream; };
		void setSPatternStream(streambuf *p_sPatternStream) { sPatternStream = p_sPatternStream; sPatternFile = "";};
		//benchFile
		string getBenchFile(void) { return benchFile; };
		void setBenchFile(string p_benchFile) { benchFile = p_benchFile; benchStream = NULL; };
		//benchStream
		streambuf *getBenchStream(void) { return benchStream; };
		void setBenchStream(streambuf *p_benchStream) { benchStream = p_benchStream; benchFile = "";};
		//learnMode
		char getLearnMode(void) { return learnMode; };
		void setLearnMode(char p_learnMode) {
			if(p_learnMode == 'y' || p_learnMode == 'n') {
				learnMode = p_learnMode;
			} else {
				stringstream ss;
				ss << "Wrong value. Value has to be 'y' or 'n'.";
				throw ss.str();
			}
		};
		//faultMode
		char getFaultMode(void) { return faultMode; };
		void setFaultMode(char p_faultMode) {
			if(p_faultMode == 'f' || p_faultMode == 'd') {
				faultMode = p_faultMode;
				if(faultMode == 'd') faultFile = "";
			} else {
				stringstream ss;
				ss << "Wrong value. Value must be 'f' or 'd'.";
				throw ss.str();
			}
		};
		//faultFile
		string getFaultFile(void) { return faultFile; };
		void setFaultFile(string p_faultFile) { faultFile = p_faultFile; faultMode = 'f'; faultStream = NULL;};
		//faultStream
		streambuf *getFaultStream(void) { return faultStream; };
		void setfaultStream(streambuf *p_faultStream) { faultStream = p_faultStream; faultMode = 'f'; faultFile = "";};
		//wFaultFile
		string getWFaultFile(void) { return wFaultFile; };
		void setWFaultFile(string p_wFaultFile) { wFaultFile = p_wFaultFile; wFaultStream = NULL; };
		//wFaultStream
		streambuf *getWFaultFileStream(void) { return wFaultStream; };
		void setWFaultStream(streambuf *p_wFaultStream) { wFaultStream = p_wFaultStream; wFaultFile = "";};
		//simMode
		char getSimMode(void) { return simMode; };
		void setSimMode(char p_simMode) {
			if(p_simMode == 'f' || p_simMode == 'h') {
				simMode = p_simMode;
			} else {
				stringstream ss;
				ss << "Wrong value. Value must be 'f' or 'h'.";
				throw ss.str();
			}
		};
		//fillMode
		char getFillMode(void) { return fillMode; };
		void setFillMode(char p_fillMode) {
			if(p_fillMode == 'r' || p_fillMode == 'x' ||
				p_fillMode == '0' || p_fillMode == '1') {
				fillMode = p_fillMode;
			} else {
				stringstream ss;
				ss << "Wrong value. Value must be '0' or '1' or 'r' or 'x'.";
				throw ss.str();
			}
		};
		//genAllPat
		char getGenAllPat(void) { return genAllPat; };
		void setGenAllPat(char p_genAllPat) {
			if(p_genAllPat == 'y' || p_genAllPat == 'n') {
				genAllPat = p_genAllPat;
			} else {
				stringstream ss;
				ss << "Wrong value. Value must be 'y' or 'n'.";
				throw ss.str();
			}
		};
		//eachLimit
		int getEachLimit(void) { return eachLimit; };
		void setEachLimit(int p_eachLimit) { eachLimit = p_eachLimit;};
		//noFaultSim
		char getNoFaultSim(void) { return noFaultSim; };
		void setNoFaultSim(char p_noFaultSim) {
			if(p_noFaultSim == 'y' || p_noFaultSim == 'n') {
				noFaultSim = p_noFaultSim;
			} else {
				stringstream ss;
				ss << "Wrong value. Values must be 'y' or 'n'.";
				throw ss.str();
			}
		};
		//uFaultMode
		int getUFaultMode(void) { return uFaultMode; };
		void setUFaultMode(int p_uFaultMode) {
			if(p_uFaultMode >= 0 && p_uFaultMode <= 2) {
				uFaultMode = p_uFaultMode;
			} else {
				stringstream ss;
				ss << "Wrong value. Value must be 0, 1 or 2.";
				throw ss.str();
			}
		};
		//udFaultsFile
		string getUdFaultsFile(void) { return udFaultsFile; };
		void setUdFaultsFile(string p_udFaultsFile) { udFaultsFile = p_udFaultsFile; udFaultsStream = NULL;	};
		//udFaultsStream
		streambuf *getUdFaultsStream(void) { return udFaultsStream; };
		void setudFaultsStream(streambuf *p_udFaultsStream) { udFaultsStream = p_udFaultsStream; udFaultsFile = "";};
		//simulationMode
		int getSimulationMode(void) { return simulationMode; };
		void setSimulationMode(int p_simulationMode) {
			if(p_simulationMode == 0 || p_simulationMode == 1) {
				simulationMode = p_simulationMode;
				if(simulationMode) {
				  simMode = 'h';
				}
			} else {
				stringstream ss;
				ss << "Wrong value. Values must be 0 or 1.";
				throw ss.str();
			}
		};
		//maskFile
		string getMaskFile(void) { return maskFile; };
		void setMaskFile(string p_maskFile) { maskFile = p_maskFile; maskStream = NULL; };
		//maskStream
		streambuf *getMaskStream(void) { return maskStream; };
		void setMaskStream(streambuf *p_maskStream) { maskStream = p_maskStream; maskFile = "";};
		//reportFile
		string getReportFile(void) { return reportFile; };
		void setReportFile(string p_reportFile) { reportFile = p_reportFile; reportStream = NULL; };
		//reportStream
		streambuf *getReportStream(void) { return reportStream; };
		void setReportStream(streambuf *p_reportStream) { reportStream = p_reportStream; reportFile = "";};
		//wTestMode
		int getWTestMode(void) { return wTestMode; };
		void setWTestMode(int p_wTestMode) {
			if(p_wTestMode >= 0 && p_wTestMode <= 4) {
				wTestMode = p_wTestMode;
			} else {
				stringstream ss;
				ss << "Wrong value. Value must be in range 0 to 4.";
				throw ss.str();
			}
		};
		//lfsrSimMode
		int getLfsrSimMode(void) { return lfsrSimMode; };
		void setLfsrSimMode(int p_lfsrSimMode) {
			if(p_lfsrSimMode >= 0 && p_lfsrSimMode <= 2) {
				lfsrSimMode = p_lfsrSimMode;
			} else {
				stringstream ss;
				ss << "Wrong value. Value must be 0, 1 or 2.";
				throw ss.str();
			}
		};
		//lfsrSimulation
		string getLfsrPoly(void) { return lfsrPoly; };
		string getLfsrSeed(void) { return lfsrSeed; };
		int getLfsrNum(void) { return lfsrNum; };
		void setLfsrSimulation(string p_lfsrPoly, string p_lfsrSeed, int p_lfsrNum) {
			lfsrPoly = p_lfsrPoly;
			lfsrSeed = p_lfsrSeed;
			lfsrNum = p_lfsrNum;
		};

		Params operator=(Params *p) {
			cctMode = p->getCctMode();
			randomLimit = p->getRandomLimit();
			iseed = p->getIseed();
			maxCompact = p->getMaxCompact();
			compact = p->getCompact();
			maxBackTrack = p->getMaxBackTrack();
			maxBackTrack1 = p->getMaxBackTrack1();
			sPatternFile = p->getSPatternFile();
			sPatternStream = p->getSPatternStream();
			benchFile = p->getBenchFile();
			benchStream = p->getBenchStream();
			learnMode = p->getLearnMode();
			faultMode = p->getFaultMode();
			faultFile = p->getFaultFile();
			faultStream = p->getFaultStream();
			wFaultFile = p->getWFaultFile();
			wFaultStream = p->getWFaultFileStream();
			simMode = p->getSimMode();
			fillMode = p->getFillMode();
			genAllPat = p->getGenAllPat();
			eachLimit = p->getEachLimit();
			noFaultSim = p->getNoFaultSim();
			uFaultMode = p->getUFaultMode();
			udFaultsFile = p->getUdFaultsFile();
			udFaultsStream = p->getUdFaultsStream();
			simulationMode = p->getSimulationMode();
			maskFile = p->getMaskFile();
			maskStream = p->getMaskStream();
			reportFile = p->getReportFile();
			reportStream = p->getReportStream();
			wTestMode = p->getWTestMode();
			lfsrSimMode = p->getLfsrSimMode();
			lfsrPoly = p->getLfsrPoly();
			lfsrSeed = p->getLfsrSeed();
			lfsrNum = p->getLfsrNum();
			return *this;
		};
	};
}
#endif //__ATALANTA_PARAMS_H__
