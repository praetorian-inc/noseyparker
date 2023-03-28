/* Hash function characterization.
 *
 * Copyright 2022 Joaquin M Lopez Munoz.
 * Distributed under the Boost Software License, Version 1.0.
 * (See accompanying file LICENSE_1_0.txt or copy at
 * http://www.boost.org/LICENSE_1_0.txt)
 *
 * See https://www.boost.org/libs/unordered for library home page.
 */

#ifndef BOOST_UNORDERED_HASH_TRAITS_HPP
#define BOOST_UNORDERED_HASH_TRAITS_HPP

#include <boost/type_traits/make_void.hpp>
#include <boost/type_traits/integral_constant.hpp>

namespace boost{
namespace unordered{

namespace detail{

template<typename Hash,typename=void>
struct hash_is_avalanching_impl: boost::false_type{};

template<typename Hash>
struct hash_is_avalanching_impl<Hash,
  typename boost::make_void<typename Hash::is_avalanching>::type>:
    boost::true_type{};

} /* namespace detail */

/* Each trait can be partially specialized by users for concrete hash functions
 * when actual characterization differs from default.
 */

/* hash_is_avalanching<Hash>::value is true when the type Hash::is_avalanching
 * is present, false otherwise.
 */
template<typename Hash>
struct hash_is_avalanching: detail::hash_is_avalanching_impl<Hash>::type{};

} /* namespace unordered */
} /* namespace boost */

#endif
