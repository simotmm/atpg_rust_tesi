#ifndef	__ATALANTA_RANDOM_H__
#define	__ATALANTA_RANDOM_H__
#include <string>
using namespace std;

namespace atalantadll {
	const unsigned int all1=~0;
	
	
	class Random 
	{
		static unsigned int iseed;
		Random();
		
	public:
		
		SHARE_EXPORT static int	seed(int startvalue);
		static void getRandompattern(int number,level *array,int nbit);
		static void getPRandompattern(int number,level *array);
		
		static int getRandomOctVector(int nbits, string *res);
	};
}
#endif //__ATALANTA_RANDOM_H__
