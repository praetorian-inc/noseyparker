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

// 128-bit SSE implementation
#if !(!defined(RELEASE_BUILD) && defined(FAT_RUNTIME) && (defined(HAVE_AVX2) || defined(HAVE_AVX512))) && defined(HAVE_SIMD_128_BITS)

template<>
really_inline SuperVector<16>::SuperVector(SuperVector const &other)
{
    u.v128[0] = other.u.v128[0];
}

template<>
really_inline SuperVector<16>::SuperVector(typename base_type::type const v)
{
    u.v128[0] = v;
};

template<>
template<>
really_inline SuperVector<16>::SuperVector(int8_t const other)
{
    u.v128[0] = _mm_set1_epi8(other);
}

template<>
template<>
really_inline SuperVector<16>::SuperVector(uint8_t const other)
{
    u.v128[0] = _mm_set1_epi8(static_cast<int8_t>(other));
}

template<>
template<>
really_inline SuperVector<16>::SuperVector(int16_t const other)
{
    u.v128[0] = _mm_set1_epi16(other);
}

template<>
template<>
really_inline SuperVector<16>::SuperVector(uint16_t const other)
{
    u.v128[0] = _mm_set1_epi16(static_cast<int16_t>(other));
}

template<>
template<>
really_inline SuperVector<16>::SuperVector(int32_t const other)
{
    u.v128[0] = _mm_set1_epi32(other);
}

template<>
template<>
really_inline SuperVector<16>::SuperVector(uint32_t const other)
{
    u.v128[0] = _mm_set1_epi32(static_cast<int32_t>(other));
}

template<>
template<>
really_inline SuperVector<16>::SuperVector(int64_t const other)
{
    u.v128[0] = _mm_set1_epi64x(other);
}

template<>
template<>
really_inline SuperVector<16>::SuperVector(uint64_t const other)
{
    u.v128[0] = _mm_set1_epi64x(static_cast<int64_t>(other));
}

// Constants
template<>
really_inline SuperVector<16> SuperVector<16>::Ones()
{
    return {_mm_set1_epi8(0xFF)};
}

template<>
really_inline SuperVector<16> SuperVector<16>::Zeroes(void)
{
    return {_mm_set1_epi8(0)};
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
    return {_mm_and_si128(u.v128[0], b.u.v128[0])};
}

template <>
really_inline SuperVector<16> SuperVector<16>::operator|(SuperVector<16> const &b) const
{
    return {_mm_or_si128(u.v128[0], b.u.v128[0])};
}

template <>
really_inline SuperVector<16> SuperVector<16>::operator^(SuperVector<16> const &b) const
{
    return {_mm_xor_si128(u.v128[0], b.u.v128[0])};
}

template <>
really_inline SuperVector<16> SuperVector<16>::operator!() const
{
    return {_mm_xor_si128(u.v128[0], u.v128[0])};
}

template <>
really_inline SuperVector<16> SuperVector<16>::opandnot(SuperVector<16> const &b) const
{
    return {_mm_andnot_si128(u.v128[0], b.u.v128[0])};
}

template <>
really_inline SuperVector<16> SuperVector<16>::operator==(SuperVector<16> const &b) const
{
    return {_mm_cmpeq_epi8(u.v128[0], b.u.v128[0])};
}

template <>
really_inline SuperVector<16> SuperVector<16>::operator!=(SuperVector<16> const &b) const
{
    return !(*this == b);
}

template <>
really_inline SuperVector<16> SuperVector<16>::operator>(SuperVector<16> const &b) const
{
    return {_mm_cmpgt_epi8(u.v128[0], b.u.v128[0])};
}

template <>
really_inline SuperVector<16> SuperVector<16>::operator<(SuperVector<16> const &b) const
{
    return {_mm_cmplt_epi8(u.v128[0], b.u.v128[0])};
}

template <>
really_inline SuperVector<16> SuperVector<16>::operator>=(SuperVector<16> const &b) const
{
    return !(*this < b);
}

template <>
really_inline SuperVector<16> SuperVector<16>::operator<=(SuperVector<16> const &b) const
{
    return !(*this > b);
}

template <>
really_inline SuperVector<16> SuperVector<16>::eq(SuperVector<16> const &b) const
{
    return (*this == b);
}

template <>
really_inline typename SuperVector<16>::comparemask_type
SuperVector<16>::comparemask(void) const {
    return (u32)_mm_movemask_epi8(u.v128[0]);
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

// template <>
// template<uint8_t N>
// really_inline SuperVector<16> SuperVector<16>::vshl_8_imm() const
// {
//     const uint8_t i = N;
//     return {_mm_slli_epi8(u.v128[0], i)};
// }

template <>
template<uint8_t N>
really_inline SuperVector<16> SuperVector<16>::vshl_16_imm() const
{
    return {_mm_slli_epi16(u.v128[0], N)};
}

template <>
template<uint8_t N>
really_inline SuperVector<16> SuperVector<16>::vshl_32_imm() const
{
    return {_mm_slli_epi32(u.v128[0], N)};
}

template <>
template<uint8_t N>
really_inline SuperVector<16> SuperVector<16>::vshl_64_imm() const
{
    return {_mm_slli_epi64(u.v128[0], N)};
}

template <>
template<uint8_t N>
really_inline SuperVector<16> SuperVector<16>::vshl_128_imm() const
{
    return {_mm_slli_si128(u.v128[0], N)};
}

template <>
template<uint8_t N>
really_inline SuperVector<16> SuperVector<16>::vshl_imm() const
{
    return vshl_128_imm<N>();
}

// template <>
// template<uint8_t N>
// really_inline SuperVector<16> SuperVector<16>::vshr_8_imm() const
// {
//     return {_mm_srli_epi8(u.v128[0], N)};
// }

template <>
template<uint8_t N>
really_inline SuperVector<16> SuperVector<16>::vshr_16_imm() const
{
    return {_mm_srli_epi16(u.v128[0], N)};
}

template <>
template<uint8_t N>
really_inline SuperVector<16> SuperVector<16>::vshr_32_imm() const
{
    return {_mm_srli_epi32(u.v128[0], N)};
}
  
template <>
template<uint8_t N>
really_inline SuperVector<16> SuperVector<16>::vshr_64_imm() const
{
    return {_mm_srli_epi64(u.v128[0], N)};
}

template <>
template<uint8_t N>
really_inline SuperVector<16> SuperVector<16>::vshr_128_imm() const
{
    return {_mm_srli_si128(u.v128[0], N)};
}

template <>
template<uint8_t N>
really_inline SuperVector<16> SuperVector<16>::vshr_imm() const
{
    return vshr_128_imm<N>();
}

#if !defined(HS_OPTIMIZE)
template SuperVector<16> SuperVector<16>::vshl_16_imm<1>() const;
template SuperVector<16> SuperVector<16>::vshl_64_imm<1>() const;
template SuperVector<16> SuperVector<16>::vshl_64_imm<4>() const;
template SuperVector<16> SuperVector<16>::vshl_128_imm<1>() const;
template SuperVector<16> SuperVector<16>::vshl_128_imm<4>() const;
template SuperVector<16> SuperVector<16>::vshr_16_imm<1>() const;
template SuperVector<16> SuperVector<16>::vshr_64_imm<1>() const;
template SuperVector<16> SuperVector<16>::vshr_64_imm<4>() const;
template SuperVector<16> SuperVector<16>::vshr_128_imm<1>() const;
template SuperVector<16> SuperVector<16>::vshr_128_imm<4>() const;
#endif

// template <>
// really_inline SuperVector<16> SuperVector<16>::vshl_8  (uint8_t const N) const
// {
//     Unroller<0, 15>::iterator([&,v=this](int i) { if (N == i) return {_mm_slli_epi8(v->u.v128[0], i)}; });
//     if (N == 16) return Zeroes();
// }

template <>
really_inline SuperVector<16> SuperVector<16>::vshl_16 (uint8_t const N) const
{
#if defined(HAVE__BUILTIN_CONSTANT_P)
    if (__builtin_constant_p(N)) {
        return {_mm_slli_epi16(u.v128[0], N)};
    }
#endif
    if (N == 0) return *this;
    if (N == 16) return Zeroes();
    SuperVector result;
    Unroller<1, 16>::iterator([&,v=this](auto const i) { constexpr uint8_t n = i.value; if (N == n) result = {_mm_slli_epi16(v->u.v128[0], n)}; });
    return result;
}

template <>
really_inline SuperVector<16> SuperVector<16>::vshl_32 (uint8_t const N) const
{
#if defined(HAVE__BUILTIN_CONSTANT_P)
    if (__builtin_constant_p(N)) {
        return {_mm_slli_epi32(u.v128[0], N)};
    }
#endif
    if (N == 0) return *this;
    if (N == 16) return Zeroes();
    SuperVector result;
    Unroller<1, 16>::iterator([&,v=this](auto const i) { constexpr uint8_t n = i.value; if (N == n) result = {_mm_slli_epi32(v->u.v128[0], n)}; });
    return result;
}

template <>
really_inline SuperVector<16> SuperVector<16>::vshl_64 (uint8_t const N) const
{
#if defined(HAVE__BUILTIN_CONSTANT_P)
    if (__builtin_constant_p(N)) {
        return {_mm_slli_epi64(u.v128[0], N)};
    }
#endif
    if (N == 0) return *this;
    if (N == 16) return Zeroes();
    SuperVector result;
    Unroller<1, 16>::iterator([&,v=this](auto const i) { constexpr uint8_t n = i.value; if (N == n) result = {_mm_slli_epi64(v->u.v128[0], n)}; });
    return result;
}

template <>
really_inline SuperVector<16> SuperVector<16>::vshl_128(uint8_t const N) const
{
#if defined(HAVE__BUILTIN_CONSTANT_P)
    if (__builtin_constant_p(N)) {
        return {_mm_slli_si128(u.v128[0], N)};
    }
#endif
    if (N == 0) return *this;
    if (N == 16) return Zeroes();
    SuperVector result;
    Unroller<1, 16>::iterator([&,v=this](auto const i) { constexpr uint8_t n = i.value; if (N == n) result = {_mm_slli_si128(v->u.v128[0], n)}; });
    return result;
}

template <>
really_inline SuperVector<16> SuperVector<16>::vshl(uint8_t const N) const
{
    return vshl_128(N);
}

// template <>
// really_inline SuperVector<16> SuperVector<16>::vshr_8  (uint8_t const N) const
// {
//     SuperVector<16> result;
//     Unroller<0, 15>::iterator([&,v=this](uint8_t const i) { if (N == i) result = {_mm_srli_epi8(v->u.v128[0], i)}; });
//     if (N == 16) result = Zeroes();
//     return result;
// }

template <>
really_inline SuperVector<16> SuperVector<16>::vshr_16 (uint8_t const N) const
{
#if defined(HAVE__BUILTIN_CONSTANT_P)
    if (__builtin_constant_p(N)) {
        return {_mm_srli_epi16(u.v128[0], N)};
    }
#endif
    if (N == 0) return *this;
    if (N == 16) return Zeroes();
    SuperVector result;
    Unroller<1, 16>::iterator([&,v=this](auto const i) { constexpr uint8_t n = i.value; if (N == n) result = {_mm_srli_epi16(v->u.v128[0], n)}; });
    return result;
}

template <>
really_inline SuperVector<16> SuperVector<16>::vshr_32 (uint8_t const N) const
{
#if defined(HAVE__BUILTIN_CONSTANT_P)
    if (__builtin_constant_p(N)) {
        return {_mm_srli_epi32(u.v128[0], N)};
    }
#endif
    if (N == 0) return *this;
    if (N == 16) return Zeroes();
    SuperVector result;
    Unroller<1, 16>::iterator([&,v=this](auto const i) { constexpr uint8_t n = i.value; if (N == n) result = {_mm_srli_epi32(v->u.v128[0], n)}; });
    return result;
}

template <>
really_inline SuperVector<16> SuperVector<16>::vshr_64 (uint8_t const N) const
{
#if defined(HAVE__BUILTIN_CONSTANT_P)
    if (__builtin_constant_p(N)) {
        return {_mm_srli_epi64(u.v128[0], N)};
    }
#endif
    if (N == 0) return *this;
    if (N == 16) return Zeroes();
    SuperVector result;
    Unroller<1, 16>::iterator([&,v=this](auto const i) { constexpr uint8_t n = i.value; if (N == n) result = {_mm_srli_epi64(v->u.v128[0], n)}; });
    return result;
}

template <>
really_inline SuperVector<16> SuperVector<16>::vshr_128(uint8_t const N) const
{
#if defined(HAVE__BUILTIN_CONSTANT_P)
    if (__builtin_constant_p(N)) {
        return {_mm_srli_si128(u.v128[0], N)};
    }
#endif
    if (N == 0) return *this;
    if (N == 16) return Zeroes();
    SuperVector result;
    Unroller<1, 16>::iterator([&,v=this](auto const i) { constexpr uint8_t n = i.value; if (N == n) result = {_mm_srli_si128(v->u.v128[0], n)}; });
    return result;
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
    if (__builtin_constant_p(N)) {
        return {_mm_srli_si128(u.v128[0], N)};
    }
#endif
    return vshr_128(N);
}

template <>
really_inline SuperVector<16> SuperVector<16>::operator<<(uint8_t const N) const
{
#if defined(HAVE__BUILTIN_CONSTANT_P)
    if (__builtin_constant_p(N)) {
        return {_mm_slli_si128(u.v128[0], N)};
    }
#endif
    return vshl_128(N);
}

template<>
really_inline SuperVector<16> SuperVector<16>::Ones_vshr(uint8_t const N)
{
    if (N == 0) return Ones();
    else return Ones().vshr_128(N);
}

template<>
really_inline SuperVector<16> SuperVector<16>::Ones_vshl(uint8_t const N)
{
    if (N == 0) return Ones();
    else return Ones().vshr_128(N);
}

template <>
really_inline SuperVector<16> SuperVector<16>::loadu(void const *ptr)
{
    return _mm_loadu_si128((const m128 *)ptr);
}

template <>
really_inline SuperVector<16> SuperVector<16>::load(void const *ptr)
{
    assert(ISALIGNED_N(ptr, alignof(SuperVector::size)));
    ptr = assume_aligned(ptr, SuperVector::size);
    return _mm_load_si128((const m128 *)ptr);
}

template <>
really_inline SuperVector<16> SuperVector<16>::loadu_maskz(void const *ptr, uint8_t const len)
{
    SuperVector mask = Ones_vshr(16 -len);
    mask.print8("mask");
    SuperVector v = _mm_loadu_si128((const m128 *)ptr);
    v.print8("v");
    return mask & v;
}

template<>
really_inline SuperVector<16> SuperVector<16>::alignr(SuperVector<16> &other, int8_t offset)
{
#if defined(HAVE__BUILTIN_CONSTANT_P)
    if (__builtin_constant_p(offset)) {
        if (offset == 16) {
            return *this;
        } else {
            return {_mm_alignr_epi8(u.v128[0], other.u.v128[0], offset)};
        }
    }
#endif
    switch(offset) {
    case 0: return other; break;
    case 1: return {_mm_alignr_epi8(u.v128[0], other.u.v128[0], 1)}; break;
    case 2: return {_mm_alignr_epi8(u.v128[0], other.u.v128[0], 2)}; break;
    case 3: return {_mm_alignr_epi8(u.v128[0], other.u.v128[0], 3)}; break;
    case 4: return {_mm_alignr_epi8(u.v128[0], other.u.v128[0], 4)}; break;
    case 5: return {_mm_alignr_epi8(u.v128[0], other.u.v128[0], 5)}; break;
    case 6: return {_mm_alignr_epi8(u.v128[0], other.u.v128[0], 6)}; break;
    case 7: return {_mm_alignr_epi8(u.v128[0], other.u.v128[0], 7)}; break;
    case 8: return {_mm_alignr_epi8(u.v128[0], other.u.v128[0], 8)}; break;
    case 9: return {_mm_alignr_epi8(u.v128[0], other.u.v128[0], 9)}; break;
    case 10: return {_mm_alignr_epi8(u.v128[0], other.u.v128[0], 10)}; break;
    case 11: return {_mm_alignr_epi8(u.v128[0], other.u.v128[0], 11)}; break;
    case 12: return {_mm_alignr_epi8(u.v128[0], other.u.v128[0], 12)}; break;
    case 13: return {_mm_alignr_epi8(u.v128[0], other.u.v128[0], 13)}; break;
    case 14: return {_mm_alignr_epi8(u.v128[0], other.u.v128[0], 14)}; break;
    case 15: return {_mm_alignr_epi8(u.v128[0], other.u.v128[0], 15)}; break;
    default: break;
    }
    return *this;
}

template<>
template<>
really_inline SuperVector<16> SuperVector<16>::pshufb<true>(SuperVector<16> b)
{
    return {_mm_shuffle_epi8(u.v128[0], b.u.v128[0])};
}

template<>
really_inline SuperVector<16> SuperVector<16>::pshufb_maskz(SuperVector<16> b, uint8_t const len)
{
    SuperVector mask = Ones_vshr(16 -len);
    return mask & pshufb(b);
}

#endif // !defined(FAT_RUNTIME) && !defined(HAVE_AVX2)

// 256-bit AVX2 implementation
#if !(!defined(RELEASE_BUILD) && defined(FAT_RUNTIME) && defined(HAVE_AVX512)) && defined(HAVE_AVX2)

template<>
really_inline SuperVector<32>::SuperVector(SuperVector const &other)
{
    u.v256[0] = other.u.v256[0];
}

template<>
really_inline SuperVector<32>::SuperVector(typename base_type::type const v)
{
    u.v256[0] = v;
};

template<>
template<>
really_inline SuperVector<32>::SuperVector(m128 const v)
{
    u.v256[0] = _mm256_broadcastsi128_si256(v);
};

template<>
really_inline SuperVector<32>::SuperVector(m128 const lo, m128 const hi)
{
    u.v128[0] = lo;
    u.v128[1] = hi;
};

template<>
really_inline SuperVector<32>::SuperVector(SuperVector<16> const lo, SuperVector<16> const hi)
{
    u.v128[0] = lo.u.v128[0];
    u.v128[1] = hi.u.v128[0];
};

template<>
template<>
really_inline SuperVector<32>::SuperVector(int8_t const other)
{
    u.v256[0] = _mm256_set1_epi8(other);
}

template<>
template<>
really_inline SuperVector<32>::SuperVector(uint8_t const other)
{
    u.v256[0] = _mm256_set1_epi8(static_cast<int8_t>(other));
}

template<>
template<>
really_inline SuperVector<32>::SuperVector(int16_t const other)
{
    u.v256[0] = _mm256_set1_epi16(other);
}

template<>
template<>
really_inline SuperVector<32>::SuperVector(uint16_t const other)
{
    u.v256[0] = _mm256_set1_epi16(static_cast<int16_t>(other));
}

template<>
template<>
really_inline SuperVector<32>::SuperVector(int32_t const other)
{
    u.v256[0] = _mm256_set1_epi32(other);
}

template<>
template<>
really_inline SuperVector<32>::SuperVector(uint32_t const other)
{
    u.v256[0] = _mm256_set1_epi32(static_cast<int32_t>(other));
}

template<>
template<>
really_inline SuperVector<32>::SuperVector(int64_t const other)
{
    u.v256[0] = _mm256_set1_epi64x(other);
}

template<>
template<>
really_inline SuperVector<32>::SuperVector(uint64_t const other)
{
    u.v256[0] = _mm256_set1_epi64x(static_cast<int64_t>(other));
}

// Constants
template<>
really_inline SuperVector<32> SuperVector<32>::Ones(void)
{
    return {_mm256_set1_epi8(0xFF)};
}

template<>
really_inline SuperVector<32> SuperVector<32>::Zeroes(void)
{
    return {_mm256_set1_epi8(0)};
}

template <>
really_inline void SuperVector<32>::operator=(SuperVector<32> const &other)
{
    u.v256[0] = other.u.v256[0];
}

template <>
really_inline SuperVector<32> SuperVector<32>::operator&(SuperVector<32> const &b) const
{
    return {_mm256_and_si256(u.v256[0], b.u.v256[0])};
}

template <>
really_inline SuperVector<32> SuperVector<32>::operator|(SuperVector<32> const &b) const
{
    return {_mm256_or_si256(u.v256[0], b.u.v256[0])};
}

template <>
really_inline SuperVector<32> SuperVector<32>::operator^(SuperVector<32> const &b) const
{
    return {_mm256_xor_si256(u.v256[0], b.u.v256[0])};
}

template <>
really_inline SuperVector<32> SuperVector<32>::operator!() const
{
    return {_mm256_xor_si256(u.v256[0], u.v256[0])};
}

template <>
really_inline SuperVector<32> SuperVector<32>::opandnot(SuperVector<32> const &b) const
{
    return {_mm256_andnot_si256(u.v256[0], b.u.v256[0])};
}

template <>
really_inline SuperVector<32> SuperVector<32>::operator==(SuperVector<32> const &b) const
{
    return {_mm256_cmpeq_epi8(u.v256[0], b.u.v256[0])};
}

template <>
really_inline SuperVector<32> SuperVector<32>::operator!=(SuperVector<32> const &b) const
{
    return !(*this == b);
}

template <>
really_inline SuperVector<32> SuperVector<32>::operator>(SuperVector<32> const &b) const
{
    return {_mm256_cmpgt_epi8(u.v256[0], b.u.v256[0])};
}

template <>
really_inline SuperVector<32> SuperVector<32>::operator<(SuperVector<32> const &b) const
{
    return (b > *this);
}

template <>
really_inline SuperVector<32> SuperVector<32>::operator>=(SuperVector<32> const &b) const
{
    return !(*this < b);
}

template <>
really_inline SuperVector<32> SuperVector<32>::operator<=(SuperVector<32> const &b) const
{
    return !(*this > b);
}

template <>
really_inline SuperVector<32> SuperVector<32>::eq(SuperVector<32> const &b) const
{
    return (*this == b);
}

template <>
really_inline typename SuperVector<32>::comparemask_type
SuperVector<32>::comparemask(void) const {
    return (u32)_mm256_movemask_epi8(u.v256[0]);
}

template <>
really_inline typename SuperVector<32>::comparemask_type
SuperVector<32>::eqmask(SuperVector<32> const b) const {
    return eq(b).comparemask();
}

template <> really_inline u32 SuperVector<32>::mask_width() { return 1; }

template <>
really_inline typename SuperVector<32>::comparemask_type
SuperVector<32>::iteration_mask(
    typename SuperVector<32>::comparemask_type mask) {
    return mask;
}

// template <>
// template<uint8_t N>
// really_inline SuperVector<32> SuperVector<32>::vshl_8_imm() const
// {
//     const uint8_t i = N;
//     return {_mm256_slli_epi8(u.v256[0], i)};
// }

template <>
template<uint8_t N>
really_inline SuperVector<32> SuperVector<32>::vshl_16_imm() const
{
    return {_mm256_slli_epi16(u.v256[0], N)};
}

template <>
template<uint8_t N>
really_inline SuperVector<32> SuperVector<32>::vshl_32_imm() const
{
    return {_mm256_slli_epi32(u.v256[0], N)};
}

template <>
template<uint8_t N>
really_inline SuperVector<32> SuperVector<32>::vshl_64_imm() const
{
    return {_mm256_slli_epi64(u.v256[0], N)};
}

template <>
template<uint8_t N>
really_inline SuperVector<32> SuperVector<32>::vshl_128_imm() const
{
    return {_mm256_slli_si256(u.v256[0], N)};
}

template <>
template<uint8_t N>
really_inline SuperVector<32> SuperVector<32>::vshl_256_imm() const
{
    if (N == 0) return *this;
    if (N == 16) return {_mm256_permute2x128_si256(u.v256[0], u.v256[0], _MM_SHUFFLE(0, 0, 2, 0))};
    if (N == 32) return Zeroes();
    if (N < 16) {
        return {_mm256_alignr_epi8(u.v256[0], _mm256_permute2x128_si256(u.v256[0], u.v256[0], _MM_SHUFFLE(0, 0, 2, 0)), 16 - N)};
    } else {
        return {_mm256_slli_si256(_mm256_permute2x128_si256(u.v256[0], u.v256[0], _MM_SHUFFLE(0, 0, 2, 0)), N - 16)};
    }
}

template <>
template<uint8_t N>
really_inline SuperVector<32> SuperVector<32>::vshl_imm() const
{
    return vshl_256_imm<N>();
}

// template <>
// template<uint8_t N>
// really_inline SuperVector<32> SuperVector<32>::vshr_8_imm() const
// {
//     return {_mm256_srli_epi8(u.v256[0], N)};
// }

template <>
template<uint8_t N>
really_inline SuperVector<32> SuperVector<32>::vshr_16_imm() const
{
    return {_mm256_srli_epi16(u.v256[0], N)};
}

template <>
template<uint8_t N>
really_inline SuperVector<32> SuperVector<32>::vshr_32_imm() const
{
    return {_mm256_srli_epi32(u.v256[0], N)};
}
  
template <>
template<uint8_t N>
really_inline SuperVector<32> SuperVector<32>::vshr_64_imm() const
{
    return {_mm256_srli_epi64(u.v256[0], N)};
}

template <>
template<uint8_t N>
really_inline SuperVector<32> SuperVector<32>::vshr_128_imm() const
{
    return {_mm256_srli_si256(u.v256[0], N)};
}

template <>
template<uint8_t N>
really_inline SuperVector<32> SuperVector<32>::vshr_256_imm() const
{
    if (N == 0) return *this;
    if (N == 16) return {_mm256_permute2x128_si256(u.v256[0], u.v256[0], _MM_SHUFFLE(2, 0, 0, 1))};
    if (N == 32) return Zeroes();
    if (N < 16) {
        return {_mm256_alignr_epi8(u.v256[0], _mm256_permute2x128_si256(u.v256[0], u.v256[0], _MM_SHUFFLE(0, 0, 2, 0)), 16 - N)};
    } else {
        return {_mm256_srli_si256(_mm256_permute2x128_si256(u.v256[0], u.v256[0], _MM_SHUFFLE(2, 0, 0, 1)), N - 16)};
    }
}

template <>
template<uint8_t N>
really_inline SuperVector<32> SuperVector<32>::vshr_imm() const
{
    return vshr_256_imm<N>();
}

#if !defined(HS_OPTIMIZE)
template SuperVector<32> SuperVector<32>::vshl_16_imm<1>() const;
template SuperVector<32> SuperVector<32>::vshl_64_imm<1>() const;
template SuperVector<32> SuperVector<32>::vshl_64_imm<4>() const;
template SuperVector<32> SuperVector<32>::vshl_128_imm<1>() const;
template SuperVector<32> SuperVector<32>::vshl_128_imm<4>() const;
template SuperVector<32> SuperVector<32>::vshr_16_imm<1>() const;
template SuperVector<32> SuperVector<32>::vshr_64_imm<1>() const;
template SuperVector<32> SuperVector<32>::vshr_64_imm<4>() const;
template SuperVector<32> SuperVector<32>::vshr_128_imm<1>() const;
template SuperVector<32> SuperVector<32>::vshr_128_imm<4>() const;
template SuperVector<32> SuperVector<32>::vshr_256_imm<1>() const;
template SuperVector<32> SuperVector<32>::vshr_imm<1>() const;
#endif

// template <>
// really_inline SuperVector<16> SuperVector<16>::vshl_8  (uint8_t const N) const
// {
//     Unroller<0, 15>::iterator([&,v=this](int i) { if (N == i) return {_mm256_slli_epi8(v->u.v256[0], i)}; });
//     if (N == 16) return Zeroes();
// }

template <>
really_inline SuperVector<32> SuperVector<32>::vshl_16 (uint8_t const N) const
{
    if (N == 0) return *this;
    if (N == 32) return Zeroes();
    SuperVector result;
    Unroller<1, 32>::iterator([&,v=this](auto const i) { constexpr uint8_t n = i.value; if (N == n) result = {_mm256_slli_epi16(v->u.v256[0], n)}; });
    return result;
}

template <>
really_inline SuperVector<32> SuperVector<32>::vshl_32 (uint8_t const N) const
{
    if (N == 0) return *this;
    if (N == 32) return Zeroes();
    SuperVector result;
    Unroller<1, 32>::iterator([&,v=this](auto const i) { constexpr uint8_t n = i.value; if (N == n) result = {_mm256_slli_epi32(v->u.v256[0], n)}; });
    return result;
}

template <>
really_inline SuperVector<32> SuperVector<32>::vshl_64 (uint8_t const N) const
{
    if (N == 0) return *this;
    if (N == 32) return Zeroes();
    SuperVector result;
    Unroller<1, 32>::iterator([&,v=this](auto const i) { constexpr uint8_t n = i.value; if (N == n) result = {_mm256_slli_epi64(v->u.v256[0], n)}; });
    return result;
}

template <>
really_inline SuperVector<32> SuperVector<32>::vshl_128(uint8_t const N) const
{
    if (N == 0) return *this;
    if (N == 32) return Zeroes();
    SuperVector result;
    Unroller<1, 32>::iterator([&,v=this](auto const i) { constexpr uint8_t n = i.value; if (N == n) result = {_mm256_slli_si256(v->u.v256[0], n)}; });
    return result;
}

template <>
really_inline SuperVector<32> SuperVector<32>::vshl_256(uint8_t const N) const
{
    if (N == 0) return *this;
    if (N == 16) return {_mm256_permute2x128_si256(u.v256[0], u.v256[0], _MM_SHUFFLE(0, 0, 2, 0))};
    if (N == 32) return Zeroes();
    SuperVector result;
    Unroller<1, 16>::iterator([&,v=this](auto const i) {
        constexpr uint8_t n = i.value;
        if (N == n) result = {_mm256_alignr_epi8(u.v256[0], _mm256_permute2x128_si256(v->u.v256[0], v->u.v256[0], _MM_SHUFFLE(0, 0, 2, 0)), 16 - n)};;
    });
    Unroller<17, 32>::iterator([&,v=this](auto const i) {
        constexpr uint8_t n = i.value;
        if (N == n) result = {_mm256_slli_si256(_mm256_permute2x128_si256(v->u.v256[0], v->u.v256[0], _MM_SHUFFLE(0, 0, 2, 0)), n - 16)};
    });
    return result;
}

template <>
really_inline SuperVector<32> SuperVector<32>::vshl(uint8_t const N) const
{
    return vshl_256(N);
}

// template <>
// really_inline SuperVector<16> SuperVector<16>::vshr_8  (uint8_t const N) const
// {
//     SuperVector<16> result;
//     Unroller<0, 15>::iterator([&,v=this](uint8_t const i) { if (N == i) result = {_mm_srli_epi8(v->u.v128[0], i)}; });
//     if (N == 16) result = Zeroes();
//     return result;
// }

template <>
really_inline SuperVector<32> SuperVector<32>::vshr_16 (uint8_t const N) const
{
    if (N == 0) return *this;
    if (N == 32) return Zeroes();
    SuperVector result;
    Unroller<1, 32>::iterator([&,v=this](auto const i) { constexpr uint8_t n = i.value; if (N == n) result = {_mm256_srli_epi16(v->u.v256[0], n)}; });
    return result;
}

template <>
really_inline SuperVector<32> SuperVector<32>::vshr_32 (uint8_t const N) const
{
    if (N == 0) return *this;
    if (N == 32) return Zeroes();
    SuperVector result;
    Unroller<1, 32>::iterator([&,v=this](auto const i) { constexpr uint8_t n = i.value; if (N == n) result = {_mm256_srli_epi32(v->u.v256[0], n)}; });
    return result;
}

template <>
really_inline SuperVector<32> SuperVector<32>::vshr_64 (uint8_t const N) const
{
    if (N == 0) return *this;
    if (N == 32) return Zeroes();
    SuperVector result;
    Unroller<1, 32>::iterator([&,v=this](auto const i) { constexpr uint8_t n = i.value; if (N == n) result = {_mm256_srli_epi64(v->u.v256[0], n)}; });
    return result;
}

template <>
really_inline SuperVector<32> SuperVector<32>::vshr_128(uint8_t const N) const
{
    if (N == 0) return *this;
    if (N == 32) return Zeroes();
    SuperVector result;
    Unroller<1, 32>::iterator([&,v=this](auto const i) { constexpr uint8_t n = i.value; if (N == n) result = {_mm256_srli_si256(v->u.v256[0], n)}; });
    return result;
}

template <>
really_inline SuperVector<32> SuperVector<32>::vshr_256(uint8_t const N) const
{
    if (N == 0) return *this;
    if (N == 16) return {_mm256_permute2x128_si256(u.v256[0], u.v256[0], _MM_SHUFFLE(2, 0, 0, 1))};
    if (N == 32) return Zeroes();
    SuperVector result;
    Unroller<1, 16>::iterator([&,v=this](auto const i) {
        constexpr uint8_t n = i.value;
        if (N == n) result = {_mm256_alignr_epi8(_mm256_permute2x128_si256(v->u.v256[0], v->u.v256[0], _MM_SHUFFLE(2, 0, 0, 1)), v->u.v256[0], n)};
    });
    Unroller<17, 32>::iterator([&,v=this](auto const i) {
        constexpr uint8_t n = i.value;
        if (N == n) result = {_mm256_srli_si256(_mm256_permute2x128_si256(v->u.v256[0], v->u.v256[0], _MM_SHUFFLE(2, 0, 0, 1)), n - 16)};
    });
    return result;
}

template <>
really_inline SuperVector<32> SuperVector<32>::vshr(uint8_t const N) const
{
    return vshr_256(N);
}

template <>
really_inline SuperVector<32> SuperVector<32>::operator>>(uint8_t const N) const
{
#if defined(HAVE__BUILTIN_CONSTANT_P)
    if (__builtin_constant_p(N)) {
        // As found here: https://stackoverflow.com/questions/25248766/emulating-shifts-on-32-bytes-with-avx
        if (N < 16) {
            return {_mm256_alignr_epi8(_mm256_permute2x128_si256(u.v256[0], u.v256[0], _MM_SHUFFLE(2, 0, 0, 1)), u.v256[0], N)};
        } else if (N == 16) {
            return {_mm256_permute2x128_si256(u.v256[0], u.v256[0], _MM_SHUFFLE(2, 0, 0, 1))};
        } else {
            return {_mm256_srli_si256(_mm256_permute2x128_si256(u.v256[0], u.v256[0], _MM_SHUFFLE(2, 0, 0, 1)), N - 16)};
        }
    }
#endif
    return vshr_256(N);
}

template <>
really_inline SuperVector<32> SuperVector<32>::operator<<(uint8_t const N) const
{
#if defined(HAVE__BUILTIN_CONSTANT_P)
    if (__builtin_constant_p(N)) {
        // As found here: https://stackoverflow.com/questions/25248766/emulating-shifts-on-32-bytes-with-avx
        if (N < 16) {
            return {_mm256_alignr_epi8(u.v256[0], _mm256_permute2x128_si256(u.v256[0], u.v256[0], _MM_SHUFFLE(0, 0, 2, 0)), 16 - N)};
        } else if (N == 16) {
            return {_mm256_permute2x128_si256(u.v256[0], u.v256[0], _MM_SHUFFLE(0, 0, 2, 0))};
        } else {
            return {_mm256_slli_si256(_mm256_permute2x128_si256(u.v256[0], u.v256[0], _MM_SHUFFLE(0, 0, 2, 0)), N - 16)};
        }
    }
#endif
    return vshl_256(N);
}

template<>
really_inline SuperVector<32> SuperVector<32>::Ones_vshr(uint8_t const N)
{
    if (N == 0) return Ones();
    if (N >= 16)
        return {SuperVector<16>::Ones_vshr(N - 16), SuperVector<16>::Zeroes()};
    else
        return {SuperVector<16>::Ones(), SuperVector<16>::Ones_vshr(N)};
}

template<>
really_inline SuperVector<32> SuperVector<32>::Ones_vshl(uint8_t const N)
{
    if (N == 0) return Ones();
    if (N >= 16)
        return {SuperVector<16>::Zeroes(), SuperVector<16>::Ones_vshl(N - 16)};
    else
        return {SuperVector<16>::Ones_vshl(N), SuperVector<16>::Ones()};
}

template <>
really_inline SuperVector<32> SuperVector<32>::loadu(void const *ptr)
{
    return {_mm256_loadu_si256((const m256 *)ptr)};
}

template <>
really_inline SuperVector<32> SuperVector<32>::load(void const *ptr)
{
    assert(ISALIGNED_N(ptr, alignof(SuperVector::size)));
    ptr = assume_aligned(ptr, SuperVector::size);
    return {_mm256_load_si256((const m256 *)ptr)};
}

template <>
really_inline SuperVector<32> SuperVector<32>::loadu_maskz(void const *ptr, uint8_t const len)
{
#ifdef HAVE_AVX512
    u32 mask = (~0ULL) >> (32 - len);
    SuperVector<32> v = _mm256_mask_loadu_epi8(Zeroes().u.v256[0], mask, (const m256 *)ptr);
    v.print8("v");
    return v;
#else
    DEBUG_PRINTF("len = %d", len);
    SuperVector<32> mask = Ones_vshr(32 -len);
    mask.print8("mask");
    (Ones() >> (32 - len)).print8("mask");
    SuperVector<32> v = _mm256_loadu_si256((const m256 *)ptr);
    v.print8("v");
    return mask & v;
#endif
}

template<>
really_inline SuperVector<32> SuperVector<32>::alignr(SuperVector<32> &other, int8_t offset)
{
#if defined(HAVE__BUILTIN_CONSTANT_P)
    if (__builtin_constant_p(offset)) {
        if (offset == 16) {
            return *this;
        } else {
            return {_mm256_alignr_epi8(u.v256[0], other.u.v256[0], offset)};
        }
    }
#endif
    // As found here: https://stackoverflow.com/questions/8517970/mm-alignr-epi8-palignr-equivalent-in-avx2#8637458
    switch (offset){ 
    case 0 : return _mm256_set_m128i(_mm_alignr_epi8(u.v128[0], other.u.v128[1], 0), _mm_alignr_epi8(other.u.v128[1], other.u.v128[0], 0)); break;
    case 1 : return _mm256_set_m128i(_mm_alignr_epi8(u.v128[0], other.u.v128[1], 1), _mm_alignr_epi8(other.u.v128[1], other.u.v128[0], 1)); break;
    case 2 : return _mm256_set_m128i(_mm_alignr_epi8(u.v128[0], other.u.v128[1], 2), _mm_alignr_epi8(other.u.v128[1], other.u.v128[0], 2)); break;
    case 3 : return _mm256_set_m128i(_mm_alignr_epi8(u.v128[0], other.u.v128[1], 3), _mm_alignr_epi8(other.u.v128[1], other.u.v128[0], 3)); break;
    case 4 : return _mm256_set_m128i(_mm_alignr_epi8(u.v128[0], other.u.v128[1], 4), _mm_alignr_epi8(other.u.v128[1], other.u.v128[0], 4)); break;
    case 5 : return _mm256_set_m128i(_mm_alignr_epi8(u.v128[0], other.u.v128[1], 5), _mm_alignr_epi8(other.u.v128[1], other.u.v128[0], 5)); break;
    case 6 : return _mm256_set_m128i(_mm_alignr_epi8(u.v128[0], other.u.v128[1], 6), _mm_alignr_epi8(other.u.v128[1], other.u.v128[0], 6)); break;
    case 7 : return _mm256_set_m128i(_mm_alignr_epi8(u.v128[0], other.u.v128[1], 7), _mm_alignr_epi8(other.u.v128[1], other.u.v128[0], 7)); break;
    case 8 : return _mm256_set_m128i(_mm_alignr_epi8(u.v128[0], other.u.v128[1], 8), _mm_alignr_epi8(other.u.v128[1], other.u.v128[0], 8)); break;
    case 9 : return _mm256_set_m128i(_mm_alignr_epi8(u.v128[0], other.u.v128[1], 9), _mm_alignr_epi8(other.u.v128[1], other.u.v128[0], 9)); break;
    case 10 : return _mm256_set_m128i(_mm_alignr_epi8(u.v128[0], other.u.v128[1], 10), _mm_alignr_epi8(other.u.v128[1], other.u.v128[0], 10)); break;
    case 11 : return _mm256_set_m128i(_mm_alignr_epi8(u.v128[0], other.u.v128[1], 11), _mm_alignr_epi8(other.u.v128[1], other.u.v128[0], 11)); break;
    case 12 : return _mm256_set_m128i(_mm_alignr_epi8(u.v128[0], other.u.v128[1], 12), _mm_alignr_epi8(other.u.v128[1], other.u.v128[0], 12)); break;
    case 13 : return _mm256_set_m128i(_mm_alignr_epi8(u.v128[0], other.u.v128[1], 13), _mm_alignr_epi8(other.u.v128[1], other.u.v128[0], 13)); break;
    case 14 : return _mm256_set_m128i(_mm_alignr_epi8(u.v128[0], other.u.v128[1], 14), _mm_alignr_epi8(other.u.v128[1], other.u.v128[0], 14)); break;
    case 15 : return _mm256_set_m128i(_mm_alignr_epi8(u.v128[0], other.u.v128[1], 15), _mm_alignr_epi8(other.u.v128[1], other.u.v128[0], 15)); break;
    case 16 : return _mm256_set_m128i(_mm_alignr_epi8(u.v128[1], u.v128[0], 0), _mm_alignr_epi8(u.v128[0], other.u.v128[1], 0)); break;
    case 17 : return _mm256_set_m128i(_mm_alignr_epi8(u.v128[1], u.v128[0], 1), _mm_alignr_epi8(u.v128[0], other.u.v128[1], 1)); break;
    case 18 : return _mm256_set_m128i(_mm_alignr_epi8(u.v128[1], u.v128[0], 2), _mm_alignr_epi8(u.v128[0], other.u.v128[1], 2)); break;
    case 19 : return _mm256_set_m128i(_mm_alignr_epi8(u.v128[1], u.v128[0], 3), _mm_alignr_epi8(u.v128[0], other.u.v128[1], 3)); break;
    case 20 : return _mm256_set_m128i(_mm_alignr_epi8(u.v128[1], u.v128[0], 4), _mm_alignr_epi8(u.v128[0], other.u.v128[1], 4)); break;
    case 21 : return _mm256_set_m128i(_mm_alignr_epi8(u.v128[1], u.v128[0], 5), _mm_alignr_epi8(u.v128[0], other.u.v128[1], 5)); break;
    case 22 : return _mm256_set_m128i(_mm_alignr_epi8(u.v128[1], u.v128[0], 6), _mm_alignr_epi8(u.v128[0], other.u.v128[1], 6)); break;
    case 23 : return _mm256_set_m128i(_mm_alignr_epi8(u.v128[1], u.v128[0], 7), _mm_alignr_epi8(u.v128[0], other.u.v128[1], 7)); break;
    case 24 : return _mm256_set_m128i(_mm_alignr_epi8(u.v128[1], u.v128[0], 8), _mm_alignr_epi8(u.v128[0], other.u.v128[1], 8)); break;
    case 25 : return _mm256_set_m128i(_mm_alignr_epi8(u.v128[1], u.v128[0], 9), _mm_alignr_epi8(u.v128[0], other.u.v128[1], 9)); break;
    case 26 : return _mm256_set_m128i(_mm_alignr_epi8(u.v128[1], u.v128[0], 10), _mm_alignr_epi8(u.v128[0], other.u.v128[1], 10)); break;
    case 27 : return _mm256_set_m128i(_mm_alignr_epi8(u.v128[1], u.v128[0], 11), _mm_alignr_epi8(u.v128[0], other.u.v128[1], 11)); break;
    case 28 : return _mm256_set_m128i(_mm_alignr_epi8(u.v128[1], u.v128[0], 12), _mm_alignr_epi8(u.v128[0], other.u.v128[1], 12)); break;
    case 29 : return _mm256_set_m128i(_mm_alignr_epi8(u.v128[1], u.v128[0], 13), _mm_alignr_epi8(u.v128[0], other.u.v128[1], 13)); break;
    case 30 : return _mm256_set_m128i(_mm_alignr_epi8(u.v128[1], u.v128[0], 14), _mm_alignr_epi8(u.v128[0], other.u.v128[1], 14)); break;
    case 31 : return _mm256_set_m128i(_mm_alignr_epi8(u.v128[1], u.v128[0], 15), _mm_alignr_epi8(u.v128[0], other.u.v128[1], 15)); break;  
    default: break;
    }
    return *this;
}

template<>
template<>
really_inline SuperVector<32> SuperVector<32>::pshufb<true>(SuperVector<32> b)
{
    return {_mm256_shuffle_epi8(u.v256[0], b.u.v256[0])};
}

template<>
really_inline SuperVector<32> SuperVector<32>::pshufb_maskz(SuperVector<32> b, uint8_t const len)
{
    SuperVector<32> mask = Ones_vshr(32 -len);
    return mask & pshufb(b);
}

#endif // HAVE_AVX2


// 512-bit AVX512 implementation
#if defined(HAVE_AVX512)

template<>
really_inline SuperVector<64>::SuperVector(SuperVector const &o)
{
    u.v512[0] = o.u.v512[0];
}

template<>
really_inline SuperVector<64>::SuperVector(typename base_type::type const v)
{
    u.v512[0] = v;
};

template<>
template<>
really_inline SuperVector<64>::SuperVector(m256 const v)
{
    u.v512[0] = _mm512_broadcast_i64x4(v);
};

template<>
really_inline SuperVector<64>::SuperVector(m256 const lo, m256 const hi)
{
    u.v256[0] = lo;
    u.v256[1] = hi;
};

template<>
really_inline SuperVector<64>::SuperVector(SuperVector<32> const lo, SuperVector<32> const hi)
{
    u.v256[0] = lo.u.v256[0];
    u.v256[1] = hi.u.v256[0];
};

template<>
template<>
really_inline SuperVector<64>::SuperVector(m128 const v)
{
    u.v512[0] = _mm512_broadcast_i32x4(v);
};

template<>
template<>
really_inline SuperVector<64>::SuperVector(int8_t const o)
{
    u.v512[0] = _mm512_set1_epi8(o);
}

template<>
template<>
really_inline SuperVector<64>::SuperVector(uint8_t const o)
{
    u.v512[0] = _mm512_set1_epi8(static_cast<int8_t>(o));
}

template<>
template<>
really_inline SuperVector<64>::SuperVector(int16_t const o)
{
    u.v512[0] = _mm512_set1_epi16(o);
}

template<>
template<>
really_inline SuperVector<64>::SuperVector(uint16_t const o)
{
    u.v512[0] = _mm512_set1_epi16(static_cast<int16_t>(o));
}

template<>
template<>
really_inline SuperVector<64>::SuperVector(int32_t const o)
{
    u.v512[0] = _mm512_set1_epi32(o);
}

template<>
template<>
really_inline SuperVector<64>::SuperVector(uint32_t const o)
{
    u.v512[0] = _mm512_set1_epi32(static_cast<int32_t>(o));
}

template<>
template<>
really_inline SuperVector<64>::SuperVector(int64_t const o)
{
    u.v512[0] = _mm512_set1_epi64(o);
}

template<>
template<>
really_inline SuperVector<64>::SuperVector(uint64_t const o)
{
    u.v512[0] = _mm512_set1_epi64(static_cast<int64_t>(o));
}

// Constants
template<>
really_inline SuperVector<64> SuperVector<64>::Ones(void)
{
    return {_mm512_set1_epi8(0xFF)};
}

template<>
really_inline SuperVector<64> SuperVector<64>::Zeroes(void)
{
    return {_mm512_set1_epi8(0)};
}

// Methods
template <>
really_inline void SuperVector<64>::operator=(SuperVector<64> const &o)
{
    u.v512[0] = o.u.v512[0];
}

template <>
really_inline SuperVector<64> SuperVector<64>::operator&(SuperVector<64> const &b) const
{
    return {_mm512_and_si512(u.v512[0], b.u.v512[0])};
}

template <>
really_inline SuperVector<64> SuperVector<64>::operator|(SuperVector<64> const &b) const
{
    return {_mm512_or_si512(u.v512[0], b.u.v512[0])};
}

template <>
really_inline SuperVector<64> SuperVector<64>::operator^(SuperVector<64> const &b) const
{
    return {_mm512_xor_si512(u.v512[0], b.u.v512[0])};
}

template <>
really_inline SuperVector<64> SuperVector<64>::operator!() const
{
    return {_mm512_xor_si512(u.v512[0], u.v512[0])};
}

template <>
really_inline SuperVector<64> SuperVector<64>::opandnot(SuperVector<64> const &b) const
{
    return {_mm512_andnot_si512(u.v512[0], b.u.v512[0])};
}

template <>
really_inline SuperVector<64> SuperVector<64>::operator==(SuperVector<64> const &b) const
{
    SuperVector<64>::comparemask_type mask =
        _mm512_cmpeq_epi8_mask(u.v512[0], b.u.v512[0]);
    return {_mm512_movm_epi8(mask)};
}

template <>
really_inline SuperVector<64> SuperVector<64>::operator!=(SuperVector<64> const &b) const
{
    SuperVector<64>::comparemask_type mask =
        _mm512_cmpneq_epi8_mask(u.v512[0], b.u.v512[0]);
    return {_mm512_movm_epi8(mask)};
}

template <>
really_inline SuperVector<64> SuperVector<64>::operator>(SuperVector<64> const &b) const
{
    SuperVector<64>::comparemask_type mask =
        _mm512_cmpgt_epi8_mask(u.v512[0], b.u.v512[0]);
    return {_mm512_movm_epi8(mask)};
}

template <>
really_inline SuperVector<64> SuperVector<64>::operator<(SuperVector<64> const &b) const
{
    SuperVector<64>::comparemask_type mask =
        _mm512_cmplt_epi8_mask(u.v512[0], b.u.v512[0]);
    return {_mm512_movm_epi8(mask)};
}

template <>
really_inline SuperVector<64> SuperVector<64>::operator>=(SuperVector<64> const &b) const
{
    SuperVector<64>::comparemask_type mask =
        _mm512_cmpge_epi8_mask(u.v512[0], b.u.v512[0]);
    return {_mm512_movm_epi8(mask)};
}

template <>
really_inline SuperVector<64> SuperVector<64>::operator<=(SuperVector<64> const &b) const
{
    SuperVector<64>::comparemask_type mask =
        _mm512_cmple_epi8_mask(u.v512[0], b.u.v512[0]);
    return {_mm512_movm_epi8(mask)};
}

template <>
really_inline SuperVector<64> SuperVector<64>::eq(SuperVector<64> const &b) const
{
    return (*this == b);
}

template <>
really_inline typename SuperVector<64>::comparemask_type
SuperVector<64>::comparemask(void) const {
    __m512i msb = _mm512_set1_epi8(0xFF);
    __m512i mask = _mm512_and_si512(msb, u.v512[0]);
    return _mm512_cmpeq_epi8_mask(mask, msb);
}

template <>
really_inline typename SuperVector<64>::comparemask_type
SuperVector<64>::eqmask(SuperVector<64> const b) const {
    return _mm512_cmpeq_epi8_mask(u.v512[0], b.u.v512[0]);
}

template <> really_inline u32 SuperVector<64>::mask_width() { return 1; }

template <>
really_inline typename SuperVector<64>::comparemask_type
SuperVector<64>::iteration_mask(
    typename SuperVector<64>::comparemask_type mask) {
    return mask;
}

// template <>
// template<uint8_t N>
// really_inline SuperVector<16> SuperVector<16>::vshl_8_imm() const
// {
//     const uint8_t i = N;
//     return {_mm_slli_epi8(u.v128[0], i)};
// }

template <>
template<uint8_t N>
really_inline SuperVector<64> SuperVector<64>::vshl_16_imm() const
{
    return {_mm512_slli_epi16(u.v512[0], N)};
}

template <>
template<uint8_t N>
really_inline SuperVector<64> SuperVector<64>::vshl_32_imm() const
{
    return {_mm512_slli_epi32(u.v512[0], N)};
}

template <>
template<uint8_t N>
really_inline SuperVector<64> SuperVector<64>::vshl_64_imm() const
{
    return {_mm512_slli_epi64(u.v512[0], N)};
}

template <>
template<uint8_t N>
really_inline SuperVector<64> SuperVector<64>::vshl_128_imm() const
{
    return {_mm512_bslli_epi128(u.v512[0], N)};
}

template <>
template<uint8_t N>
really_inline SuperVector<64> SuperVector<64>::vshl_256_imm() const
{
    return {};
}

template <>
template<uint8_t N>
really_inline SuperVector<64> SuperVector<64>::vshl_512_imm() const
{
    return {};
}

template <>
template<uint8_t N>
really_inline SuperVector<64> SuperVector<64>::vshl_imm() const
{
    return vshl_512_imm<N>();
}

// template <>
// template<uint8_t N>
// really_inline SuperVector<64> SuperVector<64>::vshr_8_imm() const
// {
//     return {_mm_srli_epi8(u.v128[0], N)};
// }

template <>
template<uint8_t N>
really_inline SuperVector<64> SuperVector<64>::vshr_16_imm() const
{
    return {_mm512_srli_epi16(u.v512[0], N)};
}

template <>
template<uint8_t N>
really_inline SuperVector<64> SuperVector<64>::vshr_32_imm() const
{
    return {_mm512_srli_epi32(u.v512[0], N)};
}
  
template <>
template<uint8_t N>
really_inline SuperVector<64> SuperVector<64>::vshr_64_imm() const
{
    return {_mm512_srli_epi64(u.v512[0], N)};
}

template <>
template<uint8_t N>
really_inline SuperVector<64> SuperVector<64>::vshr_128_imm() const
{
    return {_mm512_bsrli_epi128(u.v512[0], N)};
}

template <>
template<uint8_t N>
really_inline SuperVector<64> SuperVector<64>::vshr_256_imm() const
{
    return {};
}

template <>
template<uint8_t N>
really_inline SuperVector<64> SuperVector<64>::vshr_512_imm() const
{
    return {};
}

template <>
template<uint8_t N>
really_inline SuperVector<64> SuperVector<64>::vshr_imm() const
{
    return vshr_512_imm<N>();
}

#if !defined(HS_OPTIMIZE)
template SuperVector<64> SuperVector<64>::vshl_16_imm<1>() const;
template SuperVector<64> SuperVector<64>::vshl_64_imm<1>() const;
template SuperVector<64> SuperVector<64>::vshl_64_imm<4>() const;
template SuperVector<64> SuperVector<64>::vshl_128_imm<1>() const;
template SuperVector<64> SuperVector<64>::vshl_128_imm<4>() const;
template SuperVector<64> SuperVector<64>::vshr_16_imm<1>() const;
template SuperVector<64> SuperVector<64>::vshr_64_imm<1>() const;
template SuperVector<64> SuperVector<64>::vshr_64_imm<4>() const;
template SuperVector<64> SuperVector<64>::vshr_128_imm<1>() const;
template SuperVector<64> SuperVector<64>::vshr_128_imm<4>() const;
#endif

// template <>
// really_inline SuperVector<64> SuperVector<64>::vshl_8  (uint8_t const N) const
// {
//     Unroller<0, 15>::iterator([&,v=this](int i) { if (N == i) return {_mm_slli_epi8(v->u.v128[0], i)}; });
//     if (N == 16) return Zeroes();
// }
template <>
really_inline SuperVector<64> SuperVector<64>::vshl_16 (uint8_t const N) const
{
    if (N == 0) return *this;
    if (N == 64) return Zeroes();
    SuperVector result;
    Unroller<1, 64>::iterator([&,v=this](auto const i) { constexpr uint8_t n = i.value; if (N == n) result = {_mm512_slli_epi16(v->u.v512[0], n)}; });
    return result;
}

template <>
really_inline SuperVector<64> SuperVector<64>::vshl_32 (uint8_t const N) const
{
    if (N == 0) return *this;
    if (N == 64) return Zeroes();
    SuperVector result;
    Unroller<1, 64>::iterator([&,v=this](auto const i) { constexpr uint8_t n = i.value; if (N == n) result = {_mm512_slli_epi32(v->u.v512[0], n)}; });
    return result;
}

template <>
really_inline SuperVector<64> SuperVector<64>::vshl_64 (uint8_t const N) const
{
    if (N == 0) return *this;
    if (N == 64) return Zeroes();
    SuperVector result;
    Unroller<1, 64>::iterator([&,v=this](auto const i) { constexpr uint8_t n = i.value; if (N == n) result = {_mm512_slli_epi64(v->u.v512[0], n)}; });
    return result;
}

template <>
really_inline SuperVector<64> SuperVector<64>::vshl_128(uint8_t const N) const
{
    if (N == 0) return *this;
    if (N == 64) return Zeroes();
    SuperVector result;
    Unroller<1, 64>::iterator([&,v=this](auto const i) { constexpr uint8_t n = i.value; if (N == n) result = {_mm512_bslli_epi128(v->u.v512[0], n)}; });
    return result;
}

template <>
really_inline SuperVector<64> SuperVector<64>::vshl_256(uint8_t const N) const
{
    return vshl_128(N);
}

template <>
really_inline SuperVector<64> SuperVector<64>::vshl_512(uint8_t const N) const
{
    return vshl_128(N);
}

template <>
really_inline SuperVector<64> SuperVector<64>::vshl(uint8_t const N) const
{
    return vshl_512(N);
}

// template <>
// really_inline SuperVector<16> SuperVector<16>::vshr_8  (uint8_t const N) const
// {
//     SuperVector<16> result;
//     Unroller<0, 15>::iterator([&,v=this](uint8_t const i) { if (N == i) result = {_mm_srli_epi8(v->u.v128[0], i)}; });
//     if (N == 16) result = Zeroes();
//     return result;
// }

template <>
really_inline SuperVector<64> SuperVector<64>::vshr_16 (uint8_t const N) const
{
    if (N == 0) return *this;
    if (N == 64) return Zeroes();
    SuperVector result;
    Unroller<1, 64>::iterator([&,v=this](auto const i) { constexpr uint8_t n = i.value; if (N == n) result = {_mm512_srli_epi16(v->u.v512[0], n)}; });
    return result;
}

template <>
really_inline SuperVector<64> SuperVector<64>::vshr_32 (uint8_t const N) const
{
    if (N == 0) return *this;
    if (N == 64) return Zeroes();
    SuperVector result;
    Unroller<1, 64>::iterator([&,v=this](auto const i) { constexpr uint8_t n = i.value; if (N == n) result = {_mm512_srli_epi32(v->u.v512[0], n)}; });
    return result;
}

template <>
really_inline SuperVector<64> SuperVector<64>::vshr_64 (uint8_t const N) const
{
    if (N == 0) return *this;
    if (N == 16) return Zeroes();
    SuperVector result;
    Unroller<1, 64>::iterator([&,v=this](auto const i) { constexpr uint8_t n = i.value; if (N == n) result = {_mm512_srli_epi64(v->u.v512[0], n)}; });
    return result;
}

template <>
really_inline SuperVector<64> SuperVector<64>::vshr_128(uint8_t const N) const
{
    if (N == 0) return *this;
    if (N == 64) return Zeroes();
    SuperVector result;
    Unroller<1, 64>::iterator([&,v=this](auto const i) { constexpr uint8_t n = i.value; if (N == n) result = {_mm512_bsrli_epi128(v->u.v512[0], n)}; });
    return result;
}

template <>
really_inline SuperVector<64> SuperVector<64>::vshr_256(uint8_t const N) const
{
    return vshr_128(N);
}

template <>
really_inline SuperVector<64> SuperVector<64>::vshr_512(uint8_t const N) const
{
    return vshr_128(N);
}

template <>
really_inline SuperVector<64> SuperVector<64>::vshr(uint8_t const N) const
{
    return vshr_512(N);
}

template<>
really_inline SuperVector<64> SuperVector<64>::Ones_vshr(uint8_t const N)
{
    if (N == 0) return Ones();
    if (N >= 32)
        return {SuperVector<32>::Ones_vshr(N - 32), SuperVector<32>::Zeroes()};
    else
        return {SuperVector<32>::Ones(), SuperVector<32>::Ones_vshr(N)};
}

template<>
really_inline SuperVector<64> SuperVector<64>::Ones_vshl(uint8_t const N)
{
    if (N == 0) return Ones();
    if (N >= 32)
        return {SuperVector<32>::Zeroes(), SuperVector<32>::Ones_vshl(N - 32)};
    else
        return {SuperVector<32>::Ones_vshl(N), SuperVector<32>::Ones()};
}

template <>
really_inline SuperVector<64> SuperVector<64>::operator>>(uint8_t const N) const
{
    if (N == 0) {
        return *this;
    } else if (N < 32) {
        SuperVector<32> lo256 = u.v256[0];
        SuperVector<32> hi256 = u.v256[1];
        SuperVector<32> carry = hi256 << (32 - N);
        hi256 = hi256 >> N;
        lo256 = (lo256 >> N) | carry;
        return SuperVector(lo256, hi256);
    } else if (N == 32) {
        SuperVector<32> hi256 = u.v256[1];
        return SuperVector(hi256, SuperVector<32>::Zeroes());
    } else if (N < 64) {
        SuperVector<32> hi256 = u.v256[1];
        return SuperVector(hi256 >> (N - 32), SuperVector<32>::Zeroes());
    } else {
        return Zeroes();
    }
}

template <>
really_inline SuperVector<64> SuperVector<64>::operator<<(uint8_t const N) const
{
    if (N == 0) {
        return *this;
    } else if (N < 32) {
        SuperVector<32> lo256 = u.v256[0];
        SuperVector<32> hi256 = u.v256[1];
        SuperVector<32> carry = lo256 >> (32 - N);
        hi256 = (hi256 << N) | carry;
        lo256 = lo256 << N;
        return SuperVector(lo256, hi256);
    } else if (N == 32) {
        SuperVector<32> lo256 = u.v256[0];
        return SuperVector(SuperVector<32>::Zeroes(), lo256);
    } else if (N < 64) {
        SuperVector<32> lo256 = u.v256[0];
        return SuperVector(SuperVector<32>::Zeroes(), lo256 << (N - 32));
    } else {
        return Zeroes();
    }
}

template <>
really_inline SuperVector<64> SuperVector<64>::loadu(void const *ptr)
{
    return {_mm512_loadu_si512((const m512 *)ptr)};
}

template <>
really_inline SuperVector<64> SuperVector<64>::load(void const *ptr)
{
    assert(ISALIGNED_N(ptr, alignof(SuperVector::size)));
    ptr = assume_aligned(ptr, SuperVector::size);
    return {_mm512_load_si512((const m512 *)ptr)};
}

template <>
really_inline SuperVector<64> SuperVector<64>::loadu_maskz(void const *ptr, uint8_t const len)
{
    u64a mask = (~0ULL) >> (64 - len);
    DEBUG_PRINTF("mask = %016llx\n", mask);
    SuperVector<64> v = _mm512_mask_loadu_epi8(Zeroes().u.v512[0], mask, (const m512 *)ptr);
    v.print8("v");
    return v;
}

template<>
template<>
really_inline SuperVector<64> SuperVector<64>::pshufb<true>(SuperVector<64> b)
{
    return {_mm512_shuffle_epi8(u.v512[0], b.u.v512[0])};
}

template<>
really_inline SuperVector<64> SuperVector<64>::pshufb_maskz(SuperVector<64> b, uint8_t const len)
{
    u64a mask = (~0ULL) >> (64 - len);
    DEBUG_PRINTF("mask = %016llx\n", mask);
    return {_mm512_maskz_shuffle_epi8(mask, u.v512[0], b.u.v512[0])};
}

template<>
really_inline SuperVector<64> SuperVector<64>::alignr(SuperVector<64> &l, int8_t offset)
{
#if defined(HAVE__BUILTIN_CONSTANT_P)
    if (__builtin_constant_p(offset)) {
        if (offset == 16) {
            return *this;
        } else {
            return {_mm512_alignr_epi8(u.v512[0], l.u.v512[0], offset)};
        }
    }
#endif
    if(offset == 0) {
        return *this;
    } else if (offset < 32){
        SuperVector<32> lo256 = u.v256[0];
        SuperVector<32> hi256 = u.v256[1];
        SuperVector<32> o_lo256 = l.u.v256[0];
        SuperVector<32> carry1 = hi256.alignr(lo256,offset);
        SuperVector<32> carry2 = o_lo256.alignr(hi256,offset);
        return SuperVector(carry1, carry2);
    } else if (offset <= 64){
        SuperVector<32> hi256 = u.v256[1];
        SuperVector<32> o_lo256 = l.u.v256[0];
        SuperVector<32> o_hi256 = l.u.v256[1];
        SuperVector<32> carry1 = o_lo256.alignr(hi256, offset - 32);
        SuperVector<32> carry2 = o_hi256.alignr(o_lo256,offset -32);
        return SuperVector(carry1, carry2);
    } else {
        return *this;
    }
}

#endif // HAVE_AVX512

#endif // SIMD_IMPL_HPP
