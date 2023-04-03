/*
 * Copyright (c) 2015, Intel Corporation
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

#include "config.h"

#include "gtest/gtest.h"
#include "nfa/vermicelli.hpp"

#define BOUND (~(VERM_BOUNDARY - 1))

TEST(RVermicelli, ExecNoMatch1) {
    char t1[] = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    for (size_t i = 0; i < 16; i++) {
        for (size_t j = 0; j < 16; j++) {
            const u8 *begin = (const u8 *)t1 + i;
            const u8 *end = (const u8 *)t1 + strlen(t1) - j;

            const u8 *rv = rvermicelliExec('a', 0, begin, end);
            ASSERT_EQ(begin - 1, rv);

            rv = rvermicelliExec('B', 0, begin, end);
            ASSERT_EQ(begin - 1, rv);

            rv = rvermicelliExec('A', 1, begin, end);
            ASSERT_EQ(begin - 1, rv);
        }
    }
}

TEST(RVermicelli, Exec1) {
    char t1[] = "bbbbbbbbbbbbbbbbbabbbbbbbbbbbbbbbbbbbbbbbbbbbbbbabbbbbbbbbbbbbbbbbbbbb";

    for (size_t i = 0; i < 16; i++) {
        const u8 *rv = rvermicelliExec('a', 0, (u8 *)t1,
                                      (u8 *)t1 + strlen(t1) - i);

        ASSERT_EQ((size_t)t1 + 48, (size_t)rv);

        rv = rvermicelliExec('A', 1, (u8 *)t1 + i, (u8 *)t1 + strlen(t1));

        ASSERT_EQ((size_t)t1 + 48, (size_t)rv);
    }
}

TEST(RVermicelli, Exec2) {
    char t1[] = "bbbbbbbbbbbbbbbbbabbbbbbbbaaaaaaaaaaaaaaaaaaaaaaabbbbbbbbbbbbbbbbbbbbbb";

    for (size_t i = 0; i < 16; i++) {
        const u8 *rv = rvermicelliExec('a', 0, (u8 *)t1,
                                       (u8 *)t1 + strlen(t1) - i);

        ASSERT_EQ((size_t)t1 + 48, (size_t)rv);

        rv = rvermicelliExec('A', 1, (u8 *)t1, (u8 *)t1 + strlen(t1) - i);

        ASSERT_EQ((size_t)t1 + 48, (size_t)rv);
    }
}

TEST(RVermicelli, Exec3) {
    char t1[] = "bbbbbbbbbbbbbbbbbabbbbbbbbaaaaaaaaaaaaaaaaaaaaaaAbbbbbbbbbbbbbbbbbbbbbb";

    for (size_t i = 0; i < 16; i++) {
        const u8 *rv = rvermicelliExec('a', 0, (u8 *)t1,
                                      (u8 *)t1 + strlen(t1) - i);

        ASSERT_EQ((size_t)t1 + 47, (size_t)rv);

        rv = rvermicelliExec('A', 1, (u8 *)t1, (u8 *)t1 + strlen(t1) - i);

        ASSERT_EQ((size_t)t1 + 48, (size_t)rv);
    }
}

TEST(RVermicelli, Exec4) {
    char t1[] = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    for (size_t i = 0; i < 31; i++) {
        t1[16 + i] = 'a';
        const u8 *rv = rvermicelliExec('a', 0, (u8 *)t1, (u8 *)t1 + strlen(t1));

        ASSERT_EQ((size_t)&t1[16 + i], (size_t)rv);

        rv = rvermicelliExec('A', 1, (u8 *)t1, (u8 *)t1 + strlen(t1));

        ASSERT_EQ((size_t)&t1[16 + i], (size_t)rv);
    }
}

TEST(RNVermicelli, ExecNoMatch1) {
    char t1[] = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const u8 *buf = (const u8 *)t1;

    for (size_t i = 0; i < 16; i++) {
        SCOPED_TRACE(i);
        for (size_t j = 0; j < 16; j++) {
            SCOPED_TRACE(j);
            const u8 *rv = rnvermicelliExec('b', 0, buf + i,
                                                    buf + strlen(t1) - j);

            ASSERT_EQ(buf + i - 1, rv);

            rv = rnvermicelliExec('B', 1, buf + i, buf + strlen(t1) - j);

            ASSERT_EQ(buf + i - 1, rv);
        }
    }
}

TEST(RNVermicelli, Exec1) {
    char t1[] = "bbbbbbbbbbbbbbbbbabbbbbbbbbbbbbbbbbbbbbbbbbbbbbbabbbbbbbbbbbbbbbbbbbbb";
    const u8 *buf = (const u8 *)t1;

    for (size_t i = 0; i < 16; i++) {
        SCOPED_TRACE(i);
        const u8 *rv = rnvermicelliExec('b', 0, buf, buf + strlen(t1) - i);

        ASSERT_EQ(buf + 48, rv);

        rv = rnvermicelliExec('B', 1, buf + i, buf + strlen(t1) - i);

        ASSERT_EQ(buf + 48, rv);
    }
}

TEST(RNVermicelli,  Exec2) {
    char t1[] = "bbbbbbbbbbbbbbbbbabbbbbbbbaaaaaaaaaaaaaaaaaaaaaaabbbbbbbbbbbbbbbbbbbbb";
    const u8 *buf = (const u8 *)t1;

    for (size_t i = 0; i < 16; i++) {
        SCOPED_TRACE(i);
        const u8 *rv = rnvermicelliExec('b', 0, buf, buf + strlen(t1) - i);

        ASSERT_EQ(buf + 48, rv);

        rv = rnvermicelliExec('B', 1, buf + i, buf + strlen(t1) - i);

        ASSERT_EQ(buf + 48, rv);
    }
}

TEST(RNVermicelli,  Exec3) {
    char t1[] = "bbbbbbbbbbbbbbbbbabbbbbbbbaaaaaaaaaaaaaaaaaaaaaaAbbbbbbbbbbbbbbbbbbbbbb";
    const u8 *buf = (const u8 *)t1;

    for (size_t i = 0; i < 16; i++) {
        SCOPED_TRACE(i);
        const u8 *rv = rnvermicelliExec('b', 0, buf + i, buf + strlen(t1));

        ASSERT_EQ(buf + 48, rv);

        rv = rnvermicelliExec('B', 1, buf + i, buf + strlen(t1));

        ASSERT_EQ(buf + 48, rv);
    }
}

TEST(RNVermicelli, Exec4) {
    char t1[] = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const u8 *buf = (const u8 *)t1;

    for (size_t i = 0; i < 31; i++) {
        SCOPED_TRACE(i);
        t1[16 + i] = 'a';
        const u8 *rv = rnvermicelliExec('b', 0, buf, buf + strlen(t1));

        ASSERT_EQ(buf + 16 + i, rv);

        rv = rnvermicelliExec('B', 1, buf, buf + strlen(t1));

        ASSERT_EQ(buf + 16 + i, rv);
    }
}


TEST(RDoubleVermicelli, Exec1) {
    char t1[] = "bbbbbbbbbbbbbbbbbbabbbbbbbbbbbbbbbbbbbbbbbbbbbbbbabbbbbbbbbbbbbbbbbbbbb";

    for (size_t i = 0; i < 16; i++) {
        const u8 *rv = rvermicelliDoubleExec('a', 'b', 0, (u8 *)t1,
                                      (u8 *)t1 + strlen(t1) - i);

        ASSERT_EQ((size_t)t1 + 50, (size_t)rv);

        rv = rvermicelliDoubleExec('A', 'B', 1, (u8 *)t1 + i,
                            (u8 *)t1 + strlen(t1));

        ASSERT_EQ((size_t)t1 + 50, (size_t)rv);

        rv = rvermicelliDoubleExec('b', 'a', 0, (u8 *)t1 + i,
                            (u8 *)t1 + strlen(t1));

        ASSERT_EQ((size_t)t1 + 49, (size_t)rv);

        rv = rvermicelliDoubleExec('B', 'A', 1, (u8 *)t1 + i,
                            (u8 *)t1 + strlen(t1));

        ASSERT_EQ((size_t)t1 + 49, (size_t)rv);
    }
}

TEST(RDoubleVermicelli, Exec2) {
    char t1[] = "bbbbbbbbbbbbbbbbbaaaaaaaaaaaaaaaaaaaaaaaabbbbbbbaaaaabbbbbbbbbbbbbbbbbb";

    for (size_t i = 0; i < 16; i++) {
        const u8 *rv = rvermicelliDoubleExec('a', 'a', 0, (u8 *)t1,
                                      (u8 *)t1 + strlen(t1) - i);

        ASSERT_EQ((size_t)t1 + 52, (size_t)rv);

        rv = rvermicelliDoubleExec('A', 'A', 1, (u8 *)t1,
                            (u8 *)t1 + strlen(t1) - i);

        ASSERT_EQ((size_t)t1 + 52, (size_t)rv);
    }
}

TEST(RDoubleVermicelli, Exec3) {
    /*           012345678901234567890123 */
    char t1[] = "bbbbbbbbbbbbbbbbbaAaaAAaaaaaaaaaaaaaaaaaabbbbbbbaaaaabbbbbbbbbbbbbbbbbb";

    for (size_t i = 0; i < 16; i++) {
        const u8 *rv = rvermicelliDoubleExec('A', 'a', 0, (u8 *)t1,
                                      (u8 *)t1 + strlen(t1) - i );

        ASSERT_EQ((size_t)t1 + 23, (size_t)rv);

        rv = rvermicelliDoubleExec('A', 'A', 1, (u8 *)t1,
                                  (u8 *)t1 + strlen(t1) - i);

        ASSERT_EQ((size_t)t1 + 52, (size_t)rv);

        rv = rvermicelliDoubleExec('A', 'A', 0, (u8 *)t1,
                                  (u8 *)t1 + strlen(t1) - i);

        ASSERT_EQ((size_t)t1 + 22, (size_t)rv);

        rv = rvermicelliDoubleExec('a', 'A', 0, (u8 *)t1,
                                  (u8 *)t1 + strlen(t1) - i);

        ASSERT_EQ((size_t)t1 + 21, (size_t)rv);
    }
}

TEST(RDoubleVermicelli, Exec4) {
    char t1[] = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    for (size_t i = 0; i < 31; i++) {
        t1[32 + i] = 'a';
        t1[32 + i - 1] = 'a';
        const u8 *rv = rvermicelliDoubleExec('a', 'a', 0, (u8 *)t1,
                                            (u8 *)t1 + strlen(t1));
        ASSERT_EQ((size_t)&t1[32 + i], (size_t)rv);

        rv = rvermicelliDoubleExec('A', 'A', 1, (u8 *)t1, (u8 *)t1 + strlen(t1));

        ASSERT_EQ((size_t)&t1[32 + i], (size_t)rv);
    }
}

TEST(RDoubleVermicelli, Exec5) {
    char t1[] = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    for (size_t i = 0; i < 16; i++) {
        for (size_t j = 1; j <= 16; j++) {
            t1[strlen(t1) - i - j] = 'a';
            const u8 *rv = rvermicelliDoubleExec('b', 'a', 0, (u8 *)t1,
                                                 (u8 *)t1 + strlen(t1) - i);

            ASSERT_EQ((size_t)&t1[strlen(t1) - i - j], (size_t)rv);

            rv = rvermicelliDoubleExec('B', 'A', 1, (u8 *)t1,
                                       (u8 *)t1 + strlen(t1) -i );

            ASSERT_EQ((size_t)&t1[strlen(t1) - i - j], (size_t)rv);

            t1[strlen(t1) - i - j] = 'b';
        }
    }
}

#ifdef HAVE_SVE2

#include "nfa/vermicellicompile.h"
using namespace ue2;

TEST(RVermicelli16, ExecNoMatch1) {
    char t1[] = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    CharReach chars;
    chars.set('a');
    chars.set('B');
    chars.set('A');
    m128 matches;
    bool ret = vermicelli16Build(chars, (u8 *)&matches);
    ASSERT_TRUE(ret);

    for (size_t i = 0; i < 16; i++) {
        for (size_t j = 0; j < 16; j++) {
            const u8 *begin = (const u8 *)t1 + i;
            const u8 *end = (const u8 *)t1 + strlen(t1) - j;

            const u8 *rv = rvermicelli16Exec(matches, begin, end);
            ASSERT_EQ(begin - 1, rv);
        }
    }
}

TEST(RVermicelli16, Exec1) {
    char t1[] = "bbbbbbbbbbbbbbbbbabbbbbbbbbbbbbbbbbbbbbbbbbbbbbbabbbbbbbbbbbbbbbbbbbbb";
    const u8 *buf = (const u8 *)t1;

    CharReach chars;
    chars.set('a');
    chars.set('A');
    m128 matches;
    bool ret = vermicelli16Build(chars, (u8 *)&matches);
    ASSERT_TRUE(ret);

    for (size_t i = 0; i < 16; i++) {
        const u8 *rv = rvermicelli16Exec(matches, buf, buf + strlen(t1) - i);
        ASSERT_EQ(buf + 48, rv);
    }
}

TEST(RVermicelli16,  Exec2) {
    char t1[] = "bbbbbbbbbbbbbbbbbabbbbbbbbaaaaaaaaaaaaaaaaaaaaaaabbbbbbbbbbbbbbbbbbbbbb";
    const u8 *buf = (const u8 *)t1;

    CharReach chars;
    chars.set('a');
    chars.set('A');
    m128 matches;
    bool ret = vermicelli16Build(chars, (u8 *)&matches);
    ASSERT_TRUE(ret);

    for (size_t i = 0; i < 16; i++) {
        const u8 *rv = rvermicelli16Exec(matches, buf + i, buf + strlen(t1));
        ASSERT_EQ(buf + 48, rv);
    }
}

TEST(RVermicelli16,  Exec3) {
    char t1[] = "bbbbbbbbbbbbbbbbbabbbbbbbbaaaaaaaaaaaaaaaaaaaaaaAbbbbbbbbbbbbbbbbbbbbbb";
    const u8 *buf = (const u8 *)t1;

    CharReach chars;
    chars.set('a');
    m128 matches_a;
    bool ret = vermicelli16Build(chars, (u8 *)&matches_a);
    ASSERT_TRUE(ret);

    chars.set('A');
    m128 matches_A;
    ret = vermicelli16Build(chars, (u8 *)&matches_A);
    ASSERT_TRUE(ret);

    for (size_t i = 0; i < 16; i++) {
        const u8 *rv = rvermicelli16Exec(matches_a, buf, buf + strlen(t1) - i);
        ASSERT_EQ(buf + 47, rv);

        rv = rvermicelli16Exec(matches_A, buf, buf + strlen(t1) - i);
        ASSERT_EQ(buf + 48, rv);
    }
}

TEST(RVermicelli16, Exec4) {
    char t1[] = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const u8 *buf = (const u8 *)t1;

    CharReach chars;
    chars.set('a');
    m128 matches_a;
    bool ret = vermicelli16Build(chars, (u8 *)&matches_a);
    ASSERT_TRUE(ret);

    chars.set('A');
    m128 matches_A;
    ret = vermicelli16Build(chars, (u8 *)&matches_A);
    ASSERT_TRUE(ret);

    for (size_t i = 0; i < 31; i++) {
        t1[16 + i] = 'a';
        const u8 *rv = rvermicelli16Exec(matches_a, buf, buf + strlen(t1));
        ASSERT_EQ(buf + 16 + i, rv);

        rv = rvermicelli16Exec(matches_A, buf, buf + strlen(t1));
        ASSERT_EQ(buf + 16 + i, rv);
    }
}

TEST(RVermicelli16, Exec5) {
    char t1[] = "qqqqqqqqqqqqqqqqqabcdefghijklmnopqqqqqqqqqqqqqqqqqqqqq";
    const u8 *buf = (const u8 *)t1;

    CharReach chars;
    m128 matches[16];
    bool ret;

    for (int i = 0; i < 16; ++i) {
        chars.set('a' + i);
        ret = vermicelli16Build(chars, (u8 *)&matches[i]);
        ASSERT_TRUE(ret);
    }

    for (int j = 0; j < 16; ++j) {
        for (size_t i = 0; i < 16; i++) {
            const u8 *rv = rvermicelli16Exec(matches[j], buf, buf + strlen(t1) - i);
            ASSERT_EQ(buf + j + 17, rv);
        }
    }
}

TEST(RNVermicelli16, ExecNoMatch1) {
    char t1[] = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const u8 *buf = (const u8 *)t1;

    CharReach chars;
    chars.set('b');
    chars.set('B');
    chars.set('A');
    m128 matches;
    bool ret = vermicelli16Build(chars, (u8 *)&matches);
    ASSERT_TRUE(ret);

    for (size_t i = 0; i < 16; i++) {
        for (size_t j = 0; j < 16; j++) {
            const u8 *rv = rnvermicelli16Exec(matches, buf + i, buf + strlen(t1) - j);
            ASSERT_EQ(buf + i - 1, rv);
        }
    }
}

TEST(RNVermicelli16, Exec1) {
    char t1[] = "bbbbbbbbbbbbbbbbbabbbbbbbbbbbbbbbbbbbbbbbbbbbbbbabbbbbbbbbbbbbbbbbbbbb";
    const u8 *buf = (const u8 *)t1;

    CharReach chars;
    chars.set('b');
    chars.set('A');
    m128 matches;
    bool ret = vermicelli16Build(chars, (u8 *)&matches);
    ASSERT_TRUE(ret);

    for (size_t i = 0; i < 16; i++) {
        const u8 *rv = rnvermicelli16Exec(matches, buf + i, buf + strlen(t1) - i);
        ASSERT_EQ(buf + 48, rv);
    }
}

TEST(RNVermicelli16,  Exec2) {
    char t1[] = "bbbbbbbbbbbbbbbbbabbbbbbbbaaaaaaaaaaaaaaaaaaaaaaabbbbbbbbbbbbbbbbbbbbb";
    const u8 *buf = (const u8 *)t1;

    CharReach chars;
    chars.set('b');
    chars.set('A');
    m128 matches;
    bool ret = vermicelli16Build(chars, (u8 *)&matches);
    ASSERT_TRUE(ret);

    for (size_t i = 0; i < 16; i++) {
        const u8 *rv = rnvermicelli16Exec(matches, buf, buf + strlen(t1) - i);
        ASSERT_EQ(buf + 48, rv);
    }
}

TEST(RNVermicelli16,  Exec3) {
    char t1[] = "bbbbbbbbbbbbbbbbbabbbbbbbbaaaaaaaaaaaaaaaaaaaaaaAbbbbbbbbbbbbbbbbbbbbbb";
    const u8 *buf = (const u8 *)t1;

    CharReach chars;
    chars.set('b');
    m128 matches_b;
    bool ret = vermicelli16Build(chars, (u8 *)&matches_b);
    ASSERT_TRUE(ret);

    chars.set('A');
    m128 matches_A;
    ret = vermicelli16Build(chars, (u8 *)&matches_A);
    ASSERT_TRUE(ret);

    for (size_t i = 0; i < 16; i++) {
        const u8 *rv = rnvermicelli16Exec(matches_b, buf + i, buf + strlen(t1));
        ASSERT_EQ(buf + 48, rv);

        rv = rnvermicelli16Exec(matches_A, buf + i, buf + strlen(t1));
        ASSERT_EQ(buf + 47, rv);
    }
}

TEST(RNVermicelli16, Exec4) {
    char t1[] = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const u8 *buf = (const u8 *)t1;

    CharReach chars;
    chars.set('b');
    m128 matches_b;
    bool ret = vermicelli16Build(chars, (u8 *)&matches_b);
    ASSERT_TRUE(ret);

    chars.set('A');
    m128 matches_A;
    ret = vermicelli16Build(chars, (u8 *)&matches_A);
    ASSERT_TRUE(ret);

    for (size_t i = 0; i < 31; i++) {
        t1[16 + i] = 'a';
        const u8 *rv = rnvermicelli16Exec(matches_b, buf, buf + strlen(t1));
        ASSERT_EQ(buf + 16 + i, rv);

        rv = rnvermicelli16Exec(matches_A, buf, buf + strlen(t1));
        ASSERT_EQ(buf + 16 + i, rv);
    }
}

TEST(RNVermicelli16, Exec5) {
    char t1[] = "aaaaaaaaaaaaaaaaaabcdefghijklmnopqqqqqqqqqqqqqqqqqqqqqqqq";
    const u8 *buf = (const u8 *)t1;

    CharReach chars;
    m128 matches[16];
    bool ret;

    for (int i = 0; i < 16; ++i) {
        chars.set('q' - i);
        ret = vermicelli16Build(chars, (u8 *)&matches[i]);
        ASSERT_TRUE(ret);
    }

    for (int j = 0; j < 16; ++j) {
        for (size_t i = 0; i < 16; i++) {
            const u8 *rv = rnvermicelli16Exec(matches[j], buf, buf + strlen(t1) - i);
            ASSERT_EQ(buf - j + 32, rv);
        }
    }
}

#endif // HAVE_SVE2
