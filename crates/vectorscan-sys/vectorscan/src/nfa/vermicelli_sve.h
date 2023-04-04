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

/** \file
 * \brief Vermicelli: AArch64 SVE implementation.
 *
 * (users should include vermicelli.h instead of this)
 */

static really_inline
int dvermSearchGetOffset(svbool_t matched, svbool_t matched_rot) {
    int offset = accelSearchGetOffset(matched);
    int offset_rot = accelSearchGetOffset(matched_rot) - 1;
    return (offset_rot < offset) ? offset_rot : offset;
}

static really_inline
uint64_t rdvermSearchGetSingleOffset(svbool_t matched) {
    return svcntp_b8(svptrue_b8(), svbrkb_z(svptrue_b8(), svrev_b8(matched)));
}

static really_inline
uint64_t rdvermSearchGetOffset(svbool_t matched, svbool_t matched_rot) {
    uint64_t offset = rdvermSearchGetSingleOffset(matched);
    uint64_t offset_rot = rdvermSearchGetSingleOffset(matched_rot) - 1;
    return (offset_rot < offset) ? offset_rot : offset;
}

static really_inline
const u8 *dvermSearchCheckMatched(const u8 *buf, svbool_t matched,
                                  svbool_t matched_rot, svbool_t any) {
    if (unlikely(svptest_any(svptrue_b8(), any))) {
        const u8 *matchPos = buf + dvermSearchGetOffset(matched, matched_rot);
        DEBUG_PRINTF("match pos %p\n", matchPos);
        return matchPos;
    }
    return NULL;
}

static really_inline
const u8 *rdvermSearchCheckMatched(const u8 *buf, svbool_t matched,
                                   svbool_t matched_rot, svbool_t any) {
    if (unlikely(svptest_any(svptrue_b8(), any))) {
        const u8 *matchPos = buf + (svcntb() -
                                rdvermSearchGetOffset(matched, matched_rot));
        DEBUG_PRINTF("match pos %p\n", matchPos);
        return matchPos;
    }
    return NULL;
}

static really_inline
svbool_t singleMatched(svuint8_t chars, const u8 *buf, svbool_t pg,
                       bool negate, const int64_t vnum) {
    svuint8_t vec = svld1_vnum_u8(pg, buf, vnum);
    if (negate) {
        return svnmatch(pg, vec, chars);
    } else {
        return svmatch(pg, vec, chars);
    }
}

static really_inline
svbool_t doubleMatched(svuint16_t chars, const u8 *buf, const u8 *buf_rot,
                       svbool_t pg, svbool_t pg_rot, svbool_t * const matched,
                       svbool_t * const matched_rot) {
    svuint16_t vec = svreinterpret_u16(svld1_u8(pg, buf));
    svuint16_t vec_rot = svreinterpret_u16(svld1_u8(pg_rot, buf_rot));
    *matched = svmatch(pg, vec, chars);
    *matched_rot = svmatch(pg_rot, vec_rot, chars);
    return svorr_z(svptrue_b8(), *matched, *matched_rot);
}

static really_inline
const u8 *vermSearchOnce(svuint8_t chars, const u8 *buf, const u8 *buf_end,
                         bool negate) {
    DEBUG_PRINTF("start %p end %p\n", buf, buf_end);
    assert(buf <= buf_end);
    DEBUG_PRINTF("l = %td\n", buf_end - buf);
    svbool_t pg = svwhilelt_b8_s64(0, buf_end - buf);
    svbool_t matched = singleMatched(chars, buf, pg, negate, 0);
    return accelSearchCheckMatched(buf, matched);
}

static really_inline
const u8 *vermSearchLoopBody(svuint8_t chars, const u8 *buf, bool negate) {
    DEBUG_PRINTF("start %p end %p\n", buf, buf + svcntb());
    svbool_t matched = singleMatched(chars, buf, svptrue_b8(), negate, 0);
    return accelSearchCheckMatched(buf, matched);
}

static really_inline
const u8 *vermSearchLoopBodyUnrolled(svuint8_t chars, const u8 *buf,
                                     bool negate) {
    DEBUG_PRINTF("start %p end %p\n", buf, buf + (2 * svcntb()));
    svbool_t matched0 = singleMatched(chars, buf, svptrue_b8(), negate, 0);
    svbool_t matched1 = singleMatched(chars, buf, svptrue_b8(), negate, 1);
    svbool_t any = svorr_z(svptrue_b8(), matched0, matched1);
    if (unlikely(svptest_any(svptrue_b8(), any))) {
        if (svptest_any(svptrue_b8(), matched0)) {
            return buf + accelSearchGetOffset(matched0);
        } else {
            return buf + svcntb() + accelSearchGetOffset(matched1);
        }
    }
    return NULL;
}

static really_inline
const u8 *rvermSearchOnce(svuint8_t chars, const u8 *buf, const u8 *buf_end,
                          bool negate) {
    DEBUG_PRINTF("start %p end %p\n", buf, buf_end);
    assert(buf <= buf_end);
    DEBUG_PRINTF("l = %td\n", buf_end - buf);
    svbool_t pg = svwhilelt_b8_s64(0, buf_end - buf);
    svbool_t matched = singleMatched(chars, buf, pg, negate, 0);
    return accelRevSearchCheckMatched(buf, matched);
}

static really_inline
const u8 *rvermSearchLoopBody(svuint8_t chars, const u8 *buf, bool negate) {
    DEBUG_PRINTF("start %p end %p\n", buf, buf + svcntb());
    svbool_t matched = singleMatched(chars, buf, svptrue_b8(), negate, 0);
    return accelRevSearchCheckMatched(buf, matched);
}

static really_inline
const u8 *dvermSearchOnce(svuint16_t chars, const u8 *buf, const u8 *buf_end) {
    DEBUG_PRINTF("start %p end %p\n", buf, buf_end);
    assert(buf < buf_end);
    DEBUG_PRINTF("l = %td\n", buf_end - buf);
    svbool_t pg = svwhilelt_b8_s64(0, buf_end - buf);
    svbool_t pg_rot = svwhilele_b8_s64(0, buf_end - buf);
    svbool_t matched, matched_rot;
    // buf - 1 won't underflow as the first position in the buffer has been
    // dealt with meaning that buf - 1 is within the buffer.
    svbool_t any = doubleMatched(chars, buf, buf - 1, pg, pg_rot,
                                 &matched, &matched_rot);
    return dvermSearchCheckMatched(buf, matched, matched_rot, any);
}

static really_inline
const u8 *dvermSearchLoopBody(svuint16_t chars, const u8 *buf) {
    DEBUG_PRINTF("start %p end %p\n", buf, buf + svcntb());
    svbool_t matched, matched_rot;
    // buf - 1 won't underflow as the first position in the buffer has been
    // dealt with meaning that buf - 1 is within the buffer.
    svbool_t any = doubleMatched(chars, buf, buf - 1, svptrue_b8(),
                                 svptrue_b8(), &matched, &matched_rot);
    return dvermSearchCheckMatched(buf, matched, matched_rot, any);
}

static really_inline
const u8 *rdvermSearchOnce(svuint16_t chars, const u8 *buf, const u8 *buf_end) {
    DEBUG_PRINTF("start %p end %p\n", buf, buf_end);
    assert(buf < buf_end);

    DEBUG_PRINTF("l = %td\n", buf_end - buf);
    // buf_end can be read as the last position in the buffer has been
    // dealt with meaning that buf_end is within the buffer.
    // buf_end needs to be read by both the buf load and the buf + 1 load,
    // this is because buf_end must be the upper 8 bits of the 16 bit element
    // to be matched.
    svbool_t pg = svwhilele_b8_s64(0, buf_end - buf);
    svbool_t pg_rot = svwhilelt_b8_s64(0, buf_end - buf);
    svbool_t matched, matched_rot;
    svbool_t any = doubleMatched(chars, buf, buf + 1, pg, pg_rot,
                                 &matched, &matched_rot);
    return rdvermSearchCheckMatched(buf, matched, matched_rot, any);
}

static really_inline
const u8 *rdvermSearchLoopBody(svuint16_t chars, const u8 *buf) {
    DEBUG_PRINTF("start %p end %p\n", buf, buf + svcntb());
    svbool_t matched, matched_rot;
    // buf + svcntb() can be read as the last position in the buffer has
    // been dealt with meaning that buf + svcntb() is within the buffer.
    svbool_t any = doubleMatched(chars, buf, buf + 1, svptrue_b8(),
                                 svptrue_b8(), &matched, &matched_rot);
    return rdvermSearchCheckMatched(buf, matched, matched_rot, any);
}

static really_inline
const u8 *vermSearch(svuint8_t chars, const u8 *buf, const u8 *buf_end,
                     bool negate) {
    assert(buf < buf_end);
    size_t len = buf_end - buf;
    if (len <= svcntb()) {
        return vermSearchOnce(chars, buf, buf_end, negate);
    }
    // peel off first part to align to the vector size
    const u8 *aligned_buf = ROUNDUP_PTR(buf, svcntb_pat(SV_POW2));
    assert(aligned_buf < buf_end);
    if (buf != aligned_buf) {
        const u8 *ptr = vermSearchLoopBody(chars, buf, negate);
        if (ptr) return ptr;
    }
    buf = aligned_buf;
    uint64_t unrolled_cntb = 2 * svcntb();
    size_t unrolled_loops = (buf_end - buf) / unrolled_cntb;
    DEBUG_PRINTF("unrolled_loops %zu \n", unrolled_loops);
    for (size_t i = 0; i < unrolled_loops; i++, buf += unrolled_cntb) {
        const u8 *ptr = vermSearchLoopBodyUnrolled(chars, buf, negate);
        if (ptr) return ptr;
    }
    size_t loops = (buf_end - buf) / svcntb();
    DEBUG_PRINTF("loops %zu \n", loops);
    for (size_t i = 0; i < loops; i++, buf += svcntb()) {
        const u8 *ptr = vermSearchLoopBody(chars, buf, negate);
        if (ptr) return ptr;
    }
    DEBUG_PRINTF("buf %p buf_end %p \n", buf, buf_end);
    return buf == buf_end ? NULL : vermSearchLoopBody(chars, buf_end - svcntb(),
                                                      negate);
}

static really_inline
const u8 *rvermSearch(svuint8_t chars, const u8 *buf, const u8 *buf_end,
                      bool negate) {
    assert(buf < buf_end);
    size_t len = buf_end - buf;
    if (len <= svcntb()) {
        return rvermSearchOnce(chars, buf, buf_end, negate);
    }
    // peel off first part to align to the vector size
    const u8 *aligned_buf_end = ROUNDDOWN_PTR(buf_end, svcntb_pat(SV_POW2));
    assert(buf < aligned_buf_end);
    if (buf_end != aligned_buf_end) {
        const u8 *ptr = rvermSearchLoopBody(chars, buf_end - svcntb(), negate);
        if (ptr) return ptr;
    }
    buf_end = aligned_buf_end;
    size_t loops = (buf_end - buf) / svcntb();
    DEBUG_PRINTF("loops %zu \n", loops);
    for (size_t i = 0; i < loops; i++) {
        buf_end -= svcntb();
        const u8 *ptr = rvermSearchLoopBody(chars, buf_end, negate);
        if (ptr) return ptr;
    }
    DEBUG_PRINTF("buf %p buf_end %p \n", buf, buf_end);
    return buf == buf_end ? NULL : rvermSearchLoopBody(chars, buf, negate);
}

static really_inline
const u8 *dvermSearch(svuint8_t chars, const u8 *buf, const u8 *buf_end) {
    size_t len = buf_end - buf;
    if (len <= svcntb()) {
        return dvermSearchOnce(svreinterpret_u16(chars), buf, buf_end);
    }
    // peel off first part to align to the vector size
    const u8 *aligned_buf = ROUNDUP_PTR(buf, svcntb_pat(SV_POW2));
    assert(aligned_buf < buf_end);
    if (buf != aligned_buf) {
        const u8 *ptr = dvermSearchLoopBody(svreinterpret_u16(chars), buf);
        if (ptr) return ptr;
    }
    buf = aligned_buf;
    size_t loops = (buf_end - buf) / svcntb();
    DEBUG_PRINTF("loops %zu \n", loops);
    for (size_t i = 0; i < loops; i++, buf += svcntb()) {
        const u8 *ptr = dvermSearchLoopBody(svreinterpret_u16(chars), buf);
        if (ptr) return ptr;
    }
    DEBUG_PRINTF("buf %p buf_end %p \n", buf, buf_end);
    return buf == buf_end ? NULL : dvermSearchLoopBody(svreinterpret_u16(chars), buf_end - svcntb());
}

static really_inline
const u8 *rdvermSearch(char c1, char c2, bool nocase, const u8 *buf,
                       const u8 *buf_end) {
    svuint16_t chars = getCharMaskDouble(c1, c2, nocase);
    size_t len = buf_end - buf;
    if (len <= svcntb()) {
        return rdvermSearchOnce(chars, buf, buf_end);
    }
    // peel off first part to align to the vector size
    const u8 *aligned_buf_end = ROUNDDOWN_PTR(buf_end, svcntb_pat(SV_POW2));
    assert(buf < aligned_buf_end);
    if (buf_end != aligned_buf_end) {
        const u8 *rv = rdvermSearchLoopBody(chars, buf_end - svcntb());
        if (rv) return rv;
    }
    buf_end = aligned_buf_end;
    size_t loops = (buf_end - buf) / svcntb();
    DEBUG_PRINTF("loops %zu \n", loops);
    for (size_t i = 0; i < loops; i++) {
        buf_end -= svcntb();
        const u8 *rv = rdvermSearchLoopBody(chars, buf_end);
        if (rv) return rv;
    }
    DEBUG_PRINTF("buf %p buf_end %p \n", buf, buf_end);
    return buf == buf_end ? NULL : rdvermSearchLoopBody(chars, buf);
}

static really_inline
const u8 *vermicelliExec(char c, bool nocase, const u8 *buf,
                         const u8 *buf_end) {
    DEBUG_PRINTF("verm scan %s\\x%02hhx over %td bytes\n",
                 nocase ? "nocase " : "", c, buf_end - buf);
    svuint8_t chars = getCharMaskSingle(c, nocase);
    const u8 *ptr = vermSearch(chars, buf, buf_end, false);
    return ptr ? ptr : buf_end;
}

/* like vermicelliExec except returns the address of the first character which
 * is not c */
static really_inline
const u8 *nvermicelliExec(char c, bool nocase, const u8 *buf,
                         const u8 *buf_end) {
    DEBUG_PRINTF("nverm scan %s\\x%02hhx over %td bytes\n",
                 nocase ? "nocase " : "", c, buf_end - buf);
    svuint8_t chars = getCharMaskSingle(c, nocase);
    const u8 *ptr = vermSearch(chars, buf, buf_end, true);
    return ptr ? ptr : buf_end;
}

// Reverse vermicelli scan. Provides exact semantics and returns (buf - 1) if
// character not found.
static really_inline
const u8 *rvermicelliExec(char c, bool nocase, const u8 *buf,
                          const u8 *buf_end) {
    DEBUG_PRINTF("rev verm scan %s\\x%02hhx over %td bytes\n",
                 nocase ? "nocase " : "", c, buf_end - buf);
    svuint8_t chars = getCharMaskSingle(c, nocase);
    const u8 *ptr = rvermSearch(chars, buf, buf_end, false);
    return ptr ? ptr : buf - 1;
}

/* like rvermicelliExec except returns the address of the last character which
 * is not c */
static really_inline
const u8 *rnvermicelliExec(char c, bool nocase, const u8 *buf,
                           const u8 *buf_end) {
    DEBUG_PRINTF("rev verm scan %s\\x%02hhx over %td bytes\n",
                 nocase ? "nocase " : "", c, buf_end - buf);
    svuint8_t chars = getCharMaskSingle(c, nocase);
    const u8 *ptr = rvermSearch(chars, buf, buf_end, true);
    return ptr ? ptr : buf - 1;
}

static really_inline
const u8 *vermicelliDoubleExec(char c1, char c2, bool nocase, const u8 *buf,
                               const u8 *buf_end) {
    DEBUG_PRINTF("double verm scan %s\\x%02hhx%02hhx over %td bytes\n",
                 nocase ? "nocase " : "", c1, c2, buf_end - buf);
    assert(buf < buf_end);
    if (buf_end - buf > 1) {
        ++buf;
        svuint8_t chars = svreinterpret_u8(getCharMaskDouble(c1, c2, nocase));
        const u8 *ptr = dvermSearch(chars, buf, buf_end);
        if (ptr) {
            return ptr;
        }
    }
    /* check for partial match at end */
    u8 mask = nocase ? CASE_CLEAR : 0xff;
    if ((buf_end[-1] & mask) == (u8)c1) {
        DEBUG_PRINTF("partial!!!\n");
        return buf_end - 1;
    }
    return buf_end;
}

/* returns highest offset of c2 (NOTE: not c1) */
static really_inline
const u8 *rvermicelliDoubleExec(char c1, char c2, bool nocase, const u8 *buf,
                                const u8 *buf_end) {
    DEBUG_PRINTF("rev double verm scan %s\\x%02hhx%02hhx over %td bytes\n",
                 nocase ? "nocase " : "", c1, c2, buf_end - buf);
    assert(buf < buf_end);
    if (buf_end - buf > 1) {
        --buf_end;
        const u8 *ptr = rdvermSearch(c1, c2, nocase, buf, buf_end);
        if (ptr) {
            return ptr;
        }
    }
    return buf - 1;
}

static really_inline
svuint8_t getDupSVEMaskFrom128(m128 mask) {
    return svld1rq_u8(svptrue_b8(), (const uint8_t *)&mask);
}

static really_inline
const u8 *vermicelli16Exec(const m128 mask, const u8 *buf,
                           const u8 *buf_end) {
    DEBUG_PRINTF("verm16 scan over %td bytes\n", buf_end - buf);
    svuint8_t chars = getDupSVEMaskFrom128(mask);
    const u8 *ptr = vermSearch(chars, buf, buf_end, false);
    return ptr ? ptr : buf_end;
}

static really_inline
const u8 *nvermicelli16Exec(const m128 mask, const u8 *buf,
                            const u8 *buf_end) {
    DEBUG_PRINTF("nverm16 scan over %td bytes\n", buf_end - buf);
    svuint8_t chars = getDupSVEMaskFrom128(mask);
    const u8 *ptr = vermSearch(chars, buf, buf_end, true);
    return ptr ? ptr : buf_end;
}

static really_inline
const u8 *rvermicelli16Exec(const m128 mask, const u8 *buf,
                            const u8 *buf_end) {
    DEBUG_PRINTF("rverm16 scan over %td bytes\n", buf_end - buf);
    svuint8_t chars = getDupSVEMaskFrom128(mask);
    const u8 *ptr = rvermSearch(chars, buf, buf_end, false);
    return ptr ? ptr : buf - 1;
}

static really_inline
const u8 *rnvermicelli16Exec(const m128 mask, const u8 *buf,
                             const u8 *buf_end) {
    DEBUG_PRINTF("rnverm16 scan over %td bytes\n", buf_end - buf);
    svuint8_t chars = getDupSVEMaskFrom128(mask);
    const u8 *ptr = rvermSearch(chars, buf, buf_end, true);
    return ptr ? ptr : buf - 1;
}

static really_inline
bool vermicelliDouble16CheckPartial(const u64a first_chars, const u8 *buf_end) {
    svuint8_t firsts = svreinterpret_u8(svdup_u64(first_chars));
    svbool_t matches = svcmpeq(svptrue_b8(), firsts, svdup_u8(buf_end[-1]));
    return svptest_any(svptrue_b8(), matches);
}

static really_inline
const u8 *vermicelliDouble16Exec(const m128 mask, const u64a firsts,
                                 const u8 *buf, const u8 *buf_end) {
    assert(buf < buf_end);
    DEBUG_PRINTF("double verm16 scan over %td bytes\n", buf_end - buf);
    if (buf_end - buf > 1) {
        ++buf;
        svuint8_t chars = svreinterpret_u8(getDupSVEMaskFrom128(mask));
        const u8 *ptr = dvermSearch(chars, buf, buf_end);
        if (ptr) {
            return ptr;
        }
    }
    /* check for partial match at end */
    if (vermicelliDouble16CheckPartial(firsts, buf_end)) {
        DEBUG_PRINTF("partial!!!\n");
        return buf_end - 1;
    }
    return buf_end;
}

static really_inline
const u8 *vermicelliDoubleMasked16Exec(const m128 mask, char c1, char m1,
                                       const u8 *buf, const u8 *buf_end) {
    assert(buf < buf_end);
    DEBUG_PRINTF("double verm16 masked scan over %td bytes\n", buf_end - buf);
    if (buf_end - buf > 1) {
        ++buf;
        svuint8_t chars = getDupSVEMaskFrom128(mask);
        const u8 *ptr = dvermSearch(chars, buf, buf_end);
        if (ptr) {
            return ptr;
        }
    }
    /* check for partial match at end */
    if ((buf_end[-1] & m1) == (u8)c1) {
        DEBUG_PRINTF("partial!!!\n");
        return buf_end - 1;
    }

    return buf_end;
}

// returns NULL if not found
static really_inline
const u8 *dvermPreconditionMasked(m128 chars1, m128 chars2,
                                  m128 mask1, m128 mask2, const u8 *buf) {
    m128 data = loadu128(buf); // unaligned
    m128 v1 = eq128(chars1, and128(data, mask1));
    m128 v2 = eq128(chars2, and128(data, mask2));
    u32 z = movemask128(and128(v1, rshiftbyte_m128(v2, 1)));

    /* no fixup of the boundary required - the aligned run will pick it up */
    if (unlikely(z)) {
        u32 pos = ctz32(z);
        return buf + pos;
    }
    return NULL;
}

static really_inline
const u8 *dvermSearchAlignedMasked(m128 chars1, m128 chars2,
                                   m128 mask1, m128 mask2, u8 c1, u8 c2, u8 m1,
                                   u8 m2, const u8 *buf, const u8 *buf_end) {
    assert((size_t)buf % 16 == 0);

    for (; buf + 16 < buf_end; buf += 16) {
        m128 data = load128(buf);
        m128 v1 = eq128(chars1, and128(data, mask1));
        m128 v2 = eq128(chars2, and128(data, mask2));
        u32 z = movemask128(and128(v1, rshiftbyte_m128(v2, 1)));

        if ((buf[15] & m1) == c1 && (buf[16] & m2) == c2) {
            z |= (1 << 15);
        }
        if (unlikely(z)) {
            u32 pos = ctz32(z);
            return buf + pos;
        }
    }

    return NULL;
}

static really_inline
const u8 *vermicelliDoubleMaskedExec(char c1, char c2, char m1, char m2,
                                     const u8 *buf, const u8 *buf_end) {
    DEBUG_PRINTF("double verm scan (\\x%02hhx&\\x%02hhx)(\\x%02hhx&\\x%02hhx) "
                 "over %zu bytes\n", c1, m1, c2, m2, (size_t)(buf_end - buf));
    assert(buf < buf_end);

    m128 chars1 = set1_16x8(c1);
    m128 chars2 = set1_16x8(c2);
    m128 mask1 = set1_16x8(m1);
    m128 mask2 = set1_16x8(m2);

    assert((buf_end - buf) >= 16);
    uintptr_t min = (uintptr_t)buf % 16;
    if (min) {
        // Input isn't aligned, so we need to run one iteration with an
        // unaligned load, then skip buf forward to the next aligned address.
        // There's some small overlap here, but we don't mind scanning it twice
        // if we can do it quickly, do we?
        const u8 *p = dvermPreconditionMasked(chars1, chars2, mask1, mask2, buf);
        if (p) {
            return p;
        }

        buf += 16 - min;
        assert(buf < buf_end);
    }

    // Aligned loops from here on in
    const u8 *ptr = dvermSearchAlignedMasked(chars1, chars2, mask1, mask2, c1,
                                             c2, m1, m2, buf, buf_end);
    if (ptr) {
        return ptr;
    }

    // Tidy up the mess at the end
    ptr = dvermPreconditionMasked(chars1, chars2, mask1, mask2,
                                  buf_end - 16);

    if (ptr) {
        return ptr;
    }

    /* check for partial match at end */
    if ((buf_end[-1] & m1) == (u8)c1) {
        DEBUG_PRINTF("partial!!!\n");
        return buf_end - 1;
    }

    return buf_end;
}
