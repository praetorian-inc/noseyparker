/*
 * Copyright (c) 2020, 2021, VectorCamp PC
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

#include <iostream>
#include <chrono>
#include <cstring>
#include <ctime>
#include <cstdlib>
#include <memory>
#include <functional>

#include "benchmarks.hpp"

#define MAX_LOOPS    1000000000
#define MAX_MATCHES  5
#define N            8

struct hlmMatchEntry {
    size_t to;
    u32 id;
    hlmMatchEntry(size_t end, u32 identifier) :
            to(end), id(identifier) {}
};

std::vector<hlmMatchEntry> ctxt;

static
hwlmcb_rv_t hlmSimpleCallback(size_t to, u32 id,
                              UNUSED struct hs_scratch *scratch) {
    DEBUG_PRINTF("match @%zu = %u\n", to, id);

    ctxt.push_back(hlmMatchEntry(to, id));

    return HWLM_CONTINUE_MATCHING;
}

template<typename InitFunc, typename BenchFunc>
static void run_benchmarks(int size, int loops, int max_matches, bool is_reverse, MicroBenchmark &bench, InitFunc &&init, BenchFunc &&func) {
    init(bench);
    double total_sec = 0.0;            
    u64a total_size = 0;
    double bw = 0.0;
    double avg_bw = 0.0;
    double max_bw = 0.0;
    double avg_time = 0.0;
    if (max_matches) {
        int pos = 0;
        for(int j = 0; j < max_matches - 1; j++) {
            bench.buf[pos] = 'b';
            pos = (j+1) *size / max_matches ;
            bench.buf[pos] = 'a';
            u64a actual_size = 0;
            auto start = std::chrono::steady_clock::now();
            for(int i = 0; i < loops; i++) { 
                const u8 *res = func(bench);
		if (is_reverse)
		   actual_size += bench.buf.data() + size - res;
		else
                   actual_size += res - bench.buf.data();
            }
            auto end = std::chrono::steady_clock::now();
            double dt = std::chrono::duration_cast<std::chrono::microseconds>(end - start).count();
            total_sec += dt;
            /*convert microseconds to seconds*/
            /*calculate bandwidth*/
            bw  = (actual_size / dt) * 1000000.0 / 1048576.0;
	    /*std::cout << "act_size = " << act_size << std::endl;
	    std::cout << "dt = " << dt << std::endl;
	    std::cout << "bw = " << bw << std::endl;*/
	    avg_bw += bw;
            /*convert to MB/s*/
            max_bw = std::max(bw, max_bw);
            /*calculate average time*/
            avg_time += total_sec / loops;
        }
        avg_time /= max_matches;
        avg_bw /= max_matches;
	total_sec /= 1000000.0;
        /*convert average time to us*/
        printf(KMAG "%s: %u matches, %u * %u iterations," KBLU " total elapsed time =" RST " %.3f s, " 
               KBLU "average time per call =" RST " %.3f μs," KBLU " max bandwidth = " RST " %.3f MB/s," KBLU " average bandwidth =" RST " %.3f MB/s \n",
               bench.label, max_matches, size ,loops, total_sec, avg_time, max_bw, avg_bw);
    } else {
        auto start = std::chrono::steady_clock::now();
        for (int i = 0; i < loops; i++) {
            const u8 *res = func(bench);
        }
        auto end = std::chrono::steady_clock::now();
        total_sec += std::chrono::duration_cast<std::chrono::microseconds>(end - start).count();
        /*calculate transferred size*/
        total_size = size * loops;
        /*calculate average time*/
        avg_time = total_sec / loops;
        /*convert microseconds to seconds*/
        total_sec /= 1000000.0;
        /*calculate maximum bandwidth*/
        max_bw = total_size / total_sec;
        /*convert to MB/s*/
        max_bw /= 1048576.0;
        printf(KMAG "%s: no matches, %u * %u iterations," KBLU " total elapsed time =" RST " %.3f s, " 
               KBLU "average time per call =" RST " %.3f μs ," KBLU " bandwidth = " RST " %.3f MB/s \n",
               bench.label, size ,loops, total_sec, avg_time, max_bw );
    }
}

int main(){
    int matches[] = {0, MAX_MATCHES};
    std::vector<size_t> sizes;
    for (size_t i = 0; i < N; i++) sizes.push_back(16000 << i*2);
    const char charset[] = "aAaAaAaAAAaaaaAAAAaaaaAAAAAAaaaAAaaa"; 
  
    for (int m = 0; m < 2; m++) {
        for (size_t i = 0; i < std::size(sizes); i++) {
            MicroBenchmark bench("Shufti", sizes[i]);
            run_benchmarks(sizes[i], MAX_LOOPS / sizes[i], matches[m], false, bench,
                [&](MicroBenchmark &b) {
                    b.chars.set('a');
                    ue2::shuftiBuildMasks(b.chars, (u8 *)&b.lo, (u8 *)&b.hi);
                    memset(b.buf.data(), 'b', b.size);
                },
                [&](MicroBenchmark &b) {
                    return shuftiExec(b.lo, b.hi, b.buf.data(), b.buf.data() + b.size);
                }
            );
        }

        for (size_t i = 0; i < std::size(sizes); i++) {
            MicroBenchmark bench("Reverse Shufti", sizes[i]);
            run_benchmarks(sizes[i], MAX_LOOPS / sizes[i], matches[m], true, bench,
                [&](MicroBenchmark &b) {
                    b.chars.set('a');
                    ue2::shuftiBuildMasks(b.chars, (u8 *)&b.lo, (u8 *)&b.hi);
                    memset(b.buf.data(), 'b', b.size);
                },
                [&](MicroBenchmark &b) {
                    return rshuftiExec(b.lo, b.hi, b.buf.data(), b.buf.data() + b.size);
                }
            );
        }

        for (size_t i = 0; i < std::size(sizes); i++) {
            MicroBenchmark bench("Truffle", sizes[i]);
            run_benchmarks(sizes[i], MAX_LOOPS / sizes[i], matches[m], false, bench,
                [&](MicroBenchmark &b) {
                    b.chars.set('a');
                    ue2::truffleBuildMasks(b.chars, (u8 *)&b.lo, (u8 *)&b.hi);
                    memset(b.buf.data(), 'b', b.size);
                },
                [&](MicroBenchmark &b) {
                    return truffleExec(b.lo, b.hi, b.buf.data(), b.buf.data() + b.size);
                }
            );
        }

        for (size_t i = 0; i < std::size(sizes); i++) {
            MicroBenchmark bench("Reverse Truffle", sizes[i]);
            run_benchmarks(sizes[i], MAX_LOOPS / sizes[i], matches[m], true, bench,
                [&](MicroBenchmark &b) {
                    b.chars.set('a');
                    ue2::truffleBuildMasks(b.chars, (u8 *)&b.lo, (u8 *)&b.hi);
                    memset(b.buf.data(), 'b', b.size);
                },
                [&](MicroBenchmark &b) {
                    return rtruffleExec(b.lo, b.hi, b.buf.data(), b.buf.data() + b.size);
                }
            );
        }

        for (size_t i = 0; i < std::size(sizes); i++) {
            MicroBenchmark bench("Vermicelli", sizes[i]);
            run_benchmarks(sizes[i], MAX_LOOPS / sizes[i], matches[m], false, bench,
                [&](MicroBenchmark &b) {
                    b.chars.set('a');
                    ue2::truffleBuildMasks(b.chars, (u8 *)&b.lo, (u8 *)&b.hi);
                    memset(b.buf.data(), 'b', b.size);
                },
                [&](MicroBenchmark &b) {
                    return vermicelliExec('a', 'b', b.buf.data(), b.buf.data() + b.size);
                }
            );
        }

        for (size_t i = 0; i < std::size(sizes); i++) {
            MicroBenchmark bench("Reverse Vermicelli", sizes[i]);
            run_benchmarks(sizes[i], MAX_LOOPS / sizes[i], matches[m], true, bench,
                [&](MicroBenchmark &b) {
                    b.chars.set('a');
                    ue2::truffleBuildMasks(b.chars, (u8 *)&b.lo, (u8 *)&b.hi);
                    memset(b.buf.data(), 'b', b.size);
                },
                [&](MicroBenchmark &b) {
                    return rvermicelliExec('a', 'b', b.buf.data(), b.buf.data() + b.size);
                }
            );
        }

        for (size_t i = 0; i < std::size(sizes); i++) {
            //we imitate the noodle unit tests
            std::string str;
            const size_t char_len = 5;
            str.resize(char_len + 1);
            for (size_t j=0; j < char_len; j++) {
                srand (time(NULL));
                int key = rand() % + 36 ;
                str[char_len] = charset[key];
                str[char_len + 1] = '\0';
            }

            MicroBenchmark bench("Noodle", sizes[i]);
            run_benchmarks(sizes[i], MAX_LOOPS / sizes[i], matches[m], false, bench,
                [&](MicroBenchmark &b) {
                    ctxt.clear();
                    memset(b.buf.data(), 'a', b.size);
                    u32 id = 1000;
                    ue2::hwlmLiteral lit(str, true, id);
                    b.nt = ue2::noodBuildTable(lit);
                    assert(b.nt != nullptr);
                },
                [&](MicroBenchmark &b) {
                    noodExec(b.nt.get(), b.buf.data(), b.size, 0, hlmSimpleCallback, &b.scratch);
                    return b.buf.data() + b.size;
                }
           );
        }
    }

    return 0;
}
