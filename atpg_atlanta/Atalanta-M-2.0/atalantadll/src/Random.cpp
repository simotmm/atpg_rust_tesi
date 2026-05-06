#include <stdio.h>
#include <stdlib.h>
#include <time.h>
#include <memory>

#include "Defines.h"
#include "Random.h"

namespace atalantadll {
	unsigned int Random::iseed;

	int Random::seed(int startvalue)
	{
		long now;

		if(startvalue==0) //Use current time as seed
		{	
			now=(long)time(NULL);
			srand((unsigned int)now);
			iseed=(unsigned int)now;
			return (unsigned int)now;
		}
		else  //Use input as seed
		{
			srand(startvalue);
			iseed=startvalue;
			return startvalue;
		}
	}

	//	GetRandompattern:
	//	Generates a given # of parallel random patterns.
	//	Only the given number of bits are valid.

	void Random::getRandompattern(int number,level *array,int nbit)
	{
		int i, mask;

		if(nbit == sizeof(int)*8)
			for(i=0; i<number; i++) array[i]=(level)rand();
		else {
			mask = ~(all1<<nbit);
			for(i=0; i<number; i++) array[i]=(level)rand() & mask;
		}
	}

	//	GetPRandompattern
	//	Generates 32 parallel random patterns.
	void Random::getPRandompattern(int number,level *array)
	{
		int i;
		for(i=0;i<number;i++) array[i]=(level)rand();
	}

	int Random::getRandomOctVector(int nbits, string *res)
{
	int b, c = 0;
	int pos = 0;
	int i;
/*	memset(res, '0', (nbits / 3) + (nbits % 3 ? 1 : 0));
	res[nbits] = 0;*/
	res->erase();
	res->append((nbits / 3) + (nbits % 3 ? 1 : 0), '0');
	for(i = 0; i < nbits; i++) {
		if(i % 9 == 0) {
			b = rand() % 512;
		}
		if(b & 1) {
			c |= 1;
			pos = i;
		}
		if(i % 3 == 2) {
			(*res)[(nbits - i) / 3] = '0' + c;
			c = 0;
		}
		else {
			if(i != nbits - 1) {
  		  c <<= 1;
			}
		}
		b >>= 1;
	}
	if(c) {
		(*res)[((nbits - i) / 3)] = '0' + c;
	}
	//res[(nbits / 3) + (nbits % 3 ? 1 : 0)] = 0;
	
	return pos;
}

	
}
