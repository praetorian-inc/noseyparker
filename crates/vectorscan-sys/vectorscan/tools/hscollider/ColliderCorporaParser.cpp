
#line 1 "tools/hscollider/ColliderCorporaParser.rl"
/*
 * Copyright (c) 2015-2017, Intel Corporation
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

#include "ColliderCorporaParser.h"
#include "Corpora.h"

#include "ue2common.h"

#include <cassert>
#include <cstdlib>
#include <string>
#include <cstdio>

using namespace std;

namespace /* anonymous */ {

// Take a string like '\xFF' and convert it to the character it represents
char unhex(const char *start, UNUSED const char *end) {
    assert(start + 4 == end);
    assert(start[0] == '\\');
    assert(start[1] == 'x');
    assert(isxdigit(start[2]));
    assert(isxdigit(start[2]));

    char temp[3] = {start[2], start[3], 0};

    return strtol(temp, nullptr, 16);
}


#line 62 "tools/hscollider/ColliderCorporaParser.cpp"
static const char _FileCorporaParser_actions[] = {
	0, 1, 0, 1, 3, 1, 4, 1, 
	5, 1, 6, 1, 7, 1, 8, 1, 
	9, 1, 10, 1, 11, 1, 12, 1, 
	13, 1, 14, 1, 15, 1, 16, 1, 
	17, 1, 18, 1, 19, 1, 20, 1, 
	21, 1, 22, 1, 23, 1, 24, 2, 
	0, 2, 2, 3, 0, 3, 1, 0, 
	2
};

static const char _FileCorporaParser_to_state_actions[] = {
	0, 9, 0, 0, 0, 0, 0, 0, 
	0, 0, 9, 0, 9, 0, 9, 9, 
	0, 0
};

static const char _FileCorporaParser_from_state_actions[] = {
	0, 0, 0, 0, 0, 0, 0, 0, 
	0, 0, 11, 0, 11, 0, 11, 11, 
	0, 0
};

static const int FileCorporaParser_start = 1;
static const int FileCorporaParser_first_final = 9;
static const int FileCorporaParser_error = 0;

static const int FileCorporaParser_en_corpus_old = 10;
static const int FileCorporaParser_en_corpus_new = 12;
static const int FileCorporaParser_en_colon_sep = 14;
static const int FileCorporaParser_en_match_list = 15;
static const int FileCorporaParser_en_main = 1;


#line 89 "tools/hscollider/ColliderCorporaParser.rl"


} // namespace

bool parseCorpus(const string &line, Corpus &c, unsigned int &id) {
    const char *p = line.c_str();
    const char *pe = p + line.size();
    const char *eof = pe;
    const char *ts;
    const char *te;
    int cs;
    UNUSED int act;

    // For storing integers as they're scanned
    unsigned int num = 0;

    string &sout = c.data;

    
#line 117 "tools/hscollider/ColliderCorporaParser.cpp"
	{
	cs = FileCorporaParser_start;
	ts = 0;
	te = 0;
	act = 0;
	}

#line 125 "tools/hscollider/ColliderCorporaParser.cpp"
	{
	const char *_acts;
	unsigned int _nacts;

	if ( p == pe )
		goto _test_eof;
	if ( cs == 0 )
		goto _out;
_resume:
	_acts = _FileCorporaParser_actions + _FileCorporaParser_from_state_actions[cs];
	_nacts = (unsigned int) *_acts++;
	while ( _nacts-- > 0 ) {
		switch ( *_acts++ ) {
	case 7:
#line 1 "NONE"
	{ts = p;}
	break;
#line 143 "tools/hscollider/ColliderCorporaParser.cpp"
		}
	}

	switch ( cs ) {
case 1:
	if ( 48u <= (*p) && (*p) <= 57u )
		goto tr0;
	goto tr1;
case 0:
	goto _out;
case 2:
	switch( (*p) ) {
		case 58u: goto tr3;
		case 61u: goto tr4;
	}
	if ( 48u <= (*p) && (*p) <= 57u )
		goto tr2;
	goto tr1;
case 9:
	goto tr1;
case 3:
	if ( (*p) == 34u )
		goto tr5;
	goto tr1;
case 10:
	if ( (*p) == 92u )
		goto tr15;
	goto tr14;
case 11:
	switch( (*p) ) {
		case 48u: goto tr18;
		case 97u: goto tr18;
		case 110u: goto tr18;
		case 114u: goto tr18;
		case 116u: goto tr18;
		case 118u: goto tr18;
		case 120u: goto tr19;
	}
	if ( (*p) < 98u ) {
		if ( (*p) > 57u ) {
			if ( 65u <= (*p) && (*p) <= 90u )
				goto tr16;
		} else if ( (*p) >= 49u )
			goto tr16;
	} else if ( (*p) > 100u ) {
		if ( (*p) > 102u ) {
			if ( 103u <= (*p) && (*p) <= 122u )
				goto tr16;
		} else if ( (*p) >= 101u )
			goto tr18;
	} else
		goto tr16;
	goto tr17;
case 4:
	if ( (*p) < 65u ) {
		if ( 48u <= (*p) && (*p) <= 57u )
			goto tr7;
	} else if ( (*p) > 70u ) {
		if ( 97u <= (*p) && (*p) <= 102u )
			goto tr7;
	} else
		goto tr7;
	goto tr6;
case 5:
	if ( (*p) < 65u ) {
		if ( 48u <= (*p) && (*p) <= 57u )
			goto tr8;
	} else if ( (*p) > 70u ) {
		if ( 97u <= (*p) && (*p) <= 102u )
			goto tr8;
	} else
		goto tr8;
	goto tr6;
case 12:
	switch( (*p) ) {
		case 34u: goto tr21;
		case 92u: goto tr22;
	}
	goto tr20;
case 13:
	switch( (*p) ) {
		case 48u: goto tr25;
		case 97u: goto tr25;
		case 110u: goto tr25;
		case 114u: goto tr25;
		case 116u: goto tr25;
		case 118u: goto tr25;
		case 120u: goto tr26;
	}
	if ( (*p) < 98u ) {
		if ( (*p) > 57u ) {
			if ( 65u <= (*p) && (*p) <= 90u )
				goto tr23;
		} else if ( (*p) >= 49u )
			goto tr23;
	} else if ( (*p) > 100u ) {
		if ( (*p) > 102u ) {
			if ( 103u <= (*p) && (*p) <= 122u )
				goto tr23;
		} else if ( (*p) >= 101u )
			goto tr25;
	} else
		goto tr23;
	goto tr24;
case 6:
	if ( (*p) < 65u ) {
		if ( 48u <= (*p) && (*p) <= 57u )
			goto tr10;
	} else if ( (*p) > 70u ) {
		if ( 97u <= (*p) && (*p) <= 102u )
			goto tr10;
	} else
		goto tr10;
	goto tr9;
case 7:
	if ( (*p) < 65u ) {
		if ( 48u <= (*p) && (*p) <= 57u )
			goto tr11;
	} else if ( (*p) > 70u ) {
		if ( 97u <= (*p) && (*p) <= 102u )
			goto tr11;
	} else
		goto tr11;
	goto tr9;
case 14:
	if ( (*p) == 58u )
		goto tr27;
	goto tr1;
case 15:
	if ( (*p) == 32u )
		goto tr28;
	if ( 48u <= (*p) && (*p) <= 57u )
		goto tr29;
	goto tr1;
case 8:
	if ( (*p) == 32u )
		goto tr12;
	if ( 48u <= (*p) && (*p) <= 57u )
		goto tr13;
	goto tr1;
case 16:
	switch( (*p) ) {
		case 32u: goto tr31;
		case 44u: goto tr32;
	}
	if ( 48u <= (*p) && (*p) <= 57u )
		goto tr13;
	goto tr30;
case 17:
	switch( (*p) ) {
		case 32u: goto tr31;
		case 44u: goto tr32;
	}
	goto tr30;
	}

	tr1: cs = 0; goto _again;
	tr0: cs = 2; goto f1;
	tr2: cs = 2; goto f2;
	tr4: cs = 3; goto _again;
	tr19: cs = 4; goto _again;
	tr7: cs = 5; goto _again;
	tr26: cs = 6; goto _again;
	tr10: cs = 7; goto _again;
	tr12: cs = 8; goto _again;
	tr28: cs = 8; goto f22;
	tr3: cs = 9; goto f3;
	tr5: cs = 9; goto f4;
	tr6: cs = 10; goto f5;
	tr8: cs = 10; goto f6;
	tr14: cs = 10; goto f11;
	tr16: cs = 10; goto f13;
	tr17: cs = 10; goto f14;
	tr18: cs = 10; goto f15;
	tr15: cs = 11; goto f12;
	tr9: cs = 12; goto f7;
	tr11: cs = 12; goto f8;
	tr20: cs = 12; goto f16;
	tr21: cs = 12; goto f17;
	tr23: cs = 12; goto f18;
	tr24: cs = 12; goto f19;
	tr25: cs = 12; goto f20;
	tr22: cs = 13; goto f12;
	tr27: cs = 14; goto f21;
	tr30: cs = 15; goto f24;
	tr32: cs = 15; goto f25;
	tr13: cs = 16; goto f9;
	tr29: cs = 16; goto f23;
	tr31: cs = 17; goto _again;

	f9: _acts = _FileCorporaParser_actions + 1; goto execFuncs;
	f22: _acts = _FileCorporaParser_actions + 3; goto execFuncs;
	f3: _acts = _FileCorporaParser_actions + 5; goto execFuncs;
	f4: _acts = _FileCorporaParser_actions + 7; goto execFuncs;
	f12: _acts = _FileCorporaParser_actions + 13; goto execFuncs;
	f6: _acts = _FileCorporaParser_actions + 15; goto execFuncs;
	f15: _acts = _FileCorporaParser_actions + 17; goto execFuncs;
	f14: _acts = _FileCorporaParser_actions + 19; goto execFuncs;
	f11: _acts = _FileCorporaParser_actions + 21; goto execFuncs;
	f13: _acts = _FileCorporaParser_actions + 23; goto execFuncs;
	f5: _acts = _FileCorporaParser_actions + 25; goto execFuncs;
	f8: _acts = _FileCorporaParser_actions + 27; goto execFuncs;
	f20: _acts = _FileCorporaParser_actions + 29; goto execFuncs;
	f19: _acts = _FileCorporaParser_actions + 31; goto execFuncs;
	f16: _acts = _FileCorporaParser_actions + 33; goto execFuncs;
	f17: _acts = _FileCorporaParser_actions + 35; goto execFuncs;
	f18: _acts = _FileCorporaParser_actions + 37; goto execFuncs;
	f7: _acts = _FileCorporaParser_actions + 39; goto execFuncs;
	f21: _acts = _FileCorporaParser_actions + 41; goto execFuncs;
	f25: _acts = _FileCorporaParser_actions + 43; goto execFuncs;
	f24: _acts = _FileCorporaParser_actions + 45; goto execFuncs;
	f2: _acts = _FileCorporaParser_actions + 47; goto execFuncs;
	f23: _acts = _FileCorporaParser_actions + 50; goto execFuncs;
	f1: _acts = _FileCorporaParser_actions + 53; goto execFuncs;

execFuncs:
	_nacts = *_acts++;
	while ( _nacts-- > 0 ) {
		switch ( *_acts++ ) {
	case 0:
#line 62 "tools/hscollider/ColliderCorporaParser.rl"
	{
        num = (num * 10) + ((*p) - '0');
    }
	break;
	case 1:
#line 108 "tools/hscollider/ColliderCorporaParser.rl"
	{num = 0;}
	break;
	case 2:
#line 108 "tools/hscollider/ColliderCorporaParser.rl"
	{id = num;}
	break;
	case 3:
#line 134 "tools/hscollider/ColliderCorporaParser.rl"
	{num = 0;}
	break;
	case 4:
#line 138 "tools/hscollider/ColliderCorporaParser.rl"
	{ {cs = 10;goto _again;} }
	break;
	case 5:
#line 141 "tools/hscollider/ColliderCorporaParser.rl"
	{ c.hasMatches = true; {cs = 12;goto _again;} }
	break;
	case 8:
#line 1 "NONE"
	{te = p+1;}
	break;
	case 9:
#line 66 "tools/hscollider/ColliderCorporaParser.rl"
	{te = p+1;{
        sout.push_back(unhex(ts, te));
    }}
	break;
	case 10:
#line 70 "tools/hscollider/ColliderCorporaParser.rl"
	{te = p+1;{
        switch (*(ts+1)) {
            case '0': sout.push_back('\x00'); break;
            case 'a': sout.push_back('\x07'); break;
            case 'e': sout.push_back('\x1b'); break;
            case 'f': sout.push_back('\x0c'); break;
            case 'n': sout.push_back('\x0a'); break;
            case 'v': sout.push_back('\x0b'); break;
            case 'r': sout.push_back('\x0d'); break;
            case 't': sout.push_back('\x09'); break;
            default: {p++; goto _out; }
        }
    }}
	break;
	case 11:
#line 117 "tools/hscollider/ColliderCorporaParser.rl"
	{te = p+1;{ sout.push_back(*(ts + 1)); }}
	break;
	case 12:
#line 118 "tools/hscollider/ColliderCorporaParser.rl"
	{te = p+1;{ sout.push_back(*ts); }}
	break;
	case 13:
#line 118 "tools/hscollider/ColliderCorporaParser.rl"
	{te = p;p--;{ sout.push_back(*ts); }}
	break;
	case 14:
#line 118 "tools/hscollider/ColliderCorporaParser.rl"
	{{p = ((te))-1;}{ sout.push_back(*ts); }}
	break;
	case 15:
#line 66 "tools/hscollider/ColliderCorporaParser.rl"
	{te = p+1;{
        sout.push_back(unhex(ts, te));
    }}
	break;
	case 16:
#line 70 "tools/hscollider/ColliderCorporaParser.rl"
	{te = p+1;{
        switch (*(ts+1)) {
            case '0': sout.push_back('\x00'); break;
            case 'a': sout.push_back('\x07'); break;
            case 'e': sout.push_back('\x1b'); break;
            case 'f': sout.push_back('\x0c'); break;
            case 'n': sout.push_back('\x0a'); break;
            case 'v': sout.push_back('\x0b'); break;
            case 'r': sout.push_back('\x0d'); break;
            case 't': sout.push_back('\x09'); break;
            default: {p++; goto _out; }
        }
    }}
	break;
	case 17:
#line 124 "tools/hscollider/ColliderCorporaParser.rl"
	{te = p+1;{ sout.push_back(*(ts + 1)); }}
	break;
	case 18:
#line 125 "tools/hscollider/ColliderCorporaParser.rl"
	{te = p+1;{ sout.push_back(*ts); }}
	break;
	case 19:
#line 126 "tools/hscollider/ColliderCorporaParser.rl"
	{te = p+1;{ {cs = 14;goto _again;} }}
	break;
	case 20:
#line 125 "tools/hscollider/ColliderCorporaParser.rl"
	{te = p;p--;{ sout.push_back(*ts); }}
	break;
	case 21:
#line 125 "tools/hscollider/ColliderCorporaParser.rl"
	{{p = ((te))-1;}{ sout.push_back(*ts); }}
	break;
	case 22:
#line 130 "tools/hscollider/ColliderCorporaParser.rl"
	{te = p+1;{{cs = 15;goto _again;} }}
	break;
	case 23:
#line 84 "tools/hscollider/ColliderCorporaParser.rl"
	{te = p+1;{
        c.matches.insert(num);
    }}
	break;
	case 24:
#line 84 "tools/hscollider/ColliderCorporaParser.rl"
	{te = p;p--;{
        c.matches.insert(num);
    }}
	break;
#line 489 "tools/hscollider/ColliderCorporaParser.cpp"
		}
	}
	goto _again;

_again:
	_acts = _FileCorporaParser_actions + _FileCorporaParser_to_state_actions[cs];
	_nacts = (unsigned int) *_acts++;
	while ( _nacts-- > 0 ) {
		switch ( *_acts++ ) {
	case 6:
#line 1 "NONE"
	{ts = 0;}
	break;
#line 503 "tools/hscollider/ColliderCorporaParser.cpp"
		}
	}

	if ( cs == 0 )
		goto _out;
	if ( ++p != pe )
		goto _resume;
	_test_eof: {}
	if ( p == eof )
	{
	switch ( cs ) {
	case 11: goto tr16;
	case 4: goto tr6;
	case 5: goto tr6;
	case 13: goto tr23;
	case 6: goto tr9;
	case 7: goto tr9;
	case 16: goto tr30;
	case 17: goto tr30;
	}
	}

	_out: {}
	}

#line 148 "tools/hscollider/ColliderCorporaParser.rl"


    return (cs != FileCorporaParser_error) && (p == pe);
}
