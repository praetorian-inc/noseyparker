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
 * \brief Extended Unicode sequences (\\X)
 */


#include "ComponentEUS.h"

#include "buildstate.h"
#include "position.h"
#include "position_info.h"
#include "Parser.h"
#include "nfagraph/ng_builder.h"
#include "util/charreach.h"

using namespace std;

namespace ue2 {

ComponentEUS::ComponentEUS(u32 loc_in, const ParseMode &mode)
    : loc(loc_in), utf8(mode.utf8),
      position(GlushkovBuildState::POS_UNINITIALIZED) {}

ComponentEUS::~ComponentEUS() {}

ComponentEUS * ComponentEUS::clone() const {
    return new ComponentEUS(*this);
}

vector<PositionInfo> ComponentEUS::first() const {
    return vector<PositionInfo>(1, PositionInfo(position));
}

vector<PositionInfo> ComponentEUS::last() const {
    return vector<PositionInfo>(1, PositionInfo(position));
}

void ComponentEUS::notePositions(GlushkovBuildState &bs) {
    NFABuilder &builder = bs.getBuilder();
    position = builder.makePositions(1);
    builder.addCharReach(position, CharReach::dot());
    builder.setNodeReportID(position, 0 /* offset adj */);
    if (utf8) { /* we are prefiltering, turn to.+ */
        builder.addEdge(position, position);
    }
}

} // namespace ue2
