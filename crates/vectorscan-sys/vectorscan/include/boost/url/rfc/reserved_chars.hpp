//
// Copyright (c) 2016-2019 Vinnie Falco (vinnie dot falco at gmail dot com)
//
// Distributed under the Boost Software License, Version 1.0. (See accompanying
// file LICENSE_1_0.txt or copy at http://www.boost.org/LICENSE_1_0.txt)
//
// Official repository: https://github.com/boostorg/url
//

#ifndef BOOST_URL_RFC_RESERVED_CHARS_HPP
#define BOOST_URL_RFC_RESERVED_CHARS_HPP

#include <boost/url/detail/config.hpp>
#include <boost/url/rfc/unreserved_chars.hpp>

namespace boost {
namespace urls {

/** The reserved character set

    @par Example
    Character sets are used with rules and
    the functions @ref grammar::find_if and
    @ref grammar::find_if_not.
    @code
    result< decode_view > rv = grammar::parse( "Program%20Files", pct_encoded_rule( reserved_chars ) );
    @endcode

    @par Specification
    @li <a href="https://datatracker.ietf.org/doc/html/rfc3986#section-2.3"
        >2.3. Unreserved Characters (rfc3986)</a>

    @see
        @ref grammar::find_if,
        @ref grammar::find_if_not,
        @ref grammar::parse,
        @ref pct_encoded_rule.
*/
constexpr auto reserved_chars = ~unreserved_chars;

} // urls
} // boost

#endif
