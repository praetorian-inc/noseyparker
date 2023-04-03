# Possible values:
# - `address` (ASan)
# - `memory` (MSan)
# - `undefined` (UBSan)
# - "" (no sanitizing)
option (SANITIZE "Enable one of the code sanitizers" "")

set (SAN_FLAGS "${SAN_FLAGS} -g -fno-omit-frame-pointer -DSANITIZER")

if (SANITIZE)
    if (SANITIZE STREQUAL "address")
        set (ASAN_FLAGS "-fsanitize=address -fsanitize-address-use-after-scope")
        set (CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} ${SAN_FLAGS} ${ASAN_FLAGS}")
        set (CMAKE_C_FLAGS "${CMAKE_C_FLAGS} ${SAN_FLAGS} ${ASAN_FLAGS}")

        if (CMAKE_CXX_COMPILER_ID STREQUAL "GNU")
            set (CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} ${ASAN_FLAGS}")
        endif()

    elseif (SANITIZE STREQUAL "memory")
        if (CMAKE_CXX_COMPILER_ID STREQUAL "GNU")
	    set (FATAL_ERROR "GCC does not have memory sanitizer")
        endif()
	# MemorySanitizer flags are set according to the official documentation:
        # https://clang.llvm.org/docs/MemorySanitizer.html#usage
        set (MSAN_FLAGS "-fsanitize=memory -fsanitize-memory-use-after-dtor -fsanitize-memory-track-origins -fno-optimize-sibling-calls")

        set (CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} ${SAN_FLAGS} ${MSAN_FLAGS}")
        set (CMAKE_C_FLAGS "${CMAKE_C_FLAGS} ${SAN_FLAGS} ${MSAN_FLAGS}")
    elseif (SANITIZE STREQUAL "undefined")
        set (UBSAN_FLAGS "-fsanitize=undefined")
        set (CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} ${SAN_FLAGS} ${UBSAN_FLAGS}")
        set (CMAKE_C_FLAGS "${CMAKE_C_FLAGS} ${SAN_FLAGS} ${UBSAN_FLAGS}")
        if (CMAKE_CXX_COMPILER_ID STREQUAL "GNU")
            set (CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} -fsanitize=undefined")
        endif()
    else ()
        message (FATAL_ERROR "Unknown sanitizer type: ${SANITIZE}")
    endif ()
endif()
