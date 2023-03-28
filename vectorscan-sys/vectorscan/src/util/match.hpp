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

#ifndef MATCH_HPP
#define MATCH_HPP

#include "ue2common.h"
#include "util/arch.h"
#include "util/bitutils.h"
#include "util/unaligned.h"

#include "util/supervector/supervector.hpp"

template <u16 S>
const u8 *first_non_zero_match(const u8 *buf, SuperVector<S> v, u16 const len = S);

template <u16 S>
const u8 *last_non_zero_match(const u8 *buf, SuperVector<S> v, u16 const len = S);

template <u16 S>
const u8 *first_zero_match_inverted(const u8 *buf, SuperVector<S> v, u16 const len = S);

template <u16 S>
const u8 *last_zero_match_inverted(const u8 *buf, SuperVector<S> v, u16 len = S);

#if defined(ARCH_IA32) || defined(ARCH_X86_64)
#include "util/arch/x86/match.hpp"
#elif defined(ARCH_ARM32) || defined(ARCH_AARCH64)
#include "util/arch/arm/match.hpp"
#elif defined(ARCH_PPC64EL)
#include "util/arch/ppc64el/match.hpp"
#endif

#endif // MATCH_HPP
