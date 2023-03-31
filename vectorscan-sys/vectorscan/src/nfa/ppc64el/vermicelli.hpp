/*
 * Copyright (c) 2015-2020, Intel Corporation
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
 * \brief Vermicelli: single-byte and double-byte acceleration.
 */

template <uint16_t S>
static really_inline
const u8 *vermicelliBlock(SuperVector<S> const data, SuperVector<S> const chars, SuperVector<S> const casemask, u8 const *buf, u16 const len) {

    SuperVector<S> mask = chars.eq(casemask & data);
    return first_non_zero_match<S>(buf, mask, len);
}

template <uint16_t S>
static really_inline
const u8 *vermicelliBlockNeg(SuperVector<S> const data, SuperVector<S> const chars, SuperVector<S> const casemask, u8 const *buf, u16 const len) {

    SuperVector<S> mask = chars.eq(casemask & data);
    return first_zero_match_inverted<S>(buf, mask, len);
}

template <uint16_t S>
static really_inline
const u8 *rvermicelliBlock(SuperVector<S> const data, SuperVector<S> const chars, SuperVector<S> const casemask, u8 const *buf, u16 const len) {

    SuperVector<S> mask = chars.eq(casemask & data);
    return last_non_zero_match<S>(buf, mask, len);
}

template <uint16_t S>
static really_inline
const u8 *rvermicelliBlockNeg(SuperVector<S> const data, SuperVector<S> const chars, SuperVector<S> const casemask, const u8 *buf, u16 const len) {

    data.print8("data");
    chars.print8("chars");
    casemask.print8("casemask");
    SuperVector<S> mask = chars.eq(casemask & data);
    mask.print8("mask");
    return last_zero_match_inverted<S>(buf, mask, len);
}

template <uint16_t S, bool check_partial>
static really_inline
const u8 *vermicelliDoubleBlock(SuperVector<S> const data, SuperVector<S> const chars1, SuperVector<S> const chars2, SuperVector<S> const casemask,
                                u8 const c1, u8 const c2, u8 const casechar, u8 const *buf, u16 const len) {

    SuperVector<S> v = casemask & data;
    SuperVector<S> mask1 = chars1.eq(v);
    SuperVector<S> mask2 = chars2.eq(v);
    SuperVector<S> mask = mask1 & (mask2 >> 1);

    DEBUG_PRINTF("rv[0] = %02hhx, rv[-1] = %02hhx\n", buf[0], buf[-1]);
    bool partial_match = (check_partial && ((buf[0] & casechar) == c2) && ((buf[-1] & casechar) == c1));
    DEBUG_PRINTF("partial = %d\n", partial_match);
    if (partial_match) {
        mask = mask | ((SuperVector<S>::Ones() >> (S-1)) << (S-1));
    }

    return first_non_zero_match<S>(buf, mask, len);
}

template <uint16_t S, bool check_partial>
static really_inline
const u8 *rvermicelliDoubleBlock(SuperVector<S> const data, SuperVector<S> const chars1, SuperVector<S> const chars2, SuperVector<S> const casemask,
                                 u8 const c1, u8 const c2, u8 const casechar, u8 const *buf, u16 const len) {

    SuperVector<S> v = casemask & data;
    SuperVector<S> mask1 = chars1.eq(v);
    SuperVector<S> mask2 = chars2.eq(v);
    SuperVector<S> mask = (mask1 << 1)& mask2;

    DEBUG_PRINTF("buf[0] = %02hhx, buf[-1] = %02hhx\n", buf[0], buf[-1]);
    bool partial_match = (check_partial && ((buf[0] & casechar) == c2) && ((buf[-1] & casechar) == c1));
    DEBUG_PRINTF("partial = %d\n", partial_match);
    if (partial_match) {
        mask = mask | (SuperVector<S>::Ones() >> (S-1));
    }

    return last_non_zero_match<S>(buf, mask, len);
}

template <uint16_t S, bool check_partial>
static really_inline
const u8 *vermicelliDoubleMaskedBlock(SuperVector<S> const data, SuperVector<S> const chars1, SuperVector<S> const chars2,
                                      SuperVector<S> const mask1, SuperVector<S> const mask2,
                                      u8 const c1, u8 const c2, u8 const m1, u8 const m2, u8 const *buf, u16 const len) {

    SuperVector<S> v1 = chars1.eq(data & mask1);
    SuperVector<S> v2 = chars2.eq(data & mask2);
    SuperVector<S> mask = v1 & (v2 >> 1);

    DEBUG_PRINTF("rv[0] = %02hhx, rv[-1] = %02hhx\n", buf[0], buf[-1]);
    bool partial_match = (check_partial && ((buf[0] & m2) == c2) && ((buf[-1] & m1) == c1));
    DEBUG_PRINTF("partial = %d\n", partial_match);
    if (partial_match) {
        mask = mask | ((SuperVector<S>::Ones() >> (S-1)) << (S-1));
    }

    return first_non_zero_match<S>(buf, mask, len);
}


