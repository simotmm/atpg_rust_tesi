#ifndef	__ATALANTA_ERROR_H__
#define	__ATALANTA_ERROR_H__

namespace atalantadll {
	class Error
	{
	private:
		Error();

	public:
		static void fatalerror(int errorcode);
	};
}
#endif //__ATALANTA_ERROR_H__

