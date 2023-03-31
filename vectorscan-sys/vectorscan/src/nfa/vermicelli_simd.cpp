/*
 * Copyright (c) 2015-2020, Intel Corporation
 * Copyright (c) 2020-2021, VectorCamp PC
 * Copyright (c) 2021, Arm Limited
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

/** \file
 * \brief Vermicelli: single-byte and double-byte acceleration.
 */

#include "util/bitutils.h"
#include "util/simd_utils.h"

#include "vermicelli.hpp"
#include "util/supervector/casemask.hpp"
#include "util/match.hpp"

template <uint16_t S>
static really_inline
const u8 *vermicelliBlock(SuperVector<S> const data, SuperVector<S> const chars, SuperVector<S> const casemask, u8 const *buf, u16 const len);

template <uint16_t S>
static really_inline
const u8 *vermicelliBlockNeg(SuperVector<S> const data, SuperVector<S> const chars, SuperVector<S> const casemask, u8 const *buf, u16 const len);

template <uint16_t S>
static really_inline
const u8 *rvermicelliBlock(SuperVector<S> const data, SuperVector<S> const chars, SuperVector<S> const casemask, u8 const *buf, u16 const len);

template <uint16_t S>
static really_inline
const u8 *rvermicelliBlockNeg(SuperVector<S> const data, SuperVector<S> const chars, SuperVector<S> const casemask, const u8 *buf, u16 const len);

template <uint16_t S, bool check_partial = true>
static really_inline
const u8 *vermicelliDoubleBlock(SuperVector<S> const data, SuperVector<S> const chars1, SuperVector<S> const chars2, SuperVector<S> const casemask,
                                u8 const c1, u8 const c2, u8 const casechar, u8 const *buf, u16 const len);

template <uint16_t S, bool check_partial = true>
static really_inline
const u8 *rvermicelliDoubleBlock(SuperVector<S> const data, SuperVector<S> const chars1, SuperVector<S> const chars2, SuperVector<S> const casemask,
                                 u8 const c1, u8 const c2, u8 const casechar, u8 const *buf, u16 const len);

template <uint16_t S, bool check_partial = true>
static really_inline
const u8 *vermicelliDoubleMaskedBlock(SuperVector<S> const data, SuperVector<S> const chars1, SuperVector<S> const chars2,
                                      SuperVector<S> const mask1, SuperVector<S> const mask2,
                                      u8 const c1, u8 const c2, u8 const m1, u8 const m2, u8 const *buf, u16 const len);

#if defined(ARCH_IA32) || defined(ARCH_X86_64)
#include "x86/vermicelli.hpp"
#elif defined(ARCH_ARM32) || defined(ARCH_AARCH64)
#include "arm/vermicelli.hpp"
#elif defined(ARCH_PPC64EL)
#include "ppc64el/vermicelli.hpp"
#endif

template <uint16_t S>
static const u8 *vermicelliExecReal(SuperVector<S> const chars, SuperVector<S> const casemask, u8 const *buf, u8 const *buf_end) {
    assert(buf && buf_end);
    assert(buf < buf_end);
    DEBUG_PRINTF("verm %p len %zu\n", buf, buf_end - buf);
    DEBUG_PRINTF("b %s\n", buf);

    const u8 *d = buf;
    const u8 *rv;

    __builtin_prefetch(d +   64);
    __builtin_prefetch(d + 2*64);
    __builtin_prefetch(d + 3*64);
    __builtin_prefetch(d + 4*64);
    DEBUG_PRINTF("start %p end %p \n", d, buf_end);
    assert(d < buf_end);
    if (d + S <= buf_end) {
        // Reach vector aligned boundaries
        DEBUG_PRINTF("until aligned %p \n", ROUNDUP_PTR(d, S));
        if (!ISALIGNED_N(d, S)) {
            u8 const *d1 = ROUNDUP_PTR(d, S);
            SuperVector<S> data = SuperVector<S>::loadu(d);
            rv = vermicelliBlock(data, chars, casemask, d, S);
            if (rv) return rv;
            d = d1;
        }

        while(d + S <= buf_end) {
            __builtin_prefetch(d + 64);
            DEBUG_PRINTF("d %p \n", d);
            SuperVector<S> data = SuperVector<S>::load(d);
            rv = vermicelliBlock(data, chars, casemask, d, S);
            if (rv) return rv;
            d += S;
        }
    }

    DEBUG_PRINTF("d %p e %p \n", d, buf_end);
    // finish off tail

    if (d != buf_end) {
        SuperVector<S> data = SuperVector<S>::loadu(buf_end - S);
        rv = vermicelliBlock(data, chars, casemask, buf_end - S, buf_end - d);
        DEBUG_PRINTF("rv %p \n", rv);
        if (rv && rv < buf_end) return rv;
    }

    return buf_end;
}

template <uint16_t S>
static const u8 *nvermicelliExecReal(SuperVector<S> const chars, SuperVector<S> const casemask, const u8 *buf, const u8 *buf_end) {
    assert(buf && buf_end);
    assert(buf < buf_end);
    DEBUG_PRINTF("verm %p len %zu\n", buf, buf_end - buf);
    DEBUG_PRINTF("b %s\n", buf);

    const u8 *d = buf;
    const u8 *rv;

    __builtin_prefetch(d +   64);
    __builtin_prefetch(d + 2*64);
    __builtin_prefetch(d + 3*64);
    __builtin_prefetch(d + 4*64);
    DEBUG_PRINTF("start %p end %p \n", d, buf_end);
    assert(d < buf_end);
    if (d + S <= buf_end) {
        // Reach vector aligned boundaries
        DEBUG_PRINTF("until aligned %p \n", ROUNDUP_PTR(d, S));
        if (!ISALIGNED_N(d, S)) {
            u8 const *d1 = ROUNDUP_PTR(d, S);
            SuperVector<S> data = SuperVector<S>::loadu(d);
            rv = vermicelliBlockNeg(data, chars, casemask, d, S);
            if (rv) return rv;
            d = d1;
        }

        while(d + S <= buf_end) {
            __builtin_prefetch(d + 64);
            DEBUG_PRINTF("d %p \n", d);
            SuperVector<S> data = SuperVector<S>::load(d);
            rv = vermicelliBlockNeg(data, chars, casemask, d, S);
            if (rv) return rv;
            d += S;
        }
    }

    DEBUG_PRINTF("d %p e %p \n", d, buf_end);
    // finish off tail

    if (d != buf_end) {
        SuperVector<S> data = SuperVector<S>::loadu(buf_end - S);
        rv = vermicelliBlockNeg(data, chars, casemask, buf_end - S, buf_end - d);
        DEBUG_PRINTF("rv %p \n", rv);
        if (rv && rv < buf_end) return rv;
    }

    return buf_end;
}

// Reverse vermicelli scan. Provides exact semantics and returns (buf - 1) if
// character not found.
template <uint16_t S>
const u8 *rvermicelliExecReal(SuperVector<S> const chars, SuperVector<S> const casemask, const u8 *buf, const u8 *buf_end) {
    assert(buf && buf_end);
    assert(buf < buf_end);
    DEBUG_PRINTF("rverm %p len %zu\n", buf, buf_end - buf);
    DEBUG_PRINTF("b %s\n", buf);

    const u8 *d = buf_end;
    const u8 *rv;

    __builtin_prefetch(d -   64);
    __builtin_prefetch(d - 2*64);
    __builtin_prefetch(d - 3*64);
    __builtin_prefetch(d - 4*64);
    DEBUG_PRINTF("start %p end %p \n", buf, d);
    assert(d > buf);
    if (d - S >= buf) {
        // Reach vector aligned boundaries
        DEBUG_PRINTF("until aligned %p \n", ROUNDDOWN_PTR(d, S));
        if (!ISALIGNED_N(d, S)) {
            u8 const *d1 = ROUNDDOWN_PTR(d, S);
            SuperVector<S> data = SuperVector<S>::loadu(d - S);
            rv = rvermicelliBlock(data, chars, casemask, d - S, S);
            DEBUG_PRINTF("rv %p \n", rv);
            if (rv) return rv;
            d = d1;
        }

        while (d - S >= buf) {
            DEBUG_PRINTF("aligned %p \n", d);
            // On large packet buffers, this prefetch appears to get us about 2%.
            __builtin_prefetch(d - 64);

            d -= S;
            SuperVector<S> data = SuperVector<S>::load(d);
            rv = rvermicelliBlock(data, chars, casemask, d, S);
            if (rv) return rv;
        }
    }

    DEBUG_PRINTF("tail d %p e %p \n", buf, d);
    // finish off head

    if (d != buf) {
        SuperVector<S> data = SuperVector<S>::loadu(buf);
        rv = rvermicelliBlock(data, chars, casemask, buf, d - buf);
        DEBUG_PRINTF("rv %p \n", rv);
        if (rv && rv < buf_end) return rv;
    }

    return buf - 1;
}

// Reverse vermicelli scan. Provides exact semantics and returns (buf - 1) if
// character not found.
template <uint16_t S>
const u8 *rnvermicelliExecReal(SuperVector<S> const chars, SuperVector<S> const casemask, const u8 *buf, const u8 *buf_end) {
    assert(buf && buf_end);
    assert(buf < buf_end);
    DEBUG_PRINTF("rverm %p len %zu\n", buf, buf_end - buf);
    DEBUG_PRINTF("b %s\n", buf);

    const u8 *d = buf_end;
    const u8 *rv;

    __builtin_prefetch(d -   64);
    __builtin_prefetch(d - 2*64);
    __builtin_prefetch(d - 3*64);
    __builtin_prefetch(d - 4*64);
    DEBUG_PRINTF("start %p end %p \n", buf, d);
    assert(d > buf);
    if (d - S >= buf) {
        // Reach vector aligned boundaries
        DEBUG_PRINTF("until aligned %p \n", ROUNDDOWN_PTR(d, S));
        if (!ISALIGNED_N(d, S)) {
            u8 const *d1 = ROUNDDOWN_PTR(d, S);
            SuperVector<S> data = SuperVector<S>::loadu(d - S);
            rv = rvermicelliBlockNeg(data, chars, casemask, d - S, S);
            DEBUG_PRINTF("rv %p \n", rv);
            if (rv) return rv;
            d = d1;
        }

        while (d - S >= buf) {
            DEBUG_PRINTF("aligned %p \n", d);
            // On large packet buffers, this prefetch appears to get us about 2%.
            __builtin_prefetch(d - 64);

            d -= S;
            SuperVector<S> data = SuperVector<S>::load(d);
            rv = rvermicelliBlockNeg(data, chars, casemask, d, S);
            if (rv) return rv;
        }
    }

    DEBUG_PRINTF("tail d %p e %p \n", buf, d);
    // finish off head

    if (d != buf) {
        SuperVector<S> data = SuperVector<S>::loadu(buf);
        rv = rvermicelliBlockNeg(data, chars, casemask, buf, d - buf);
        DEBUG_PRINTF("rv %p \n", rv);
        if (rv && rv < buf_end) return rv;
    }

    return buf - 1;
}

template <uint16_t S>
static const u8 *vermicelliDoubleExecReal(u8 const c1, u8 const c2, SuperVector<S> const casemask,
                                          const u8 *buf, const u8 *buf_end) {
    assert(buf && buf_end);
    assert(buf < buf_end);
    DEBUG_PRINTF("verm %p len %zu\n", buf, buf_end - buf);
    DEBUG_PRINTF("b %s\n", buf);

    const u8 *d = buf;
    const u8 *rv;
    // SuperVector<S> lastmask1{0};
    const SuperVector<VECTORSIZE> chars1 = SuperVector<VECTORSIZE>::dup_u8(c1);
    const SuperVector<VECTORSIZE> chars2 = SuperVector<VECTORSIZE>::dup_u8(c2);
    const u8 casechar = casemask.u.u8[0];

    __builtin_prefetch(d +   64);
    __builtin_prefetch(d + 2*64);
    __builtin_prefetch(d + 3*64);
    __builtin_prefetch(d + 4*64);
    DEBUG_PRINTF("start %p end %p \n", d, buf_end);
    assert(d < buf_end);
    if (d + S < buf_end) {
        // Reach vector aligned boundaries
        DEBUG_PRINTF("until aligned %p \n", ROUNDUP_PTR(d, S));
        if (!ISALIGNED_N(d, S)) {
            u8 const *d1 = ROUNDUP_PTR(d, S);
            SuperVector<S> data = SuperVector<S>::loadu(d);
            rv = vermicelliDoubleBlock(data, chars1, chars2, casemask, c1, c2, casechar, d + S, S);
            if (rv) return rv - S;
            d = d1;
        }

        while(d + S < buf_end) {
            __builtin_prefetch(d + 64);
            DEBUG_PRINTF("d %p \n", d);
            SuperVector<S> data = SuperVector<S>::load(d);
            rv = vermicelliDoubleBlock(data, chars1, chars2, casemask, c1, c2, casechar, d + S, S);
            if (rv) return rv - S;
            d += S;
        }
    }

    DEBUG_PRINTF("tail d %p e %p \n", d, buf_end);
    // finish off tail

    if (d != buf_end) {
        SuperVector<S> data = SuperVector<S>::Zeroes();
        if (buf_end - d < S) {
          memcpy(&data.u, d, buf_end - d);
        } else {
          data = SuperVector<S>::loadu(d);
        }
        rv = vermicelliDoubleBlock<S, false>(data, chars1, chars2, casemask, c1, c2, casechar, d, buf_end - d);
        DEBUG_PRINTF("rv %p \n", rv);
        if (rv && rv < buf_end) return rv;
    }

    DEBUG_PRINTF("real tail d %p e %p \n", d, buf_end);
    /* check for partial match at end */
    u8 mask = casemask.u.u8[0];
    if ((buf_end[-1] & mask) == (u8)c1) {
        DEBUG_PRINTF("partial!!!\n");
        return buf_end - 1;
    }

    return buf_end;
}

// /* returns highest offset of c2 (NOTE: not c1) */
template <uint16_t S>
const u8 *rvermicelliDoubleExecReal(char c1, char c2, SuperVector<S> const casemask, const u8 *buf, const u8 *buf_end) {
    assert(buf && buf_end);
    assert(buf < buf_end);
    DEBUG_PRINTF("rverm %p len %zu\n", buf, buf_end - buf);
    DEBUG_PRINTF("b %s\n", buf);
    char s[255];
    snprintf(s, buf_end - buf + 1, "%s", buf);
    DEBUG_PRINTF("b %s\n", s);

    const u8 *d = buf_end;
    const u8 *rv;
    const SuperVector<VECTORSIZE> chars1 = SuperVector<VECTORSIZE>::dup_u8(c1);
    const SuperVector<VECTORSIZE> chars2 = SuperVector<VECTORSIZE>::dup_u8(c2);
    const u8 casechar = casemask.u.u8[0];

    __builtin_prefetch(d -   64);
    __builtin_prefetch(d - 2*64);
    __builtin_prefetch(d - 3*64);
    __builtin_prefetch(d - 4*64);
    DEBUG_PRINTF("start %p end %p \n", buf, d);
    assert(d > buf);
    if (d - S > buf) {
        // Reach vector aligned boundaries
        DEBUG_PRINTF("until aligned %p \n", ROUNDDOWN_PTR(d, S));
        if (!ISALIGNED_N(d, S)) {
            u8 const *d1 = ROUNDDOWN_PTR(d, S);
            SuperVector<S> data = SuperVector<S>::loadu(d - S);
            rv = rvermicelliDoubleBlock(data, chars1, chars2, casemask, c1, c2, casechar, d - S, S);
            DEBUG_PRINTF("rv %p \n", rv);
            if (rv && rv < buf_end) return rv;
            d = d1;
        }

        while (d - S > buf) {
            DEBUG_PRINTF("aligned %p \n", d);
            // On large packet buffers, this prefetch appears to get us about 2%.
            __builtin_prefetch(d - 64);

            d -= S;
            SuperVector<S> data = SuperVector<S>::load(d);
            rv = rvermicelliDoubleBlock(data, chars1, chars2, casemask, c1, c2, casechar, d, S);
            if (rv) return rv;
        }
    }

    DEBUG_PRINTF("tail d %p e %p \n", buf, d);
    // finish off head

    if (d != buf) {
        SuperVector<S> data = SuperVector<S>::Zeroes();
        if (d - buf < S) {
          memcpy(&data.u, buf, d - buf);
        } else {
          data = SuperVector<S>::loadu(buf);
        }
        rv = rvermicelliDoubleBlock<S, false>(data, chars1, chars2, casemask, c1, c2, casechar, buf, d - buf);
        DEBUG_PRINTF("rv %p \n", rv);
        if (rv && rv < buf_end) return rv;
    }

    return buf - 1;
}

template <uint16_t S>
static const u8 *vermicelliDoubleMaskedExecReal(u8 const c1, u8 const c2, u8 const m1, u8 const m2,
                                                const u8 *buf, const u8 *buf_end) {
    assert(buf && buf_end);
    assert(buf < buf_end);
    DEBUG_PRINTF("verm %p len %zu\n", buf, buf_end - buf);
    DEBUG_PRINTF("b %s\n", buf);

    const u8 *d = buf;
    const u8 *rv;
    // SuperVector<S> lastmask1{0};
    const SuperVector<VECTORSIZE> chars1 = SuperVector<VECTORSIZE>::dup_u8(c1);
    const SuperVector<VECTORSIZE> chars2 = SuperVector<VECTORSIZE>::dup_u8(c2);
    const SuperVector<VECTORSIZE> mask1 = SuperVector<VECTORSIZE>::dup_u8(m1);
    const SuperVector<VECTORSIZE> mask2 = SuperVector<VECTORSIZE>::dup_u8(m2);

    __builtin_prefetch(d +   64);
    __builtin_prefetch(d + 2*64);
    __builtin_prefetch(d + 3*64);
    __builtin_prefetch(d + 4*64);
    DEBUG_PRINTF("start %p end %p \n", d, buf_end);
    assert(d < buf_end);
    if (d + S < buf_end) {
        // Reach vector aligned boundaries
        DEBUG_PRINTF("until aligned %p \n", ROUNDUP_PTR(d, S));
        if (!ISALIGNED_N(d, S)) {
            u8 const *d1 = ROUNDUP_PTR(d, S);
            SuperVector<S> data = SuperVector<S>::loadu(d);
            rv = vermicelliDoubleMaskedBlock(data, chars1, chars2, mask1, mask2, c1, c2, m1, m2, d + S, S);
            if (rv) return rv - S;
            d = d1;
        }

        while(d + S < buf_end) {
            __builtin_prefetch(d + 64);
            DEBUG_PRINTF("d %p \n", d);
            SuperVector<S> data = SuperVector<S>::load(d);
            rv = vermicelliDoubleMaskedBlock(data, chars1, chars2, mask1, mask2, c1, c2, m1, m2, d + S, S);
            if (rv) return rv - S;
            d += S;
        }
    }

    DEBUG_PRINTF("tail d %p e %p \n", d, buf_end);
    // finish off tail

    if (d != buf_end) {
        SuperVector<S> data = SuperVector<S>::Zeroes();
        if (buf_end - d < S) {
          memcpy(&data.u, d, buf_end - d);
        } else {
          data = SuperVector<S>::loadu(d);
        }
        rv = vermicelliDoubleMaskedBlock<S, false>(data, chars1, chars2, mask1, mask2, c1, c2, m1, m2, d, buf_end - d);
        DEBUG_PRINTF("rv %p \n", rv);
        if (rv && rv < buf_end) return rv;
    }

    DEBUG_PRINTF("real tail d %p e %p \n", d, buf_end);
    /* check for partial match at end */
    if ((buf_end[-1] & m1) == (u8)c1) {
        DEBUG_PRINTF("partial!!!\n");
        return buf_end - 1;
    }

    return buf_end;
}

extern "C" const u8 *vermicelliExec(char c, char nocase, const u8 *buf, const u8 *buf_end) {
    DEBUG_PRINTF("verm scan %s\\x%02hhx over %zu bytes\n",
                 nocase ? "nocase " : "", c, (size_t)(buf_end - buf));
    assert(buf < buf_end);

    // Small ranges.
    if (buf_end - buf < VECTORSIZE) {
        for (; buf < buf_end; buf++) {
            char cur = (char)*buf;
            if (nocase) {
                cur &= CASE_CLEAR;
            }
            if (cur == c) {
                break;
            }
        }
        return buf;
    }

    const SuperVector<VECTORSIZE> chars = SuperVector<VECTORSIZE>::dup_u8(c);
    const SuperVector<VECTORSIZE> casemask{nocase ? getCaseMask<VECTORSIZE>() : SuperVector<VECTORSIZE>::Ones()};

    return vermicelliExecReal<VECTORSIZE>(chars, casemask, buf, buf_end);
}

/* like vermicelliExec except returns the address of the first character which
 * is not c */
extern "C" const u8 *nvermicelliExec(char c, char nocase, const u8 *buf, const u8 *buf_end) {
    DEBUG_PRINTF("nverm scan %s\\x%02hhx over %zu bytes\n",
                 nocase ? "nocase " : "", c, (size_t)(buf_end - buf));
    assert(buf < buf_end);

    // Small ranges.
    if (buf_end - buf < VECTORSIZE) {
        for (; buf < buf_end; buf++) {
            char cur = *buf;
            if (nocase) {
                cur &= CASE_CLEAR;
            }
            if (cur != c) {
                break;
            }
        }
        return buf;
    }

    const SuperVector<VECTORSIZE> chars = SuperVector<VECTORSIZE>::dup_u8(c);
    const SuperVector<VECTORSIZE> casemask{nocase ? getCaseMask<VECTORSIZE>() : SuperVector<VECTORSIZE>::Ones()};

    return nvermicelliExecReal<VECTORSIZE>(chars, casemask, buf, buf_end);
}

extern "C" const u8 *rvermicelliExec(char c, char nocase, const u8 *buf, const u8 *buf_end) {
    DEBUG_PRINTF("rev verm scan %s\\x%02hhx over %zu bytes\n",
                 nocase ? "nocase " : "", c, (size_t)(buf_end - buf));
    assert(buf < buf_end);

    // Small ranges.
    if (buf_end - buf < VECTORSIZE) {
        for (buf_end--; buf_end >= buf; buf_end--) {
            char cur = (char)*buf_end;
            if (nocase) {
                cur &= CASE_CLEAR;
            }
            if (cur == c) {
                break;
            }
        }
        return buf_end;
    }

    const SuperVector<VECTORSIZE> chars = SuperVector<VECTORSIZE>::dup_u8(c);
    const SuperVector<VECTORSIZE> casemask{nocase ? getCaseMask<VECTORSIZE>() : SuperVector<VECTORSIZE>::Ones()};

    return rvermicelliExecReal<VECTORSIZE>(chars, casemask, buf, buf_end);
}

extern "C" const u8 *rnvermicelliExec(char c, char nocase, const u8 *buf, const u8 *buf_end) {
     DEBUG_PRINTF("rev verm scan %s\\x%02hhx over %zu bytes\n",
                  nocase ? "nocase " : "", c, (size_t)(buf_end - buf));
    assert(buf < buf_end);

    // Small ranges.
    if (buf_end - buf < VECTORSIZE) {
        for (buf_end--; buf_end >= buf; buf_end--) {
            char cur = (char)*buf_end;
            if (nocase) {
                cur &= CASE_CLEAR;
            }
            if (cur != c) {
                break;
            }
        }
        return buf_end;
    }

    const SuperVector<VECTORSIZE> chars = SuperVector<VECTORSIZE>::dup_u8(c);
    const SuperVector<VECTORSIZE> casemask{nocase ? getCaseMask<VECTORSIZE>() : SuperVector<VECTORSIZE>::Ones()};

    return rnvermicelliExecReal<VECTORSIZE>(chars, casemask, buf, buf_end);
}

extern "C" const u8 *vermicelliDoubleExec(char c1, char c2, char nocase, const u8 *buf, const u8 *buf_end) {
    DEBUG_PRINTF("double verm scan %s\\x%02hhx%02hhx over %zu bytes\n",
                 nocase ? "nocase " : "", c1, c2, (size_t)(buf_end - buf));
    assert(buf < buf_end);

    const SuperVector<VECTORSIZE> casemask{nocase ? getCaseMask<VECTORSIZE>() : SuperVector<VECTORSIZE>::Ones()};

    return vermicelliDoubleExecReal<VECTORSIZE>(c1, c2, casemask, buf, buf_end);
}

extern "C" const u8 *rvermicelliDoubleExec(char c1, char c2, char nocase, const u8 *buf, const u8 *buf_end) {
    DEBUG_PRINTF("rev double verm scan %s\\x%02hhx%02hhx over %zu bytes\n",
                 nocase ? "nocase " : "", c1, c2, (size_t)(buf_end - buf));
    assert(buf < buf_end);

    const SuperVector<VECTORSIZE> casemask{nocase ? getCaseMask<VECTORSIZE>() : SuperVector<VECTORSIZE>::Ones()};

    return rvermicelliDoubleExecReal<VECTORSIZE>(c1, c2, casemask, buf, buf_end);
}

extern "C" const u8 *vermicelliDoubleMaskedExec(char c1, char c2, char m1, char m2,
                                     const u8 *buf, const u8 *buf_end) {
    DEBUG_PRINTF("double verm scan (\\x%02hhx&\\x%02hhx)(\\x%02hhx&\\x%02hhx) "
                 "over %zu bytes\n", c1, m1, c2, m2, (size_t)(buf_end - buf));
    assert(buf < buf_end);

    return vermicelliDoubleMaskedExecReal<VECTORSIZE>(c1, c2, m1, m2, buf, buf_end);
}
