/*
 * Copyright (c) 2015-2016, Intel Corporation
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

#ifndef LBR_H
#define LBR_H

#include "ue2common.h"

struct mq;
struct NFA;

#ifdef __cplusplus
extern "C"
{
#endif

// LBR Dot

char nfaExecLbrDot_Q(const struct NFA *n, struct mq *q, s64a end);
char nfaExecLbrDot_Q2(const struct NFA *n, struct mq *q, s64a end);
char nfaExecLbrDot_QR(const struct NFA *n, struct mq *q, ReportID report);
char nfaExecLbrDot_reportCurrent(const struct NFA *n, struct mq *q);
char nfaExecLbrDot_inAccept(const struct NFA *n, ReportID report, struct mq *q);
char nfaExecLbrDot_inAnyAccept(const struct NFA *n, struct mq *q);
char nfaExecLbrDot_queueInitState(const struct NFA *n, struct mq *q);
char nfaExecLbrDot_initCompressedState(const struct NFA *n, u64a offset,
                                       void *state, u8 key);
char nfaExecLbrDot_queueCompressState(const struct NFA *nfa, const struct mq *q,
                                      s64a loc);
char nfaExecLbrDot_expandState(const struct NFA *nfa, void *dest,
                               const void *src, u64a offset, u8 key);

#define nfaExecLbrDot_testEOD NFA_API_NO_IMPL
#define nfaExecLbrDot_B_Reverse NFA_API_NO_IMPL
#define nfaExecLbrDot_zombie_status NFA_API_ZOMBIE_NO_IMPL

// LBR Verm

char nfaExecLbrVerm_Q(const struct NFA *n, struct mq *q, s64a end);
char nfaExecLbrVerm_Q2(const struct NFA *n, struct mq *q, s64a end);
char nfaExecLbrVerm_QR(const struct NFA *n, struct mq *q, ReportID report);
char nfaExecLbrVerm_reportCurrent(const struct NFA *n, struct mq *q);
char nfaExecLbrVerm_inAccept(const struct NFA *n, ReportID report,
                             struct mq *q);
char nfaExecLbrVerm_inAnyAccept(const struct NFA *n, struct mq *q);
char nfaExecLbrVerm_queueInitState(const struct NFA *n, struct mq *q);
char nfaExecLbrVerm_initCompressedState(const struct NFA *n, u64a offset,
                                        void *state, u8 key);
char nfaExecLbrVerm_queueCompressState(const struct NFA *nfa,
                                       const struct mq *q, s64a loc);
char nfaExecLbrVerm_expandState(const struct NFA *nfa, void *dest,
                                const void *src, u64a offset, u8 key);

#define nfaExecLbrVerm_testEOD NFA_API_NO_IMPL
#define nfaExecLbrVerm_B_Reverse NFA_API_NO_IMPL
#define nfaExecLbrVerm_zombie_status NFA_API_ZOMBIE_NO_IMPL

// LBR Negated Verm

char nfaExecLbrNVerm_Q(const struct NFA *n, struct mq *q, s64a end);
char nfaExecLbrNVerm_Q2(const struct NFA *n, struct mq *q, s64a end);
char nfaExecLbrNVerm_QR(const struct NFA *n, struct mq *q, ReportID report);
char nfaExecLbrNVerm_reportCurrent(const struct NFA *n, struct mq *q);
char nfaExecLbrNVerm_inAccept(const struct NFA *n, ReportID report,
                              struct mq *q);
char nfaExecLbrNVerm_inAnyAccept(const struct NFA *n, struct mq *q);
char nfaExecLbrNVerm_queueInitState(const struct NFA *n, struct mq *q);
char nfaExecLbrNVerm_initCompressedState(const struct NFA *n, u64a offset,
                                         void *state, u8 key);
char nfaExecLbrNVerm_queueCompressState(const struct NFA *nfa,
                                        const struct mq *q, s64a loc);
char nfaExecLbrNVerm_expandState(const struct NFA *nfa, void *dest,
                                 const void *src, u64a offset, u8 key);

#define nfaExecLbrNVerm_testEOD NFA_API_NO_IMPL
#define nfaExecLbrNVerm_B_Reverse NFA_API_NO_IMPL
#define nfaExecLbrNVerm_zombie_status NFA_API_ZOMBIE_NO_IMPL

#ifdef HAVE_SVE2

// LBR Verm16

char nfaExecLbrVerm16_Q(const struct NFA *n, struct mq *q, s64a end);
char nfaExecLbrVerm16_Q2(const struct NFA *n, struct mq *q, s64a end);
char nfaExecLbrVerm16_QR(const struct NFA *n, struct mq *q, ReportID report);
char nfaExecLbrVerm16_reportCurrent(const struct NFA *n, struct mq *q);
char nfaExecLbrVerm16_inAccept(const struct NFA *n, ReportID report,
                               struct mq *q);
char nfaExecLbrVerm16_inAnyAccept(const struct NFA *n, struct mq *q);
char nfaExecLbrVerm16_queueInitState(const struct NFA *n, struct mq *q);
char nfaExecLbrVerm16_initCompressedState(const struct NFA *n, u64a offset,
                                          void *state, u8 key);
char nfaExecLbrVerm16_queueCompressState(const struct NFA *nfa,
                                         const struct mq *q, s64a loc);
char nfaExecLbrVerm16_expandState(const struct NFA *nfa, void *dest,
                                  const void *src, u64a offset, u8 key);

#define nfaExecLbrVerm16_testEOD NFA_API_NO_IMPL
#define nfaExecLbrVerm16_B_Reverse NFA_API_NO_IMPL
#define nfaExecLbrVerm16_zombie_status NFA_API_ZOMBIE_NO_IMPL

// LBR Negated Verm16

char nfaExecLbrNVerm16_Q(const struct NFA *n, struct mq *q, s64a end);
char nfaExecLbrNVerm16_Q2(const struct NFA *n, struct mq *q, s64a end);
char nfaExecLbrNVerm16_QR(const struct NFA *n, struct mq *q, ReportID report);
char nfaExecLbrNVerm16_reportCurrent(const struct NFA *n, struct mq *q);
char nfaExecLbrNVerm16_inAccept(const struct NFA *n, ReportID report,
                                struct mq *q);
char nfaExecLbrNVerm16_inAnyAccept(const struct NFA *n, struct mq *q);
char nfaExecLbrNVerm16_queueInitState(const struct NFA *n, struct mq *q);
char nfaExecLbrNVerm16_initCompressedState(const struct NFA *n, u64a offset,
                                           void *state, u8 key);
char nfaExecLbrNVerm16_queueCompressState(const struct NFA *nfa,
                                          const struct mq *q, s64a loc);
char nfaExecLbrNVerm16_expandState(const struct NFA *nfa, void *dest,
                                   const void *src, u64a offset, u8 key);

#define nfaExecLbrNVerm16_testEOD NFA_API_NO_IMPL
#define nfaExecLbrNVerm16_B_Reverse NFA_API_NO_IMPL
#define nfaExecLbrNVerm16_zombie_status NFA_API_ZOMBIE_NO_IMPL

#endif // HAVE_SVE2

// LBR Shuf

char nfaExecLbrShuf_Q(const struct NFA *n, struct mq *q, s64a end);
char nfaExecLbrShuf_Q2(const struct NFA *n, struct mq *q, s64a end);
char nfaExecLbrShuf_QR(const struct NFA *n, struct mq *q, ReportID report);
char nfaExecLbrShuf_reportCurrent(const struct NFA *n, struct mq *q);
char nfaExecLbrShuf_inAccept(const struct NFA *n, ReportID report,
                             struct mq *q);
char nfaExecLbrShuf_inAnyAccept(const struct NFA *n, struct mq *q);
char nfaExecLbrShuf_queueInitState(const struct NFA *n, struct mq *q);
char nfaExecLbrShuf_initCompressedState(const struct NFA *n, u64a offset,
                                        void *state, u8 key);
char nfaExecLbrShuf_queueCompressState(const struct NFA *nfa,
                                       const struct mq *q, s64a loc);
char nfaExecLbrShuf_expandState(const struct NFA *nfa, void *dest,
                                const void *src, u64a offset, u8 key);

#define nfaExecLbrShuf_testEOD NFA_API_NO_IMPL
#define nfaExecLbrShuf_B_Reverse NFA_API_NO_IMPL
#define nfaExecLbrShuf_zombie_status NFA_API_ZOMBIE_NO_IMPL

// LBR Truffle

char nfaExecLbrTruf_Q(const struct NFA *n, struct mq *q, s64a end);
char nfaExecLbrTruf_Q2(const struct NFA *n, struct mq *q, s64a end);
char nfaExecLbrTruf_QR(const struct NFA *n, struct mq *q, ReportID report);
char nfaExecLbrTruf_reportCurrent(const struct NFA *n, struct mq *q);
char nfaExecLbrTruf_inAccept(const struct NFA *n, ReportID report,
                             struct mq *q);
char nfaExecLbrTruf_inAnyAccept(const struct NFA *n, struct mq *q);
char nfaExecLbrTruf_queueInitState(const struct NFA *n, struct mq *q);
char nfaExecLbrTruf_initCompressedState(const struct NFA *n, u64a offset,
                                        void *state, u8 key);
char nfaExecLbrTruf_queueCompressState(const struct NFA *nfa,
                                       const struct mq *q, s64a loc);
char nfaExecLbrTruf_expandState(const struct NFA *nfa, void *dest,
                                const void *src, u64a offset, u8 key);

#define nfaExecLbrTruf_testEOD NFA_API_NO_IMPL
#define nfaExecLbrTruf_B_Reverse NFA_API_NO_IMPL
#define nfaExecLbrTruf_zombie_status NFA_API_ZOMBIE_NO_IMPL

#ifdef __cplusplus
}
#endif

#endif
