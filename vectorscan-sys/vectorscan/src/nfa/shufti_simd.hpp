/*
 * Copyright (c) 2015-2017, Intel Corporation
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
 * \brief Shufti: character class acceleration.
 *
 * Utilises the SSSE3 pshufb shuffle instruction
 */

#include <algorithm>

#include "shufti.h"
#include "ue2common.h"
#include "util/arch.h"
#include "util/bitutils.h"
#include "util/unaligned.h"

#include "util/supervector/supervector.hpp"
#include "util/match.hpp"

template <uint16_t S>
static really_inline
const SuperVector<S> blockSingleMask(SuperVector<S> mask_lo, SuperVector<S> mask_hi, SuperVector<S> chars);
template <uint16_t S>
static really_inline
SuperVector<S> blockDoubleMask(SuperVector<S> mask1_lo, SuperVector<S> mask1_hi, SuperVector<S> mask2_lo, SuperVector<S> mask2_hi, SuperVector<S> chars);

#if defined(ARCH_IA32) || defined(ARCH_X86_64)
#include "x86/shufti.hpp"
#elif defined(ARCH_ARM32) || defined(ARCH_AARCH64)
#include "arm/shufti.hpp"
#elif defined(ARCH_PPC64EL)
#include "ppc64el/shufti.hpp"
#endif

template <uint16_t S>
static really_inline
const u8 *fwdBlock(SuperVector<S> mask_lo, SuperVector<S> mask_hi, SuperVector<S> chars, const u8 *buf) {
    SuperVector<S> v = blockSingleMask(mask_lo, mask_hi, chars);

    return first_zero_match_inverted<S>(buf, v);
}

template <uint16_t S>
static really_inline
const u8 *revBlock(SuperVector<S> mask_lo, SuperVector<S> mask_hi, SuperVector<S> chars, const u8 *buf) {
    SuperVector<S> v = blockSingleMask(mask_lo, mask_hi, chars);

    return last_zero_match_inverted<S>(buf, v);
}

template <uint16_t S>
static really_inline
const u8 *fwdBlockDouble(SuperVector<S> mask1_lo, SuperVector<S> mask1_hi, SuperVector<S> mask2_lo, SuperVector<S> mask2_hi, SuperVector<S> chars, const u8 *buf) {

    SuperVector<S> mask = blockDoubleMask(mask1_lo, mask1_hi, mask2_lo, mask2_hi, chars);

    return first_zero_match_inverted<S>(buf, mask);
}

template <uint16_t S>
const u8 *shuftiExecReal(m128 mask_lo, m128 mask_hi, const u8 *buf, const u8 *buf_end) {
    assert(buf && buf_end);
    assert(buf < buf_end);
    DEBUG_PRINTF("shufti %p len %zu\n", buf, buf_end - buf);
    DEBUG_PRINTF("b %s\n", buf);

    const SuperVector<S> wide_mask_lo(mask_lo);
    const SuperVector<S> wide_mask_hi(mask_hi);

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
            SuperVector<S> chars = SuperVector<S>::loadu(d);
            rv = fwdBlock(wide_mask_lo, wide_mask_hi, chars, d);
            if (rv) return rv;
            d = ROUNDUP_PTR(d, S);
        }

        while(d + S <= buf_end) {
            __builtin_prefetch(d + 64);
            DEBUG_PRINTF("d %p \n", d);
            SuperVector<S> chars = SuperVector<S>::load(d);
            rv = fwdBlock(wide_mask_lo, wide_mask_hi, chars, d);
            if (rv) return rv;
            d += S;
        }
    }

    DEBUG_PRINTF("d %p e %p \n", d, buf_end);
    // finish off tail

    if (d != buf_end) {
        SuperVector<S> chars = SuperVector<S>::loadu(buf_end - S);
        rv = fwdBlock(wide_mask_lo, wide_mask_hi, chars, buf_end - S);
        DEBUG_PRINTF("rv %p \n", rv);
        if (rv && rv < buf_end) return rv;
    }

    return buf_end;
}

template <uint16_t S>
const u8 *rshuftiExecReal(m128 mask_lo, m128 mask_hi, const u8 *buf, const u8 *buf_end) {
    assert(buf && buf_end);
    assert(buf < buf_end);
    DEBUG_PRINTF("rshufti %p len %zu\n", buf, buf_end - buf);
    DEBUG_PRINTF("b %s\n", buf);

    const SuperVector<S> wide_mask_lo(mask_lo);
    const SuperVector<S> wide_mask_hi(mask_hi);

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
            rv = revBlock(wide_mask_lo, wide_mask_hi, chars, d - S);
            DEBUG_PRINTF("rv %p \n", rv);
            if (rv) return rv;
            d = ROUNDDOWN_PTR(d, S);
        }

        while (d - S >= buf) {
            DEBUG_PRINTF("aligned %p \n", d);
            // On large packet buffers, this prefetch appears to get us about 2%.
            __builtin_prefetch(d - 64);

            d -= S;
            SuperVector<S> chars = SuperVector<S>::load(d);
            rv = revBlock(wide_mask_lo, wide_mask_hi, chars, d);
            if (rv) return rv;
        }
    }

    DEBUG_PRINTF("tail d %p e %p \n", buf, d);
    // finish off head

    if (d != buf) {
        SuperVector<S> chars = SuperVector<S>::loadu(buf);
        rv = revBlock(wide_mask_lo, wide_mask_hi, chars, buf);
        DEBUG_PRINTF("rv %p \n", rv);
        if (rv && rv < buf_end) return rv;
    }

    return buf - 1;
}

template <uint16_t S>
const u8 *shuftiDoubleExecReal(m128 mask1_lo, m128 mask1_hi, m128 mask2_lo, m128 mask2_hi,
                           const u8 *buf, const u8 *buf_end) {
    assert(buf && buf_end);
    assert(buf < buf_end);
    DEBUG_PRINTF("shufti %p len %zu\n", buf, buf_end - buf);
    DEBUG_PRINTF("b %s\n", buf);

    const SuperVector<S> wide_mask1_lo(mask1_lo);
    const SuperVector<S> wide_mask1_hi(mask1_hi);
    const SuperVector<S> wide_mask2_lo(mask2_lo);
    const SuperVector<S> wide_mask2_hi(mask2_hi);

    const u8 *d = buf;
    const u8 *rv;

    __builtin_prefetch(d +   64);
    __builtin_prefetch(d + 2*64);
    __builtin_prefetch(d + 3*64);
    __builtin_prefetch(d + 4*64);
    DEBUG_PRINTF("start %p end %p \n", d, buf_end);
    assert(d < buf_end);
    if (d + S <= buf_end) {
        // peel off first part to cacheline boundary
        DEBUG_PRINTF("until aligned %p \n", ROUNDUP_PTR(d, S));
        if (!ISALIGNED_N(d, S)) {
            SuperVector<S> chars = SuperVector<S>::loadu(d);
            rv = fwdBlockDouble(wide_mask1_lo, wide_mask1_hi, wide_mask2_lo, wide_mask2_hi, chars, d);
            DEBUG_PRINTF("rv %p \n", rv);
            if (rv) return rv;
            d = ROUNDUP_PTR(d, S);
        }

        while(d + S <= buf_end) {
            __builtin_prefetch(d + 64);
            DEBUG_PRINTF("d %p \n", d);

            SuperVector<S> chars = SuperVector<S>::load(d);
            rv = fwdBlockDouble(wide_mask1_lo, wide_mask1_hi, wide_mask2_lo, wide_mask2_hi, chars, d);
            if (rv) return rv;
            d += S;
        }
    }

    DEBUG_PRINTF("tail d %p e %p \n", d, buf_end);
    // finish off tail

    if (d != buf_end) {
        SuperVector<S> chars = SuperVector<S>::Zeroes();
        const u8 *end_buf;
        if (buf_end - buf < S) {
          memcpy(&chars.u, buf, buf_end - buf);
          end_buf = buf;
        } else {
          chars = SuperVector<S>::loadu(buf_end - S);
          end_buf = buf_end - S;
        }
        rv = fwdBlockDouble(wide_mask1_lo, wide_mask1_hi, wide_mask2_lo, wide_mask2_hi, chars, end_buf);
        DEBUG_PRINTF("rv %p \n", rv);
        if (rv && rv < buf_end) return rv;
    }

    return buf_end;
}

const u8 *shuftiExec(m128 mask_lo, m128 mask_hi, const u8 *buf,
                      const u8 *buf_end) {
  if (buf_end - buf < VECTORSIZE) {
    return shuftiFwdSlow((const u8 *)&mask_lo, (const u8 *)&mask_hi, buf, buf_end);
  }
  return shuftiExecReal<VECTORSIZE>(mask_lo, mask_hi, buf, buf_end);
}

const u8 *rshuftiExec(m128 mask_lo, m128 mask_hi, const u8 *buf,
                       const u8 *buf_end) {
    if (buf_end - buf < VECTORSIZE) {
      return shuftiRevSlow((const u8 *)&mask_lo, (const u8 *)&mask_hi, buf, buf_end);
    }
    return rshuftiExecReal<VECTORSIZE>(mask_lo, mask_hi, buf, buf_end);
}

const u8 *shuftiDoubleExec(m128 mask1_lo, m128 mask1_hi,
                            m128 mask2_lo, m128 mask2_hi,
                            const u8 *buf, const u8 *buf_end) {
    return shuftiDoubleExecReal<VECTORSIZE>(mask1_lo, mask1_hi, mask2_lo, mask2_hi, buf, buf_end);
}
