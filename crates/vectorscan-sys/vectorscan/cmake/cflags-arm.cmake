if (NOT FAT_RUNTIME)
    if (BUILD_SVE2_BITPERM)
        message (STATUS "SVE2_BITPERM implies SVE2, enabling BUILD_SVE2")
        set(BUILD_SVE2 ON)
    endif ()
    if (BUILD_SVE2)
        message (STATUS "SVE2 implies SVE, enabling BUILD_SVE")
        set(BUILD_SVE ON)
    endif ()
endif ()


if (CMAKE_COMPILER_IS_GNUCXX)
    set(ARMV9BASE_MINVER "12")
    if (CMAKE_CXX_COMPILER_VERSION VERSION_LESS ARMV9BASE_MINVER)
        set(SVE2_ARCH "armv8-a+sve2")
    else()
        set(SVE2_ARCH "armv9-a")
    endif()
else()
    set(SVE2_ARCH "armv9-a")
endif()

set(ARMV8_ARCH "armv8-a")
set(SVE_ARCH "${ARMV8_ARCH}+sve")
set(SVE2_BITPERM_ARCH "${SVE2_ARCH}+sve2-bitperm")

CHECK_INCLUDE_FILE_CXX(arm_neon.h HAVE_C_ARM_NEON_H)
if (BUILD_SVE OR BUILD_SVE2 OR BUILD_SVE2_BITPERM OR FAT_RUNTIME)
  set(CMAKE_REQUIRED_FLAGS "-march=${SVE_ARCH}")
  CHECK_INCLUDE_FILE_CXX(arm_sve.h HAVE_C_ARM_SVE_H)
  if (NOT HAVE_C_ARM_SVE_H)
    message(FATAL_ERROR "arm_sve.h is required to build for SVE.")
  endif()
endif()

CHECK_C_SOURCE_COMPILES("#include <arm_neon.h>
int main() {
    int32x4_t a = vdupq_n_s32(1);
    (void)a;
}" HAVE_NEON)

if (BUILD_SVE2_BITPERM)
    set(CMAKE_REQUIRED_FLAGS "-march=${SVE2_BITPERM_ARCH}")
    CHECK_C_SOURCE_COMPILES("#include <arm_sve.h>
    int main() {
        svuint8_t a = svbext(svdup_u8(1), svdup_u8(2));
        (void)a;
    }" HAVE_SVE2_BITPERM)
endif()
if (BUILD_SVE2)
    set(CMAKE_REQUIRED_FLAGS "-march=${SVE2_ARCH}")
    CHECK_C_SOURCE_COMPILES("#include <arm_sve.h>
        int main() {
            svuint8_t a = svbsl(svdup_u8(1), svdup_u8(2), svdup_u8(3));
            (void)a;
    }" HAVE_SVE2)
endif()
if (BUILD_SVE)
    set(CMAKE_REQUIRED_FLAGS "-march=${SVE_ARCH}")
    CHECK_C_SOURCE_COMPILES("#include <arm_sve.h>
        int main() {
            svuint8_t a = svdup_u8(1);
            (void)a;
    }" HAVE_SVE)
endif ()

if (FAT_RUNTIME)
    if (NOT HAVE_NEON)
        message(FATAL_ERROR "NEON support required to build fat runtime")
    endif ()
    if (BUILD_SVE AND NOT HAVE_SVE)
        message(FATAL_ERROR "SVE support required to build fat runtime")
    endif ()
    if (BUILD_SVE2 AND NOT HAVE_SVE2)
        message(FATAL_ERROR "SVE2 support required to build fat runtime")
    endif ()
    if (BUILD_SVE2_BITPERM AND NOT HAVE_SVE2_BITPERM)
        message(FATAL_ERROR "SVE2 support required to build fat runtime")
    endif ()
else (NOT FAT_RUNTIME)
    if (NOT BUILD_SVE)
        message(STATUS "Building without SVE support")
    endif ()
    if (NOT BUILD_SVE2)
        message(STATUS "Building without SVE2 support")
    endif ()
    if (NOT HAVE_NEON)
        message(FATAL_ERROR "Neon/ASIMD support required for Arm support")
    endif ()
endif ()


