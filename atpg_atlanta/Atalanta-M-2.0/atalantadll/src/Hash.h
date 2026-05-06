#ifndef	__ATALANTA_HASH_H__
#define	__ATALANTA_HASH_H__

#include "Defines.h"
#include "Parameters.h"
#include <string>

using namespace std;

namespace atalantadll {
	const char EDIGIT=4;
	const char BASIS=('Z'-'A'+19);

	class Gate;

	class HashData
	{
	public:
		int key;
		string symbol;
		Gate *pnode;

		HashData(int key,string *symbol):key(key) {this->symbol=*symbol;pnode=0;}
	};

	class Hash
	{
		int size;

		list<HashData*> *data;

		int keyValue(string *s);

	public:
		Hash(void) {
			size = HASHSIZE;
			data=new list<HashData *>[HASHSIZE];
		}
		Hash(int size) {
			this->size = size;
			data=new list<HashData*>[size];
		};		//Creates new hash table of the given size
		SHARE_EXPORT HashData *findHash(string *symbol,int key);	//Find a symbol in a hash table
		HashData *insertHash(string *symbol,int key); //Adds a symbol to hash table
		HashData *findAndInsertHash(string symbol,int key); //Searches and inserts a symbol to a hash table
		void clear(void) { for(int i = 0; i < size; i++) { data[i].clear(); }; }
	};

}
#endif //__ATALANTA_HASH_H__
