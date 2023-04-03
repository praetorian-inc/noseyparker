/*
 * Copyright (c) 2015, Intel Corporation
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
 * \brief Lookahead/lookbehind zero-width assertions.
 */

#ifndef _RE_COMPONENTASSERTION_H_
#define _RE_COMPONENTASSERTION_H_

#include "ComponentSequence.h"

namespace ue2 {

class ComponentAssertion : public ComponentSequence {
    friend class DumpVisitor;
    friend class PrintVisitor;
public:
    enum Direction {
        LOOKAHEAD, //!< lookahead (forward) assertion
        LOOKBEHIND //!< lookbehind (backward) assertion
    };

    enum Sense {
        POS, //!< positive assertion, (?=...) or (?<=...)
        NEG  //!< negative assertion, (?!...) or (?<!...)
    };

    ComponentAssertion(enum Direction dir, enum Sense sense);
    ~ComponentAssertion() override;
    ComponentAssertion *clone() const override;
    Component *accept(ComponentVisitor &v) override;
    void accept(ConstComponentVisitor &v) const override;

    std::vector<PositionInfo> first() const override;
    std::vector<PositionInfo> last() const override;

    bool empty() const override;
    void notePositions(GlushkovBuildState &bs) override;
    void buildFollowSet(GlushkovBuildState &bs,
                        const std::vector<PositionInfo> &lastPos) override;
    bool repeatable() const override;

private:
    enum Direction m_dir;
    enum Sense m_sense;
};

} // namespace ue2

#endif
