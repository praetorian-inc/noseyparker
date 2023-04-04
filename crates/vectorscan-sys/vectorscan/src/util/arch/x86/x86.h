/*
 * Copyright (c) 2017-2020, Intel Corporation
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
 * \brief Per-platform architecture definitions
 */

#ifndef UTIL_ARCH_X86_H_
#define UTIL_ARCH_X86_H_

#if defined(__SSE2__) || defined(_M_X64) || (_M_IX86_FP >= 2)
#define HAVE_SSE2
#define HAVE_SIMD_128_BITS
#endif

#if defined(__SSE4_1__) || defined(__AVX__)
#define HAVE_SSE41
#define HAVE_SIMD_128_BITS
#endif

#if defined(__SSE4_2__) || defined(__AVX__)
#define HAVE_SSE42
#define HAVE_SIMD_128_BITS
#endif

#if defined(__AVX__) && defined(BUILD_AVX2)
#define HAVE_AVX
#define HAVE_SIMD_256_BITS
#endif

#if defined(__AVX2__) && defined(BUILD_AVX2)
#define HAVE_AVX2
#define HAVE_SIMD_256_BITS
#endif

#if defined(__AVX512BW__) && defined(BUILD_AVX512)
#define HAVE_AVX512
#define HAVE_SIMD_512_BITS
#endif

#if defined(__AVX512VBMI__) && defined(BUILD_AVX512)
#define HAVE_AVX512VBMI
#endif

#if defined(HAVE_SIMD_512_BITS)
#define CHUNKSIZE 512
#define VECTORSIZE 64
#elif defined(HAVE_SIMD_256_BITS)
#define CHUNKSIZE 256
#define VECTORSIZE 32
#elif defined(HAVE_SIMD_128_BITS)
#define CHUNKSIZE 128
#define VECTORSIZE 16
#endif

#if defined(__POPCNT__)
#define HAVE_POPCOUNT_INSTR
#endif

#if defined(__BMI__)
#define HAVE_BMI
#endif

#if defined(__BMI2__)
#define HAVE_BMI2
#endif

#endif // UTIL_ARCH_X86_H_
