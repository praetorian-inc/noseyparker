
#line 1 "util/ExpressionParser.rl"
/*
 * Copyright (c) 2015-2018, Intel Corporation
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

#include "config.h"

#include "ExpressionParser.h"

#include <cassert>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>

#include "ue2common.h"
#include "hs_compile.h"


using std::string;

namespace { // anon

enum ParamKey {
    PARAM_NONE,
    PARAM_MIN_OFFSET,
    PARAM_MAX_OFFSET,
    PARAM_MIN_LENGTH,
    PARAM_EDIT_DISTANCE,
    PARAM_HAMM_DISTANCE
};


#line 60 "util/ExpressionParser.cpp"
static const char _ExpressionParser_actions[] = {
	0, 1, 0, 1, 1, 1, 2, 1, 
	3, 1, 4, 1, 5, 1, 6, 1, 
	7, 1, 9, 1, 10, 2, 8, 0
	
};

static const char _ExpressionParser_eof_actions[] = {
	0, 19, 19, 19, 19, 19, 19, 19, 
	19, 19, 19, 19, 19, 19, 19, 19, 
	19, 19, 19, 19, 19, 19, 19, 19, 
	19, 19, 19, 19, 19, 19, 19, 19, 
	19, 19, 19, 19, 19, 19, 19, 19, 
	19, 19, 19, 19, 19, 19, 19, 19, 
	19, 19, 19, 19, 19, 19, 19, 19, 
	0, 0
};

static const int ExpressionParser_start = 56;
static const int ExpressionParser_first_final = 56;
static const int ExpressionParser_error = 0;

static const int ExpressionParser_en_main = 56;


#line 116 "util/ExpressionParser.rl"


} // namespace

static
void initExt(hs_expr_ext *ext) {
    memset(ext, 0, sizeof(*ext));
    ext->max_offset = MAX_OFFSET;
}

bool HS_CDECL readExpression(const std::string &input, std::string &expr,
                             unsigned int *flags, hs_expr_ext *ext,
                             bool *must_be_ordered) {
    assert(flags);
    assert(ext);

    // Init flags and ext params.
    *flags = 0;
    initExt(ext);
    if (must_be_ordered) {
        *must_be_ordered = false;
    }

    // Extract expr, which is easier to do in straight C++ than with Ragel.
    if (input.empty() || input[0] != '/') {
        return false;
    }
    size_t end = input.find_last_of('/');
    if (end == string::npos || end == 0) {
        return false;
    }
    expr = input.substr(1, end - 1);

    // Use a Ragel scanner to handle flags and params.
    const char *p = input.c_str() + end + 1;
    const char *pe = input.c_str() + input.size();
    UNUSED const char *eof = pe;
    UNUSED const char *ts = p, *te = p;
    int cs;
    UNUSED int act;

    assert(p);
    assert(pe);

    // For storing integers as they're scanned.
    u64a num = 0;
    enum ParamKey key = PARAM_NONE;

    
#line 136 "util/ExpressionParser.cpp"
	{
	cs = ExpressionParser_start;
	}

#line 141 "util/ExpressionParser.cpp"
	{
	const char *_acts;
	unsigned int _nacts;

	if ( p == pe )
		goto _test_eof;
	if ( cs == 0 )
		goto _out;
_resume:
	switch ( cs ) {
case 56:
	switch( (*p) ) {
		case 56u: goto tr66;
		case 67u: goto tr66;
		case 72u: goto tr66;
		case 76u: goto tr66;
		case 105u: goto tr66;
		case 109u: goto tr66;
		case 115u: goto tr66;
		case 123u: goto tr67;
	}
	if ( (*p) > 81u ) {
		if ( 86u <= (*p) && (*p) <= 87u )
			goto tr66;
	} else if ( (*p) >= 79u )
		goto tr66;
	goto tr0;
case 0:
	goto _out;
case 1:
	switch( (*p) ) {
		case 32u: goto tr1;
		case 101u: goto tr2;
		case 104u: goto tr3;
		case 109u: goto tr4;
	}
	goto tr0;
case 2:
	switch( (*p) ) {
		case 32u: goto tr5;
		case 101u: goto tr6;
		case 104u: goto tr7;
		case 109u: goto tr8;
	}
	goto tr0;
case 3:
	if ( (*p) == 100u )
		goto tr9;
	goto tr0;
case 4:
	if ( (*p) == 105u )
		goto tr10;
	goto tr0;
case 5:
	if ( (*p) == 116u )
		goto tr11;
	goto tr0;
case 6:
	if ( (*p) == 95u )
		goto tr12;
	goto tr0;
case 7:
	if ( (*p) == 100u )
		goto tr13;
	goto tr0;
case 8:
	if ( (*p) == 105u )
		goto tr14;
	goto tr0;
case 9:
	if ( (*p) == 115u )
		goto tr15;
	goto tr0;
case 10:
	if ( (*p) == 116u )
		goto tr16;
	goto tr0;
case 11:
	if ( (*p) == 97u )
		goto tr17;
	goto tr0;
case 12:
	if ( (*p) == 110u )
		goto tr18;
	goto tr0;
case 13:
	if ( (*p) == 99u )
		goto tr19;
	goto tr0;
case 14:
	if ( (*p) == 101u )
		goto tr20;
	goto tr0;
case 15:
	if ( (*p) == 61u )
		goto tr21;
	goto tr0;
case 16:
	if ( 48u <= (*p) && (*p) <= 57u )
		goto tr22;
	goto tr0;
case 17:
	switch( (*p) ) {
		case 32u: goto tr23;
		case 44u: goto tr24;
		case 125u: goto tr26;
	}
	if ( 48u <= (*p) && (*p) <= 57u )
		goto tr25;
	goto tr0;
case 18:
	switch( (*p) ) {
		case 32u: goto tr23;
		case 44u: goto tr24;
		case 125u: goto tr26;
	}
	goto tr0;
case 57:
	goto tr0;
case 19:
	if ( (*p) == 97u )
		goto tr27;
	goto tr0;
case 20:
	if ( (*p) == 109u )
		goto tr28;
	goto tr0;
case 21:
	if ( (*p) == 109u )
		goto tr29;
	goto tr0;
case 22:
	if ( (*p) == 105u )
		goto tr30;
	goto tr0;
case 23:
	if ( (*p) == 110u )
		goto tr31;
	goto tr0;
case 24:
	if ( (*p) == 103u )
		goto tr32;
	goto tr0;
case 25:
	if ( (*p) == 95u )
		goto tr33;
	goto tr0;
case 26:
	if ( (*p) == 100u )
		goto tr34;
	goto tr0;
case 27:
	if ( (*p) == 105u )
		goto tr35;
	goto tr0;
case 28:
	if ( (*p) == 115u )
		goto tr36;
	goto tr0;
case 29:
	if ( (*p) == 116u )
		goto tr37;
	goto tr0;
case 30:
	if ( (*p) == 97u )
		goto tr38;
	goto tr0;
case 31:
	if ( (*p) == 110u )
		goto tr39;
	goto tr0;
case 32:
	if ( (*p) == 99u )
		goto tr40;
	goto tr0;
case 33:
	if ( (*p) == 101u )
		goto tr41;
	goto tr0;
case 34:
	switch( (*p) ) {
		case 97u: goto tr42;
		case 105u: goto tr43;
	}
	goto tr0;
case 35:
	if ( (*p) == 120u )
		goto tr44;
	goto tr0;
case 36:
	if ( (*p) == 95u )
		goto tr45;
	goto tr0;
case 37:
	if ( (*p) == 111u )
		goto tr46;
	goto tr0;
case 38:
	if ( (*p) == 102u )
		goto tr47;
	goto tr0;
case 39:
	if ( (*p) == 102u )
		goto tr48;
	goto tr0;
case 40:
	if ( (*p) == 115u )
		goto tr49;
	goto tr0;
case 41:
	if ( (*p) == 101u )
		goto tr50;
	goto tr0;
case 42:
	if ( (*p) == 116u )
		goto tr51;
	goto tr0;
case 43:
	if ( (*p) == 110u )
		goto tr52;
	goto tr0;
case 44:
	if ( (*p) == 95u )
		goto tr53;
	goto tr0;
case 45:
	switch( (*p) ) {
		case 108u: goto tr54;
		case 111u: goto tr55;
	}
	goto tr0;
case 46:
	if ( (*p) == 101u )
		goto tr56;
	goto tr0;
case 47:
	if ( (*p) == 110u )
		goto tr57;
	goto tr0;
case 48:
	if ( (*p) == 103u )
		goto tr58;
	goto tr0;
case 49:
	if ( (*p) == 116u )
		goto tr59;
	goto tr0;
case 50:
	if ( (*p) == 104u )
		goto tr60;
	goto tr0;
case 51:
	if ( (*p) == 102u )
		goto tr61;
	goto tr0;
case 52:
	if ( (*p) == 102u )
		goto tr62;
	goto tr0;
case 53:
	if ( (*p) == 115u )
		goto tr63;
	goto tr0;
case 54:
	if ( (*p) == 101u )
		goto tr64;
	goto tr0;
case 55:
	if ( (*p) == 116u )
		goto tr65;
	goto tr0;
	}

	tr0: cs = 0; goto f0;
	tr67: cs = 1; goto _again;
	tr24: cs = 1; goto f4;
	tr5: cs = 2; goto _again;
	tr1: cs = 2; goto f1;
	tr6: cs = 3; goto _again;
	tr2: cs = 3; goto f1;
	tr9: cs = 4; goto _again;
	tr10: cs = 5; goto _again;
	tr11: cs = 6; goto _again;
	tr12: cs = 7; goto _again;
	tr13: cs = 8; goto _again;
	tr14: cs = 9; goto _again;
	tr15: cs = 10; goto _again;
	tr16: cs = 11; goto _again;
	tr17: cs = 12; goto _again;
	tr18: cs = 13; goto _again;
	tr19: cs = 14; goto _again;
	tr20: cs = 15; goto f2;
	tr41: cs = 15; goto f6;
	tr51: cs = 15; goto f7;
	tr60: cs = 15; goto f8;
	tr65: cs = 15; goto f9;
	tr21: cs = 16; goto _again;
	tr22: cs = 17; goto f3;
	tr25: cs = 17; goto f5;
	tr23: cs = 18; goto _again;
	tr7: cs = 19; goto _again;
	tr3: cs = 19; goto f1;
	tr27: cs = 20; goto _again;
	tr28: cs = 21; goto _again;
	tr29: cs = 22; goto _again;
	tr30: cs = 23; goto _again;
	tr31: cs = 24; goto _again;
	tr32: cs = 25; goto _again;
	tr33: cs = 26; goto _again;
	tr34: cs = 27; goto _again;
	tr35: cs = 28; goto _again;
	tr36: cs = 29; goto _again;
	tr37: cs = 30; goto _again;
	tr38: cs = 31; goto _again;
	tr39: cs = 32; goto _again;
	tr40: cs = 33; goto _again;
	tr8: cs = 34; goto _again;
	tr4: cs = 34; goto f1;
	tr42: cs = 35; goto _again;
	tr44: cs = 36; goto _again;
	tr45: cs = 37; goto _again;
	tr46: cs = 38; goto _again;
	tr47: cs = 39; goto _again;
	tr48: cs = 40; goto _again;
	tr49: cs = 41; goto _again;
	tr50: cs = 42; goto _again;
	tr43: cs = 43; goto _again;
	tr52: cs = 44; goto _again;
	tr53: cs = 45; goto _again;
	tr54: cs = 46; goto _again;
	tr56: cs = 47; goto _again;
	tr57: cs = 48; goto _again;
	tr58: cs = 49; goto _again;
	tr59: cs = 50; goto _again;
	tr55: cs = 51; goto _again;
	tr61: cs = 52; goto _again;
	tr62: cs = 53; goto _again;
	tr63: cs = 54; goto _again;
	tr64: cs = 55; goto _again;
	tr66: cs = 56; goto f10;
	tr26: cs = 57; goto f4;

	f5: _acts = _ExpressionParser_actions + 1; goto execFuncs;
	f10: _acts = _ExpressionParser_actions + 3; goto execFuncs;
	f4: _acts = _ExpressionParser_actions + 5; goto execFuncs;
	f9: _acts = _ExpressionParser_actions + 7; goto execFuncs;
	f7: _acts = _ExpressionParser_actions + 9; goto execFuncs;
	f8: _acts = _ExpressionParser_actions + 11; goto execFuncs;
	f2: _acts = _ExpressionParser_actions + 13; goto execFuncs;
	f6: _acts = _ExpressionParser_actions + 15; goto execFuncs;
	f1: _acts = _ExpressionParser_actions + 17; goto execFuncs;
	f0: _acts = _ExpressionParser_actions + 19; goto execFuncs;
	f3: _acts = _ExpressionParser_actions + 21; goto execFuncs;

execFuncs:
	_nacts = *_acts++;
	while ( _nacts-- > 0 ) {
		switch ( *_acts++ ) {
	case 0:
#line 60 "util/ExpressionParser.rl"
	{
        num = (num * 10) + ((*p) - '0');
    }
	break;
	case 1:
#line 64 "util/ExpressionParser.rl"
	{
        switch ((*p)) {
            case 'i': *flags |= HS_FLAG_CASELESS; break;
            case 's': *flags |= HS_FLAG_DOTALL; break;
            case 'm': *flags |= HS_FLAG_MULTILINE; break;
            case 'H': *flags |= HS_FLAG_SINGLEMATCH; break;
            case 'O':
                if (must_be_ordered) {
                    *must_be_ordered = true;
                }
                break;
            case 'V': *flags |= HS_FLAG_ALLOWEMPTY; break;
            case 'W': *flags |= HS_FLAG_UCP; break;
            case '8': *flags |= HS_FLAG_UTF8; break;
            case 'P': *flags |= HS_FLAG_PREFILTER; break;
            case 'L': *flags |= HS_FLAG_SOM_LEFTMOST; break;
            case 'C': *flags |= HS_FLAG_COMBINATION; break;
            case 'Q': *flags |= HS_FLAG_QUIET; break;
            default: {p++; goto _out; }
        }
    }
	break;
	case 2:
#line 86 "util/ExpressionParser.rl"
	{
        switch (key) {
            case PARAM_MIN_OFFSET:
                ext->flags |= HS_EXT_FLAG_MIN_OFFSET;
                ext->min_offset = num;
                break;
            case PARAM_MAX_OFFSET:
                ext->flags |= HS_EXT_FLAG_MAX_OFFSET;
                ext->max_offset = num;
                break;
            case PARAM_MIN_LENGTH:
                ext->flags |= HS_EXT_FLAG_MIN_LENGTH;
                ext->min_length = num;
                break;
            case PARAM_EDIT_DISTANCE:
                ext->flags |= HS_EXT_FLAG_EDIT_DISTANCE;
                ext->edit_distance = num;
                break;
            case PARAM_HAMM_DISTANCE:
                ext->flags |= HS_EXT_FLAG_HAMMING_DISTANCE;
                ext->hamming_distance = num;
                break;
            case PARAM_NONE:
            default:
                // No key specified, syntax invalid.
                return false;
        }
    }
	break;
	case 3:
#line 166 "util/ExpressionParser.rl"
	{ key = PARAM_MIN_OFFSET; }
	break;
	case 4:
#line 167 "util/ExpressionParser.rl"
	{ key = PARAM_MAX_OFFSET; }
	break;
	case 5:
#line 168 "util/ExpressionParser.rl"
	{ key = PARAM_MIN_LENGTH; }
	break;
	case 6:
#line 169 "util/ExpressionParser.rl"
	{ key = PARAM_EDIT_DISTANCE; }
	break;
	case 7:
#line 170 "util/ExpressionParser.rl"
	{ key = PARAM_HAMM_DISTANCE; }
	break;
	case 8:
#line 172 "util/ExpressionParser.rl"
	{num = 0;}
	break;
	case 9:
#line 173 "util/ExpressionParser.rl"
	{ key = PARAM_NONE; }
	break;
	case 10:
#line 178 "util/ExpressionParser.rl"
	{ return false; }
	break;
#line 593 "util/ExpressionParser.cpp"
		}
	}
	goto _again;

_again:
	if ( cs == 0 )
		goto _out;
	if ( ++p != pe )
		goto _resume;
	_test_eof: {}
	if ( p == eof )
	{
	const char *__acts = _ExpressionParser_actions + _ExpressionParser_eof_actions[cs];
	unsigned int __nacts = (unsigned int) *__acts++;
	while ( __nacts-- > 0 ) {
		switch ( *__acts++ ) {
	case 10:
#line 178 "util/ExpressionParser.rl"
	{ return false; }
	break;
#line 614 "util/ExpressionParser.cpp"
		}
	}
	}

	_out: {}
	}

#line 183 "util/ExpressionParser.rl"


    DEBUG_PRINTF("expr='%s', flags=%u\n", expr.c_str(), *flags);

    return (cs != ExpressionParser_error) && (p == pe);
}
