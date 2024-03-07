# determine compiler
if (CMAKE_CXX_COMPILER_ID MATCHES "Clang")
    set(CMAKE_COMPILER_IS_CLANG TRUE)
    set(CLANGCXX_MINVER "5")
    message(STATUS "clang++ version ${CMAKE_CXX_COMPILER_VERSION}")
    if (CMAKE_CXX_COMPILER_VERSION VERSION_LESS CLANGCXX_MINVER)
        message(FATAL_ERROR "A minimum of clang++ ${CLANGCXX_MINVER} is required for C++17 support")
    endif()
endif()

# compiler version checks TODO: test more compilers
if (CMAKE_COMPILER_IS_GNUCXX)
    set(GNUCXX_MINVER "9")
    message(STATUS "g++ version ${CMAKE_CXX_COMPILER_VERSION}")
    if (CMAKE_CXX_COMPILER_VERSION VERSION_LESS GNUCXX_MINVER)
        message(FATAL_ERROR "A minimum of g++ ${GNUCXX_MINVER} is required for C++17 support")
    endif()
endif()

