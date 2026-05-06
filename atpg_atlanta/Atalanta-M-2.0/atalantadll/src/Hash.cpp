#include <string>
#include <list>

#include "Parameters.h"
#include "Defines.h"
#include "Error.h"

#include "Hash.h"
#include "Gate.h"
#include "Stack.h"

#include "Globals.h"

using namespace std;

namespace atalantadll {
	int Hash::keyValue(string *s)
	{
		char c;
		int j, val=0;
		int multi=1;


		for(int i=0;i<s->length();)
		{
			c=s->at(i);
			if(c>='0' && c<='9') j=c-'0'+1;
			else if(c>='a' && c<='z') j=c-'a'+11;
			else if(c>='A' && c<='Z') j=c-'A'+11;
			else if(c=='_') j='Z'-'A'+12;
			else if(c=='#') j='Z'-'A'+13;
			else if(c=='@') j='Z'-'A'+14;
			else if(c=='$') j='Z'-'A'+15;
			else if(c=='/') j='Z'-'A'+16;
			else if(c=='<' || c=='>') j='Z'-'A'+17;
			else if(c=='[' || c==']') j='Z'-'A'+18;
			else j='Z'-'A'+18;

			val+=j*multi;
			i++;
			if(i%EDIGIT == 0) multi=1; else multi*=BASIS;
		};	

		return(val);
	}

	/*Hash::Hash(int size):size(size)
	{
		data=new list<HashData*>[size];
	};*/

	HashData* Hash::findHash(string *symbol,int key)
	{
		list<HashData*>::iterator iter,finaliter;
		int h;

		if(key==0) key=keyValue(symbol);

		h=key%size;

		iter=data[h].begin();
		finaliter=data[h].end();
		while(iter!=finaliter)
		{
			if((*iter)->key==key && *symbol==(*iter)->symbol) return (*iter);
			iter++;
		}
		return 0;
	}

	HashData* Hash::insertHash(string *symbol,int key)
	{
		HashData *hp;
		int h;

		if(key==0) key=keyValue(symbol);

		h=key%size;

		hp=new HashData(key,symbol);
		data[h].push_front(hp);
		return hp;
	}

	HashData* Hash::findAndInsertHash(string symbol,int key)
	{
		HashData *hashPointer;

		if(key==0) key=keyValue(&symbol);

		hashPointer=findHash(&symbol,key);
		if(!hashPointer) hashPointer=insertHash(&symbol,key);
		return hashPointer;
	}
}
