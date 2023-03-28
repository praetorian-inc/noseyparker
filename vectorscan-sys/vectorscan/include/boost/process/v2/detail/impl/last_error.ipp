// Copyright (c) 2022 Klemens D. Morgenstern
//
// Distributed under the Boost Software License, Version 1.0. (See accompanying
// file LICENSE_1_0.txt or copy at http://www.boost.org/LICENSE_1_0.txt)
#ifndef BOOST_PROCESS_V2_DETAIL_IMPL_LAST_ERROR_IPP
#define BOOST_PROCESS_V2_DETAIL_IMPL_LAST_ERROR_IPP

#include <boost/process/v2/detail/config.hpp>
#include <boost/process/v2/detail/last_error.hpp>

#if defined(BOOST_PROCESS_V2_WINDOWS)
#include <windows.h>
#else
#include <cerrno>
#endif

BOOST_PROCESS_V2_BEGIN_NAMESPACE
namespace detail
{

error_code get_last_error()
{
#if defined(BOOST_PROCESS_V2_WINDOWS)
    return error_code(::GetLastError(), system_category());
#else
    return error_code(errno, system_category());
#endif

}

void throw_last_error()
{
    throw system_error(get_last_error());
}
void throw_last_error(const char * msg)
{
    throw system_error(get_last_error(), msg);
}
void throw_last_error(const std::string & msg)
{
    throw system_error(get_last_error(), msg);
}


}
BOOST_PROCESS_V2_END_NAMESPACE

#endif //BOOST_PROCESS_V2_DETAIL_IMPL_LAST_ERROR_IPP