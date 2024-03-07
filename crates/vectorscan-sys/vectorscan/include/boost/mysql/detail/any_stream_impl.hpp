//
// Copyright (c) 2019-2023 Ruben Perez Hidalgo (rubenperez038 at gmail dot com)
//
// Distributed under the Boost Software License, Version 1.0. (See accompanying
// file LICENSE_1_0.txt or copy at http://www.boost.org/LICENSE_1_0.txt)
//

#ifndef BOOST_MYSQL_DETAIL_ANY_STREAM_IMPL_HPP
#define BOOST_MYSQL_DETAIL_ANY_STREAM_IMPL_HPP

#include <boost/mysql/error_code.hpp>

#include <boost/mysql/detail/any_stream.hpp>
#include <boost/mysql/detail/config.hpp>
#include <boost/mysql/detail/socket_stream.hpp>

#include <boost/asio/ip/tcp.hpp>
#include <boost/asio/ssl/stream.hpp>
#include <boost/config.hpp>

#include <type_traits>

namespace boost {
namespace mysql {
namespace detail {

// Connect and close helpers
template <class Stream>
const typename Stream::lowest_layer_type::endpoint_type cast_endpoint(const void* input) noexcept
{
    return *static_cast<const typename Stream::lowest_layer_type::endpoint_type*>(input);
}

template <class Stream>
void do_connect_impl(Stream&, const void*, error_code&, std::false_type)
{
    BOOST_ASSERT(false);
}

template <class Stream>
void do_connect_impl(Stream& stream, const void* endpoint, error_code& ec, std::true_type)
{
    stream.lowest_layer().connect(cast_endpoint<Stream>(endpoint), ec);
}

template <class Stream>
void do_connect(Stream& stream, const void* endpoint, error_code& ec)
{
    do_connect_impl(stream, endpoint, ec, is_socket_stream<Stream>{});
}

template <class Stream>
void do_async_connect_impl(
    Stream&,
    const void*,
    asio::any_completion_handler<void(error_code)>&&,
    std::false_type
)
{
    BOOST_ASSERT(false);
}

template <class Stream>
void do_async_connect_impl(
    Stream& stream,
    const void* endpoint,
    asio::any_completion_handler<void(error_code)>&& handler,
    std::true_type
)
{
    stream.lowest_layer().async_connect(cast_endpoint<Stream>(endpoint), std::move(handler));
}

template <class Stream>
void do_async_connect(
    Stream& stream,
    const void* endpoint,
    asio::any_completion_handler<void(error_code)>&& handler
)
{
    do_async_connect_impl(stream, endpoint, std::move(handler), is_socket_stream<Stream>{});
}

template <class Stream>
void do_close_impl(Stream&, error_code&, std::false_type)
{
    BOOST_ASSERT(false);
}

template <class Stream>
void do_close_impl(Stream& stream, error_code& ec, std::true_type)
{
    stream.lowest_layer().shutdown(asio::socket_base::shutdown_both, ec);
    stream.lowest_layer().close(ec);
}

template <class Stream>
void do_close(Stream& stream, error_code& ec)
{
    do_close_impl(stream, ec, is_socket_stream<Stream>{});
}

template <class Stream>
bool do_is_open_impl(const Stream&, std::false_type) noexcept
{
    return false;
}

template <class Stream>
bool do_is_open_impl(const Stream& stream, std::true_type) noexcept
{
    return stream.lowest_layer().is_open();
}

template <class Stream>
bool do_is_open(const Stream& stream) noexcept
{
    return do_is_open_impl(stream, is_socket_stream<Stream>{});
}

template <class Stream>
class any_stream_impl final : public any_stream
{
    Stream stream_;

public:
    template <class... Args>
    any_stream_impl(Args&&... args) : any_stream(false), stream_(std::forward<Args>(args)...)
    {
    }

    Stream& stream() noexcept { return stream_; }
    const Stream& stream() const noexcept { return stream_; }

    executor_type get_executor() override final { return stream_.get_executor(); }

    // SSL
    void handshake(error_code&) final override { BOOST_ASSERT(false); }
    void async_handshake(asio::any_completion_handler<void(error_code)>) final override
    {
        BOOST_ASSERT(false);
    }
    void shutdown(error_code&) final override { BOOST_ASSERT(false); }
    void async_shutdown(asio::any_completion_handler<void(error_code)>) final override
    {
        BOOST_ASSERT(false);
    }

    // Reading
    std::size_t read_some(boost::asio::mutable_buffer buff, error_code& ec) final override
    {
        return stream_.read_some(buff, ec);
    }
    void async_read_some(
        boost::asio::mutable_buffer buff,
        asio::any_completion_handler<void(error_code, std::size_t)> handler
    ) final override
    {
        return stream_.async_read_some(buff, std::move(handler));
    }

    // Writing
    std::size_t write_some(boost::asio::const_buffer buff, error_code& ec) final override
    {
        return stream_.write_some(buff, ec);
    }
    void async_write_some(
        boost::asio::const_buffer buff,
        asio::any_completion_handler<void(error_code, std::size_t)> handler
    ) final override
    {
        return stream_.async_write_some(buff, std::move(handler));
    }

    // Connect and close
    void connect(const void* endpoint, error_code& ec) override final { do_connect(stream_, endpoint, ec); }
    void async_connect(const void* endpoint, asio::any_completion_handler<void(error_code)> handler)
        override final
    {
        do_async_connect(stream_, endpoint, std::move(handler));
    }
    void close(error_code& ec) override final { do_close(stream_, ec); }
    bool is_open() const noexcept override { return do_is_open(stream_); }
};

template <class Stream>
class any_stream_impl<asio::ssl::stream<Stream>> final : public any_stream
{
    asio::ssl::stream<Stream> stream_;

public:
    template <class... Args>
    any_stream_impl(Args&&... args) : any_stream(true), stream_(std::forward<Args>(args)...)
    {
    }

    asio::ssl::stream<Stream>& stream() noexcept { return stream_; }
    const asio::ssl::stream<Stream>& stream() const noexcept { return stream_; }

    executor_type get_executor() override final { return stream_.get_executor(); }

    // SSL
    void handshake(error_code& ec) override final
    {
        set_ssl_active();
        stream_.handshake(boost::asio::ssl::stream_base::client, ec);
    }
    void async_handshake(asio::any_completion_handler<void(error_code)> handler) override final
    {
        set_ssl_active();
        stream_.async_handshake(boost::asio::ssl::stream_base::client, std::move(handler));
    }
    void shutdown(error_code& ec) override final { stream_.shutdown(ec); }
    void async_shutdown(asio::any_completion_handler<void(error_code)> handler) override final
    {
        return stream_.async_shutdown(std::move(handler));
    }

    // Reading
    std::size_t read_some(boost::asio::mutable_buffer buff, error_code& ec) override final
    {
        if (ssl_active())
        {
            return stream_.read_some(buff, ec);
        }
        else
        {
            return stream_.next_layer().read_some(buff, ec);
        }
    }
    void async_read_some(
        boost::asio::mutable_buffer buff,
        asio::any_completion_handler<void(error_code, std::size_t)> handler
    ) override final
    {
        if (ssl_active())
        {
            return stream_.async_read_some(buff, std::move(handler));
        }
        else
        {
            return stream_.next_layer().async_read_some(buff, std::move(handler));
        }
    }

    // Writing
    std::size_t write_some(boost::asio::const_buffer buff, error_code& ec) override final
    {
        if (ssl_active())
        {
            return stream_.write_some(buff, ec);
        }
        else
        {
            return stream_.next_layer().write_some(buff, ec);
        }
    }
    void async_write_some(
        boost::asio::const_buffer buff,
        asio::any_completion_handler<void(error_code, std::size_t)> handler
    ) override final
    {
        if (ssl_active())
        {
            stream_.async_write_some(buff, std::move(handler));
        }
        else
        {
            return stream_.next_layer().async_write_some(buff, std::move(handler));
        }
    }

    // Connect and close
    void connect(const void* endpoint, error_code& ec) override final { do_connect(stream_, endpoint, ec); }
    void async_connect(const void* endpoint, asio::any_completion_handler<void(error_code)> handler)
        override final
    {
        do_async_connect(stream_, endpoint, std::move(handler));
    }
    void close(error_code& ec) override final { do_close(stream_, ec); }
    bool is_open() const noexcept override { return do_is_open(stream_); }
};

template <class Stream>
const Stream& cast(const any_stream& obj) noexcept
{
    return static_cast<const any_stream_impl<Stream>&>(obj).stream();
}

template <class Stream>
Stream& cast(any_stream& obj) noexcept
{
    return static_cast<any_stream_impl<Stream>&>(obj).stream();
}

#ifdef BOOST_MYSQL_SEPARATE_COMPILATION
extern template class any_stream_impl<asio::ssl::stream<asio::ip::tcp::socket>>;
extern template class any_stream_impl<asio::ip::tcp::socket>;
#endif

}  // namespace detail
}  // namespace mysql
}  // namespace boost

// any_stream_impl.ipp explicitly instantiates any_stream_impl, so not included here

#endif
