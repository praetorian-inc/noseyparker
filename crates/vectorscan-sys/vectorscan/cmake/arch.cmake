# detect architecture features
#
# must be called after determining where compiler intrinsics are defined

if (HAVE_C_X86INTRIN_H)
    set (INTRIN_INC_H "x86intrin.h")
elseif (HAVE_C_INTRIN_H)
    set (INTRIN_INC_H "intrin.h")
elseif (HAVE_C_ARM_NEON_H)
    set (INTRIN_INC_H "arm_neon.h")
    set (FAT_RUNTIME OFF)
elseif (HAVE_C_PPC64EL_ALTIVEC_H)
    set (INTRIN_INC_H "altivec.h")
    set (FAT_RUNTIME OFF)
else()
    message (FATAL_ERROR "No intrinsics header found")
endif ()

if (ARCH_ARM32 OR ARCH_AARCH64)
    CHECK_C_SOURCE_COMPILES("#include <${INTRIN_INC_H}>
int main() {
    int32x4_t a = vdupq_n_s32(1);
    (void)a;
}" HAVE_NEON)
endif ()

if (ARCH_AARCH64)
    set(PREV_FLAGS "${CMAKE_C_FLAGS}")
    if (BUILD_SVE2_BITPERM)
        set(CMAKE_C_FLAGS "-march=${GNUCC_ARCH} ${CMAKE_C_FLAGS}")
        CHECK_C_SOURCE_COMPILES("#include <arm_sve.h>
        int main() {
            svuint8_t a = svbext(svdup_u8(1), svdup_u8(2));
            (void)a;
        }" HAVE_SVE2_BITPERM)
        if (HAVE_SVE2_BITPERM)
            add_definitions(-DHAVE_SVE2_BITPERM)
        endif ()
    endif()
    if (BUILD_SVE2)
        set(CMAKE_C_FLAGS "-march=${GNUCC_ARCH} ${CMAKE_C_FLAGS}")
        CHECK_C_SOURCE_COMPILES("#include <arm_sve.h>
        int main() {
            svuint8_t a = svbsl(svdup_u8(1), svdup_u8(2), svdup_u8(3));
            (void)a;
        }" HAVE_SVE2)
    endif()
    if (HAVE_SVE2 OR HAVE_SVE2_BITPERM)
        add_definitions(-DHAVE_SVE2)
    endif ()
    if (BUILD_SVE)
        set(CMAKE_C_FLAGS "-march=${GNUCC_ARCH} ${CMAKE_C_FLAGS}")
        CHECK_C_SOURCE_COMPILES("#include <arm_sve.h>
        int main() {
            svuint8_t a = svdup_u8(1);
            (void)a;
        }" HAVE_SVE)
    endif ()
    if (HAVE_SVE OR HAVE_SVE2 OR HAVE_SVE2_BITPERM)
        add_definitions(-DHAVE_SVE)
    endif ()
    set(CMAKE_C_FLAGS "${PREV_FLAGS}")
endif()

if (BUILD_AVX512)
    CHECK_C_COMPILER_FLAG(${SKYLAKE_FLAG} HAS_ARCH_SKYLAKE)
    if (NOT HAS_ARCH_SKYLAKE)
        message (FATAL_ERROR "AVX512 not supported by compiler")
    endif ()
endif ()

if (BUILD_AVX512VBMI)
    CHECK_C_COMPILER_FLAG(${ICELAKE_FLAG} HAS_ARCH_ICELAKE)
    if (NOT HAS_ARCH_ICELAKE)
        message (FATAL_ERROR "AVX512VBMI not supported by compiler")
    endif ()
endif ()

if (FAT_RUNTIME)
    if (NOT DEFINED(BUILD_AVX2))
        set(BUILD_AVX2 TRUE)
    endif ()
    # test the highest level microarch to make sure everything works
    if (BUILD_AVX512)
        if (BUILD_AVX512VBMI)
            set (CMAKE_REQUIRED_FLAGS "${CMAKE_C_FLAGS} ${EXTRA_C_FLAGS} ${ICELAKE_FLAG}")
        else ()
            set (CMAKE_REQUIRED_FLAGS "${CMAKE_C_FLAGS} ${EXTRA_C_FLAGS} ${SKYLAKE_FLAG}")
        endif (BUILD_AVX512VBMI)
    elseif (BUILD_AVX2)
        set (CMAKE_REQUIRED_FLAGS "${CMAKE_C_FLAGS} ${EXTRA_C_FLAGS} -march=core-avx2 -mavx2")
    elseif ()
        set (CMAKE_REQUIRED_FLAGS "${CMAKE_C_FLAGS} ${EXTRA_C_FLAGS} -march=core-i7 -mssse3")
    endif ()
else (NOT FAT_RUNTIME)
    # if not fat runtime, then test given cflags
    set (CMAKE_REQUIRED_FLAGS "${CMAKE_C_FLAGS} ${EXTRA_C_FLAGS} ${ARCH_C_FLAGS}")
endif ()

if (ARCH_IA32 OR ARCH_X86_64)
    # ensure we have the minimum of SSE4.2 - call a SSE4.2 intrinsic
    CHECK_C_SOURCE_COMPILES("#include <${INTRIN_INC_H}>
int main() {
    __m128i a = _mm_set1_epi8(1);
    (void)_mm_shuffle_epi8(a, a);
}" HAVE_SSE42)

    # now look for AVX2
    CHECK_C_SOURCE_COMPILES("#include <${INTRIN_INC_H}>
#if !defined(__AVX2__)
#error no avx2
#endif

int main(){
    __m256i z = _mm256_setzero_si256();
    (void)_mm256_xor_si256(z, z);
}" HAVE_AVX2)

    # and now for AVX512
    CHECK_C_SOURCE_COMPILES("#include <${INTRIN_INC_H}>
#if !defined(__AVX512BW__)
#error no avx512bw
#endif

int main(){
    __m512i z = _mm512_setzero_si512();
    (void)_mm512_abs_epi8(z);
}" HAVE_AVX512)

    # and now for AVX512VBMI
    CHECK_C_SOURCE_COMPILES("#include <${INTRIN_INC_H}>
#if !defined(__AVX512VBMI__)
#error no avx512vbmi
#endif

int main(){
    __m512i a = _mm512_set1_epi8(0xFF);
    __m512i idx = _mm512_set_epi64(3ULL, 2ULL, 1ULL, 0ULL, 7ULL, 6ULL, 5ULL, 4ULL);
    (void)_mm512_permutexvar_epi8(idx, a);
}" HAVE_AVX512VBMI)


elseif (ARCH_ARM32 OR ARCH_AARCH64)
    CHECK_C_SOURCE_COMPILES("#include <${INTRIN_INC_H}>
int main() {
    int32x4_t a = vdupq_n_s32(1);
    (void)a;
}" HAVE_NEON)
elseif (ARCH_PPC64EL)
    CHECK_C_SOURCE_COMPILES("#include <${INTRIN_INC_H}>
int main() {
    vector int a = vec_splat_s32(1);
    (void)a;
}" HAVE_VSX)
else ()
    message (FATAL_ERROR "Unsupported architecture")
endif ()

if (FAT_RUNTIME)
    if ((ARCH_IA32 OR ARCH_X86_64) AND NOT HAVE_SSE42)
        message(FATAL_ERROR "SSE4.2 support required to build fat runtime")
    endif ()
    if ((ARCH_IA32 OR ARCH_X86_64) AND BUILD_AVX2 AND NOT HAVE_AVX2)
        message(FATAL_ERROR "AVX2 support required to build fat runtime")
    endif ()
    if ((ARCH_IA32 OR ARCH_X86_64) AND BUILD_AVX512 AND NOT HAVE_AVX512)
        message(FATAL_ERROR "AVX512 support requested but not supported")
    endif ()
    if ((ARCH_IA32 OR ARCH_X86_64) AND BUILD_AVX512VBMI AND NOT HAVE_AVX512VBMI)
        message(FATAL_ERROR "AVX512VBMI support requested but not supported")
    endif ()
else (NOT FAT_RUNTIME)
    if ((ARCH_IA32 OR ARCH_X86_64) AND NOT BUILD_AVX2)
        message(STATUS "Building without AVX2 support")
    endif ()
    if ((ARCH_IA32 OR ARCH_X86_64) AND NOT HAVE_AVX512)
        message(STATUS "Building without AVX512 support")
    endif ()
    if ((ARCH_IA32 OR ARCH_X86_64) AND NOT HAVE_AVX512VBMI)
        message(STATUS "Building without AVX512VBMI support")
    endif ()
    if ((ARCH_IA32 OR ARCH_X86_64) AND NOT HAVE_SSE42)
        message(FATAL_ERROR "A minimum of SSE4.2 compiler support is required")
    endif ()
    if ((ARCH_ARM32 OR ARCH_AARCH64) AND NOT HAVE_NEON)
        message(FATAL_ERROR "NEON support required for ARM support")
    endif ()
    if (ARCH_PPPC64EL AND NOT HAVE_VSX)
        message(FATAL_ERROR "VSX support required for Power support")
    endif ()

endif ()

unset (PREV_FLAGS)
unset (CMAKE_REQUIRED_FLAGS)
unset (INTRIN_INC_H)
