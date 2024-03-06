/*
 * Copyright (c) 2016-2017, Intel Corporation
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

#include "config.h"
#include "hs_common.h"
#include "ue2common.h"
#if defined(ARCH_IA32) || defined(ARCH_X86_64)
#include "util/arch/x86/cpuid_inline.h"
#elif defined(ARCH_AARCH64)
#include "util/arch/arm/cpuid_inline.h"
#endif

HS_PUBLIC_API
hs_error_t HS_CDECL hs_valid_platform(void) {
    /* Hyperscan requires SSSE3, anything else is a bonus */
#if defined(ARCH_IA32) || defined(ARCH_X86_64)
    if (check_ssse3()) {
        return HS_SUCCESS;
    } else {
        return HS_ARCH_ERROR;
    }
#elif defined(ARCH_ARM32) || defined(ARCH_AARCH64)
   if (check_neon()) {
        return HS_SUCCESS;
    } else {
        return HS_ARCH_ERROR;
    }
#elif defined(ARCH_PPC64EL)
    return HS_SUCCESS;    
#endif
}
