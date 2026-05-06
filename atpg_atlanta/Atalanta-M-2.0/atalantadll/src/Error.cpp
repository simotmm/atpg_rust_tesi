#include <sstream>
#include <stdlib.h>
#include <stdexcept>

#include "Parameters.h"
#include "Error.h"

using namespace std;

namespace atalantadll {
	static const unsigned char messages[NUMERRORS+1][75]=
	{
		"Good status",
		"Unexpected end-of-file on circuit file",
		"Error in circuit file",
		"Error in dynamic memory allocation",
		"Error in symbol table",
		"Error in fault file",
	};

	void Error::fatalerror(int errorcode)
	{
		/*cerr<<"Fatal error:  ";
		cerr<<messages[errorcode];
		cerr<<endl;

		//exit(1);*/
		stringstream ss;
		ss << messages[errorcode] << "\0x00";
		throw ss.str();
	}
}
