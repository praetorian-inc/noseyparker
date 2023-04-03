/*
 * Copyright (c) 2015-2017, Intel Corporation
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
 * \brief Large Bounded Repeat (LBR) engine for SVE: runtime code.
 */

static really_inline
char lbrRevScanVerm16(const struct NFA *nfa, const u8 *buf,
                      size_t begin, size_t end, size_t *loc) {
    assert(begin <= end);
    assert(nfa->type == LBR_NFA_VERM16);
    const struct lbr_verm16 *l = getImplNfa(nfa);

    if (begin == end) {
        return 0;
    }

    const u8 *ptr = rvermicelli16Exec(l->mask, buf + begin, buf + end);
    if (ptr == buf + begin - 1) {
        DEBUG_PRINTF("no escape found\n");
        return 0;
    }

    assert(loc);
    *loc = ptr - buf;
    DEBUG_PRINTF("escape found at offset %zu\n", *loc);
    return 1;
}

static really_inline
char lbrRevScanNVerm16(const struct NFA *nfa, const u8 *buf,
                       size_t begin, size_t end, size_t *loc) {
    assert(begin <= end);
    assert(nfa->type == LBR_NFA_NVERM16);
    const struct lbr_verm16 *l = getImplNfa(nfa);

    if (begin == end) {
        return 0;
    }

    const u8 *ptr = rnvermicelli16Exec(l->mask, buf + begin, buf + end);
    if (ptr == buf + begin - 1) {
        DEBUG_PRINTF("no escape found\n");
        return 0;
    }

    assert(loc);
    *loc = ptr - buf;
    DEBUG_PRINTF("escape found at offset %zu\n", *loc);
    return 1;
}

static really_inline
char lbrFwdScanVerm16(const struct NFA *nfa, const u8 *buf,
                      size_t begin, size_t end, size_t *loc) {
    assert(begin <= end);
    assert(nfa->type == LBR_NFA_VERM16);
    const struct lbr_verm16 *l = getImplNfa(nfa);

    if (begin == end) {
        return 0;
    }

    const u8 *ptr = vermicelli16Exec(l->mask, buf + begin, buf + end);
    if (ptr == buf + end) {
        DEBUG_PRINTF("no escape found\n");
        return 0;
    }

    assert(loc);
    *loc = ptr - buf;
    DEBUG_PRINTF("escape found at offset %zu\n", *loc);
    return 1;
}

static really_inline
char lbrFwdScanNVerm16(const struct NFA *nfa, const u8 *buf,
                       size_t begin, size_t end, size_t *loc) {
    assert(begin <= end);
    assert(nfa->type == LBR_NFA_NVERM16);
    const struct lbr_verm16 *l = getImplNfa(nfa);

    if (begin == end) {
        return 0;
    }

    const u8 *ptr = nvermicelli16Exec(l->mask, buf + begin, buf + end);
    if (ptr == buf + end) {
        DEBUG_PRINTF("no escape found\n");
        return 0;
    }

    assert(loc);
    *loc = ptr - buf;
    DEBUG_PRINTF("escape found at offset %zu\n", *loc);
    return 1;
}

#define ENGINE_ROOT_NAME Verm16
#include "lbr_common_impl.h"

#define ENGINE_ROOT_NAME NVerm16
#include "lbr_common_impl.h"