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

static really_inline
size_t countMatches(svuint8_t chars, svbool_t pg, const u8 *buf) {
    svuint8_t vec = svld1_u8(pg, buf);
    return svcntp_b8(svptrue_b8(), svmatch(pg, vec, chars));
}

static really_inline
bool countLoopBody(svuint8_t chars, svbool_t pg, const u8 *d,
                   u32 target_count, u32 *count_inout, const u8 **d_out) {
    *count_inout += countMatches(chars, pg, d);
    if (*count_inout >= target_count) {
        *d_out = d;
        return true;
    }
    return false;
}

static really_inline
bool countOnce(svuint8_t chars, const u8 *d, const u8 *d_end,
               u32 target_count, u32 *count_inout, const u8 **d_out) {
    assert(d <= d_end);
    svbool_t pg = svwhilelt_b8_s64(0, d_end - d);
    return countLoopBody(chars, pg, d, target_count, count_inout, d_out);
}

static really_inline
bool roseCountingMiracleScan(u8 c, const u8 *d, const u8 *d_end,
                             u32 target_count, u32 *count_inout,
                             const u8 **d_out) {
    assert(d <= d_end);
    svuint8_t chars = svdup_u8(c);
    size_t len = d_end - d;
    if (len <= svcntb()) {
        bool rv = countOnce(chars, d, d_end, target_count, count_inout, d_out);
        return rv;
    }
    // peel off first part to align to the vector size
    const u8 *aligned_d_end = ROUNDDOWN_PTR(d_end, svcntb_pat(SV_POW2));
    assert(d < aligned_d_end);
    if (d_end != aligned_d_end) {
        if (countOnce(chars, aligned_d_end, d_end,
                      target_count, count_inout, d_out)) return true;
        d_end = aligned_d_end;
    }
    size_t loops = (d_end - d) / svcntb();
    for (size_t i = 0; i < loops; i++) {
        d_end -= svcntb();
        if (countLoopBody(chars, svptrue_b8(), d_end,
                          target_count, count_inout, d_out)) return true;
    }
    if (d != d_end) {
        if (countOnce(chars, d, d_end,
                      target_count, count_inout, d_out)) return true;
    }
    return false;
}