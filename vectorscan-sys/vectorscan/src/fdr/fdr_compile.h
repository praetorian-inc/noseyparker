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

/** \file
 * \brief FDR literal matcher: build API.
 */

#ifndef FDR_COMPILE_H
#define FDR_COMPILE_H

#include "ue2common.h"
#include "hwlm/hwlm_build.h"
#include "util/bytecode_ptr.h"

#include <vector>

struct FDR;

namespace ue2 {

struct hwlmLiteral;
struct Grey;
struct target_t;

bytecode_ptr<FDR> fdrBuildTable(const HWLMProto &proto, const Grey &grey);

#if !defined(RELEASE_BUILD)
std::unique_ptr<HWLMProto> fdrBuildProtoHinted(
                                          u8 engType,
                                          std::vector<hwlmLiteral> lits,
                                          bool make_small, u32 hint,
                                          const target_t &target,
                                          const Grey &grey);
#endif

std::unique_ptr<HWLMProto> fdrBuildProto(
                                     u8 engType,
                                     std::vector<hwlmLiteral> lits,
                                     bool make_small, const target_t &target,
                                     const Grey &grey);

/** \brief Returns size in bytes of the given FDR engine. */
size_t fdrSize(const struct FDR *fdr);

} // namespace ue2

#endif
