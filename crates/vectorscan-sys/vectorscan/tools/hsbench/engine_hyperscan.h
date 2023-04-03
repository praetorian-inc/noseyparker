/*
 * Copyright (c) 2016-2018, Intel Corporation
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

#ifndef ENGINEHYPERSCAN_H
#define ENGINEHYPERSCAN_H

#include "expressions.h"
#include "engine.h"
#include "hs_runtime.h"

#include <memory>
#include <string>
#include <vector>

/** Infomation about the database compile */
struct CompileHSStats {
    std::string sigs_name;
    std::string signatures;
    std::string db_info;
    size_t expressionCount = 0;
    size_t compiledSize = 0;
    uint32_t crc32 = 0;
    bool streaming;
    size_t streamSize = 0;
    size_t scratchSize = 0;
    long double compileSecs = 0;
    unsigned int peakMemorySize = 0;
};

/** Engine context which is allocated on a per-thread basis. */
class EngineHSContext : public EngineContext {
public:
    explicit EngineHSContext(const hs_database_t *db);
    ~EngineHSContext();

    hs_scratch_t *scratch = nullptr;
};

/** Streaming mode scans have persistent stream state associated with them. */
class EngineHSStream : public EngineStream {
public:
    ~EngineHSStream();
    hs_stream_t *id = nullptr;
    EngineHSContext *ctx = nullptr;
};

/** Hyperscan Engine for scanning data. */
class EngineHyperscan : public Engine {
public:
    explicit EngineHyperscan(hs_database_t *db, CompileHSStats cs);
    ~EngineHyperscan();

    std::unique_ptr<EngineContext> makeContext() const;

    void scan(const char *data, unsigned int len, unsigned int id,
              ResultEntry &result, EngineContext &ectx) const;

    void scan_vectored(const char *const *data, const unsigned int *len,
                       unsigned int count, unsigned int streamId,
                       ResultEntry &result, EngineContext &ectx) const;

    std::unique_ptr<EngineStream> streamOpen(EngineContext &ectx,
                                             unsigned id) const;

    void streamClose(std::unique_ptr<EngineStream> stream,
                     ResultEntry &result) const;

    void streamCompressExpand(EngineStream &stream,
                              std::vector<char> &temp) const;

    void streamScan(EngineStream &stream, const char *data, unsigned int len,
                    unsigned int id, ResultEntry &result) const;

    void printStats() const;

    void printCsvStats() const;

    void sqlStats(SqlDB &db) const;

private:
    hs_database_t *db;
    CompileHSStats compile_stats;
};

namespace ue2 {
struct Grey;
}

std::unique_ptr<EngineHyperscan>
buildEngineHyperscan(const ExpressionMap &expressions, ScanMode scan_mode,
                     const std::string &name, const std::string &sigs_name,
                     const ue2::Grey &grey);

#endif // ENGINEHYPERSCAN_H
