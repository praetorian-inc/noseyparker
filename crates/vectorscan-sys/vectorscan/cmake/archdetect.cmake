if (USE_CPU_NATIVE)
    # Detect best GNUCC_ARCH to tune for
    if (CMAKE_COMPILER_IS_GNUCC)
        message(STATUS "gcc version ${CMAKE_C_COMPILER_VERSION}")

        # If gcc doesn't recognise the host cpu, then mtune=native becomes
        # generic, which isn't very good in some cases. march=native looks at
        # cpuid info and then chooses the best microarch it can (and replaces
        # the flag), so use that for tune.

        set(TUNE_FLAG "mtune")
        set(GNUCC_TUNE "")
        message(STATUS "ARCH_FLAG '${ARCH_FLAG}' '${GNUCC_ARCH}', TUNE_FLAG '${TUNE_FLAG}' '${GNUCC_TUNE}' ")

        # arg1 might exist if using ccache
        string (STRIP "${CMAKE_C_COMPILER_ARG1}" CC_ARG1)
        set (EXEC_ARGS ${CC_ARG1} -c -Q --help=target -${ARCH_FLAG}=native -${TUNE_FLAG}=native)
        execute_process(COMMAND ${CMAKE_C_COMPILER} ${EXEC_ARGS}
            OUTPUT_VARIABLE _GCC_OUTPUT)
        set(_GCC_OUTPUT_TUNE ${_GCC_OUTPUT})
        string(FIND "${_GCC_OUTPUT}" "${ARCH_FLAG}=" POS)
        string(SUBSTRING "${_GCC_OUTPUT}" ${POS} -1 _GCC_OUTPUT)
        string(REGEX REPLACE "${ARCH_FLAG}=[ \t]*([^ \n]*)[ \n].*" "\\1" GNUCC_ARCH "${_GCC_OUTPUT}")

        string(FIND "${_GCC_OUTPUT_TUNE}" "${TUNE_FLAG}=" POS_TUNE)
        string(SUBSTRING "${_GCC_OUTPUT_TUNE}" ${POS_TUNE} -1 _GCC_OUTPUT_TUNE)
        string(REGEX REPLACE "${TUNE_FLAG}=[ \t]*([^ \n]*)[ \n].*" "\\1" GNUCC_TUNE "${_GCC_OUTPUT_TUNE}")

        message(STATUS "ARCH_FLAG '${ARCH_FLAG}' '${GNUCC_ARCH}', TUNE_FLAG '${TUNE_FLAG}' '${GNUCC_TUNE}' ")

        # test the parsed flag
        set (EXEC_ARGS ${CC_ARG1} -E - -${ARCH_FLAG}=${GNUCC_ARCH} -${TUNE_FLAG}=${GNUCC_TUNE})
        execute_process(COMMAND ${CMAKE_C_COMPILER} ${EXEC_ARGS}
            OUTPUT_QUIET ERROR_QUIET
            INPUT_FILE /dev/null
            RESULT_VARIABLE GNUCC_TUNE_TEST)

        if (NOT GNUCC_TUNE_TEST EQUAL 0)
            message(WARNING "Something went wrong determining gcc tune: -mtune=${GNUCC_TUNE} not valid, falling back to -mtune=native")
            set(GNUCC_TUNE native)
        else()
            set(GNUCC_TUNE ${GNUCC_TUNE})
            message(STATUS "gcc will tune for ${GNUCC_ARCH}, ${GNUCC_TUNE}")
        endif()
    elseif (CMAKE_COMPILER_IS_CLANG)
        if (ARCH_IA32 OR ARCH_X86_64)
            set(GNUCC_ARCH x86_64_v2)
            set(TUNE_FLAG generic)
        elseif(ARCH_AARCH64)
            if (BUILD_SVE2_BITPERM)
                set(GNUCC_ARCH ${SVE2_BITPERM_ARCH})
            elseif (BUILD_SVE2)
                set(GNUCC_ARCH ${SVE2_ARCH})
            elseif (BUILD_SVE)
                set(GNUCC_ARCH ${SVE_ARCH})
            else ()
                set(GNUCC_ARCH ${ARMV8_ARCH})
            endif()
            set(TUNE_FLAG generic)
        elseif(ARCH_ARM32)
            set(GNUCC_ARCH armv7a)
            set(TUNE_FLAG generic)
        else()
            set(GNUCC_ARCH native)
            set(TUNE_FLAG generic)
        endif()
        message(STATUS "clang will tune for ${GNUCC_ARCH}, ${TUNE_FLAG}")
    endif()
else()
    if (ARCH_IA32 OR ARCH_X86_64)
        set(GNUCC_ARCH native)
        set(TUNE_FLAG generic)
    elseif(ARCH_AARCH64)
        if (BUILD_SVE2_BITPERM)
            set(GNUCC_ARCH ${SVE2_BITPERM_ARCH})
        elseif (BUILD_SVE2)
            set(GNUCC_ARCH ${SVE2_ARCH})
        elseif (BUILD_SVE)
            set(GNUCC_ARCH ${SVE_ARCH})
        else ()
            set(GNUCC_ARCH ${ARMV8_ARCH})
        endif()
        set(TUNE_FLAG generic)
    elseif(ARCH_ARM32)
       set(GNUCC_ARCH armv7a)
       set(TUNE_FLAG generic)
    else()
       set(GNUCC_ARCH power9)
       set(TUNE_FLAG power9)
    endif()
endif()
