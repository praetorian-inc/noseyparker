
#line 1 "src/parser/control_verbs.rl"
/*
 * Copyright (c) 2017, Intel Corporation
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions are met:
 *
 *  * Redistributions of source code must retain the above copyright notice,
 *    this list of conditions and the following disclaimer.
 *  * Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *  * Neither the name of Intel Corporation nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
 * AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE
 * LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
 * CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
 * SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
 * CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 */

/**
 * \file
 * \brief Parser for control verbs that can occur at the beginning of a pattern.
 */

#include "parser/control_verbs.h"

#include "parser/Parser.h"
#include "parser/parse_error.h"

#include <cstring>
#include <sstream>

using namespace std;

namespace ue2 {

const char *read_control_verbs(const char *ptr, const char *end, size_t start,
                               ParseMode &mode) {
    const char *p = ptr;
    const char *pe = end;
    const char *eof = pe;
    const char *ts, *te;
    int cs;
    UNUSED int act;

    
#line 59 "src/parser/control_verbs.cpp"
static const char _ControlVerbs_actions[] = {
	0, 1, 0, 1, 1, 1, 2, 1, 
	3, 1, 4, 1, 5, 1, 6, 1, 
	7, 1, 8, 1, 9
};

static const char _ControlVerbs_to_state_actions[] = {
	0, 0, 0, 0, 0, 0, 0, 0, 
	0, 0, 0, 0, 0, 0, 0, 0, 
	0, 0, 0, 0, 0, 0, 0, 0, 
	0, 0, 0, 0, 0, 0, 0, 0, 
	0, 0, 0, 0, 0, 0, 0, 0, 
	0, 0, 0, 0, 0, 0, 0, 0, 
	0, 0, 0, 0, 0, 0, 0, 0, 
	0, 0, 0, 0, 0, 0, 0, 0, 
	0, 0, 0, 0, 0, 0, 0, 0, 
	0, 0, 0, 1, 0
};

static const char _ControlVerbs_from_state_actions[] = {
	0, 0, 0, 0, 0, 0, 0, 0, 
	0, 0, 0, 0, 0, 0, 0, 0, 
	0, 0, 0, 0, 0, 0, 0, 0, 
	0, 0, 0, 0, 0, 0, 0, 0, 
	0, 0, 0, 0, 0, 0, 0, 0, 
	0, 0, 0, 0, 0, 0, 0, 0, 
	0, 0, 0, 0, 0, 0, 0, 0, 
	0, 0, 0, 0, 0, 0, 0, 0, 
	0, 0, 0, 0, 0, 0, 0, 0, 
	0, 0, 0, 3, 0
};

static const int ControlVerbs_start = 75;
static const int ControlVerbs_first_final = 75;
static const int ControlVerbs_error = -1;

static const int ControlVerbs_en_main = 75;


#line 99 "src/parser/control_verbs.cpp"
	{
	cs = ControlVerbs_start;
	ts = 0;
	te = 0;
	act = 0;
	}

#line 106 "src/parser/control_verbs.rl"


    try {
        
#line 112 "src/parser/control_verbs.cpp"
	{
	const char *_acts;
	unsigned int _nacts;

	if ( p == pe )
		goto _test_eof;
_resume:
	_acts = _ControlVerbs_actions + _ControlVerbs_from_state_actions[cs];
	_nacts = (unsigned int) *_acts++;
	while ( _nacts-- > 0 ) {
		switch ( *_acts++ ) {
	case 1:
#line 1 "NONE"
	{ts = p;}
	break;
#line 128 "src/parser/control_verbs.cpp"
		}
	}

	switch ( cs ) {
case 75:
	if ( (*p) == 40u )
		goto tr80;
	goto tr79;
case 76:
	if ( (*p) == 42u )
		goto tr82;
	goto tr81;
case 0:
	switch( (*p) ) {
		case 41u: goto tr0;
		case 65u: goto tr2;
		case 66u: goto tr3;
		case 67u: goto tr4;
		case 76u: goto tr5;
		case 78u: goto tr6;
		case 85u: goto tr7;
	}
	goto tr1;
case 1:
	if ( (*p) == 41u )
		goto tr8;
	goto tr1;
case 2:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 78u: goto tr9;
	}
	goto tr1;
case 3:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 89u: goto tr10;
	}
	goto tr1;
case 4:
	switch( (*p) ) {
		case 41u: goto tr11;
		case 67u: goto tr12;
	}
	goto tr1;
case 5:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 82u: goto tr13;
	}
	goto tr1;
case 6:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 76u: goto tr14;
	}
	goto tr1;
case 7:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 70u: goto tr15;
	}
	goto tr1;
case 8:
	if ( (*p) == 41u )
		goto tr11;
	goto tr1;
case 9:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 83u: goto tr16;
	}
	goto tr1;
case 10:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 82u: goto tr17;
	}
	goto tr1;
case 11:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 95u: goto tr18;
	}
	goto tr1;
case 12:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 65u: goto tr19;
		case 85u: goto tr20;
	}
	goto tr1;
case 13:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 78u: goto tr21;
	}
	goto tr1;
case 14:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 89u: goto tr22;
	}
	goto tr1;
case 15:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 67u: goto tr12;
	}
	goto tr1;
case 16:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 78u: goto tr23;
	}
	goto tr1;
case 17:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 73u: goto tr24;
	}
	goto tr1;
case 18:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 67u: goto tr25;
	}
	goto tr1;
case 19:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 79u: goto tr26;
	}
	goto tr1;
case 20:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 68u: goto tr27;
	}
	goto tr1;
case 21:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 69u: goto tr15;
	}
	goto tr1;
case 22:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 82u: goto tr28;
	}
	goto tr1;
case 23:
	switch( (*p) ) {
		case 41u: goto tr11;
		case 76u: goto tr14;
	}
	goto tr1;
case 24:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 70u: goto tr15;
		case 73u: goto tr29;
	}
	goto tr1;
case 25:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 77u: goto tr30;
	}
	goto tr1;
case 26:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 73u: goto tr31;
	}
	goto tr1;
case 27:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 84u: goto tr32;
	}
	goto tr1;
case 28:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 95u: goto tr33;
	}
	goto tr1;
case 29:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 77u: goto tr34;
		case 82u: goto tr35;
	}
	goto tr1;
case 30:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 65u: goto tr36;
	}
	goto tr1;
case 31:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 84u: goto tr37;
	}
	goto tr1;
case 32:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 67u: goto tr38;
	}
	goto tr1;
case 33:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 72u: goto tr39;
	}
	goto tr1;
case 34:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 61u: goto tr40;
	}
	goto tr1;
case 35:
	if ( (*p) == 41u )
		goto tr8;
	if ( 48u <= (*p) && (*p) <= 57u )
		goto tr41;
	goto tr1;
case 36:
	if ( (*p) == 41u )
		goto tr11;
	if ( 48u <= (*p) && (*p) <= 57u )
		goto tr41;
	goto tr1;
case 37:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 69u: goto tr42;
	}
	goto tr1;
case 38:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 67u: goto tr43;
	}
	goto tr1;
case 39:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 85u: goto tr44;
	}
	goto tr1;
case 40:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 82u: goto tr45;
	}
	goto tr1;
case 41:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 83u: goto tr46;
	}
	goto tr1;
case 42:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 73u: goto tr47;
	}
	goto tr1;
case 43:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 79u: goto tr48;
	}
	goto tr1;
case 44:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 78u: goto tr39;
	}
	goto tr1;
case 45:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 79u: goto tr49;
	}
	goto tr1;
case 46:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 95u: goto tr50;
	}
	goto tr1;
case 47:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 65u: goto tr51;
		case 83u: goto tr52;
	}
	goto tr1;
case 48:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 85u: goto tr53;
	}
	goto tr1;
case 49:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 84u: goto tr54;
	}
	goto tr1;
case 50:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 79u: goto tr55;
	}
	goto tr1;
case 51:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 95u: goto tr56;
	}
	goto tr1;
case 52:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 80u: goto tr57;
	}
	goto tr1;
case 53:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 79u: goto tr58;
	}
	goto tr1;
case 54:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 83u: goto tr59;
	}
	goto tr1;
case 55:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 83u: goto tr60;
	}
	goto tr1;
case 56:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 69u: goto tr61;
	}
	goto tr1;
case 57:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 83u: goto tr62;
	}
	goto tr1;
case 58:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 83u: goto tr15;
	}
	goto tr1;
case 59:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 84u: goto tr63;
	}
	goto tr1;
case 60:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 65u: goto tr64;
	}
	goto tr1;
case 61:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 82u: goto tr65;
	}
	goto tr1;
case 62:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 84u: goto tr66;
	}
	goto tr1;
case 63:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 95u: goto tr67;
	}
	goto tr1;
case 64:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 79u: goto tr68;
	}
	goto tr1;
case 65:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 80u: goto tr69;
	}
	goto tr1;
case 66:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 84u: goto tr15;
	}
	goto tr1;
case 67:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 67u: goto tr70;
		case 84u: goto tr71;
	}
	goto tr1;
case 68:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 80u: goto tr72;
	}
	goto tr1;
case 69:
	if ( (*p) == 41u )
		goto tr73;
	goto tr1;
case 70:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 70u: goto tr74;
	}
	goto tr1;
case 71:
	switch( (*p) ) {
		case 41u: goto tr75;
		case 49u: goto tr76;
		case 51u: goto tr77;
		case 56u: goto tr78;
	}
	goto tr1;
case 72:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 54u: goto tr15;
	}
	goto tr1;
case 73:
	switch( (*p) ) {
		case 41u: goto tr8;
		case 50u: goto tr15;
	}
	goto tr1;
case 74:
	if ( (*p) == 41u )
		goto tr75;
	goto tr1;
	}

	tr82: cs = 0; goto _again;
	tr1: cs = 1; goto _again;
	tr2: cs = 2; goto _again;
	tr9: cs = 3; goto _again;
	tr10: cs = 4; goto _again;
	tr12: cs = 5; goto _again;
	tr13: cs = 6; goto _again;
	tr14: cs = 7; goto _again;
	tr15: cs = 8; goto _again;
	tr3: cs = 9; goto _again;
	tr16: cs = 10; goto _again;
	tr17: cs = 11; goto _again;
	tr18: cs = 12; goto _again;
	tr19: cs = 13; goto _again;
	tr21: cs = 14; goto _again;
	tr22: cs = 15; goto _again;
	tr20: cs = 16; goto _again;
	tr23: cs = 17; goto _again;
	tr24: cs = 18; goto _again;
	tr25: cs = 19; goto _again;
	tr26: cs = 20; goto _again;
	tr27: cs = 21; goto _again;
	tr4: cs = 22; goto _again;
	tr28: cs = 23; goto _again;
	tr5: cs = 24; goto _again;
	tr29: cs = 25; goto _again;
	tr30: cs = 26; goto _again;
	tr31: cs = 27; goto _again;
	tr32: cs = 28; goto _again;
	tr33: cs = 29; goto _again;
	tr34: cs = 30; goto _again;
	tr36: cs = 31; goto _again;
	tr37: cs = 32; goto _again;
	tr38: cs = 33; goto _again;
	tr39: cs = 34; goto _again;
	tr40: cs = 35; goto _again;
	tr41: cs = 36; goto _again;
	tr35: cs = 37; goto _again;
	tr42: cs = 38; goto _again;
	tr43: cs = 39; goto _again;
	tr44: cs = 40; goto _again;
	tr45: cs = 41; goto _again;
	tr46: cs = 42; goto _again;
	tr47: cs = 43; goto _again;
	tr48: cs = 44; goto _again;
	tr6: cs = 45; goto _again;
	tr49: cs = 46; goto _again;
	tr50: cs = 47; goto _again;
	tr51: cs = 48; goto _again;
	tr53: cs = 49; goto _again;
	tr54: cs = 50; goto _again;
	tr55: cs = 51; goto _again;
	tr56: cs = 52; goto _again;
	tr57: cs = 53; goto _again;
	tr58: cs = 54; goto _again;
	tr59: cs = 55; goto _again;
	tr60: cs = 56; goto _again;
	tr61: cs = 57; goto _again;
	tr62: cs = 58; goto _again;
	tr52: cs = 59; goto _again;
	tr63: cs = 60; goto _again;
	tr64: cs = 61; goto _again;
	tr65: cs = 62; goto _again;
	tr66: cs = 63; goto _again;
	tr67: cs = 64; goto _again;
	tr68: cs = 65; goto _again;
	tr69: cs = 66; goto _again;
	tr7: cs = 67; goto _again;
	tr70: cs = 68; goto _again;
	tr72: cs = 69; goto _again;
	tr71: cs = 70; goto _again;
	tr74: cs = 71; goto _again;
	tr76: cs = 72; goto _again;
	tr77: cs = 73; goto _again;
	tr78: cs = 74; goto _again;
	tr0: cs = 75; goto f0;
	tr8: cs = 75; goto f1;
	tr11: cs = 75; goto f2;
	tr73: cs = 75; goto f3;
	tr75: cs = 75; goto f4;
	tr79: cs = 75; goto f7;
	tr81: cs = 75; goto f9;
	tr80: cs = 76; goto f8;

	f8: _acts = _ControlVerbs_actions + 5; goto execFuncs;
	f4: _acts = _ControlVerbs_actions + 7; goto execFuncs;
	f3: _acts = _ControlVerbs_actions + 9; goto execFuncs;
	f2: _acts = _ControlVerbs_actions + 11; goto execFuncs;
	f1: _acts = _ControlVerbs_actions + 13; goto execFuncs;
	f7: _acts = _ControlVerbs_actions + 15; goto execFuncs;
	f9: _acts = _ControlVerbs_actions + 17; goto execFuncs;
	f0: _acts = _ControlVerbs_actions + 19; goto execFuncs;

execFuncs:
	_nacts = *_acts++;
	while ( _nacts-- > 0 ) {
		switch ( *_acts++ ) {
	case 2:
#line 1 "NONE"
	{te = p+1;}
	break;
	case 3:
#line 77 "src/parser/control_verbs.rl"
	{te = p+1;{
                mode.utf8 = true;
            }}
	break;
	case 4:
#line 81 "src/parser/control_verbs.rl"
	{te = p+1;{
                mode.ucp = true;
            }}
	break;
	case 5:
#line 85 "src/parser/control_verbs.rl"
	{te = p+1;{
                ostringstream str;
                str << "Unsupported control verb " << string(ts, te - ts);
                throw LocatedParseError(str.str());
            }}
	break;
	case 6:
#line 91 "src/parser/control_verbs.rl"
	{te = p+1;{
                ostringstream str;
                str << "Unknown control verb " << string(ts, te - ts);
                throw LocatedParseError(str.str());
            }}
	break;
	case 7:
#line 98 "src/parser/control_verbs.rl"
	{te = p+1;{
                p--;
                {p++; goto _out; }
            }}
	break;
	case 8:
#line 98 "src/parser/control_verbs.rl"
	{te = p;p--;{
                p--;
                {p++; goto _out; }
            }}
	break;
	case 9:
#line 98 "src/parser/control_verbs.rl"
	{{p = ((te))-1;}{
                p--;
                {p++; goto _out; }
            }}
	break;
#line 747 "src/parser/control_verbs.cpp"
		}
	}
	goto _again;

_again:
	_acts = _ControlVerbs_actions + _ControlVerbs_to_state_actions[cs];
	_nacts = (unsigned int) *_acts++;
	while ( _nacts-- > 0 ) {
		switch ( *_acts++ ) {
	case 0:
#line 1 "NONE"
	{ts = 0;}
	break;
#line 761 "src/parser/control_verbs.cpp"
		}
	}

	if ( ++p != pe )
		goto _resume;
	_test_eof: {}
	if ( p == eof )
	{
	switch ( cs ) {
	case 76: goto tr81;
	case 0: goto tr0;
	case 1: goto tr0;
	case 2: goto tr0;
	case 3: goto tr0;
	case 4: goto tr0;
	case 5: goto tr0;
	case 6: goto tr0;
	case 7: goto tr0;
	case 8: goto tr0;
	case 9: goto tr0;
	case 10: goto tr0;
	case 11: goto tr0;
	case 12: goto tr0;
	case 13: goto tr0;
	case 14: goto tr0;
	case 15: goto tr0;
	case 16: goto tr0;
	case 17: goto tr0;
	case 18: goto tr0;
	case 19: goto tr0;
	case 20: goto tr0;
	case 21: goto tr0;
	case 22: goto tr0;
	case 23: goto tr0;
	case 24: goto tr0;
	case 25: goto tr0;
	case 26: goto tr0;
	case 27: goto tr0;
	case 28: goto tr0;
	case 29: goto tr0;
	case 30: goto tr0;
	case 31: goto tr0;
	case 32: goto tr0;
	case 33: goto tr0;
	case 34: goto tr0;
	case 35: goto tr0;
	case 36: goto tr0;
	case 37: goto tr0;
	case 38: goto tr0;
	case 39: goto tr0;
	case 40: goto tr0;
	case 41: goto tr0;
	case 42: goto tr0;
	case 43: goto tr0;
	case 44: goto tr0;
	case 45: goto tr0;
	case 46: goto tr0;
	case 47: goto tr0;
	case 48: goto tr0;
	case 49: goto tr0;
	case 50: goto tr0;
	case 51: goto tr0;
	case 52: goto tr0;
	case 53: goto tr0;
	case 54: goto tr0;
	case 55: goto tr0;
	case 56: goto tr0;
	case 57: goto tr0;
	case 58: goto tr0;
	case 59: goto tr0;
	case 60: goto tr0;
	case 61: goto tr0;
	case 62: goto tr0;
	case 63: goto tr0;
	case 64: goto tr0;
	case 65: goto tr0;
	case 66: goto tr0;
	case 67: goto tr0;
	case 68: goto tr0;
	case 69: goto tr0;
	case 70: goto tr0;
	case 71: goto tr0;
	case 72: goto tr0;
	case 73: goto tr0;
	case 74: goto tr0;
	}
	}

	_out: {}
	}

#line 110 "src/parser/control_verbs.rl"
    } catch (LocatedParseError &error) {
        if (ts >= ptr && ts <= pe) {
            error.locate(ts - ptr + start);
        } else {
            error.locate(0);
        }
        throw;
    }

    return p;
}

} // namespace ue2
