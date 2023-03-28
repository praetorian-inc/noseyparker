//
// Copyright (c) 2016-2019 Vinnie Falco (vinnie dot falco at gmail dot com)
//
// Distributed under the Boost Software License, Version 1.0. (See accompanying
// file LICENSE_1_0.txt or copy at http://www.boost.org/LICENSE_1_0.txt)
//
// Official repository: https://github.com/boostorg/url
//

#ifndef BOOST_URL_RFC_URI_RULE_HPP
#define BOOST_URL_RFC_URI_RULE_HPP

#include <boost/url/detail/config.hpp>
#include <boost/url/error_types.hpp>
#include <boost/url/url_view.hpp>

namespace boost {
namespace urls {

/** Rule for URI

    @par Value Type
    @code
    using value_type = url_view;
    @endcode

    @par Example
    Rules are used with the function @ref grammar::parse.
    @code
    result< url_view > rv = grammar::parse( "https://www.example.com/index.htm?id=guest#s1", uri_rule );
    @endcode

    @par BNF
    @code
    URI           = scheme ":" hier-part [ "?" query ] [ "#" fragment ]
    @endcode

    @par Specification
    @li <a href="https://datatracker.ietf.org/doc/html/rfc3986#section-3"
        >3. Syntax Components (rfc3986)</a>

    @see
        @ref grammar::parse,
        @ref parse_uri,
        @ref url_view.
*/
#ifdef BOOST_URL_DOCS
constexpr __implementation_defined__ uri_rule{};
#else
struct uri_rule_t
{
    using value_type = url_view;

    BOOST_URL_DECL
    auto
    parse(
        char const*& it,
        char const* const end
            ) const noexcept ->
        result<value_type>;
};

constexpr uri_rule_t uri_rule{};
#endif

} // urls
} // boost

#endif
