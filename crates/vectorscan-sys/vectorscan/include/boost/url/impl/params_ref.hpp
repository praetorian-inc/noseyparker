//
// Copyright (c) 2019 Vinnie Falco (vinnie.falco@gmail.com)
// Copyright (c) 2022 Alan de Freitas (alandefreitas@gmail.com)
//
// Distributed under the Boost Software License, Version 1.0. (See accompanying
// file LICENSE_1_0.txt or copy at http://www.boost.org/LICENSE_1_0.txt)
//
// Official repository: https://github.com/boostorg/url
//

#ifndef BOOST_URL_IMPL_PARAMS_REF_HPP
#define BOOST_URL_IMPL_PARAMS_REF_HPP

#include <boost/url/params_view.hpp>
#include <boost/url/detail/any_params_iter.hpp>
#include <boost/url/detail/except.hpp>
#include <boost/url/grammar/recycled.hpp>
#include <boost/assert.hpp>

namespace boost {
namespace urls {

inline
params_ref::
params_ref(
    url_base& u,
    encoding_opts opt) noexcept
    : params_base(u.impl_, opt)
    , u_(&u)
{
}

//------------------------------------------------
//
// Special Members
//
//------------------------------------------------

inline
params_ref::
params_ref(
    params_ref const& other,
    encoding_opts opt) noexcept
    : params_ref(*other.u_, opt)
{
}

inline
auto
params_ref::
operator=(std::initializer_list<
    param_view> init) ->
        params_ref&
{
    assign(init);
    return *this;
}

//------------------------------------------------
//
// Modifiers
//
//------------------------------------------------

inline
void
params_ref::
clear() noexcept
{
    u_->remove_query();
}

//------------------------------------------------

template<class FwdIt>
void
params_ref::
assign(FwdIt first, FwdIt last)
{
/*  If you get a compile error here, it
    means that the iterators you passed
    do not meet the requirements stated
    in the documentation.
*/
    static_assert(
        std::is_convertible<
            typename std::iterator_traits<
                FwdIt>::reference,
            param_view>::value,
        "Type requirements not met");

    assign(first, last,
        typename std::iterator_traits<
            FwdIt>::iterator_category{});
}

inline
auto
params_ref::
append(
    param_view const& p) ->
        iterator
{
    return insert(end(), p);
}

inline
auto
params_ref::
append(
    std::initializer_list<
        param_view> init) ->
    iterator
{
    return insert(end(), init);
}

template<class FwdIt>
auto
params_ref::
append(FwdIt first, FwdIt last) ->
    iterator
{
/*  If you get a compile error here, it
    means that the iterators you passed
    do not meet the requirements stated
    in the documentation.
*/
    static_assert(
        std::is_convertible<
            typename std::iterator_traits<
                FwdIt>::reference,
            param_view>::value,
        "Type requirements not met");

    return insert(
        end(), first, last);
}

template<class FwdIt>
auto
params_ref::
insert(
    iterator before,
    FwdIt first,
    FwdIt last) ->
        iterator
{
/*  If you get a compile error here, it
    means that the iterators you passed
    do not meet the requirements stated
    in the documentation.
*/
    static_assert(
        std::is_convertible<
            typename std::iterator_traits<
                FwdIt>::reference,
            param_view>::value,
        "Type requirements not met");

    return insert(
        before,
        first,
        last,
        typename std::iterator_traits<
            FwdIt>::iterator_category{});
}

template<class FwdIt>
auto
params_ref::
replace(
    iterator from,
    iterator to,
    FwdIt first,
    FwdIt last) ->
        iterator
{
/*  If you get a compile error here, it
    means that the iterators you passed
    do not meet the requirements stated
    in the documentation.
*/
    static_assert(
        std::is_convertible<
            typename std::iterator_traits<
                FwdIt>::reference,
            param_view>::value,
        "Type requirements not met");

    return iterator(
        u_->edit_params(
            from.it_, to.it_,
            detail::make_params_iter(
                first, last)),
        opt_);
}

//------------------------------------------------
//
// implementation
//
//------------------------------------------------

template<class FwdIt>
void
params_ref::
assign(FwdIt first, FwdIt last,
    std::forward_iterator_tag)
{
    u_->edit_params(
        begin().it_,
        end().it_,
        detail::make_params_iter(
            first, last));
}

template<class FwdIt>
auto
params_ref::
insert(
    iterator before,
    FwdIt first,
    FwdIt last,
    std::forward_iterator_tag) ->
        iterator
{
    return iterator(
        u_->edit_params(
            before.it_,
            before.it_,
            detail::make_params_iter(
                first, last)),
        opt_);
}

} // urls
} // boost

#endif
