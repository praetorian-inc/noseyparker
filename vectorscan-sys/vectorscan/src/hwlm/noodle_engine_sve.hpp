/*
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

static really_inline
hwlm_error_t checkMatched(const struct noodTable *n, const u8 *buf, size_t len,
                          const struct cb_info *cbi, const u8 *d,
                          svbool_t matched, bool needsConfirm) {
    assert(d >= buf);
    size_t basePos = d - buf;
    svbool_t next_match = svpnext_b8(matched, svpfalse());
    do {
        svbool_t brk = svbrkb_z(svptrue_b8(), next_match);
        size_t matchPos = basePos + svcntp_b8(svptrue_b8(), brk);
        DEBUG_PRINTF("match pos %zu\n", matchPos);
        assert(matchPos < len);
        hwlmcb_rv_t rv = final(n, buf, len, needsConfirm, cbi, matchPos);
        RETURN_IF_TERMINATED(rv);
        next_match = svpnext_b8(matched, next_match);
    } while (unlikely(svptest_any(svptrue_b8(), next_match)));
    return HWLM_SUCCESS;
}

static really_inline
hwlm_error_t singleCheckMatched(const struct noodTable *n, const u8 *buf,
                                size_t len, const struct cb_info *cbi,
                                const u8 *d, svbool_t matched) {
    if (unlikely(svptest_any(svptrue_b8(), matched))) {
        hwlmcb_rv_t rv = checkMatched(n, buf, len, cbi, d, matched,
                                      n->msk_len != 1);
        RETURN_IF_TERMINATED(rv);
    }
    return HWLM_SUCCESS;
}

static really_inline
svbool_t singleMatched(svuint8_t chars, const u8 *d, svbool_t pg) {
    return svmatch(pg, svld1_u8(pg, d), chars);
}

static really_inline
hwlm_error_t scanSingleOnce(const struct noodTable *n, const u8 *buf,
                            size_t len, const struct cb_info *cbi,
                            svuint8_t chars, const u8 *d, const u8 *e) {
    DEBUG_PRINTF("start %p end %p\n", d, e);
    assert(d < e);
    assert(d >= buf);
    DEBUG_PRINTF("l = %td\n", e - d);
    svbool_t pg = svwhilelt_b8_s64(0, e - d);
    svbool_t matched = singleMatched(chars, d, pg);
    return singleCheckMatched(n, buf, len, cbi, d, matched);
}

static really_inline
hwlm_error_t scanSingleLoop(const struct noodTable *n, const u8 *buf,
                            size_t len, const struct cb_info *cbi,
                            svuint8_t chars, const u8 *d, const u8 *e) {
    assert(d < e);
    assert(d >= buf);
    size_t loops = (e - d) / svcntb();
    DEBUG_PRINTF("loops %zu \n", loops);
    assert(d + (loops * svcntb()) <= e);

    for (size_t i = 0; i < loops; i++, d += svcntb()) {
        DEBUG_PRINTF("d %p \n", d);
        svbool_t matched = singleMatched(chars, d, svptrue_b8());
        hwlmcb_rv_t rv = singleCheckMatched(n, buf, len, cbi, d, matched);
        RETURN_IF_TERMINATED(rv);
    }
    DEBUG_PRINTF("d %p e %p \n", d, e);
    return d == e ? HWLM_SUCCESS
                  : scanSingleOnce(n, buf, len, cbi, chars, d, e);
}

static really_inline
hwlm_error_t scanSingle(const struct noodTable *n, const u8 *buf, size_t len,
                        size_t offset, bool noCase, const struct cb_info *cbi) {
    if (!ourisalpha(n->key0)) {
        noCase = false; // force noCase off if we don't have an alphabetic char
    }

    size_t start = offset + n->msk_len - 1;
    const u8 *d = buf + start;
    const u8 *e = buf + len;
    DEBUG_PRINTF("start %p end %p \n", d, e);
    assert(d < e);
    assert(d >= buf);

    svuint8_t chars = getCharMaskSingle(n->key0, noCase);

    size_t scan_len = e - d;
    if (scan_len <= svcntb()) {
        return scanSingleOnce(n, buf, len, cbi, chars, d, e);
    }
    // peel off first part to align to the vector size
    const u8 *d1 = ROUNDUP_PTR(d, svcntb_pat(SV_POW2));
    if (d != d1) {
        DEBUG_PRINTF("until aligned %p \n", d1);
        hwlmcb_rv_t rv = scanSingleOnce(n, buf, len, cbi, chars, d, d1);
        RETURN_IF_TERMINATED(rv);
    }
    return scanSingleLoop(n, buf, len, cbi, chars, d1, e);
}

static really_inline
hwlm_error_t doubleCheckMatched(const struct noodTable *n, const u8 *buf,
                                size_t len, const struct cb_info *cbi,
                                const u8 *d, svbool_t matched,
                                svbool_t matched_rot, svbool_t any) {
    if (unlikely(svptest_any(svptrue_b8(), any))) {
        // Project predicate onto vector.
        svuint8_t matched_vec = svdup_u8_z(matched, 1);
        // Shift vector to right by one and project back to the predicate.
        matched = svcmpeq_n_u8(svptrue_b8(), svinsr_n_u8(matched_vec, 0), 1);
        matched = svorr_z(svptrue_b8(), matched, matched_rot);
        // d - 1 won't underflow as the first position in buf has been dealt
        // with meaning that d > buf
        assert(d > buf);
        hwlmcb_rv_t rv = checkMatched(n, buf, len, cbi, d - 1, matched,
                                      n->msk_len != 2);
        RETURN_IF_TERMINATED(rv);
    }
    return HWLM_SUCCESS;
}

static really_inline
svbool_t doubleMatched(svuint16_t chars, const u8 *d,
                       svbool_t pg, svbool_t pg_rot,
                       svbool_t * const matched, svbool_t * const matched_rot) {
    svuint16_t vec = svreinterpret_u16(svld1_u8(pg, d));
    // d - 1 won't underflow as the first position in buf has been dealt
    // with meaning that d > buf
    svuint16_t vec_rot = svreinterpret_u16(svld1_u8(pg_rot, d - 1));
    *matched = svmatch(pg, vec, chars);
    *matched_rot = svmatch(pg_rot, vec_rot, chars);
    return svorr_z(svptrue_b8(), *matched, *matched_rot);
}

static really_inline
hwlm_error_t scanDoubleOnce(const struct noodTable *n, const u8 *buf,
                            size_t len, const struct cb_info *cbi,
                            svuint8_t chars, const u8 *d, const u8 *e) {
    DEBUG_PRINTF("start %p end %p\n", d, e);
    assert(d < e);
    assert(d > buf);
    svbool_t pg = svwhilelt_b8_s64(0, e - d);
    svbool_t pg_rot = svwhilelt_b8_s64(0, e - d + 1);
    svbool_t matched, matched_rot;
    svbool_t any = doubleMatched(svreinterpret_u16(chars), d, pg, pg_rot, &matched, &matched_rot);
    return doubleCheckMatched(n, buf, len, cbi, d, matched, matched_rot, any);
}

static really_inline
hwlm_error_t scanDoubleLoop(const struct noodTable *n, const u8 *buf,
                            size_t len, const struct cb_info *cbi,
                            svuint8_t chars, const u8 *d, const u8 *e) {
    assert(d < e);
    assert(d > buf);
    size_t loops = (e - d) / svcntb();
    DEBUG_PRINTF("loops %zu \n", loops);
    assert(d + (loops * svcntb()) <= e);

    for (size_t i = 0; i < loops; i++, d += svcntb()) {
        DEBUG_PRINTF("d %p \n", d);
        svbool_t matched, matched_rot;
        svbool_t any = doubleMatched(svreinterpret_u16(chars), d, svptrue_b8(), svptrue_b8(),
                                     &matched, &matched_rot);
        hwlm_error_t rv = doubleCheckMatched(n, buf, len, cbi, d,
                                             matched, matched_rot, any);
        RETURN_IF_TERMINATED(rv);
    }
    DEBUG_PRINTF("d %p e %p \n", d, e);

    return d == e ? HWLM_SUCCESS
                  : scanDoubleOnce(n, buf, len, cbi, chars, d, e);
}

static really_inline
hwlm_error_t scanDouble(const struct noodTable *n, const u8 *buf, size_t len,
                        size_t offset, bool noCase, const struct cb_info *cbi) {
    // we stop scanning for the key-fragment when the rest of the key can't
    // possibly fit in the remaining buffer
    size_t end = len - n->key_offset + 2;

    size_t start = offset + n->msk_len - n->key_offset;

    const u8 *d = buf + start;
    const u8 *e = buf + end;
    DEBUG_PRINTF("start %p end %p \n", d, e);
    assert(d < e);
    assert(d >= buf);

    size_t scan_len = e - d;
    if (scan_len < 2) {
        return HWLM_SUCCESS;
    }
    ++d;

    svuint8_t chars = svreinterpret_u8(getCharMaskDouble(n->key0, n->key1, noCase));

    if (scan_len <= svcntb()) {
        return scanDoubleOnce(n, buf, len, cbi, chars, d, e);
    }
    // peel off first part to align to the vector size
    const u8 *d1 = ROUNDUP_PTR(d, svcntb_pat(SV_POW2));
    if (d != d1) {
        DEBUG_PRINTF("until aligned %p \n", d1);
        hwlmcb_rv_t rv = scanDoubleOnce(n, buf, len, cbi, chars,
                                        d, d1);
        RETURN_IF_TERMINATED(rv);
    }
    return scanDoubleLoop(n, buf, len, cbi, chars, d1, e);
}
