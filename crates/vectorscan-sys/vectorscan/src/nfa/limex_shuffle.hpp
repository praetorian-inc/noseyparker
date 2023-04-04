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
 * \brief Naive dynamic shuffles.
 *
 * These are written with the assumption that the provided masks are sparsely
 * populated and never contain more than 32 on bits. Other implementations will
 * be faster and actually correct if these assumptions don't hold true.
 */

#ifndef LIMEX_SHUFFLE_HPP
#define LIMEX_SHUFFLE_HPP

#include "ue2common.h"
#include "util/arch.h"
#include "util/bitutils.h"
#include "util/unaligned.h"
#include "util/supervector/supervector.hpp"

template <u16 S>
u32 packedExtract(SuperVector<S> s, const SuperVector<S> permute, const SuperVector<S> compare);


template <>
really_really_inline
u32 packedExtract<16>(SuperVector<16> s, const SuperVector<16> permute, const SuperVector<16> compare) {
    SuperVector<16> shuffled = s.pshufb<true>(permute);
    SuperVector<16> compared = shuffled & compare;
    u64a rv = (~compared.eqmask(shuffled)) & 0xffff;
    if (SuperVector<16>::mask_width() != 1) {
        u32 ans = 0;
        for (u32 i = 0; i < 16; ++i) {
            ans |= (rv & (1ull << (i * SuperVector<16>::mask_width()))) >>
                   (i * SuperVector<16>::mask_width() - i);
        }
        return ans;
    }
    return (u32)rv;
}

template <>
really_really_inline
u32 packedExtract<32>(SuperVector<32> s, const SuperVector<32> permute, const SuperVector<32> compare) {
    SuperVector<32> shuffled = s.pshufb<true>(permute);
    SuperVector<32> compared = shuffled & compare;
    // TODO(danlark1): Future ARM support might have a bug.
    u64a rv = (~compared.eqmask(shuffled)) & 0xffffffff;
    return (u32)((rv >> 16) | (rv & 0xffffU));
}

template <>
really_really_inline
u32 packedExtract<64>(SuperVector<64> s, const SuperVector<64> permute, const SuperVector<64> compare) {
    SuperVector<64> shuffled = s.pshufb<true>(permute);
    SuperVector<64> compared = shuffled & compare;
    // TODO(danlark1): Future ARM support might have a bug.
    u64a rv = ~compared.eqmask(shuffled);
    rv = rv >> 32 | rv;
    return (u32)(((rv >> 16) | rv) & 0xffffU);
}


#endif // LIMEX_SHUFFLE_HPP
