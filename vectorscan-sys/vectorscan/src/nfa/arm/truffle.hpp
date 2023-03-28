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

template <uint16_t S>
static really_inline
const SuperVector<S> blockSingleMask(SuperVector<S> shuf_mask_lo_highclear, SuperVector<S> shuf_mask_lo_highset, SuperVector<S> chars) {

    chars.print8("chars");
    shuf_mask_lo_highclear.print8("shuf_mask_lo_highclear");
    shuf_mask_lo_highset.print8("shuf_mask_lo_highset");

    SuperVector<S> highconst = SuperVector<S>::dup_u8(0x80);
    highconst.print8("highconst");
    SuperVector<S> shuf_mask_hi = SuperVector<S>::dup_u64(0x8040201008040201);
    shuf_mask_hi.print8("shuf_mask_hi");
    
    SuperVector<S> shuf1 = shuf_mask_lo_highclear.pshufb(chars);
    shuf1.print8("shuf1");
    SuperVector<S> t1 = chars ^ highconst;
    t1.print8("t1");
    SuperVector<S> shuf2 = shuf_mask_lo_highset.pshufb(t1);
    shuf2.print8("shuf2");
    SuperVector<S> t2 = highconst.opandnot(chars.template vshr_64_imm<4>());
    t2.print8("t2");
    SuperVector<S> shuf3 = shuf_mask_hi.pshufb(t2);
    shuf3.print8("shuf3");
    SuperVector<S> res = (shuf1 | shuf2) & shuf3;
    res.print8("(shuf1 | shuf2) & shuf3");

    return !res.eq(SuperVector<S>::Zeroes());
}
