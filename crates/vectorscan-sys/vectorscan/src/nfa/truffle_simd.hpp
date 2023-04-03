/*
 * Copyright (c) 2015-2017, Intel Corporation
 * Copyright (c) 2020-2021, VectorCamp PC
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
 * \brief Truffle: character class acceleration.
 *
 */

#include "truffle.h"
#include "ue2common.h"
#include "util/arch.h"
#include "util/bitutils.h"
#include "util/unaligned.h"

#include "util/supervector/supervector.hpp"
#include "util/match.hpp"

template <uint16_t S>
static really_inline
const SuperVector<S> blockSingleMask(SuperVector<S> shuf_mask_lo_highclear, SuperVector<S> shuf_mask_lo_highset, SuperVector<S> chars);

#if defined(ARCH_IA32) || defined(ARCH_X86_64)
#include "x86/truffle.hpp"
#elif defined(ARCH_ARM32) || defined(ARCH_AARCH64)
#include "arm/truffle.hpp"
#elif defined(ARCH_PPC64EL)
#include "ppc64el/truffle.hpp"
#endif

template <uint16_t S>
static really_inline
const u8 *fwdBlock(SuperVector<S> shuf_mask_lo_highclear, SuperVector<S> shuf_mask_lo_highset, SuperVector<S> chars, const u8 *buf) {
    SuperVector<S> res = blockSingleMask(shuf_mask_lo_highclear, shuf_mask_lo_highset, chars);
    return first_zero_match_inverted<S>(buf, res);
}

template <uint16_t S>
const u8 *truffleExecReal(m128 &shuf_mask_lo_highclear, m128 shuf_mask_lo_highset, const u8 *buf, const u8 *buf_end) {
    assert(buf && buf_end);
    assert(buf < buf_end);
    DEBUG_PRINTF("truffle %p len %zu\n", buf, buf_end - buf);
    DEBUG_PRINTF("b %s\n", buf);

    const SuperVector<S> wide_shuf_mask_lo_highclear(shuf_mask_lo_highclear);
    const SuperVector<S> wide_shuf_mask_lo_highset(shuf_mask_lo_highset);

    const u8 *d = buf;
    const u8 *rv;

    DEBUG_PRINTF("start %p end %p \n", d, buf_end);
    assert(d < buf_end);

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
            SuperVector<S> chars = SuperVector<S>::loadu(d);
            const u8 *dup = ROUNDUP_PTR(d, S);
            rv = fwdBlock(wide_shuf_mask_lo_highclear, wide_shuf_mask_lo_highset, chars, d);
            if (rv && rv < dup) return rv;
            d = dup;
        }

        while(d + S <= buf_end) {
            __builtin_prefetch(d + 64);
            DEBUG_PRINTF("d %p \n", d);
            SuperVector<S> chars = SuperVector<S>::load(d);
            rv = fwdBlock(wide_shuf_mask_lo_highclear, wide_shuf_mask_lo_highset, chars, d);
            if (rv) return rv;
            d += S;
        }
    }

    DEBUG_PRINTF("d %p e %p \n", d, buf_end);
    // finish off tail

    if (d != buf_end) {
        SuperVector<S> chars = SuperVector<S>::Zeroes();
        const u8* end_buf;
        if (buf_end - buf < S) {
          memcpy(&chars.u, buf, buf_end - buf);
          end_buf = buf;
        } else {
          chars = SuperVector<S>::loadu(buf_end - S);
          end_buf = buf_end - S;
        }
        rv = fwdBlock(wide_shuf_mask_lo_highclear, wide_shuf_mask_lo_highset, chars, end_buf);
        DEBUG_PRINTF("rv %p \n", rv);
        if (rv && rv < buf_end) return rv;
    }

    return buf_end;
}

template <uint16_t S>
static really_inline
const u8 *revBlock(SuperVector<S> shuf_mask_lo_highclear, SuperVector<S> shuf_mask_lo_highset, SuperVector<S> v, 
                    const u8 *buf) {
    SuperVector<S> res = blockSingleMask(shuf_mask_lo_highclear, shuf_mask_lo_highset, v);
    return last_zero_match_inverted<S>(buf, res);
}

template <uint16_t S>
const u8 *rtruffleExecReal(m128 shuf_mask_lo_highclear, m128 shuf_mask_lo_highset, const u8 *buf, const u8 *buf_end){
    assert(buf && buf_end);
    assert(buf < buf_end);
    DEBUG_PRINTF("trufle %p len %zu\n", buf, buf_end - buf);
    DEBUG_PRINTF("b %s\n", buf);

    const SuperVector<S> wide_shuf_mask_lo_highclear(shuf_mask_lo_highclear);
    const SuperVector<S> wide_shuf_mask_lo_highset(shuf_mask_lo_highset);

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
            SuperVector<S> chars = SuperVector<S>::loadu(d - S);
            const u8 *dbot = ROUNDDOWN_PTR(d, S);
            rv = revBlock(wide_shuf_mask_lo_highclear, wide_shuf_mask_lo_highset, chars, d - S);
            DEBUG_PRINTF("rv %p \n", rv);
            if (rv >= dbot) return rv;
            d = dbot;
        }

        while (d - S >= buf) {
            DEBUG_PRINTF("aligned %p \n", d);
            // On large packet buffers, this prefetch appears to get us about 2%.
            __builtin_prefetch(d - 64);

            d -= S;
            SuperVector<S> chars = SuperVector<S>::load(d);
            rv = revBlock(wide_shuf_mask_lo_highclear, wide_shuf_mask_lo_highset, chars, d);
            if (rv) return rv;
        }
    }

    DEBUG_PRINTF("tail d %p e %p \n", buf, d);
    // finish off head

    if (d != buf) {
        SuperVector<S> chars = SuperVector<S>::Zeroes();
        if (buf_end - buf < S) {
          memcpy(&chars.u, buf, buf_end - buf);
        } else {
          chars = SuperVector<S>::loadu(buf);
        }
        rv = revBlock(wide_shuf_mask_lo_highclear, wide_shuf_mask_lo_highset, chars, buf);
        DEBUG_PRINTF("rv %p \n", rv);
        if (rv && rv < buf_end) return rv;
    }

    return buf - 1;
}
