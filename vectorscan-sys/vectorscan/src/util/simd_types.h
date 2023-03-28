/*
 * Copyright (c) 2015-2017, Intel Corporation
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

#ifndef SIMD_TYPES_H
#define SIMD_TYPES_H

#include "config.h"
#include "util/arch.h"
#include "util/intrinsics.h"
#include "ue2common.h"

#if defined(ARCH_IA32) || defined(ARCH_X86_64)
#include "util/arch/x86/simd_types.h"
#elif defined(ARCH_ARM32) || defined(ARCH_AARCH64)
#include "util/arch/arm/simd_types.h"
#elif defined(ARCH_PPC64EL)
#include "util/arch/ppc64el/simd_types.h"
#endif

#if !defined(m128) && !defined(HAVE_SIMD_128_BITS)
typedef struct ALIGN_DIRECTIVE {u64a hi; u64a lo;} m128;
#endif

#if !defined(m256) && !defined(HAVE_SIMD_256_BITS)
typedef struct ALIGN_AVX_DIRECTIVE {m128 lo; m128 hi;} m256;
#endif

typedef struct {m128 lo; m128 mid; m128 hi;} m384;

#if !defined(m512) && !defined(HAVE_SIMD_512_BITS)
typedef struct ALIGN_ATTR(64) {m256 lo; m256 hi;} m512;
#endif

#endif /* SIMD_TYPES_H */

