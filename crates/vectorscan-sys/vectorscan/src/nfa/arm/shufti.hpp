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
 * \brief Shufti: character class acceleration.
 */

template <uint16_t S>
static really_inline
const SuperVector<S> blockSingleMask(SuperVector<S> mask_lo, SuperVector<S> mask_hi, SuperVector<S> chars) {
    const SuperVector<S> low4bits = SuperVector<S>::dup_u8(0xf);

    SuperVector<S> c_lo = chars & low4bits;
    SuperVector<S> c_hi = chars.template vshr_8_imm<4>();
    c_lo = mask_lo.template pshufb<false>(c_lo);
    c_hi = mask_hi.template pshufb<false>(c_hi);

    return (c_lo & c_hi) > (SuperVector<S>::Zeroes());
}

template <uint16_t S>
static really_inline
SuperVector<S> blockDoubleMask(SuperVector<S> mask1_lo, SuperVector<S> mask1_hi, SuperVector<S> mask2_lo, SuperVector<S> mask2_hi, SuperVector<S> chars) {

    const SuperVector<S> low4bits = SuperVector<S>::dup_u8(0xf);
    SuperVector<S> chars_lo = chars & low4bits;
    chars_lo.print8("chars_lo");
    SuperVector<S> chars_hi = chars.template vshr_64_imm<4>() & low4bits;
    chars_hi.print8("chars_hi");
    SuperVector<S> c1_lo = mask1_lo.template pshufb<true>(chars_lo);
    c1_lo.print8("c1_lo");
    SuperVector<S> c1_hi = mask1_hi.template pshufb<true>(chars_hi);
    c1_hi.print8("c1_hi");
    SuperVector<S> t1 = c1_lo | c1_hi;
    t1.print8("t1");

    SuperVector<S> c2_lo = mask2_lo.template pshufb<true>(chars_lo);
    c2_lo.print8("c2_lo");
    SuperVector<S> c2_hi = mask2_hi.template pshufb<true>(chars_hi);
    c2_hi.print8("c2_hi");
    SuperVector<S> t2 = c2_lo | c2_hi;
    t2.print8("t2");
    t2.template vshr_128_imm<1>().print8("t2.vshr_128(1)");
    SuperVector<S> t = t1 | (t2.template vshr_128_imm<1>());
    t.print8("t");

    return !t.eq(SuperVector<S>::Ones());
}