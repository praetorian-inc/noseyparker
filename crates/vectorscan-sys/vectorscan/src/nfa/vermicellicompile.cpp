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
 * \brief Vermicelli acceleration: compile code.
 */
#include "vermicellicompile.h"
#include "util/charreach.h"

#include <cstring>

namespace ue2 {

bool vermicelli16Build(const CharReach &chars, u8 *rv) {
    size_t i = chars.find_first();
    u8 arr[16];
    std::memset(arr, i, sizeof(arr));
    size_t count = 1;
    for (i = chars.find_next(i); i != CharReach::npos; i = chars.find_next(i)) {
        if (count == sizeof(arr)) return false;
        arr[count] = i;
        ++count;
    }
    std::memcpy(rv, arr, sizeof(arr));
    return true;
}

bool vermicelliDouble16Build(const flat_set<std::pair<u8, u8>> &twochar,
                             u8 *chars, u8 *firsts) {
    constexpr size_t count_limit = 8;
    if (twochar.size() > count_limit) return false;
    size_t count = 0;
    for (const auto &p : twochar) {
        firsts[count] = p.first;
        chars[2 * count] = p.first;
        chars[(2 * count) + 1] = p.second;
        ++count;
    }
    for(; count < count_limit; ++count) {
        firsts[count] = chars[0];
        chars[2 * count] = chars[0];
        chars[(2 * count) + 1] = chars[1];
    }
    return true;
}

static really_inline
void fillMask(u8 matches[], size_t len, u8 *rv) {
    for (size_t i = 0; i < 16; ++i) {
        rv[i] = matches[i % len];
    }
}

static really_inline
void getTwoCases(u8 cases[2], u8 bit, char c) {
    const u8 set = 1UL << bit;
    cases[0] = c & (~set);
    cases[1] = c | set;
}

static really_inline
void getFourCases(u8 cases[4], u8 bit, char case1, char case2) {
    const u8 set = 1UL << bit;
    cases[0] = case1 & (~set);
    cases[1] = case1 | set;
    cases[2] = case2 & (~set);
    cases[3] = case2 | set;
}

static really_inline
void getEightCases(u8 cases[8], u8 bit, char case1, char case2,
                                        char case3, char case4) {
    const u8 set = 1UL << bit;
    cases[0] = case1 & (~set);
    cases[1] = case1 | set;
    cases[2] = case2 & (~set);
    cases[3] = case2 | set;
    cases[4] = case3 & (~set);
    cases[5] = case3 | set;
    cases[6] = case4 & (~set);
    cases[7] = case4 | set;
}

static really_inline
bool getDoubleMatchesForBits(u8 c1, u8 c2, u8 holes[3], u8 c1_holes,
                             u8 c2_holes, u8 *rv) {
    u8 cases[8];
    switch (c1_holes) {
        case 0:
            switch (c2_holes) {
                case 0: {
                    u8 matches[2] = { c1, c2 };
                    fillMask(matches, 2, rv);
                    return true;
                }
                case 1: {
                    getTwoCases(cases, holes[0], c2);
                    u8 matches[4] = { c1, cases[0], c1, cases[1] };
                    fillMask(matches, 4, rv);
                    return true;
                }
                case 2: {
                    getTwoCases(cases, holes[0], c2);
                    getFourCases(&cases[2], holes[1], cases[0], cases[1]);
                    u8 matches[8] = { c1, cases[2], c1, cases[3],
                                      c1, cases[4], c1, cases[5] };
                    fillMask(matches, 8, rv);
                    return true;
                }
                case 3: {
                    getTwoCases(cases, holes[0], c2);
                    getFourCases(&cases[4], holes[1], cases[0], cases[1]);
                    getEightCases(cases, holes[2], cases[4], cases[5],
                                                  cases[6], cases[7]);
                    u8 matches[16] = { c1, cases[0], c1, cases[1],
                                       c1, cases[2], c1, cases[3],
                                       c1, cases[4], c1, cases[5],
                                       c1, cases[6], c1, cases[7] };
                    memcpy(rv, matches, sizeof(matches));
                    return true;
                }
                default:
                    assert(c2_holes < 4);
                    break;
            }
            break;
        case 1:
            getTwoCases(cases, holes[0], c1);
            switch (c2_holes) {
                case 0: {
                    u8 matches[4] = { cases[0] , c2, cases[1], c2 };
                    fillMask(matches, 4, rv);
                    return true;
                }
                case 1: {
                    getTwoCases(&cases[2], holes[1], c2);
                    u8 matches[8] = { cases[0], cases[2],
                                      cases[0], cases[3],
                                      cases[1], cases[2],
                                      cases[1], cases[3] };
                    fillMask(matches, 8, rv);
                    return true;
                }
                case 2: {
                    getTwoCases(&cases[2], holes[1], c2);
                    getFourCases(&cases[4], holes[2], cases[2], cases[3]);
                    u8 matches[16] = { cases[0], cases[4], cases[0], cases[5],
                                       cases[0], cases[6], cases[0], cases[7],
                                       cases[1], cases[4], cases[1], cases[5],
                                       cases[1], cases[6], cases[1], cases[7] };
                    memcpy(rv, matches, sizeof(matches));
                    return true;
                }
                default:
                    assert(c2_holes < 3);
                    break;
            }
            break;
        case 2:
            getTwoCases(cases, holes[0], c1);
            getFourCases(&cases[2], holes[1], cases[0], cases[1]);
            switch (c2_holes) {
                case 0: {
                    u8 matches[8] = { cases[2], c2, cases[3], c2,
                                      cases[4], c2, cases[5], c2 };
                    fillMask(matches, 8, rv);
                    return true;
                }
                case 1: {
                    getTwoCases(&cases[6], holes[2], c2);
                    u8 matches[16] = { cases[2], cases[6], cases[3], cases[6],
                                       cases[4], cases[6], cases[5], cases[6],
                                       cases[2], cases[7], cases[3], cases[7],
                                       cases[4], cases[7], cases[5], cases[7] };
                    memcpy(rv, matches, sizeof(matches));
                    return true;
                }
                default:
                    assert(c2_holes < 2);
                    break;
            }
            break;
        case 3: {
            assert(!c2_holes);
            getTwoCases(cases, holes[0], c1);
            getFourCases(&cases[4], holes[1], cases[0], cases[1]);
            getEightCases(cases, holes[2], cases[4], cases[5],
                                        cases[6], cases[7]);
            u8 matches[16] = { cases[0], c2, cases[1], c2,
                                cases[2], c2, cases[3], c2,
                                cases[4], c2, cases[5], c2,
                                cases[6], c2, cases[7], c2 };
            memcpy(rv, matches, sizeof(matches));
            return true;
        }
    }
    return false;
}

static really_inline
bool getDoubleMatchesForMask(char c1, char c2, char m1, char m2,
                             u8 c1_holes, u8 c2_holes, u8 *rv) {
    u8 holes[3] = { 0 };
    int count = 0;
    if (c1_holes) {
        for (int i = 0; i < 8; ++i) {
            if (!(m1 & (1UL << i))) {
                holes[count++] = i;
            }
        }
    }
    if (c2_holes) {
        for (int i = 0; i < 8; ++i) {
            if (!(m2 & (1UL << i))) {
                holes[count++] = i;
            }
        }
    }
    return getDoubleMatchesForBits(c1, c2, holes, c1_holes, c2_holes, rv);
}

bool vermicelliDoubleMasked16Build(char c1, char c2, char m1, char m2, u8 *rv) {
    u8 c1_holes = 8 - __builtin_popcount(m1);
    u8 c2_holes = 8 - __builtin_popcount(m2);
    if (c1_holes + c2_holes > 3) {
        return false;
    }
    return getDoubleMatchesForMask(c1, c2, m1, m2, c1_holes, c2_holes, rv);
}

} // namespace ue2
