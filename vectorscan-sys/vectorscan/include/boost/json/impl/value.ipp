//
// Copyright (c) 2019 Vinnie Falco (vinnie.falco@gmail.com)
//
// Distributed under the Boost Software License, Version 1.0. (See accompanying
// file LICENSE_1_0.txt or copy at http://www.boost.org/LICENSE_1_0.txt)
//
// Official repository: https://github.com/boostorg/json
//

#ifndef BOOST_JSON_IMPL_VALUE_IPP
#define BOOST_JSON_IMPL_VALUE_IPP

#include <boost/json/value.hpp>
#include <boost/json/parser.hpp>
#include <boost/json/detail/hash_combine.hpp>
#include <cstring>
#include <istream>
#include <limits>
#include <new>
#include <utility>

BOOST_JSON_NS_BEGIN

value::
~value() noexcept
{
    switch(kind())
    {
    case json::kind::null:
    case json::kind::bool_:
    case json::kind::int64:
    case json::kind::uint64:
    case json::kind::double_:
        sca_.~scalar();
        break;

    case json::kind::string:
        str_.~string();
        break;

    case json::kind::array:
        arr_.~array();
        break;

    case json::kind::object:
        obj_.~object();
        break;
    }
}

value::
value(
    value const& other,
    storage_ptr sp)
{
    switch(other.kind())
    {
    case json::kind::null:
        ::new(&sca_) scalar(
            std::move(sp));
        break;

    case json::kind::bool_:
        ::new(&sca_) scalar(
            other.sca_.b,
            std::move(sp));
        break;

    case json::kind::int64:
        ::new(&sca_) scalar(
            other.sca_.i,
            std::move(sp));
        break;

    case json::kind::uint64:
        ::new(&sca_) scalar(
            other.sca_.u,
            std::move(sp));
        break;

    case json::kind::double_:
        ::new(&sca_) scalar(
            other.sca_.d,
            std::move(sp));
        break;

    case json::kind::string:
        ::new(&str_) string(
            other.str_,
            std::move(sp));
        break;

    case json::kind::array:
        ::new(&arr_) array(
            other.arr_,
            std::move(sp));
        break;

    case json::kind::object:
        ::new(&obj_) object(
            other.obj_,
            std::move(sp));
        break;
    }
}

value::
value(value&& other) noexcept
{
    relocate(this, other);
    ::new(&other.sca_) scalar(sp_);
}

value::
value(
    value&& other,
    storage_ptr sp)
{
    switch(other.kind())
    {
    case json::kind::null:
        ::new(&sca_) scalar(
            std::move(sp));
        break;

    case json::kind::bool_:
        ::new(&sca_) scalar(
            other.sca_.b, std::move(sp));
        break;

    case json::kind::int64:
        ::new(&sca_) scalar(
            other.sca_.i, std::move(sp));
        break;

    case json::kind::uint64:
        ::new(&sca_) scalar(
            other.sca_.u, std::move(sp));
        break;

    case json::kind::double_:
        ::new(&sca_) scalar(
            other.sca_.d, std::move(sp));
        break;

    case json::kind::string:
        ::new(&str_) string(
            std::move(other.str_),
            std::move(sp));
        break;

    case json::kind::array:
        ::new(&arr_) array(
            std::move(other.arr_),
            std::move(sp));
        break;

    case json::kind::object:
        ::new(&obj_) object(
            std::move(other.obj_),
            std::move(sp));
        break;
    }
}

//----------------------------------------------------------
//
// Conversion
//
//----------------------------------------------------------

value::
value(
    std::initializer_list<value_ref> init,
    storage_ptr sp)
{
    if(value_ref::maybe_object(init))
        ::new(&obj_) object(
            value_ref::make_object(
                init, std::move(sp)));
    else
        ::new(&arr_) array(
            value_ref::make_array(
                init, std::move(sp)));
}

//----------------------------------------------------------
//
// Assignment
//
//----------------------------------------------------------

value&
value::
operator=(value const& other)
{
    value(other,
        storage()).swap(*this);
    return *this;
}

value&
value::
operator=(value&& other)
{
    value(std::move(other),
        storage()).swap(*this);
    return *this;
}

value&
value::
operator=(
    std::initializer_list<value_ref> init)
{
    value(init,
        storage()).swap(*this);
    return *this;
}

value&
value::
operator=(string_view s)
{
    value(s, storage()).swap(*this);
    return *this;
}

value&
value::
operator=(char const* s)
{
    value(s, storage()).swap(*this);
    return *this;
}

value&
value::
operator=(string const& str)
{
    value(str, storage()).swap(*this);
    return *this;
}

value&
value::
operator=(string&& str)
{
    value(std::move(str),
        storage()).swap(*this);
    return *this;
}

value&
value::
operator=(array const& arr)
{
    value(arr, storage()).swap(*this);
    return *this;
}

value&
value::
operator=(array&& arr)
{
    value(std::move(arr),
        storage()).swap(*this);
    return *this;
}

value&
value::
operator=(object const& obj)
{
    value(obj, storage()).swap(*this);
    return *this;
}

value&
value::
operator=(object&& obj)
{
    value(std::move(obj),
        storage()).swap(*this);
    return *this;
}

//----------------------------------------------------------
//
// Modifiers
//
//----------------------------------------------------------

string&
value::
emplace_string() noexcept
{
    return *::new(&str_) string(destroy());
}

array&
value::
emplace_array() noexcept
{
    return *::new(&arr_) array(destroy());
}

object&
value::
emplace_object() noexcept
{
    return *::new(&obj_) object(destroy());
}

void
value::
swap(value& other)
{
    if(*storage() == *other.storage())
    {
        // fast path
        union U
        {
            value tmp;
            U(){}
            ~U(){}
        };
        U u;
        relocate(&u.tmp, *this);
        relocate(this, other);
        relocate(&other, u.tmp);
        return;
    }

    // copy
    value temp1(
        std::move(*this),
        other.storage());
    value temp2(
        std::move(other),
        this->storage());
    other.~value();
    ::new(&other) value(pilfer(temp1));
    this->~value();
    ::new(this) value(pilfer(temp2));
}

std::istream&
operator>>(
    std::istream& is,
    value& jv)
{
    using Traits = std::istream::traits_type;

    // sentry prepares the stream for reading and finalizes it in destructor
    std::istream::sentry sentry(is);
    if( !sentry )
        return is;

    unsigned char parser_buf[BOOST_JSON_STACK_BUFFER_SIZE / 2];
    stream_parser p({}, {}, parser_buf);
    p.reset( jv.storage() );

    char read_buf[BOOST_JSON_STACK_BUFFER_SIZE / 2];
    std::streambuf& buf = *is.rdbuf();
    std::ios::iostate err = std::ios::goodbit;
    try
    {
        while( true )
        {
            error_code ec;

            // we peek the buffer; this either makes sure that there's no
            // more input, or makes sure there's something in the internal
            // buffer (so in_avail will return a positive number)
            std::istream::int_type c = is.rdbuf()->sgetc();
            // if we indeed reached EOF, we check if we parsed a full JSON
            // document; if not, we error out
            if( Traits::eq_int_type(c, Traits::eof()) )
            {
                err |= std::ios::eofbit;
                p.finish(ec);
                if( ec.failed() )
                    break;
            }

            // regardless of reaching EOF, we might have parsed a full JSON
            // document; if so, we successfully finish
            if( p.done() )
            {
                jv = p.release();
                return is;
            }

            // at this point we definitely have more input, specifically in
            // buf's internal buffer; we also definitely haven't parsed a whole
            // document
            std::streamsize available = buf.in_avail();
            // if this assert fails, the streambuf is buggy
            BOOST_ASSERT( available > 0 );

            available = std::min(
                static_cast<std::size_t>(available), sizeof(read_buf) );
            // we read from the internal buffer of buf into our buffer
            available = buf.sgetn( read_buf, available );

            std::size_t consumed = p.write_some(
                read_buf, static_cast<std::size_t>(available), ec );
            // if the parser hasn't consumed the entire input we've took from
            // buf, we put the remaining data back; this should succeed,
            // because we only read data from buf's internal buffer
            while( consumed++ < static_cast<std::size_t>(available) )
            {
                std::istream::int_type const status = buf.sungetc();
                BOOST_ASSERT( status != Traits::eof() );
                (void)status;
            }

            if( ec.failed() )
                break;
        }
    }
    catch(...)
    {
        try
        {
            is.setstate(std::ios::badbit);
        }
        // we ignore the exception, because we need to throw the original
        // exception instead
        catch( std::ios::failure const& ) { }

        if( is.exceptions() & std::ios::badbit )
            throw;
    }

    is.setstate(err | std::ios::failbit);
    return is;
}

//----------------------------------------------------------
//
// private
//
//----------------------------------------------------------

storage_ptr
value::
destroy() noexcept
{
    switch(kind())
    {
    case json::kind::null:
    case json::kind::bool_:
    case json::kind::int64:
    case json::kind::uint64:
    case json::kind::double_:
        break;

    case json::kind::string:
    {
        auto sp = str_.storage();
        str_.~string();
        return sp;
    }

    case json::kind::array:
    {
        auto sp = arr_.storage();
        arr_.~array();
        return sp;
    }

    case json::kind::object:
    {
        auto sp = obj_.storage();
        obj_.~object();
        return sp;
    }

    }
    return std::move(sp_);
}

bool
value::
equal(value const& other) const noexcept
{
    switch(kind())
    {
    default: // unreachable()?
    case json::kind::null:
        return other.kind() == json::kind::null;

    case json::kind::bool_:
        return
            other.kind() == json::kind::bool_ &&
            get_bool() == other.get_bool();

    case json::kind::int64:
        switch(other.kind())
        {
        case json::kind::int64:
            return get_int64() == other.get_int64();
        case json::kind::uint64:
            if(get_int64() < 0)
                return false;
            return static_cast<std::uint64_t>(
                get_int64()) == other.get_uint64();
        default:
            return false;
        }

    case json::kind::uint64:
        switch(other.kind())
        {
        case json::kind::uint64:
            return get_uint64() == other.get_uint64();
        case json::kind::int64:
            if(other.get_int64() < 0)
                return false;
            return static_cast<std::uint64_t>(
                other.get_int64()) == get_uint64();
        default:
            return false;
        }

    case json::kind::double_:
        return
            other.kind() == json::kind::double_ &&
            get_double() == other.get_double();

    case json::kind::string:
        return
            other.kind() == json::kind::string &&
            get_string() == other.get_string();

    case json::kind::array:
        return
            other.kind() == json::kind::array &&
            get_array() == other.get_array();

    case json::kind::object:
        return
            other.kind() == json::kind::object &&
            get_object() == other.get_object();
    }
}

//----------------------------------------------------------
//
// key_value_pair
//
//----------------------------------------------------------

// empty keys point here
BOOST_JSON_REQUIRE_CONST_INIT
char const
key_value_pair::empty_[1] = { 0 };

key_value_pair::
key_value_pair(
    pilfered<json::value> key,
    pilfered<json::value> value) noexcept
    : value_(value)
{
    std::size_t len;
    key_ = access::release_key(key.get(), len);
    len_ = static_cast<std::uint32_t>(len);
}

key_value_pair::
key_value_pair(
    key_value_pair const& other,
    storage_ptr sp)
    : value_(other.value_, std::move(sp))
{
    auto p = reinterpret_cast<
        char*>(value_.storage()->
            allocate(other.len_ + 1,
                alignof(char)));
    std::memcpy(
        p, other.key_, other.len_);
    len_ = other.len_;
    p[len_] = 0;
    key_ = p;
}

//----------------------------------------------------------

BOOST_JSON_NS_END

//----------------------------------------------------------
//
// std::hash specialization
//
//----------------------------------------------------------

std::size_t
std::hash<::boost::json::value>::operator()(
    ::boost::json::value const& jv) const noexcept
{
  std::size_t seed = static_cast<std::size_t>(jv.kind());
  switch (jv.kind()) {
    default:
    case ::boost::json::kind::null:
      return seed;
    case ::boost::json::kind::bool_:
      return ::boost::json::detail::hash_combine(
        seed,
        hash<bool>{}(jv.get_bool()));
    case ::boost::json::kind::int64:
      return ::boost::json::detail::hash_combine(
        static_cast<size_t>(::boost::json::kind::uint64),
        hash<std::uint64_t>{}(jv.get_int64()));
    case ::boost::json::kind::uint64:
      return ::boost::json::detail::hash_combine(
        seed,
        hash<std::uint64_t>{}(jv.get_uint64()));
    case ::boost::json::kind::double_:
      return ::boost::json::detail::hash_combine(
        seed,
        hash<double>{}(jv.get_double()));
    case ::boost::json::kind::string:
      return ::boost::json::detail::hash_combine(
        seed,
        hash<::boost::json::string>{}(jv.get_string()));
    case ::boost::json::kind::array:
      return ::boost::json::detail::hash_combine(
        seed,
        hash<::boost::json::array>{}(jv.get_array()));
    case ::boost::json::kind::object:
      return ::boost::json::detail::hash_combine(
        seed,
        hash<::boost::json::object>{}(jv.get_object()));
  }
}

//----------------------------------------------------------

#endif
