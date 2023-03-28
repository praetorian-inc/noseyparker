/*
 * Copyright (c) 2015-2017, Intel Corporation
 * Copyright (c) 2020, 2021, VectorCamp PC
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
 * \brief Noodle literal matcher: runtime.
 */
#include "hwlm.h"
#include "noodle_engine.h"
#include "noodle_internal.h"
#include "scratch.h"
#include "ue2common.h"
#include "util/arch.h"
#include "util/bitutils.h"
#include "util/compare.h"
#include "util/intrinsics.h"
#include "util/join.h"
#include "util/partial_store.h"
#include "util/simd_utils.h"

#if defined(HAVE_AVX2)
#include "util/arch/x86/masked_move.h"
#endif

#include <ctype.h>
#include <stdbool.h>
#include <string.h>

/** \brief Noodle runtime context. */
struct cb_info {
    HWLMCallback cb; //!< callback function called on match
    u32 id; //!< ID to pass to callback on match
    struct hs_scratch *scratch; //!< scratch to pass to callback
    size_t offsetAdj; //!< used in streaming mode
};


#define RETURN_IF_TERMINATED(x)                                                \
    {                                                                          \
        if ((x) == HWLM_TERMINATED) {                                          \
            return HWLM_TERMINATED;                                            \
        }                                                                      \
    }

// Make sure the rest of the string is there. The single character scanner
// is used only for single chars with case insensitivity used correctly,
// so it can go straight to the callback if we get this far.
static really_inline
hwlm_error_t final(const struct noodTable *n, const u8 *buf, UNUSED size_t len,
                   bool needsConfirm, const struct cb_info *cbi, size_t pos) {
    u64a v{0};
    if (!needsConfirm) {
        goto match;
    }
    assert(len >= n->msk_len);
    v = partial_load_u64a(buf + pos + n->key_offset - n->msk_len, n->msk_len);
    DEBUG_PRINTF("v %016llx msk %016llx cmp %016llx\n", v, n->msk, n->cmp);
    if ((v & n->msk) != n->cmp) {
        /* mask didn't match */
        return HWLM_SUCCESS;
    }

match:
    pos -= cbi->offsetAdj;
    DEBUG_PRINTF("match @ %zu\n", pos + n->key_offset);
    hwlmcb_rv_t rv = cbi->cb(pos + n->key_offset - 1, cbi->id, cbi->scratch);
    if (rv == HWLM_TERMINATE_MATCHING) {
        return HWLM_TERMINATED;
    }
    return HWLM_SUCCESS;
}

#ifdef HAVE_SVE2
#include "noodle_engine_sve.hpp"
#else
#include "noodle_engine_simd.hpp"
#endif

// main entry point for the scan code
static really_inline
hwlm_error_t scan(const struct noodTable *n, const u8 *buf, size_t len,
                  size_t start, char single, bool noCase,
                  const struct cb_info *cbi) {
    if (len - start < n->msk_len) {
        // can't find string of length keyLen in a shorter buffer
        return HWLM_SUCCESS;
    }

    if (single) {
        return scanSingle(n, buf, len, start, noCase, cbi);
    } else {
        return scanDouble(n, buf, len, start, noCase, cbi);
    }
}

/** \brief Block-mode scanner. */
hwlm_error_t noodExec(const struct noodTable *n, const u8 *buf, size_t len,
                      size_t start, HWLMCallback cb,
                      struct hs_scratch *scratch) {
    assert(n && buf);

    struct cb_info cbi = {cb, n->id, scratch, 0};
    DEBUG_PRINTF("nood scan of %zu bytes for %*s @ %p\n", len, n->msk_len,
                 (const char *)&n->cmp, buf);

    return scan(n, buf, len, start, n->single, n->nocase, &cbi);
}

/** \brief Streaming-mode scanner. */
hwlm_error_t noodExecStreaming(const struct noodTable *n, const u8 *hbuf,
                               size_t hlen, const u8 *buf, size_t len,
                               HWLMCallback cb, struct hs_scratch *scratch) {
    assert(n);

    if (len + hlen < n->msk_len) {
        DEBUG_PRINTF("not enough bytes for a match\n");
        return HWLM_SUCCESS;
    }

    struct cb_info cbi = {cb, n->id, scratch, 0};
    DEBUG_PRINTF("nood scan of %zu bytes (%zu hlen) for %*s @ %p\n", len, hlen,
                 n->msk_len, (const char *)&n->cmp, buf);

    if (hlen && n->msk_len > 1) {
        /*
         * we have history, so build up a buffer from enough of the history
         * buffer plus what we've been given to scan. Since this is relatively
         * short, just check against msk+cmp per byte offset for matches.
         */
        assert(hbuf);
        u8 ALIGN_DIRECTIVE temp_buf[HWLM_LITERAL_MAX_LEN * 2];
        memset(temp_buf, 0, sizeof(temp_buf));

        assert(n->msk_len);
        size_t tl1 = MIN((size_t)n->msk_len - 1, hlen);
        size_t tl2 = MIN((size_t)n->msk_len - 1, len);

        assert(tl1 + tl2 <= sizeof(temp_buf));
        assert(tl1 + tl2 >= n->msk_len);
        assert(tl1 <= sizeof(u64a));
        assert(tl2 <= sizeof(u64a));
        DEBUG_PRINTF("using %zu bytes of hist and %zu bytes of buf\n", tl1, tl2);

        unaligned_store_u64a(temp_buf,
                             partial_load_u64a(hbuf + hlen - tl1, tl1));
        unaligned_store_u64a(temp_buf + tl1, partial_load_u64a(buf, tl2));

        for (size_t i = 0; i <= tl1 + tl2 - n->msk_len; i++) {
            u64a v = unaligned_load_u64a(temp_buf + i);
            if ((v & n->msk) == n->cmp) {
                size_t m_end = -tl1 + i + n->msk_len - 1;
                DEBUG_PRINTF("match @ %zu (i %zu)\n", m_end, i);
                hwlmcb_rv_t rv = cb(m_end, n->id, scratch);
                if (rv == HWLM_TERMINATE_MATCHING) {
                    return HWLM_TERMINATED;
                }
            }
        }
    }

    assert(buf);

    cbi.offsetAdj = 0;
    return scan(n, buf, len, 0, n->single, n->nocase, &cbi);
}
