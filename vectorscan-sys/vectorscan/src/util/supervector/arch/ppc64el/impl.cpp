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

#ifndef SIMD_IMPL_HPP
#define SIMD_IMPL_HPP

#include <cstdint>
#include <cstdio>

#include "ue2common.h"
#include "util/arch.h"
#include "util/unaligned.h"
#include "util/supervector/supervector.hpp"
#include <iostream>

// 128-bit IBM Power VSX implementation

template<>
really_inline SuperVector<16>::SuperVector(SuperVector const &other)
{
    u.v128[0] = other.u.v128[0];
}

template<>
template<>
really_inline SuperVector<16>::SuperVector(char __bool __vector v)
{
    u.u8x16[0] = (uint8x16_t) v;
};

template<>
template<>
really_inline SuperVector<16>::SuperVector(int8x16_t const v)
{
    u.s8x16[0] = v;
};

template<>
template<>
really_inline SuperVector<16>::SuperVector(uint8x16_t const v)
{
    u.u8x16[0] = v;
};

template<>
template<>
really_inline SuperVector<16>::SuperVector(int16x8_t const v)
{
    u.s16x8[0] = v;
};

template<>
template<>
really_inline SuperVector<16>::SuperVector(uint16x8_t const v)
{
    u.u16x8[0] = v;
};

template<>
template<>
really_inline SuperVector<16>::SuperVector(int32x4_t const v)
{
    u.s32x4[0] = v;
};

template<>
template<>
really_inline SuperVector<16>::SuperVector(uint32x4_t const v)
{
    u.u32x4[0] = v;
};

template<>
template<>
really_inline SuperVector<16>::SuperVector(int64x2_t const v)
{
    u.s64x2[0] = v;
};

template<>
template<>
really_inline SuperVector<16>::SuperVector(uint64x2_t const v)
{
    u.u64x2[0] = v;
};

template<>
really_inline SuperVector<16>::SuperVector(typename base_type::type const v)
{
    u.v128[0] = v;
};

template<>
template<>
really_inline SuperVector<16>::SuperVector(int8_t const other)
{
    u.s8x16[0] = vec_splats(other);
}

template<>
template<>
really_inline SuperVector<16>::SuperVector(uint8_t const other)
{
    u.u8x16[0] = vec_splats(static_cast<uint8_t>(other));
}

template<>
template<>
really_inline SuperVector<16>::SuperVector(int16_t const other)
{
    u.s16x8[0] = vec_splats(other);
}

template<>
template<>
really_inline SuperVector<16>::SuperVector(uint16_t const other)
{
    u.u16x8[0] = vec_splats(static_cast<uint16_t>(other));
}

template<>
template<>
really_inline SuperVector<16>::SuperVector(int32_t const other)
{
    u.s32x4[0] = vec_splats(other);
}

template<>
template<>
really_inline SuperVector<16>::SuperVector(uint32_t const other)
{
    u.u32x4[0] = vec_splats(static_cast<uint32_t>(other));
}

template<>
template<>
really_inline SuperVector<16>::SuperVector(int64_t const other)
{
    u.s64x2[0] = (int64x2_t) vec_splats(static_cast<ulong64_t>(other));
}

template<>
template<>
really_inline SuperVector<16>::SuperVector(uint64_t const other)
{
    u.u64x2[0] = (uint64x2_t) vec_splats(static_cast<ulong64_t>(other));
}

// Constants
template<>
really_inline SuperVector<16> SuperVector<16>::Ones(void)
{
    return  { vec_splat_s8(-1)};
}

template<>
really_inline SuperVector<16> SuperVector<16>::Zeroes(void)
{
    return  { vec_splat_s8(0) };
}

// Methods

template <>
really_inline void SuperVector<16>::operator=(SuperVector<16> const &other)
{
    u.v128[0] = other.u.v128[0];
}

template <>
really_inline SuperVector<16> SuperVector<16>::operator&(SuperVector<16> const &b) const
{
    return { vec_and(u.v128[0], b.u.v128[0]) };
}

template <>
really_inline SuperVector<16> SuperVector<16>::operator|(SuperVector<16> const &b) const
{
    return  { vec_or(u.v128[0], b.u.v128[0]) };
}

template <>
really_inline SuperVector<16> SuperVector<16>::operator^(SuperVector<16> const &b) const
{
    return  { vec_xor(u.v128[0], b.u.v128[0]) };
}

template <>
really_inline SuperVector<16> SuperVector<16>::operator!() const
{
    return  { vec_xor(u.v128[0], u.v128[0]) };
}

template <>
really_inline SuperVector<16> SuperVector<16>::opandnot(SuperVector<16> const &b) const
{
   int8x16_t not_res = vec_xor(u.s8x16[0], vec_splat_s8(-1));
   return { vec_and(not_res, b.u.s8x16[0]) };
}

template <>
really_inline SuperVector<16> SuperVector<16>::operator==(SuperVector<16> const &b) const
{
    return { vec_cmpeq(u.s8x16[0], b.u.s8x16[0])};
}

template <>
really_inline SuperVector<16> SuperVector<16>::operator!=(SuperVector<16> const &b) const
{
    return !(*this == b);
}

template <>
really_inline SuperVector<16> SuperVector<16>::operator>(SuperVector<16> const &b) const
{ 
    return { vec_cmpgt(u.s8x16[0], b.u.s8x16[0])};
}

template <>
really_inline SuperVector<16> SuperVector<16>::operator>=(SuperVector<16> const &b) const
{
    return { vec_cmpge(u.s8x16[0], b.u.s8x16[0])};
}

template <>
really_inline SuperVector<16> SuperVector<16>::operator<(SuperVector<16> const &b) const
{
    return { vec_cmpgt(b.u.s8x16[0], u.s8x16[0])};
}

template <>
really_inline SuperVector<16> SuperVector<16>::operator<=(SuperVector<16> const &b) const
{   
    return { vec_cmpge(b.u.s8x16[0], u.s8x16[0])};
}

template <>
really_inline SuperVector<16> SuperVector<16>::eq(SuperVector<16> const &b) const
{
    return (*this == b);
}

template <>
really_inline typename SuperVector<16>::comparemask_type
SuperVector<16>::comparemask(void) const {
    uint8x16_t bitmask = vec_gb( u.u8x16[0]);
    static uint8x16_t perm = { 16, 24, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 };
    bitmask = (uint8x16_t) vec_perm(vec_splat_u8(0), bitmask, perm);
    u32 movemask;
    vec_ste((uint32x4_t) bitmask, 0, &movemask);
    return movemask;
}

template <>
really_inline typename SuperVector<16>::comparemask_type
SuperVector<16>::eqmask(SuperVector<16> const b) const {
    return eq(b).comparemask();
}

template <> really_inline u32 SuperVector<16>::mask_width() { return 1; }

template <>
really_inline typename SuperVector<16>::comparemask_type
SuperVector<16>::iteration_mask(
    typename SuperVector<16>::comparemask_type mask) {
    return mask;
}

template <>
template<uint8_t N>
really_inline SuperVector<16> SuperVector<16>::vshl_8_imm() const
{
    return { vec_sl(u.s8x16[0], vec_splat_u8(N)) };
}

template <>
template<uint8_t N>
really_inline SuperVector<16> SuperVector<16>::vshl_16_imm() const
{
    return { vec_sl(u.s16x8[0], vec_splat_u16(N)) };
}

template <>
template<uint8_t N>
really_inline SuperVector<16> SuperVector<16>::vshl_32_imm() const
{
    return { vec_sl(u.s32x4[0], vec_splat_u32(N)) };
}

template <>
template<uint8_t N>
really_inline SuperVector<16> SuperVector<16>::vshl_64_imm() const
{
    return { vec_sl(u.s64x2[0], vec_splats((ulong64_t) N)) };
}

template <>
template<uint8_t N>
really_inline SuperVector<16> SuperVector<16>::vshl_128_imm() const
{
    return { vec_sld(u.s8x16[0], vec_splat_s8(0), N)};
}

template <>
template<uint8_t N>
really_inline SuperVector<16> SuperVector<16>::vshl_imm() const
{
   return vshl_128_imm<N>();
}

template <>
template<uint8_t N>
really_inline SuperVector<16> SuperVector<16>::vshr_8_imm() const
{
    return { vec_sr(u.s8x16[0], vec_splat_u8(N)) };
}

template <>
template<uint8_t N>
really_inline SuperVector<16> SuperVector<16>::vshr_16_imm() const
{
    return { vec_sr(u.s16x8[0], vec_splat_u16(N)) };
}

template <>
template<uint8_t N>
really_inline SuperVector<16> SuperVector<16>::vshr_32_imm() const
{
    return { vec_sr(u.s32x4[0], vec_splat_u32(N)) };
}

template <>
template<uint8_t N>
really_inline SuperVector<16> SuperVector<16>::vshr_64_imm() const
{		 
   return { vec_sr(u.s64x2[0], vec_splats((ulong64_t)N)) };
}

template <>
template<uint8_t N>
really_inline SuperVector<16> SuperVector<16>::vshr_128_imm() const
{   
    return { vec_sld(vec_splat_s8(0), u.s8x16[0], 16 - N) };
}

template <>
template<uint8_t N>
really_inline SuperVector<16> SuperVector<16>::vshr_imm() const
{
    return vshr_128_imm<N>();
}

#if !defined(HS_OPTIMIZE)
template SuperVector<16> SuperVector<16>::vshl_8_imm<4>() const;
template SuperVector<16> SuperVector<16>::vshl_16_imm<1>() const;
template SuperVector<16> SuperVector<16>::vshl_64_imm<1>() const;
template SuperVector<16> SuperVector<16>::vshl_64_imm<4>() const;
template SuperVector<16> SuperVector<16>::vshl_128_imm<1>() const;
template SuperVector<16> SuperVector<16>::vshl_128_imm<4>() const;
template SuperVector<16> SuperVector<16>::vshr_8_imm<1>() const;
template SuperVector<16> SuperVector<16>::vshr_8_imm<4>() const;
template SuperVector<16> SuperVector<16>::vshr_16_imm<1>() const;
template SuperVector<16> SuperVector<16>::vshr_64_imm<1>() const;
template SuperVector<16> SuperVector<16>::vshr_64_imm<4>() const;
template SuperVector<16> SuperVector<16>::vshr_128_imm<1>() const;
template SuperVector<16> SuperVector<16>::vshr_128_imm<4>() const;
#endif

template <>
really_inline SuperVector<16> SuperVector<16>::vshl_8  (uint8_t const N) const
{
    if (N == 0) return *this;
    uint8x16_t shift_indices = vec_splats((uint8_t) N);
    return { vec_sl(u.u8x16[0], shift_indices) };
}

template <>
really_inline SuperVector<16> SuperVector<16>::vshl_16 (uint8_t const UNUSED N) const
{
    if (N == 0) return *this;
    uint16x8_t shift_indices = vec_splats((uint16_t) N);
    return { vec_sl(u.u16x8[0], shift_indices) };
}

template <>
really_inline SuperVector<16> SuperVector<16>::vshl_32 (uint8_t const N) const
{
    if (N == 0) return *this;
    uint32x4_t shift_indices = vec_splats((uint32_t) N);
    return { vec_sl(u.u32x4[0], shift_indices) };
}

template <>
really_inline SuperVector<16> SuperVector<16>::vshl_64 (uint8_t const N) const
{
    if (N == 0) return *this;
    uint64x2_t shift_indices = vec_splats((ulong64_t) N);
    return { vec_sl(u.u64x2[0], shift_indices) };
}

template <>
really_inline SuperVector<16> SuperVector<16>::vshl_128(uint8_t const N) const
{
    if (N == 0) return *this;
    SuperVector sl{N << 3};
    return { vec_slo(u.u8x16[0], sl.u.u8x16[0]) };
}

template <>
really_inline SuperVector<16> SuperVector<16>::vshl(uint8_t const N) const
{
    return vshl_128(N);
}

template <>
really_inline SuperVector<16> SuperVector<16>::vshr_8  (uint8_t const N) const
{
    if (N == 0) return *this;
    uint8x16_t shift_indices = vec_splats((uint8_t) N);
    return { vec_sr(u.u8x16[0], shift_indices) };
}

template <>
really_inline SuperVector<16> SuperVector<16>::vshr_16 (uint8_t const N) const
{
    if (N == 0) return *this;
    uint16x8_t shift_indices = vec_splats((uint16_t) N);
    return { vec_sr(u.u16x8[0], shift_indices) };
}

template <>
really_inline SuperVector<16> SuperVector<16>::vshr_32 (uint8_t const N) const
{
    if (N == 0) return *this;
    uint32x4_t shift_indices = vec_splats((uint32_t) N);
    return { vec_sr(u.u32x4[0], shift_indices) };
}

template <>
really_inline SuperVector<16> SuperVector<16>::vshr_64 (uint8_t const N) const
{
    if (N == 0) return *this;
    uint64x2_t shift_indices = vec_splats((ulong64_t) N);
    return { vec_sr(u.u64x2[0], shift_indices) };
}

template <>
really_inline SuperVector<16> SuperVector<16>::vshr_128(uint8_t const N) const
{
    if (N == 0) return *this;
    SuperVector sr{N << 3};
    return { vec_sro(u.u8x16[0], sr.u.u8x16[0]) };
}

template <>
really_inline SuperVector<16> SuperVector<16>::vshr(uint8_t const N) const
{
    return vshr_128(N);
}

template <>
really_inline SuperVector<16> SuperVector<16>::operator>>(uint8_t const N) const
{
#if defined(HAVE__BUILTIN_CONSTANT_P)
    if (N == 0) return *this;
    if (__builtin_constant_p(N)) {
        return { vec_sld(vec_splat_s8(0),  u.s8x16[0], 16 - N) };
    }
#endif
    return vshr_128(N);
}

template <>
really_inline SuperVector<16> SuperVector<16>::operator<<(uint8_t const N) const
{
#if defined(HAVE__BUILTIN_CONSTANT_P)
    if (N == 0) return *this;
    if (__builtin_constant_p(N)) {
        return { vec_sld(u.s8x16[0], vec_splat_s8(0), N)};
    }
#endif
    return vshl_128(N);
}

template<>
really_inline SuperVector<16> SuperVector<16>::Ones_vshr(uint8_t const N)
{
    return Ones().vshr_128(N);
}

template<>
really_inline SuperVector<16> SuperVector<16>::Ones_vshl(uint8_t const N)
{
    return Ones().vshl_128(N);
}

template <>
really_inline SuperVector<16> SuperVector<16>::loadu(void const *ptr)
{
    return { vec_xl(0, (const long64_t*)ptr) };
}

template <>
really_inline SuperVector<16> SuperVector<16>::load(void const *ptr)
{
    assert(ISALIGNED_N(ptr, alignof(SuperVector::size)));
    return { vec_xl(0, (const long64_t*)ptr) };
}

template <>
really_inline SuperVector<16> SuperVector<16>::loadu_maskz(void const *ptr, uint8_t const len)
{
    SuperVector<16> mask = Ones_vshr(16 -len);
    SuperVector<16> v = loadu(ptr);
    return mask & v;
}

template<>
really_inline SuperVector<16> SuperVector<16>::alignr(SuperVector<16> &other, int8_t offset)
{   
    if (offset == 0) return other;
    if (offset == 16) return *this;
#if defined(HAVE__BUILTIN_CONSTANT_P)
    if (__builtin_constant_p(offset)) {
        return { vec_sld(u.s8x16[0], other.u.s8x16[0], offset) };
    }
#endif
    uint8x16_t sl = vec_splats((uint8_t) (offset << 3));
    uint8x16_t sr = vec_splats((uint8_t) ((16 - offset) << 3));
    uint8x16_t rhs = vec_slo(u.u8x16[0], sr);
    uint8x16_t lhs = vec_sro(other.u.u8x16[0], sl);
    return { vec_or(lhs, rhs) };
}

template<>
template<>
really_inline SuperVector<16> SuperVector<16>::pshufb<false>(SuperVector<16> b)
{
    /* On Intel, if bit 0x80 is set, then result is zero, otherwise which the lane it is &0xf.
       In NEON or PPC, if >=16, then the result is zero, otherwise it is that lane.
       below is the version that is converted from Intel to PPC.  */
    uint8x16_t mask =(uint8x16_t)vec_cmpge(b.u.u8x16[0], vec_splats((uint8_t)0x80));
    uint8x16_t res = vec_perm (u.u8x16[0], u.u8x16[0], b.u.u8x16[0]);
    return { vec_sel(res, vec_splat_u8(0), mask) };
}

template<>
template<>
really_inline SuperVector<16> SuperVector<16>::pshufb<true>(SuperVector<16> b)
{
    /* On Intel, if bit 0x80 is set, then result is zero, otherwise which the lane it is &0xf.
       In NEON or PPC, if >=16, then the result is zero, otherwise it is that lane.
       btranslated is the version that is converted from Intel to PPC.  */
    SuperVector<16> btranslated = b & SuperVector<16>::dup_s8(0x8f);
    return pshufb<false>(btranslated);
}


template<>
really_inline SuperVector<16> SuperVector<16>::pshufb_maskz(SuperVector<16> b, uint8_t const len)
{
    SuperVector<16> mask = Ones_vshr(16 -len);
    return mask & pshufb(b);
}

#endif
