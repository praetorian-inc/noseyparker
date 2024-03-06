/*
 * Copyright (c) 2017-2020, Intel Corporation
 * Copyright (c) 2023, VectorCamp PC
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

#ifndef AARCH64_CPUID_INLINE_H_
#define AARCH64_CPUID_INLINE_H_

#if defined(__linux__)
#include <sys/auxv.h>
/* This is to help fix https://github.com/envoyproxy/envoy/pull/29881
 */
#if !defined(HWCAP2_SVE2)
#include <asm/hwcap.h>
#endif
#endif

#include "ue2common.h"
#include "util/arch/common/cpuid_flags.h"

static inline
int check_neon(void) {
    return 1;
}

#if defined(__linux__)
static inline
int check_sve(void) {
    unsigned long hwcap = getauxval(AT_HWCAP);
    if (hwcap & HWCAP_SVE) {
        return 1;
    }
    return 0;
}

static inline
int check_sve2(void) {
    unsigned long hwcap2 = getauxval(AT_HWCAP2);
    if (hwcap2 & HWCAP2_SVE2) {
        return 1;
    }
    return 0;
}
#else
static inline
int check_sve(void) {
    return 0;
}

static inline
int check_sve2(void) {
    return 0;
}
#endif

#endif // AARCH64_CPUID_INLINE_H_
