//
// Copyright (c) 2019-2023 Ruben Perez Hidalgo (rubenperez038 at gmail dot com)
//
// Distributed under the Boost Software License, Version 1.0. (See accompanying
// file LICENSE_1_0.txt or copy at http://www.boost.org/LICENSE_1_0.txt)
//

#ifndef BOOST_MYSQL_DETAIL_CONFIG_HPP
#define BOOST_MYSQL_DETAIL_CONFIG_HPP

#include <boost/config.hpp>

// clang-format off

// Concepts
#if defined(__has_include)
    #if __has_include(<version>)
        #include <version>
        #if defined(__cpp_concepts) && defined(__cpp_lib_concepts)
            #define BOOST_MYSQL_HAS_CONCEPTS
        #endif
    #endif
#endif

// C++14 conformance
#if BOOST_CXX_VERSION >= 201402L
    #define BOOST_MYSQL_CXX14
#endif

// Separate build
#if defined(BOOST_MYSQL_SEPARATE_COMPILATION)
    #define BOOST_MYSQL_DECL
    #define BOOST_MYSQL_STATIC_IF_COMPILED static
    #define BOOST_MYSQL_STATIC_OR_INLINE static
#else
    #define BOOST_MYSQL_HEADER_ONLY
    #define BOOST_MYSQL_DECL inline
    #define BOOST_MYSQL_STATIC_IF_COMPILED
    #define BOOST_MYSQL_STATIC_OR_INLINE inline
#endif

// clang-format on

#endif
