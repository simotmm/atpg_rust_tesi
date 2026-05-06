// Fan.h: interface for the Fan class.
//
//////////////////////////////////////////////////////////////////////

#ifndef __ATALANTA_FAN_H__
#define __ATALANTA_FAN_H__


#include "Parameters.h"

namespace atalantadll {
	class Fan  
	{
		void initNet(int nog, GATEPTR faulty_gate, int maxdpi);
		int setFaultyGate(FAULTPTR fault);
		level faulty_gate_eval(register GATEPTR g, FAULTPTR cf);
		status eval(register GATEPTR gate, FAULTPTR cf);
		bool imply(int maxdpi, boolean backward, int last, FAULTPTR cf);
		int unique_sensitize(register GATEPTR gate, GATEPTR faulty_gate);
		int dynamic_unique_sensitize(GATEPTR *Dfront, int nod, int maxdpi, GATEPTR *dom_array, GATEPTR faulty_gate);
		GATEPTR closest_po(struct STACK objective, int *pclose);
		status backtrace(status state);
		void find_final_objective(boolean *backtrace_flag, boolean fault_propagated_to_po, int nog, GATEPTR *last_Dfrontier);
		bool Xpath(register GATEPTR gate);
		void update_Dfrontier(void);
		void restore_faults(FAULTPTR fal);
		void justify_free_lines(int npi, FAULTPTR of, FAULTPTR cf);
		bool backtrack(GATEPTR faulty_gate, int *last, int nog);
		void faulty_line_is_free(FAULTPTR cf);

	public:
		Fan();

		status fan(int nog, int maxdpi, int npi, int npo, FAULTPTR cf, int maxbacktrack, int *nbacktrack);
		status fan1(int nog, int maxdpi, int npi, int npo, FAULTPTR cf, int maxbacktrack, int *nbacktrack);

		virtual ~Fan();

	};
}
#endif // __ATALANTA_FAN_H__
