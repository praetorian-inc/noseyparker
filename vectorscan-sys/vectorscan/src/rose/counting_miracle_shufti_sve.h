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
size_t countShuftiMatches(svuint8_t mask_lo, svuint8_t mask_hi,
                          const svbool_t pg, const u8 *buf) {
    svuint8_t vec = svld1_u8(pg, buf);
    svuint8_t c_lo = svtbl(mask_lo, svand_z(svptrue_b8(), vec, (uint8_t)0xf));
    svuint8_t c_hi = svtbl(mask_hi, svlsr_z(svptrue_b8(), vec, 4));
    svuint8_t t = svand_z(svptrue_b8(), c_lo, c_hi);
    return svcntp_b8(svptrue_b8(), svcmpne(pg, t, (uint8_t)0));
}

static really_inline
bool countShuftiLoopBody(svuint8_t mask_lo, svuint8_t mask_hi,
                         const svbool_t pg, const u8 *d, u32 target_count,
                         u32 *count_inout, const u8 **d_out) {
    *count_inout += countShuftiMatches(mask_lo, mask_hi, pg, d);
    if (*count_inout >= target_count) {
        *d_out = d;
        return true;
    }
    return false;
}

static really_inline
bool countShuftiOnce(svuint8_t mask_lo, svuint8_t mask_hi,
                     const u8 *d, const u8 *d_end, u32 target_count,
                     u32 *count_inout, const u8 **d_out) {
    svbool_t pg = svwhilelt_b8_s64(0, d_end - d);
    return countShuftiLoopBody(mask_lo, mask_hi, pg, d, target_count,
                               count_inout, d_out);
}

static really_inline
bool roseCountingMiracleScanShufti(svuint8_t mask_lo, svuint8_t mask_hi,
                                   UNUSED u8 poison, const u8 *d,
                                   const u8 *d_end, u32 target_count,
                                   u32 *count_inout, const u8 **d_out) {
    assert(d <= d_end);
    size_t len = d_end - d;
    if (len <= svcntb()) {
        char rv = countShuftiOnce(mask_lo, mask_hi, d, d_end, target_count,
                                  count_inout, d_out);
        return rv;
    }
    // peel off first part to align to the vector size
    const u8 *aligned_d_end = ROUNDDOWN_PTR(d_end, svcntb_pat(SV_POW2));
    assert(d < aligned_d_end);
    if (d_end != aligned_d_end) {
        if (countShuftiOnce(mask_lo, mask_hi, aligned_d_end, d_end,
                            target_count, count_inout, d_out)) return true;
        d_end = aligned_d_end;
    }
    size_t loops = (d_end - d) / svcntb();
    for (size_t i = 0; i < loops; i++) {
        d_end -= svcntb();
        if (countShuftiLoopBody(mask_lo, mask_hi, svptrue_b8(), d_end,
                                target_count, count_inout, d_out)) return true;
    }
    if (d != d_end) {
        if (countShuftiOnce(mask_lo, mask_hi, d, d_end,
                            target_count, count_inout, d_out)) return true;
    }
    return false;
}