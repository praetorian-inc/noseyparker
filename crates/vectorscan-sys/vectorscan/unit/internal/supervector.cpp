/*
 * Copyright (c) 2015-2017, Intel Corporation
 * Copyright (c) 2020-2021, VectorCamp PC
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

#include<iostream>
#include<cstring>
#include<time.h>
#include"gtest/gtest.h"
#include"ue2common.h"
#include"util/supervector/supervector.hpp"


TEST(SuperVectorUtilsTest, Zero128c) {
    auto zeroes = SuperVector<16>::Zeroes();
    u8 buf[16]{0};
    for(int i=0; i<16; i++) {
        ASSERT_EQ(zeroes.u.u8[i],buf[i]);
    }
}

TEST(SuperVectorUtilsTest, Ones128c) {
    auto ones = SuperVector<16>::Ones();
    u8 buf[16];
    for (int i=0; i<16; i++) { buf[i]=0xff; }
    for(int i=0; i<16; i++) {
        ASSERT_EQ(ones.u.u8[i],buf[i]);
    }
}

TEST(SuperVectorUtilsTest, Loadu128c) {
    u8 vec[32];
    for(int i=0; i<32;i++) { vec[i]=i; }
    for(int i=0; i<=16;i++) {
        auto SP = SuperVector<16>::loadu(vec+i);
        for(int j=0; j<16; j++) {
            ASSERT_EQ(SP.u.u8[j],vec[j+i]);
        }
    }
}

TEST(SuperVectorUtilsTest, Load128c) {
    u8 ALIGN_ATTR(16) vec[32];
    for(int i=0; i<32;i++) { vec[i]=i; }
    for(int i=0;i<=16;i+=16) {
        auto SP = SuperVector<16>::load(vec+i);
        for(int j=0; j<16; j++){
            ASSERT_EQ(SP.u.u8[j],vec[j+i]);
        }
    }    
}

TEST(SuperVectorUtilsTest,Equal128c){
    u8 vec[32];
     for (int i=0; i<32; i++) {vec[i]=i;};
    auto SP1 = SuperVector<16>::loadu(vec);
    auto SP2 = SuperVector<16>::loadu(vec+16);
    u8 buf[16]={0};
    /*check for equality byte by byte*/
    for (int s=0; s<16; s++){
        if(vec[s]==vec[s+16]){
            buf[s]=1;
        }
    }
    auto SPResult = SP1.eq(SP2);
    for (int i=0; i<16; i++) {
        ASSERT_EQ(SPResult.u.s8[i],buf[i]);
    }
}

TEST(SuperVectorUtilsTest,And128c){
    auto SPResult = SuperVector<16>::Zeroes() & SuperVector<16>::Ones();
    for (int i=0; i<16; i++) {
        ASSERT_EQ(SPResult.u.u8[i],0);
    }
}

TEST(SuperVectorUtilsTest,OPAnd128c){
    auto SP1 = SuperVector<16>::Zeroes(); 
    auto SP2 = SuperVector<16>::Ones();
    SP2 = SP2.opand(SP1);
    for (int i=0; i<16; i++) {
        ASSERT_EQ(SP2.u.u8[i],0);
    }
}

TEST(SuperVectorUtilsTest,OR128c){
    auto SPResult = SuperVector<16>::Zeroes() | SuperVector<16>::Ones();
    for (int i=0; i<16; i++) {
        ASSERT_EQ(SPResult.u.u8[i],0xff);
    }
}

TEST(SuperVectorUtilsTest,XOR128c){
    srand (time(NULL));
    u8 vec[16];
    for (int i=0; i<16; i++) {
        vec[i] = rand() % 100 + 1;
    }
    u8 vec2[16];
    for (int i=0; i<16; i++) {
        vec2[i] = rand() % 100 + 1;
    }
    auto SP1 = SuperVector<16>::loadu(vec);
    auto SP2 = SuperVector<16>::loadu(vec2);
    auto SPResult = SP1 ^ SP2;
    for (int i=0; i<16; i++) {
        ASSERT_EQ(SPResult.u.u8[i],vec[i] ^ vec2[i]);
    }
}


TEST(SuperVectorUtilsTest,OPXOR128c){
    srand (time(NULL));
    u8 vec[16];
    for (int i=0; i<16; i++) {
        vec[i] = rand() % 100 + 1;
    }
    u8 vec2[16];
    for (int i=0; i<16; i++) {
        vec2[i] = rand() % 100 + 1;
    }
    auto SP1 = SuperVector<16>::loadu(vec);
    auto SP2 = SuperVector<16>::loadu(vec2);
    auto SPResult = SP1.opxor(SP2);
    for (int i=0; i<16; i++) {
        ASSERT_EQ(SPResult.u.u8[i],vec[i] ^ vec2[i]);
    }
}

TEST(SuperVectorUtilsTest,OPANDNOT128c){
    auto SP1 = SuperVector<16>::Zeroes(); 
    auto SP2 = SuperVector<16>::Ones();
    SP1 = SP1.opandnot(SP2);
    for (int i=0; i<16; i++) {
        ASSERT_EQ(SP1.u.u8[i],0xff);
    }
    SP2 = SP2.opandnot(SP1);
    for (int i=0; i<16; i++) {
        ASSERT_EQ(SP2.u.u8[i],0);
    }    
}

TEST(SuperVectorUtilsTest,Movemask128c){
    srand (time(NULL));
    u8 vec[16] = {0};
    u8 vec2[16] = {0};
    u16 r = rand() % 100 + 1;
    for(int i=0; i<16; i++) {
        if (r & (1 << i)) {
            vec[i] = 0xff;
        }
    }
    auto SP = SuperVector<16>::loadu(vec);
    u64a mask = SP.comparemask();
    for (int i = 0; i < 16; i++) {
        if (mask & (1ull << (i * SuperVector<16>::mask_width()))) {
            vec2[i] = 0xff;
        }
    }
    for (int i=0; i<16; i++) {
        ASSERT_EQ(vec[i],vec2[i]);
    }
}

TEST(SuperVectorUtilsTest,Eqmask128c){
    srand (time(NULL));
    u8 vec[16];
    for (int i = 0; i<16; i++) { vec[i] = rand() % 64 + 0;}
    u8 vec2[16];
    for (int i = 0; i<16; i++) { vec2[i]= rand() % 100 + 67;}
    auto SP = SuperVector<16>::loadu(vec);
    auto SP1 = SuperVector<16>::loadu(vec2);
    u64a mask = SP.eqmask(SP);
    for (u32 i = 0; i < 16; ++i) {
        ASSERT_TRUE(mask & (1ull << (i * SuperVector<16>::mask_width())));
    }
    mask = SP.eqmask(SP1);
    ASSERT_EQ(mask,0);
    vec2[0] = vec[0];
    vec2[1] = vec[1];
    auto SP2 = SuperVector<16>::loadu(vec2);
    mask = SP.eqmask(SP2);
    ASSERT_TRUE(mask & 1);
    ASSERT_TRUE(mask & (1ull << SuperVector<16>::mask_width()));
    for (u32 i = 2; i < 16; ++i) {
        ASSERT_FALSE(mask & (1ull << (i * SuperVector<16>::mask_width())));
    }
}

/*Define LSHIFT128 macro*/
#define TEST_LSHIFT128(buf, vec, v, l) {                                                  \
                                           auto v_shifted = v << (l);                     \
                                           for (int i=15; i>= l; --i) {                   \
                                               buf[i] = vec[i-l];                         \
                                           }                                              \
                                           for (int i=0; i<l; i++) {                      \
                                               buf[i] = 0;                                \
                                           }                                              \
                                           for(int i=0; i<16; i++) {                      \
                                               ASSERT_EQ(v_shifted.u.u8[i], buf[i]);      \
                                           }                                              \
                                       }

TEST(SuperVectorUtilsTest,LShift128c){
    u8 vec[16];
    for (int i = 0; i<16; i++ ){ vec[i] = i+1; }
    auto SP = SuperVector<16>::loadu(vec);
    u8 buf[16];
    for (int j = 0; j<16; j++) { 
        TEST_LSHIFT128(buf, vec, SP, j);
    }
}

TEST(SuperVectorUtilsTest,LShift64_128c){
    u64a vec[2] = {128, 512};
    auto SP = SuperVector<16>::loadu(vec);
    for(int s = 0; s<16; s++) {
        auto SP_after_shift = SP.vshl_64(s);
        for (int i=0; i<2; i++) {
            ASSERT_EQ(SP_after_shift.u.u64[i], vec[i] << s);
        }
    }   
}

TEST(SuperVectorUtilsTest,RShift64_128c){
    u64a vec[2] = {128, 512};
    auto SP = SuperVector<16>::loadu(vec);
    for(int s = 0; s<16; s++) {
        auto SP_after_shift = SP.vshr_64(s);
        for (int i=0; i<2; i++) {
            ASSERT_EQ(SP_after_shift.u.u64[i], vec[i] >> s);
        }
    }   
}

/*Define RSHIFT128 macro*/
#define TEST_RSHIFT128(buf, vec, v, l) {                                                  \
                                           auto v_shifted = v >> (l);                     \
                                           for (int i=0; i<16-l; i++) {                   \
                                               buf[i] = vec[i+l];                         \
                                           }                                              \
                                           for (int i=16-l; i<16; i++) {                  \
                                               buf[i] = 0;                                \
                                           }                                              \
                                           for(int i=0; i<16; i++) {                      \
                                               ASSERT_EQ(v_shifted.u.u8[i], buf[i]);      \
                                           }                                              \
                                       }

TEST(SuperVectorUtilsTest,RShift128c){
    u8 vec[16];
    for (int i = 0; i<16; i++ ){ vec[i] = i+1; }
    auto SP = SuperVector<16>::loadu(vec);
    u8 buf[16];
    for (int j = 0; j<16; j++) { 
        TEST_RSHIFT128(buf, vec, SP, j);
    }
}

TEST(SuperVectorUtilsTest,pshufb128c) {
    srand (time(NULL));
    u8 vec[16];
    for (int i=0; i<16; i++) {
        vec[i] = rand() % 100 + 1;
    }
    u8 vec2[16];
    for (int i=0; i<16; i++) {
        vec2[i]=i + (rand() % 15 + 0);
    }
    auto SP1 = SuperVector<16>::loadu(vec);
    auto SP2 = SuperVector<16>::loadu(vec2);
    auto SResult = SP1.template pshufb<true>(SP2);
    for (int i=0; i<16; i++) {
	if(vec2[i] & 0x80){
	   ASSERT_EQ(SResult.u.u8[i], 0);
	}else{
           ASSERT_EQ(vec[vec2[i] % 16 ],SResult.u.u8[i]);
	}
    }
}


/*Define LSHIFT128_128 macro*/
#define TEST_LSHIFT128_128(buf, vec, v, l) {                                              \
                                           auto v_shifted = v.vshl_128(l);                \
                                           for (int i=15; i>= l; --i) {                   \
                                               buf[i] = vec[i-l];                         \
                                           }                                              \
                                           for (int i=0; i<l; i++) {                      \
                                               buf[i] = 0;                                \
                                           }                                              \
                                           for(int i=0; i<16; i++) {                      \
                                               ASSERT_EQ(v_shifted.u.u8[i], buf[i]);      \
                                           }                                              \
                                       }

TEST(SuperVectorUtilsTest,LShift128_128c){
    u8 vec[16];
    for (int i = 0; i<16; i++ ){ vec[i] = i+1; }
    auto SP = SuperVector<16>::loadu(vec);
    u8 buf[16];
    for (int j = 0; j<16; j++) { 
        TEST_LSHIFT128_128(buf, vec, SP, j);
    }   
}

/*Define RSHIFT128_128 macro*/
#define TEST_RSHIFT128_128(buf, vec, v, l) {                                              \
                                           auto v_shifted = v.vshr_128(l);                \
                                           for (int i=0; i<16-l; i++) {                   \
                                               buf[i] = vec[i+l];                         \
                                           }                                              \
                                           for (int i=16-l; i<16; i++) {                  \
                                               buf[i] = 0;                                \
                                           }                                              \
                                           for(int i=0; i<16; i++) {                      \
                                               ASSERT_EQ(v_shifted.u.u8[i], buf[i]);      \
                                           }                                              \
                                       }

TEST(SuperVectorUtilsTest,RShift128_128c){
    u8 vec[16];
    for (int i = 0; i<16; i++ ){ vec[i] = i+1; }
    auto SP = SuperVector<16>::loadu(vec);
    u8 buf[16];
    for (int j = 0; j<16; j++) { 
        TEST_RSHIFT128_128(buf, vec, SP, j);
    }
}

/*Define ALIGNR128 macro*/
#define TEST_ALIGNR128(v1, v2, buf, l) {                                                 \
                                           auto v_aligned = v2.alignr(v1, l);            \
                                           for (size_t i=0; i<16; i++) {                 \
                                               ASSERT_EQ(v_aligned.u.u8[i], vec[i + l]); \
                                           }                                             \
                                       }

TEST(SuperVectorUtilsTest,Alignr128c){
    u8 vec[32];
    for (int i=0; i<32; i++) {
        vec[i]=i;
    }
    auto SP1 = SuperVector<16>::loadu(vec);
    auto SP2 = SuperVector<16>::loadu(vec+16);
    for (int j = 0; j<16; j++){
        TEST_ALIGNR128(SP1, SP2, vec, j);
    }
}



#if defined(HAVE_AVX2)
TEST(SuperVectorUtilsTest, Zero256c) {
    auto zeroes = SuperVector<32>::Zeroes();
    u8 buf[32]{0};
    for(int i=0; i<32; i++) {
        ASSERT_EQ(zeroes.u.u8[i],buf[i]);
    }
}

TEST(SuperVectorUtilsTest, Ones256c) {
    auto ones = SuperVector<32>::Ones();
    u8 buf[32];
    for (int i=0; i<32; i++) { buf[i]=0xff; }
    for(int i=0; i<32; i++) {
        ASSERT_EQ(ones.u.u8[i],buf[i]);
    }
}

TEST(SuperVectorUtilsTest, Loadu256c) {
    u8 vec[64];
    for(int i=0; i<64;i++) { vec[i]=i; }
    for(int i=0; i<=32;i++) {
        auto SP = SuperVector<32>::loadu(vec+i);
        for(int j=0; j<32; j++) {
            ASSERT_EQ(SP.u.u8[j],vec[j+i]);
        }
    }
}

TEST(SuperVectorUtilsTest, Load256c) {
    u8 ALIGN_ATTR(32) vec[64];
    for(int i=0; i<64;i++) { vec[i]=i; }
    for(int i=0;i<=32;i+=32) {
        auto SP = SuperVector<32>::load(vec+i);
        for(int j=0; j<32; j++){
            ASSERT_EQ(SP.u.u8[j],vec[j+i]);
        }
    }    
}

TEST(SuperVectorUtilsTest,Equal256c){
    u8 vec[64];
     for (int i=0; i<64; i++) {vec[i]=i;};
    auto SP1 = SuperVector<32>::loadu(vec);
    auto SP2 = SuperVector<32>::loadu(vec+32);
    u8 buf[32]={0};
    /*check for equality byte by byte*/
    for (int s=0; s<32; s++){
        if(vec[s]==vec[s+32]){
            buf[s]=1;
        }
    }
    auto SPResult = SP1.eq(SP2);
    for (int i=0; i<32; i++) {
        ASSERT_EQ(SPResult.u.s8[i],buf[i]);
    }
}

TEST(SuperVectorUtilsTest,And256c){
    auto SPResult = SuperVector<32>::Zeroes() & SuperVector<32>::Ones();
    for (int i=0; i<32; i++) {
        ASSERT_EQ(SPResult.u.u8[i],0);
    }
}

TEST(SuperVectorUtilsTest,OPAnd256c){
    auto SP1 = SuperVector<32>::Zeroes(); 
    auto SP2 = SuperVector<32>::Ones();
    SP2 = SP2.opand(SP1);
    for (int i=0; i<32; i++) {
        ASSERT_EQ(SP2.u.u8[i],0);
    }
}

TEST(SuperVectorUtilsTest,OR256c){
    auto SPResult = SuperVector<32>::Zeroes() | SuperVector<32>::Ones();
    for (int i=0; i<32; i++) {
        ASSERT_EQ(SPResult.u.u8[i],0xff);
    }
}

TEST(SuperVectorUtilsTest,XOR256c){
    srand (time(NULL));
    u8 vec[32];
    for (int i=0; i<32; i++) {
        vec[i] = rand() % 100 + 1;
    }
    u8 vec2[32];
    for (int i=0; i<32; i++) {
        vec2[i] = rand() % 100 + 1;
    }
    auto SP1 = SuperVector<32>::loadu(vec);
    auto SP2 = SuperVector<32>::loadu(vec2);
    auto SPResult = SP1 ^ SP2;
    for (int i=0; i<32; i++) {
        ASSERT_EQ(SPResult.u.u8[i],vec[i] ^ vec2[i]);
    }
}


TEST(SuperVectorUtilsTest,OPXOR256c){
    srand (time(NULL));
    u8 vec[32];
    for (int i=0; i<32; i++) {
        vec[i] = rand() % 100 + 1;
    }
    u8 vec2[32];
    for (int i=0; i<32; i++) {
        vec2[i] = rand() % 100 + 1;
    }
    auto SP1 = SuperVector<32>::loadu(vec);
    auto SP2 = SuperVector<32>::loadu(vec2);
    auto SPResult = SP1.opxor(SP2);
    for (int i=0; i<32; i++) {
        ASSERT_EQ(SPResult.u.u8[i],vec[i] ^ vec2[i]);
    }
}

TEST(SuperVectorUtilsTest,OPANDNOT256c){
    auto SP1 = SuperVector<32>::Zeroes(); 
    auto SP2 = SuperVector<32>::Ones();
    SP2 = SP2.opandnot(SP1);
    for (int i=0; i<32; i++) {
        ASSERT_EQ(SP2.u.s8[i],0);
    }
}

TEST(SuperVectorUtilsTest,Movemask256c){
    srand (time(NULL));
    u8 vec[32] = {0};
    u8 vec2[32] = {0};
    u32 r = rand() % 100 + 1;
    for(int i=0; i<32; i++) {
        if (r & (1 << i)) {
            vec[i] = 0xff;
        }
    }
    auto SP = SuperVector<32>::loadu(vec);
    u64a mask = SP.comparemask();
    for(int i=0; i<32; i++) {
        if (mask & (1ull << (i * SuperVector<32>::mask_width()))) {
            vec2[i] = 0xff;
        }
    }
    for (int i=0; i<32; i++) {
        ASSERT_EQ(vec[i],vec2[i]);
    }
}


TEST(SuperVectorUtilsTest,Eqmask256c){
    srand (time(NULL));
    u8 vec[32];
    for (int i = 0; i<32; i++) { vec[i] = rand() % 64 + 0;}
    u8 vec2[32];
    for (int i = 0; i<32; i++) { vec2[i]= rand() % 100 + 67;}
    auto SP = SuperVector<32>::loadu(vec);
    auto SP1 = SuperVector<32>::loadu(vec2);
    u64a mask = SP.eqmask(SP);
    for (u32 i = 0; i < 32; ++i) {
        ASSERT_TRUE(mask & (1ull << (i * SuperVector<32>::mask_width())));
    }
    mask = SP.eqmask(SP1);
    ASSERT_EQ(mask,0);
    vec2[0] = vec[0];
    vec2[1] = vec[1];
    auto SP2 = SuperVector<32>::loadu(vec2);
    mask = SP.eqmask(SP2);
    ASSERT_TRUE(mask & 1);
    ASSERT_TRUE(mask & (1ull << SuperVector<32>::mask_width()));
    for (u32 i = 2; i < 32; ++i) {
        ASSERT_FALSE(mask & (1ull << (i * SuperVector<32>::mask_width())));
    }
}

TEST(SuperVectorUtilsTest,pshufb256c) {
    srand (time(NULL));
    u8 vec[32];
    for (int i=0; i<32; i++) {
        vec[i] = rand() % 100 + 1;
    }
    u8 vec2[32];
    for (int i=0; i<32; i++) {
        vec2[i]=i;
    }
    auto SP1 = SuperVector<32>::loadu(vec);
    auto SP2 = SuperVector<32>::loadu(vec2);
    auto SResult = SP1.pshufb(SP2);
    for (int i=0; i<32; i++) {
        ASSERT_EQ(vec[vec2[i]],SResult.u.u8[i]);
    }
}


/*Define LSHIFT256 macro*/
#define TEST_LSHIFT256(buf, vec, v, l) {                                                  \
                                           auto v_shifted = v << (l);                     \
                                           for (int i=31; i>= l; --i) {                   \
                                               buf[i] = vec[i-l];                         \
                                           }                                              \
                                           for (int i=0; i<l; i++) {                      \
                                               buf[i] = 0;                                \
                                           }                                              \
                                           for(int i=0; i<32; i++) {                      \
                                               ASSERT_EQ(v_shifted.u.u8[i], buf[i]);      \
                                           }                                              \
                                       }

TEST(SuperVectorUtilsTest,LShift256c){
    u8 vec[32];
    for (int i = 0; i<32; i++) { vec[i]= i+1;}
    auto SP = SuperVector<32>::loadu(vec);
    u8 buf[32];
    for (int j = 0; j<32; j++) { 
        TEST_LSHIFT256(buf, vec, SP, j);
    }
}


TEST(SuperVectorUtilsTest,LShift64_256c){
    u64a vec[4] = {128, 512, 256, 1024};
    auto SP = SuperVector<32>::loadu(vec);
    for(int s = 0; s<32; s++) {
        auto SP_after_shift = SP.vshl_64(s);
        for (int i=0; i<4; i++) {
            ASSERT_EQ(SP_after_shift.u.u64[i], vec[i] << s);
        }
    }   
}

TEST(SuperVectorUtilsTest,RShift64_256c){
    u64a vec[4] = {128, 512, 256, 1024};
    auto SP = SuperVector<32>::loadu(vec);
    for(int s = 0; s<32; s++) {
        auto SP_after_shift = SP.vshr_64(s);
        for (int i=0; i<4; i++) {
            ASSERT_EQ(SP_after_shift.u.u64[i], vec[i] >> s);
        }
    }   
}


/*Define RSHIFT256 macro*/
#define TEST_RSHIFT256(buf, vec, v, l) {                                                  \
                                           auto v_shifted = v >> (l);                     \
                                           for (int i=0; i<32-l; i++) {                   \
                                               buf[i] = vec[i+l];                         \
                                           }                                              \
                                           for (int i=32-l; i<32; i++) {                  \
                                               buf[i] = 0;                                \
                                           }                                              \
                                           for(int i=0; i<32; i++) {                      \
                                               ASSERT_EQ(v_shifted.u.u8[i], buf[i]);      \
                                           }                                              \
                                       }

TEST(SuperVectorUtilsTest,RShift256c){
    u8 vec[32];
    for (int i = 0; i<32; i++) { vec[i]= i+1;}
    auto SP = SuperVector<32>::loadu(vec);
    u8 buf[32];
    for (int j = 0; j<32; j++) { 
        TEST_RSHIFT256(buf, vec, SP, j);
    }
}





/*Define LSHIFT128_256 macro*/
#define TEST_LSHIFT128_256(buf, vec, v, l) {                                              \
                                           auto v_shifted = v.vshl_128(l);                \
                                           for (int i=15; i>= l; --i) {                   \
                                               buf[i] = vec[i-l];                         \
                                               buf[i+16] = vec[(16+i)-l];                 \
                                           }                                              \
                                           for (int i=0; i<l; i++) {                      \
                                               buf[i] = 0;                                \
                                               buf[i+16]= 0;                              \
                                           }                                              \
                                           for(int i=0; i<32; i++) {                      \
                                               ASSERT_EQ(v_shifted.u.u8[i], buf[i]);      \
                                           }                                              \
                                       }

TEST(SuperVectorUtilsTest,LShift128_256c){
    u8 vec[32];
    for (int i = 0; i<32; i++) { vec[i]= i+1;}
    auto SP = SuperVector<32>::loadu(vec);
    u8 buf[32];
    for (int j=0; j<16; j++) {
        TEST_LSHIFT128_256(buf, vec, SP, j);
    }
}

/*Define RSHIFT128_128 macro*/
#define TEST_RSHIFT128_256(buf, vec, v, l) {                                              \
                                           auto v_shifted = v.vshr_128(l);                \
                                           for (int i=0; i<16-l; i++) {                   \
                                               buf[i] = vec[i+l];                         \
                                               buf[i+16] = vec[(i+16)+l];                 \
                                           }                                              \
                                           for (int i=16-l; i<16; i++) {                  \
                                               buf[i] = 0;                                \
                                               buf[i+16] = 0;                             \
                                           }                                              \
                                           for(int i=0; i<32; i++) {                      \
                                               ASSERT_EQ(v_shifted.u.u8[i], buf[i]);      \
                                           }                                              \
                                       }

TEST(SuperVectorUtilsTest,RShift128_256c){
    u8 vec[32];
    for (int i = 0; i<32; i++ ){ vec[i] = i+1; }
    auto SP = SuperVector<32>::loadu(vec);
    u8 buf[32];
    for(int j=0; j<16; j++) {
        TEST_RSHIFT128_256(buf, vec, SP, j);
    }
}


/*Define ALIGNR256 macro*/
#define TEST_ALIGNR256(v1, v2, buf, l) {                                                  \
                                           auto v_aligned = v2.alignr(v1, l);             \
                                           for (size_t i=0; i<32; i++) {                  \
                                               ASSERT_EQ(v_aligned.u.u8[i], vec[i + l]);  \
                                           }                                              \
                                       }

TEST(SuperVectorUtilsTest,Alignr256c){
    u8 vec[64];
    for (int i=0; i<64; i++) {
        vec[i]=i;
    }
    auto SP1 = SuperVector<32>::loadu(vec);
    auto SP2 = SuperVector<32>::loadu(vec+32);
    for(int j=0; j<32; j++) {
        TEST_ALIGNR256(SP1, SP2, vec, j);
    }
}

#endif // HAVE_AVX2


#if defined(HAVE_AVX512)

TEST(SuperVectorUtilsTest, Zero512c) {
    auto zeroes = SuperVector<64>::Zeroes();
    u8 buf[64]{0};
    for(int i=0; i<64; i++) {
        ASSERT_EQ(zeroes.u.u8[i],buf[i]);
    }
}

TEST(SuperVectorUtilsTest, Ones512c) {
    auto ones = SuperVector<64>::Ones();
    u8 buf[64];
    for (int i=0; i<64; i++) { buf[i]=0xff; }
    for(int i=0; i<64; i++) {
        ASSERT_EQ(ones.u.u8[i],buf[i]);
    }
}

TEST(SuperVectorUtilsTest, Loadu512c) {
    u8 vec[128];
    for(int i=0; i<128;i++) { vec[i]=i; }
    for(int i=0; i<=64;i++) {
        auto SP = SuperVector<64>::loadu(vec+i);
        for(int j=0; j<64; j++) {
            ASSERT_EQ(SP.u.u8[j],vec[j+i]);
        }
    }
}

TEST(SuperVectorUtilsTest, Load512c) {
    u8 ALIGN_ATTR(64) vec[128];
    for(int i=0; i<128;i++) { vec[i]=i; }
    for(int i=0;i<=64;i+=64) {
        auto SP = SuperVector<64>::load(vec+i);
        for(int j=0; j<64; j++){
            ASSERT_EQ(SP.u.u8[j],vec[j+i]);
        }
    }    
}

TEST(SuperVectorUtilsTest,Equal512c){
    u8 vec[128];
     for (int i=0; i<128; i++) {vec[i]=i;};
    auto SP1 = SuperVector<64>::loadu(vec);
    auto SP2 = SuperVector<64>::loadu(vec+64);
    u8 buf[64]={0};
    /*check for equality byte by byte*/
    for (int s=0; s<64; s++){
        if(vec[s]==vec[s+64]){
            buf[s]=1;
        }
    }
    auto SPResult = SP1.eq(SP2);
    for (int i=0; i<64; i++) {
        ASSERT_EQ(SPResult.u.s8[i],buf[i]);
    }
}

TEST(SuperVectorUtilsTest,And512c){
    auto SPResult = SuperVector<64>::Zeroes() & SuperVector<64>::Ones();
    for (int i=0; i<32; i++) {
        ASSERT_EQ(SPResult.u.u8[i],0);
    }
}

TEST(SuperVectorUtilsTest,OPAnd512c){
    auto SP1 = SuperVector<64>::Zeroes(); 
    auto SP2 = SuperVector<64>::Ones();
    SP2 = SP2.opand(SP1);
    for (int i=0; i<64; i++) {
        ASSERT_EQ(SP2.u.u8[i],0);
    }
}

TEST(SuperVectorUtilsTest,OR512c){
    auto SPResult = SuperVector<64>::Zeroes() | SuperVector<64>::Ones();
    for (int i=0; i<64; i++) {
        ASSERT_EQ(SPResult.u.u8[i],0xff);
    }
}

TEST(SuperVectorUtilsTest,XOR512c){
    srand (time(NULL));
    u8 vec[64];
    for (int i=0; i<64; i++) {
        vec[i] = rand() % 100 + 1;
    }
    u8 vec2[64];
    for (int i=0; i<64; i++) {
        vec2[i] = rand() % 100 + 1;
    }
    auto SP1 = SuperVector<64>::loadu(vec);
    auto SP2 = SuperVector<64>::loadu(vec2);
    auto SPResult = SP1 ^ SP2;
    for (int i=0; i<64; i++) {
        ASSERT_EQ(SPResult.u.u8[i],vec[i] ^ vec2[i]);
    }
}


TEST(SuperVectorUtilsTest,OPXOR512c){
    srand (time(NULL));
    u8 vec[64];
    for (int i=0; i<64; i++) {
        vec[i] = rand() % 100 + 1;
    }
    u8 vec2[64];
    for (int i=0; i<64; i++) {
        vec2[i] = rand() % 100 + 1;
    }
    auto SP1 = SuperVector<64>::loadu(vec);
    auto SP2 = SuperVector<64>::loadu(vec2);
    auto SPResult = SP1.opxor(SP2);
    for (int i=0; i<64; i++) {
        ASSERT_EQ(SPResult.u.u8[i],vec[i] ^ vec2[i]);
    }
}

TEST(SuperVectorUtilsTest,OPANDNOT512c){
    auto SP1 = SuperVector<64>::Zeroes(); 
    auto SP2 = SuperVector<64>::Ones();
    SP2 = SP2.opandnot(SP1);
    for (int i=0; i<64; i++) {
        ASSERT_EQ(SP2.u.s8[i],0);
    }
}


TEST(SuperVectorUtilsTest,Movemask512c){
    srand (time(NULL));
    u8 vec[64] = {0};
    u64a r = rand() % 100 + 1;
    for(int i=0; i<64; i++) {
        if (r & (1ULL << i)) {
            vec[i] = 0xff;
        }
    }
    auto SP = SuperVector<64>::loadu(vec);
    u8 vec2[64] = {0};
    u64a mask = SP.comparemask();
    for(int i=0; i<64; i++) {
        if (mask & (1ULL << i)) {
            vec2[i] = 0xff;
        }
    }
    for (int i=0; i<64; i++){
        //printf("%d)  vec =%i , vec2 = %i \n",i,vec[i],vec2[i]);
        ASSERT_EQ(vec[i],vec2[i]);
    }
}


TEST(SuperVectorUtilsTest,Eqmask512c){
    srand (time(NULL));
    u8 vec[64];
    for (int i = 0; i<64; i++) { vec[i] = rand() % 64 + 0;}
    u8 vec2[64];
    for (int i = 0; i<64; i++) { vec2[i]= rand() % 100 + 67;}
    auto SP = SuperVector<64>::loadu(vec);
    auto SP1 = SuperVector<64>::loadu(vec2);
    u64a mask = SP.eqmask(SP);
    // Mask width for 64 bit type cannot be more than 1.
    ASSERT_EQ(SuperVector<64>::mask_width(), 1);
    ASSERT_EQ(mask,0xFFFFFFFFFFFFFFFF);
    mask = SP.eqmask(SP1);
    ASSERT_EQ(mask,0);
    vec2[0] = vec[0];
    vec2[1] = vec[1];
    auto SP2 = SuperVector<64>::loadu(vec2);
    mask = SP.eqmask(SP2);
    ASSERT_EQ(mask,3);
}

TEST(SuperVectorUtilsTest,pshufb512c) {
    srand (time(NULL));
    u8 vec[64];
    for (int i=0; i<64; i++) {
        vec[i] = rand() % 100 + 1;
    }
    u8 vec2[64];
    for (int i=0; i<64; i++) {
        vec2[i]=i;
    }
    auto SP1 = SuperVector<64>::loadu(vec);
    auto SP2 = SuperVector<64>::loadu(vec2);
    auto SResult = SP1.pshufb(SP2);
    for (int i=0; i<64; i++) {
        ASSERT_EQ(vec[vec2[i]],SResult.u.u8[i]);
    }
}

/*Define LSHIFT512 macro*/
#define TEST_LSHIFT512(buf, vec, v, l) {                                                  \
                                           auto v_shifted = v << (l);                     \
                                           for (int i=63; i>= l; --i) {                   \
                                               buf[i] = vec[i-l];                         \
                                           }                                              \
                                           for (int i=0; i<l; i++) {                      \
                                               buf[i] = 0;                                \
                                           }                                              \
                                           for(int i=0; i<64; i++) {                      \
                                               ASSERT_EQ(v_shifted.u.u8[i], buf[i]);      \
                                           }                                              \
                                       }

TEST(SuperVectorUtilsTest,LShift512c){
    u8 vec[64];
    for (int i = 0; i<64; i++) { vec[i]= i+1;}
    auto SP = SuperVector<64>::loadu(vec);
    u8 buf[64];
    for (int j = 0; j<64; j++) { 
        TEST_LSHIFT512(buf, vec, SP, j);
    }
}


TEST(SuperVectorUtilsTest,LShift64_512c){
    u64a vec[8] = {32, 64, 128, 256, 512, 512, 256, 1024};
    auto SP = SuperVector<64>::loadu(vec);
    for(int s = 0; s<64; s++) {
        auto SP_after_shift = SP.vshl_64(s);
        for (int i=0; i<8; i++) {
            ASSERT_EQ(SP_after_shift.u.u64[i], vec[i] << s);
        }
    }   
}

TEST(SuperVectorUtilsTest,RShift64_512c){
    u64a vec[8] = {32, 64, 128, 256, 512, 512, 256, 1024};
    auto SP = SuperVector<64>::loadu(vec);
    for(int s = 0; s<64; s++) {
        auto SP_after_shift = SP.vshr_64(s);
        for (int i=0; i<8; i++) {
            ASSERT_EQ(SP_after_shift.u.u64[i], vec[i] >> s);
        }
    }   
}


/*Define RSHIFT512 macro*/
#define TEST_RSHIFT512(buf, vec, v, l) {                                                  \
                                           auto v_shifted = v >> (l);                     \
                                           for (int i=0; i<64-l; i++) {                   \
                                               buf[i] = vec[i+l];                         \
                                           }                                              \
                                           for (int i=64-l; i<64; i++) {                  \
                                               buf[i] = 0;                                \
                                           }                                              \
                                           for(int i=0; i<64; i++) {                      \
                                               ASSERT_EQ(v_shifted.u.u8[i], buf[i]);      \
                                           }                                              \
                                       }

TEST(SuperVectorUtilsTest,RShift512c){
    u8 vec[64];
    for (int i = 0; i<64; i++) { vec[i]= i+1;}
    auto SP = SuperVector<64>::loadu(vec);
    u8 buf[64];
    for (int j = 0; j<64; j++) { 
        TEST_RSHIFT512(buf, vec, SP, j);
    }
}


/*Define RSHIFT128_512 macro*/
#define TEST_RSHIFT128_512(buf, vec, v, l) {                                              \
                                           auto v_shifted = v.vshr_128(l);                \
                                           for (int i=0; i<16-l; i++) {                   \
                                               buf[i] = vec[i+l];                         \
                                               buf[i+16] = vec[(i+16)+l];                 \
                                               buf[i+32] = vec[(i+32)+l];                 \
                                               buf[i+48] = vec[(i+48)+l];                 \
                                           }                                              \
                                           for (int i=16-l; i<16; i++) {                  \
                                               buf[i] = 0;                                \
                                               buf[i+16] = 0;                             \
                                               buf[i+32] = 0;                             \
                                               buf[i+48] = 0;                             \
                                           }                                              \
                                           for(int i=0; i<64; i++) {                      \
                                               ASSERT_EQ(v_shifted.u.u8[i], buf[i]);      \
                                           }                                              \
                                       }
TEST(SuperVectorUtilsTest,RShift128_512c){
    u8 vec[64];
    for (int i = 0; i<64; i++ ){ vec[i] = i+1; }
    auto SP = SuperVector<64>::loadu(vec);
    u8 buf[64] = {1};
    for(int j=0; j<16; j++){
        TEST_RSHIFT128_512(buf, vec, SP, j)
    }      
}

/*Define LSHIFT512 macro*/
#define TEST_LSHIFT128_512(buf, vec, v, l) {                                              \
                                           auto v_shifted = v.vshl_128(l);                \
                                           for (int i=15; i>=l; --i) {                    \
                                               buf[i] = vec[i-l];                         \
                                               buf[i+16] = vec[(i+16)-l];                 \
                                               buf[i+32] = vec[(i+32)-l];                 \
                                               buf[i+48] = vec[(i+48)-l];                 \
                                           }                                              \
                                           for (int i=0; i<l; i++) {                      \
                                               buf[i] = 0;                                \
                                               buf[i+16] = 0;                             \
                                               buf[i+32] = 0;                             \
                                               buf[i+48] = 0;                             \
                                           }                                              \
                                           for(int i=0; i<64; i++) {                      \
                                               ASSERT_EQ(v_shifted.u.u8[i], buf[i]);      \
                                           }                                              \
                                       }

TEST(SuperVectorUtilsTest,LShift128_512c){
    u8 vec[64];
    for (int i = 0; i<64; i++) { vec[i]= i+1;}
    auto SP = SuperVector<64>::loadu(vec);
    u8 buf[64] = {1};
    for(int j=0; j<16;j++){
        TEST_LSHIFT128_512(buf, vec, SP, j);
    }
}


/*Define ALIGNR512 macro*/
#define TEST_ALIGNR512(v1, v2, buf, l) {                                                 \
                                           auto v_aligned = v1.alignr(v2, l);            \
                                           for (size_t i=0; i<64; i++) {                 \
                                               ASSERT_EQ(v_aligned.u.u8[i], vec[i + l]); \
                                           }                                             \
                                       }

TEST(SuperVectorUtilsTest,Alignr512c){
    u8 vec[128];
    for (int i=0; i<128; i++) {
        vec[i]=i;
    }
    auto SP1 = SuperVector<64>::loadu(vec);
    auto SP2 = SuperVector<64>::loadu(vec+64);
    for(int j=0; j<64; j++){
        TEST_ALIGNR512(SP1, SP2, vec, j);
    }
}

#endif // HAVE_AVX512
