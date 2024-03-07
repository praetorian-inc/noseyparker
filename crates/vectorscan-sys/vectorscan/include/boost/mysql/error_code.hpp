//
// Copyright (c) 2019-2023 Ruben Perez Hidalgo (rubenperez038 at gmail dot com)
//
// Distributed under the Boost Software License, Version 1.0. (See accompanying
// file LICENSE_1_0.txt or copy at http://www.boost.org/LICENSE_1_0.txt)
//

#ifndef BOOST_MYSQL_ERROR_CODE_HPP
#define BOOST_MYSQL_ERROR_CODE_HPP

#include <boost/system/error_code.hpp>

namespace boost {
namespace mysql {

/// An alias for boost::system error codes.
using error_code = boost::system::error_code;

}  // namespace mysql
}  // namespace boost

#endif
