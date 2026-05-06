// ResultsReader.cpp: implementation of the ResultsReader class.
//
//////////////////////////////////////////////////////////////////////
#include <string>
#include <list>
#include <fstream>

#include "Defines.h"
#include "Truthtable.h"
#include "Parameters.h"

#include "Hash.h"
#include "Fault.h"

#include "Gate.h"

#include "ResultsReader.h"

namespace atalantadll {
	//////////////////////////////////////////////////////////////////////
	// Construction/Destruction
	//////////////////////////////////////////////////////////////////////

	ResultsReader::ResultsReader()
	{

	}

	int ResultsReader::pgetTest(std::fstream *fp,level *input,int npi,int nbit)
	{
		int i;
		int c;
		int ntest;
		unsigned mask1=~(ALL1<<1);
		bool valid=false;

		i=0;
		for(ntest=0;ntest<nbit;ntest++)
		{
			while(!fp->eof())
			{
				*fp>>c;
				switch(c)
				{
				case '*': fp->ignore(-1,'\n'); break;
				case ':': valid=true; break;
				case '0': if(valid) input[i++]<<=1; break;
				case '1': if(valid) {input[i]<<=1; input[i++]|=mask1;} break;
				}
				if(i==npi) {i=0; valid=false; break;}
			}
			if(fp->eof()) break;
		}

		if(i>0 && i<npi) return 0;
		return ntest;
	}

	ResultsReader::~ResultsReader()
	{

	}
}
