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
 * \brief SIMD types and primitive operations.
 */

#ifndef ARCH_PPC64EL_SIMD_UTILS_H
#define ARCH_PPC64EL_SIMD_UTILS_H

#include <stdio.h>

#include "ue2common.h"
#include "util/simd_types.h"
#include "util/unaligned.h"
#include "util/intrinsics.h"

#include <string.h> // for memcpy

typedef __vector unsigned long long int  uint64x2_t;
typedef __vector   signed long long int   int64x2_t;
typedef __vector unsigned int            uint32x4_t;
typedef __vector   signed int             int32x4_t;
typedef __vector unsigned short int      uint16x8_t;
typedef __vector   signed short int       int16x8_t;
typedef __vector unsigned char           uint8x16_t;
typedef __vector  signed char             int8x16_t;

typedef unsigned long long int ulong64_t;
typedef   signed long long int  long64_t;

static really_inline m128 ones128(void) {
    return (m128) vec_splat_u8(-1);
}

static really_inline m128 zeroes128(void) {
    return (m128) vec_splat_s32(0);
}

/** \brief Bitwise not for m128*/
static really_inline m128 not128(m128 a) {
    //return (m128)vec_xor(a, a);
    return (m128) vec_xor(a,ones128());
}

/** \brief Return 1 if a and b are different otherwise 0 */
static really_inline int diff128(m128 a, m128 b) {
    return vec_any_ne(a, b);
}

static really_inline int isnonzero128(m128 a) {
    return !!diff128(a, zeroes128());
}

/**
 * "Rich" version of diff128(). Takes two vectors a and b and returns a 4-bit
 * mask indicating which 32-bit words contain differences.
 */
static really_inline u32 diffrich128(m128 a, m128 b) {
    static const m128 movemask = { 1, 2, 4, 8 };  
    m128 mask = (m128) vec_cmpeq(a, b); // _mm_cmpeq_epi32 (a, b);
    mask = vec_and(not128(mask), movemask);
    m128 sum = vec_sums(mask, zeroes128()); 
    return sum[3];
}

/**
 * "Rich" version of diff128(), 64-bit variant. Takes two vectors a and b and
 * returns a 4-bit mask indicating which 64-bit words contain differences.
 */
static really_inline u32 diffrich64_128(m128 a, m128 b) {
    static const uint64x2_t movemask = { 1, 4 };
    uint64x2_t mask = (uint64x2_t) vec_cmpeq((uint64x2_t)a, (uint64x2_t)b);
    mask = (uint64x2_t) vec_and((uint64x2_t)not128((m128)mask), movemask);
    m128 sum = vec_sums((m128)mask, zeroes128());
    return sum[3];
}

static really_really_inline
m128 add_2x64(m128 a, m128 b) {
    return (m128) vec_add((uint64x2_t)a, (uint64x2_t)b);
}

static really_really_inline
m128 sub_2x64(m128 a, m128 b) {
    return (m128) vec_sub((uint64x2_t)a, (uint64x2_t)b);
}

static really_really_inline
m128 lshift_m128(m128 a, unsigned b) {
    if (b == 0) return a;
    m128 sl = (m128) vec_splats((uint8_t) b << 3);
    m128 result = (m128) vec_slo((uint8x16_t) a, (uint8x16_t) sl);
    return result;
}

static really_really_inline
m128 rshift_m128(m128 a, unsigned b) {
    if (b == 0) return a;
    m128 sl = (m128) vec_splats((uint8_t) b << 3);
    m128 result = (m128) vec_sro((uint8x16_t) a, (uint8x16_t) sl);
    return result;
}

static really_really_inline
m128 lshift64_m128(m128 a, unsigned b) {
  uint64x2_t shift_indices = vec_splats((ulong64_t)b); 
  return (m128) vec_sl((int64x2_t)a, shift_indices);
}

static really_really_inline
m128 rshift64_m128(m128 a, unsigned  b) {
  uint64x2_t shift_indices = vec_splats((ulong64_t)b); 
  return (m128) vec_sr((int64x2_t)a, shift_indices);
}

static really_inline m128 eq128(m128 a, m128 b) {
   return (m128) vec_cmpeq((uint8x16_t)a, (uint8x16_t)b);
}

static really_inline m128 eq64_m128(m128 a, m128 b) {
   return (m128) vec_cmpeq((uint64x2_t)a, (uint64x2_t)b);
}

static really_inline u32 movemask128(m128 a) {
   static uint8x16_t perm = { 16, 24, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 };
   uint8x16_t bitmask = vec_gb((uint8x16_t) a);
   bitmask = (uint8x16_t) vec_perm(vec_splat_u8(0), bitmask, perm);
   u32 movemask;
   vec_ste((uint32x4_t) bitmask, 0, &movemask);
   return movemask;
}

static really_inline m128 set1_16x8(u8 c) {
    return (m128) vec_splats(c);
}

static really_inline m128 set1_4x32(u32 c) {
    return (m128) vec_splats(c);
}

static really_inline m128 set1_2x64(u64a c) {
    return (m128) vec_splats(c);
}

static really_inline u32 movd(const m128 in) {
   return (u32) vec_extract((uint32x4_t)in, 0);
}

static really_inline u64a movq(const m128 in) {
    u64a ALIGN_ATTR(16) a[2];
    vec_xst((uint64x2_t) in, 0, a);
    return a[0];  
}

/* another form of movq */
static really_inline
m128 load_m128_from_u64a(const u64a *p) {
    m128 vec =(m128) vec_splats(*p);
    return rshift_m128(vec,8);
}


static really_inline u32 extract32from128(const m128 in, unsigned imm) {
u32 ALIGN_ATTR(16) a[4];
vec_xst((uint32x4_t) in, 0, a);
switch (imm) {
    case 0:
        return a[0];break;
    case 1:
        return a[1];break;
    case 2:
        return a[2];break;
    case 3:
        return a[3];break;
    default:
	return 0;break;
    }
}

static really_inline u64a extract64from128(const m128 in, unsigned imm) {
u64a ALIGN_ATTR(16) a[2];
vec_xst((uint64x2_t) in, 0, a);
switch (imm) {
    case 0:
        return a[0];break;
    case 1:
        return a[1];break;
    default:
	return 0;
	break;
    }
}

static really_inline m128 low64from128(const m128 in) {
    return rshift_m128(in,8); 
}

static really_inline m128 high64from128(const m128 in) {
    return lshift_m128(in,8); 
}


static really_inline m128 add128(m128 a, m128 b) {
    return (m128) vec_add((uint64x2_t)a, (uint64x2_t)b);
}

static really_inline m128 and128(m128 a, m128 b) {
    return (m128) vec_and((int8x16_t)a, (int8x16_t)b);
}

static really_inline m128 xor128(m128 a, m128 b) {
    return (m128) vec_xor((int8x16_t)a, (int8x16_t)b);
}

static really_inline m128 or128(m128 a, m128 b) {
    return (m128) vec_or((int8x16_t)a, (int8x16_t)b);
}

static really_inline m128 andnot128(m128 a, m128 b) {
    return (m128) and128(not128(a),b);
}

// aligned load
static really_inline m128 load128(const void *ptr) {
    assert(ISALIGNED_N(ptr, alignof(m128)));
    return (m128) vec_xl(0, (const int32_t*)ptr);
}

// aligned store
static really_inline void store128(void *ptr, m128 a) { 	
    assert(ISALIGNED_N(ptr, alignof(m128)));
    vec_st(a, 0, (int32_t*)ptr);
}

// unaligned load
static really_inline m128 loadu128(const void *ptr) {
    return (m128) vec_xl(0, (const int32_t*)ptr);
}

// unaligned store
static really_inline void storeu128(void *ptr, m128 a) {
    vec_xst(a, 0, (int32_t*)ptr);
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

#define CASE_ALIGN_VECTORS(a, b, offset)  case offset: return (m128)vec_sld((int8x16_t)(b), (int8x16_t)(a), (16 - offset)); break;

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
    default: return zeroes128(); break;
    } 
}

static really_really_inline
m128 palignr(m128 r, m128 l, int offset) {
    if (offset == 0) return l;
    if (offset == 16) return r;
#if defined(HAVE__BUILTIN_CONSTANT_P)
    if (__builtin_constant_p(offset)) {
        return (m128)vec_sld((int8x16_t)(r), (int8x16_t)(l), 16 - offset);
    }
#endif
    m128 sl = (m128) vec_splats((uint8_t) (offset << 3));
    m128 sr = (m128) vec_splats((uint8_t) ((16 - offset) << 3));
    m128 rhs = (m128) vec_slo((uint8x16_t) r, (uint8x16_t) sr);
    m128 lhs = (m128) vec_sro((uint8x16_t) l, (uint8x16_t) sl);
    return or128(lhs, rhs);
}

#undef CASE_ALIGN_VECTORS

static really_really_inline
m128 rshiftbyte_m128(m128 a, unsigned b) {
    return palignr_imm(zeroes128(), a, b);
}

static really_really_inline
m128 lshiftbyte_m128(m128 a, unsigned b) {
    return palignr_imm(a, zeroes128(), 16 - b);
}

static really_inline
m128 variable_byte_shift_m128(m128 in, s32 amount) {
    assert(amount >= -16 && amount <= 16);
    if (amount < 0) {
        return rshiftbyte_m128(in, -amount);
    } else {
        return lshiftbyte_m128(in, amount);
    }
}

static really_inline
m128 mask1bit128(unsigned int n) {
    assert(n < sizeof(m128) * 8);
    static uint64x2_t onebit = { 1, 0 };
    m128 octets = (m128) vec_splats((uint8_t) ((n / 8) << 3));
    m128 bits = (m128) vec_splats((uint8_t) ((n % 8)));
    m128 mask = (m128) vec_slo((uint8x16_t) onebit, (uint8x16_t) octets);
    return (m128) vec_sll((uint8x16_t) mask, (uint8x16_t) bits);
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
       In NEON or PPC, if >=16, then the result is zero, otherwise it is that lane.
       below is the version that is converted from Intel to PPC.  */
    uint8x16_t mask =(uint8x16_t)vec_cmpge((uint8x16_t)b, (uint8x16_t)vec_splats((uint8_t)0x80));
    uint8x16_t res = vec_perm ((uint8x16_t)a, (uint8x16_t)a, (uint8x16_t)b);
    return (m128) vec_sel((uint8x16_t)res, (uint8x16_t)zeroes128(), (uint8x16_t)mask);
}

static really_inline
m128 max_u8_m128(m128 a, m128 b) {
    return (m128) vec_max((uint8x16_t)a, (uint8x16_t)b);
}

static really_inline
m128 min_u8_m128(m128 a, m128 b) {
    return (m128) vec_min((uint8x16_t)a, (uint8x16_t)b);
}

static really_inline
m128 sadd_u8_m128(m128 a, m128 b) {
    return (m128) vec_adds((uint8x16_t)a, (uint8x16_t)b);
}

static really_inline
m128 sub_u8_m128(m128 a, m128 b) {
    return (m128) vec_sub((uint8x16_t)a, (uint8x16_t)b);
}

static really_inline
m128 set4x32(u32 x3, u32 x2, u32 x1, u32  x0) {
    uint32x4_t v = { x0, x1, x2, x3 };
    return (m128) v;
}

static really_inline
m128 set2x64(u64a hi, u64a lo) {
    uint64x2_t v = { lo, hi };
    return (m128) v;
}

#endif // ARCH_PPC64EL_SIMD_UTILS_H
