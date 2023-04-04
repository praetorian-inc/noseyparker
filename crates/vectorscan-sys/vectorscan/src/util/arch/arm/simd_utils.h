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
 * \brief SIMD types and primitive operations.
 */

#ifndef ARCH_ARM_SIMD_UTILS_H
#define ARCH_ARM_SIMD_UTILS_H

#include <stdio.h>
#include <stdbool.h>

#include "ue2common.h"
#include "util/simd_types.h"
#include "util/unaligned.h"
#include "util/intrinsics.h"

#ifdef HAVE_SVE
#include "simd_utils_sve.h"
#endif

#ifdef HAVE_SVE2
#include "simd_utils_sve2.h"
#endif

#include <string.h> // for memcpy

static really_inline m128 ones128(void) {
    return (m128) vdupq_n_s8(0xFF);
}

static really_inline m128 zeroes128(void) {
    return (m128) vdupq_n_s32(0);
}

/** \brief Bitwise not for m128*/
static really_inline m128 not128(m128 a) {
    return (m128) vmvnq_s32(a);
}

/** \brief Return 1 if a and b are different otherwise 0 */
static really_inline int diff128(m128 a, m128 b) {
    uint64_t res = vget_lane_u64(
        (uint64x1_t)vshrn_n_u16((uint16x8_t)vceqq_s32(a, b), 4), 0);
    return (~0ull != res);
}

static really_inline int isnonzero128(m128 a) {
    return diff128(a, zeroes128());
}

/**
 * "Rich" version of diff128(). Takes two vectors a and b and returns a 4-bit
 * mask indicating which 32-bit words contain differences.
 */
static really_inline u32 diffrich128(m128 a, m128 b) {
    static const uint32x4_t movemask = { 1, 2, 4, 8 };
    return vaddvq_u32(vandq_u32(vmvnq_u32(vceqq_u32((uint32x4_t)a, (uint32x4_t)b)), movemask));
}

/**
 * "Rich" version of diff128(), 64-bit variant. Takes two vectors a and b and
 * returns a 4-bit mask indicating which 64-bit words contain differences.
 */
static really_inline u32 diffrich64_128(m128 a, m128 b) {
    static const uint64x2_t movemask = { 1, 4 };
    return (u32) vaddvq_u64(vandq_u64((uint64x2_t)vmvnq_u32((uint32x4_t)vceqq_u64((uint64x2_t)a, (uint64x2_t)b)), movemask));
}

static really_really_inline
m128 add_2x64(m128 a, m128 b) {
    return (m128) vaddq_u64((uint64x2_t)a, (uint64x2_t)b);
}

static really_really_inline
m128 sub_2x64(m128 a, m128 b) {
    return (m128) vsubq_u64((uint64x2_t)a, (uint64x2_t)b);
}

static really_inline
m128 lshift_m128(m128 a, unsigned b) {
#if defined(HAVE__BUILTIN_CONSTANT_P)
    if (__builtin_constant_p(b)) {
        return (m128) vshlq_n_u32((uint32x4_t)a, b);
    }
#endif
  int32x4_t shift_indices = vdupq_n_s32(b);
  return (m128) vshlq_s32(a, shift_indices);
}

static really_really_inline
m128 rshift_m128(m128 a, unsigned b) {
#if defined(HAVE__BUILTIN_CONSTANT_P)
    if (__builtin_constant_p(b)) {
        return (m128) vshrq_n_u32((uint32x4_t)a, b);
    }
#endif
  int32x4_t shift_indices = vdupq_n_s32(-b);
  return (m128) vshlq_s32(a, shift_indices);
}

static really_really_inline
m128 lshift64_m128(m128 a, unsigned b) {
#if defined(HAVE__BUILTIN_CONSTANT_P)
    if (__builtin_constant_p(b)) {
        return (m128) vshlq_n_u64((uint64x2_t)a, b);
    }
#endif
  int64x2_t shift_indices = vdupq_n_s64(b);
  return (m128) vshlq_s64((int64x2_t) a, shift_indices);
}

static really_really_inline
m128 rshift64_m128(m128 a, unsigned b) {
#if defined(HAVE__BUILTIN_CONSTANT_P)
    if (__builtin_constant_p(b)) {
        return (m128) vshrq_n_u64((uint64x2_t)a, b);
    }
#endif
  int64x2_t shift_indices = vdupq_n_s64(-b);
  return (m128) vshlq_s64((int64x2_t) a, shift_indices);
}

static really_inline m128 eq128(m128 a, m128 b) {
    return (m128) vceqq_u8((uint8x16_t)a, (uint8x16_t)b);
}

static really_inline m128 eq64_m128(m128 a, m128 b) {
    return (m128) vceqq_u64((uint64x2_t)a, (uint64x2_t)b);
}

static really_inline u32 movemask128(m128 a) {
    uint8x16_t input = vreinterpretq_u8_s32(a);
    uint16x8_t high_bits = vreinterpretq_u16_u8(vshrq_n_u8(input, 7));
    uint32x4_t paired16 =
        vreinterpretq_u32_u16(vsraq_n_u16(high_bits, high_bits, 7));
    uint64x2_t paired32 =
        vreinterpretq_u64_u32(vsraq_n_u32(paired16, paired16, 14));
    uint8x16_t paired64 =
        vreinterpretq_u8_u64(vsraq_n_u64(paired32, paired32, 28));
    return vgetq_lane_u8(paired64, 0) | ((int) vgetq_lane_u8(paired64, 8) << 8);
}

static really_inline m128 set1_16x8(u8 c) {
    return (m128) vdupq_n_u8(c);
}

static really_inline m128 set1_4x32(u32 c) {
    return (m128) vdupq_n_u32(c);
}

static really_inline m128 set1_2x64(u64a c) {
    return (m128) vdupq_n_u64(c);
}

static really_inline u32 movd(const m128 in) {
    return vgetq_lane_u32((uint32x4_t) in, 0);
}

static really_inline u64a movq(const m128 in) {
    return vgetq_lane_u64((uint64x2_t) in, 0);
}

/* another form of movq */
static really_inline
m128 load_m128_from_u64a(const u64a *p) {
    return (m128) vsetq_lane_u64(*p, (uint64x2_t) zeroes128(), 0);
}

static really_inline u32 extract32from128(const m128 in, unsigned imm) {
#if defined(HAVE__BUILTIN_CONSTANT_P)
    if (__builtin_constant_p(imm)) {
        return vgetq_lane_u32((uint32x4_t) in, imm);
    }
#endif
    switch (imm) {
    case 0:
        return vgetq_lane_u32((uint32x4_t) in, 0);
	break;
    case 1:
        return vgetq_lane_u32((uint32x4_t) in, 1);
	break;
    case 2:
        return vgetq_lane_u32((uint32x4_t) in, 2);
	break;
    case 3:
        return vgetq_lane_u32((uint32x4_t) in, 3);
	break;
    default:
	return 0;
	break;
    }
}

static really_inline u64a extract64from128(const m128 in, unsigned imm) {
#if defined(HAVE__BUILTIN_CONSTANT_P)
    if (__builtin_constant_p(imm)) {
        return vgetq_lane_u64((uint64x2_t) in, imm);
    }
#endif
    switch (imm) {
    case 0:
        return vgetq_lane_u64((uint64x2_t) in, 0);
	break;
    case 1:
        return vgetq_lane_u64((uint64x2_t) in, 1);
	break;
    default:
	return 0;
	break;
    }
}

static really_inline m128 low64from128(const m128 in) {
    return (m128) vcombine_u64(vget_low_u64((uint64x2_t)in), vdup_n_u64(0));
}

static really_inline m128 high64from128(const m128 in) {
    return (m128) vcombine_u64(vget_high_u64((uint64x2_t)in), vdup_n_u64(0));
}

static really_inline m128 add128(m128 a, m128 b) {
    return (m128) vaddq_u64((uint64x2_t)a, (uint64x2_t)b);
}

static really_inline m128 and128(m128 a, m128 b) {
    return (m128) vandq_s8((int8x16_t)a, (int8x16_t)b);
}

static really_inline m128 xor128(m128 a, m128 b) {
    return (m128) veorq_s8((int8x16_t)a, (int8x16_t)b);
}

static really_inline m128 or128(m128 a, m128 b) {
    return (m128) vorrq_s8((int8x16_t)a, (int8x16_t)b);
}

static really_inline m128 andnot128(m128 a, m128 b) {
    return (m128) vandq_s8( vmvnq_s8((int8x16_t) a), (int8x16_t) b);
}

// aligned load
static really_inline m128 load128(const void *ptr) {
    assert(ISALIGNED_N(ptr, alignof(m128)));
    return (m128) vld1q_s32((const int32_t *)ptr);
}

// aligned store
static really_inline void store128(void *ptr, m128 a) {
    assert(ISALIGNED_N(ptr, alignof(m128)));
    vst1q_s32((int32_t *)ptr, a);
}

// unaligned load
static really_inline m128 loadu128(const void *ptr) {
    return (m128) vld1q_s32((const int32_t *)ptr);
}

// unaligned store
static really_inline void storeu128(void *ptr, m128 a) {
    vst1q_s32((int32_t *)ptr, a);
}

// packed unaligned store of first N bytes
static really_inline
void storebytes128(void *ptr, m128 a, unsigned int n) {
    assert(n <= sizeof(a));
    memcpy(ptr, &a, n);
}

// packed unaligned load of first N bytes, pad with zero
static really_inline
m128 loadbytes128(const void *ptr, unsigned int n) {
    m128 a = zeroes128();
    assert(n <= sizeof(a));
    memcpy(&a, ptr, n);
    return a;
}

#define CASE_ALIGN_VECTORS(a, b, offset)  case offset: return (m128)vextq_s8((int8x16_t)(a), (int8x16_t)(b), (offset)); break;

static really_really_inline
m128 palignr_imm(m128 r, m128 l, int offset) {
    switch (offset) {
    case 0: return l; break;
    CASE_ALIGN_VECTORS(l, r, 1);
    CASE_ALIGN_VECTORS(l, r, 2);
    CASE_ALIGN_VECTORS(l, r, 3);
    CASE_ALIGN_VECTORS(l, r, 4);
    CASE_ALIGN_VECTORS(l, r, 5);
    CASE_ALIGN_VECTORS(l, r, 6);
    CASE_ALIGN_VECTORS(l, r, 7);
    CASE_ALIGN_VECTORS(l, r, 8);
    CASE_ALIGN_VECTORS(l, r, 9);
    CASE_ALIGN_VECTORS(l, r, 10);
    CASE_ALIGN_VECTORS(l, r, 11);
    CASE_ALIGN_VECTORS(l, r, 12);
    CASE_ALIGN_VECTORS(l, r, 13);
    CASE_ALIGN_VECTORS(l, r, 14);
    CASE_ALIGN_VECTORS(l, r, 15);
    case 16: return r; break;
    default:
	return zeroes128();
	break;
    }
}

static really_really_inline
m128 palignr(m128 r, m128 l, int offset) {
#if defined(HAVE__BUILTIN_CONSTANT_P)
    if (__builtin_constant_p(offset)) {
        return (m128)vextq_s8((int8x16_t)l, (int8x16_t)r, offset);
    }
#endif
    return palignr_imm(r, l, offset);
}
#undef CASE_ALIGN_VECTORS

static really_really_inline
m128 rshiftbyte_m128(m128 a, unsigned b) {
    if (b == 0) {
        return a;
    }
    return palignr(zeroes128(), a, b);
}

static really_really_inline
m128 lshiftbyte_m128(m128 a, unsigned b) {
    if (b == 0) {
        return a;
    }
    return palignr(a, zeroes128(), 16 - b);
}

static really_inline
m128 variable_byte_shift_m128(m128 in, s32 amount) {
    assert(amount >= -16 && amount <= 16);
    if (amount < 0) {
        return palignr_imm(zeroes128(), in, -amount);
    } else {
        return palignr_imm(in, zeroes128(), 16 - amount);
    }
}

static really_inline
m128 mask1bit128(unsigned int n) {
    assert(n < sizeof(m128) * 8);
    static m128 onebit = { 1, 0 };
    m128 mask = lshiftbyte_m128( onebit, n / 8 );
    return lshift64_m128( mask, n % 8 );
}

// switches on bit N in the given vector.
static really_inline
void setbit128(m128 *ptr, unsigned int n) {
    *ptr = or128(mask1bit128(n), *ptr);
}

// switches off bit N in the given vector.
static really_inline
void clearbit128(m128 *ptr, unsigned int n) {
    *ptr = andnot128(mask1bit128(n), *ptr);
}

// tests bit N in the given vector.
static really_inline
char testbit128(m128 val, unsigned int n) {
    const m128 mask = mask1bit128(n);

    return isnonzero128(and128(mask, val));
}

static really_inline
m128 pshufb_m128(m128 a, m128 b) {
    /* On Intel, if bit 0x80 is set, then result is zero, otherwise which the lane it is &0xf.
       In NEON, if >=16, then the result is zero, otherwise it is that lane.
       btranslated is the version that is converted from Intel to NEON.  */
    int8x16_t btranslated = vandq_s8((int8x16_t)b,vdupq_n_s8(0x8f));
    return (m128)vqtbl1q_s8((int8x16_t)a, (uint8x16_t)btranslated);
}

static really_inline
m128 max_u8_m128(m128 a, m128 b) {
    return (m128) vmaxq_u8((uint8x16_t)a, (uint8x16_t)b);
}

static really_inline
m128 min_u8_m128(m128 a, m128 b) {
    return (m128) vminq_u8((uint8x16_t)a, (uint8x16_t)b);
}

static really_inline
m128 sadd_u8_m128(m128 a, m128 b) {
    return (m128) vqaddq_u8((uint8x16_t)a, (uint8x16_t)b);
}

static really_inline
m128 sub_u8_m128(m128 a, m128 b) {
    return (m128) vsubq_u8((uint8x16_t)a, (uint8x16_t)b);
}

static really_inline
m128 set4x32(u32 x3, u32 x2, u32 x1, u32 x0) {
    uint32_t ALIGN_ATTR(16) data[4] = { x0, x1, x2, x3 };
    return (m128) vld1q_u32((uint32_t *) data);
}

static really_inline
m128 set2x64(u64a hi, u64a lo) {
    uint64_t ALIGN_ATTR(16) data[2] = { lo, hi };
    return (m128) vld1q_u64((uint64_t *) data);
}

#endif // ARCH_ARM_SIMD_UTILS_H
