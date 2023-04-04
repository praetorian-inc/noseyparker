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
 * \brief SVE primitive operations.
 */

static really_inline
svuint8_t getCharMaskSingle(const u8 c, bool noCase) {
    if (noCase) {
        uint16_t chars_u16 = (c & 0xdf) | ((c | 0x20) << 8);
        return svreinterpret_u8(svdup_u16(chars_u16));
    } else {
        return svdup_u8(c);
    }
}

static really_inline
svuint16_t getCharMaskDouble(const u8 c0, const u8 c1, bool noCase) {
    if (noCase) {
        const uint64_t lowerFirst = c0 & 0xdf;
        const uint64_t upperFirst = c0 | 0x20;
        const uint64_t lowerSecond = c1 & 0xdf;
        const uint64_t upperSecond = c1 | 0x20;
        const uint64_t chars = lowerFirst | (lowerSecond << 8)
                          | (lowerFirst << 16) | (upperSecond) << 24
                          | (upperFirst << 32) | (lowerSecond) << 40
                          | (upperFirst << 48) | (upperSecond) << 56;
        return svreinterpret_u16(svdup_u64(chars));
    } else {
        uint16_t chars_u16 = c0 | (c1 << 8);
        return svdup_u16(chars_u16);
    }
}