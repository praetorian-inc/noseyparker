
CHECK_INCLUDE_FILE_CXX(altivec.h HAVE_C_PPC64EL_ALTIVEC_H)

if (HAVE_C_PPC64EL_ALTIVEC_H)
    set (INTRIN_INC_H "altivec.h")
else()
    message (FATAL_ERROR "No intrinsics header found for VSX")
endif ()

CHECK_C_SOURCE_COMPILES("#include <${INTRIN_INC_H}>
int main() {
    vector int a = vec_splat_s32(1);
    (void)a;
}" HAVE_VSX)

if (NOT HAVE_VSX)
    message(FATAL_ERROR "VSX support required for Power support")
endif ()
