//
// Copyright (c) 2019 Vinnie Falco (vinnie.falco@gmail.com)
// Copyright (c) 2020 Krystian Stasiowski (sdkrystian@gmail.com)
// Copyright (c) 2022 Dmitry Arkhipov (grisumbras@gmail.com)
//
// Distributed under the Boost Software License, Version 1.0. (See accompanying
// file LICENSE_1_0.txt or copy at http://www.boost.org/LICENSE_1_0.txt)
//
// Official repository: https://github.com/boostorg/json
//

#ifndef BOOST_JSON_VALUE_FROM_HPP
#define BOOST_JSON_VALUE_FROM_HPP

#include <boost/json/detail/value_from.hpp>

BOOST_JSON_NS_BEGIN

/** Convert an object of type `T` to @ref value.

    This function attempts to convert an object
    of type `T` to @ref value using

    @li one of @ref value's constructors,

    @li a library-provided generic conversion, or

    @li a user-provided overload of `tag_invoke`.

    Out of the box the function supports types satisfying
    <a href="https://en.cppreference.com/w/cpp/named_req/SequenceContainer"><em>SequenceContainer</em></a>,
    arrays, arithmetic types, `bool`, `std::tuple`, `std::pair`,
    `std::variant`, `std::optional`, `std::monostate`, and `std::nullopt_t`.

    Conversion of other types is done by calling an overload of `tag_invoke`
    found by argument-dependent lookup. Its signature should be similar to:

    @code
    void tag_invoke( value_from_tag, value&, T );
    @endcode

    @par Exception Safety
    Strong guarantee.

    @tparam T The type of the object to convert.

    @param t The object to convert.

    @param jv @ref value out parameter.

    @see @ref value_from_tag, @ref value_to,
    <a href="http://www.open-std.org/jtc1/sc22/wg21/docs/papers/2019/p1895r0.pdf">
        tag_invoke: A general pattern for supporting customisable functions</a>
*/
template<class T>
void
value_from(
    T&& t,
    value& jv)
{
    using bare_T = detail::remove_cvref<T>;
    BOOST_STATIC_ASSERT(detail::conversion_round_trips<
        bare_T, detail::value_from_conversion>::value);
    using impl = detail::value_from_implementation<bare_T>;
    detail::value_from_helper(jv, std::forward<T>(t), impl());
}

/** Convert an object of type `T` to @ref value.

    This function attempts to convert an object
    of type `T` to @ref value using

    @li one of @ref value's constructors,

    @li a library-provided generic conversion, or

    @li a user-provided overload of `tag_invoke`.

    Out of the box the function supports types satisfying
    <a href="https://en.cppreference.com/w/cpp/named_req/SequenceContainer"><em>SequenceContainer</em></a>,
    arrays, arithmetic types, `bool`, `std::tuple`, `std::pair`,
    `std::variant`, `std::optional`, `std::monostate`, and `std::nullopt_t`.

    Conversion of other types is done by calling an overload of `tag_invoke`
    found by argument-dependent lookup. Its signature should be similar to:

    @code
    void tag_invoke( value_from_tag, value&, T );
    @endcode

    A @ref value constructed
    with the @ref storage_ptr passed to @ref value_from is
    passed as the second argument to ensure that the memory
    resource is correctly propagated.

    @par Exception Safety
    Strong guarantee.

    @tparam T The type of the object to convert.

    @returns `t` converted to @ref value.

    @param t The object to convert.

    @param sp A storage pointer referring to the memory resource
    to use for the returned @ref value. The default argument for this
    parameter is `{}`.

    @see @ref value_from_tag, @ref value_to,
    <a href="http://www.open-std.org/jtc1/sc22/wg21/docs/papers/2019/p1895r0.pdf">
        tag_invoke: A general pattern for supporting customisable functions</a>
*/
template<class T>
value
value_from(
    T&& t,
    storage_ptr sp = {})
{
    value jv(std::move(sp));
    json::value_from(
        std::forward<T>(t), jv);
    return jv;
}

/** Determine if `T` can be converted to @ref value.

    If `T` can be converted to @ref value via a
    call to @ref value_from, the static data member `value`
    is defined as `true`. Otherwise, `value` is
    defined as `false`.

    @see @ref value_from
*/
#ifdef BOOST_JSON_DOCS
template<class T>
using has_value_from = __see_below__;
#else
template<class T>
using has_value_from = detail::can_convert<
    detail::remove_cvref<T>, detail::value_from_conversion>;
#endif

BOOST_JSON_NS_END

#endif
