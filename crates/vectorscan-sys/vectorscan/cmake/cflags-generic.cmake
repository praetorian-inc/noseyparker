# set compiler flags - more are tested and added later
set(EXTRA_C_FLAGS "${OPT_C_FLAG} -std=c17 -Wall -Wextra -Wshadow -Wcast-qual -fno-strict-aliasing")
set(EXTRA_CXX_FLAGS "${OPT_CXX_FLAG} -std=c++17 -Wall -Wextra -Wshadow -Wswitch -Wreturn-type -Wcast-qual -Wno-deprecated -Wnon-virtual-dtor -fno-strict-aliasing")
if (NOT CMAKE_COMPILER_IS_CLANG)
    set(EXTRA_CXX_FLAGS "${EXTRA_CXX_FLAGS} -fno-new-ttp-matching")
endif()

if (NOT RELEASE_BUILD)
    # -Werror is most useful during development, don't potentially break
    # release builds
    set(EXTRA_C_FLAGS "${EXTRA_C_FLAGS} -Werror")
    set(EXTRA_CXX_FLAGS "${EXTRA_CXX_FLAGS} -Werror")
    if (CMAKE_COMPILER_IS_CLANG)
    	if (CMAKE_C_COMPILER_VERSION VERSION_GREATER "13.0")
           set(EXTRA_C_FLAGS "${EXTRA_C_FLAGS} -Wno-unused-but-set-variable")
           set(EXTRA_CXX_FLAGS "${EXTRA_CXX_FLAGS} -Wno-unused-but-set-variable")
        endif()
    endif()
endif()

if (DISABLE_ASSERTS)
    set(EXTRA_C_FLAGS "${EXTRA_C_FLAGS} -DNDEBUG")
    set(EXTRA_CXX_FLAGS "${EXTRA_CXX_FLAGS} -DNDEBUG")
endif()

if(CMAKE_COMPILER_IS_GNUCC)
    # spurious warnings?
    set(EXTRA_C_FLAGS "${EXTRA_C_FLAGS} -Wno-array-bounds -Wno-maybe-uninitialized")
endif()

if(CMAKE_COMPILER_IS_GNUCXX)
    set(EXTRA_CXX_FLAGS "${EXTRA_CXX_FLAGS} -Wno-maybe-uninitialized")
    if (CMAKE_CXX_COMPILER_VERSION VERSION_LESS 5.0)
        set(EXTRA_CXX_FLAGS "${EXTRA_CXX_FLAGS} -fabi-version=0")
    endif ()
    # don't complain about abi
    set(EXTRA_C_FLAGS "${EXTRA_C_FLAGS} -Wno-abi")
    set(EXTRA_CXX_FLAGS "${EXTRA_CXX_FLAGS} -Wno-abi")
endif()

if (NOT(ARCH_IA32 AND RELEASE_BUILD))
    set(EXTRA_C_FLAGS "${EXTRA_C_FLAGS} -fno-omit-frame-pointer")
    set(EXTRA_CXX_FLAGS "${EXTRA_CXX_FLAGS} -fno-omit-frame-pointer")
endif()

CHECK_INCLUDE_FILES(unistd.h HAVE_UNISTD_H)
CHECK_FUNCTION_EXISTS(posix_memalign HAVE_POSIX_MEMALIGN)
CHECK_FUNCTION_EXISTS(_aligned_malloc HAVE__ALIGNED_MALLOC)

# these end up in the config file
CHECK_C_COMPILER_FLAG(-fvisibility=hidden HAS_C_HIDDEN)
CHECK_CXX_COMPILER_FLAG(-fvisibility=hidden HAS_CXX_HIDDEN)

# are we using libc++
CHECK_CXX_SYMBOL_EXISTS(_LIBCPP_VERSION ciso646 HAVE_LIBCPP)

if (RELEASE_BUILD)
    if (HAS_C_HIDDEN)
        set(EXTRA_C_FLAGS "${EXTRA_C_FLAGS} -fvisibility=hidden")
    endif()
    if (HAS_CXX_HIDDEN)
        set(EXTRA_CXX_FLAGS "${EXTRA_CXX_FLAGS} -fvisibility=hidden")
    endif()
endif()

# testing a builtin takes a little more work
CHECK_C_SOURCE_COMPILES("void *aa_test(void *x) { return __builtin_assume_aligned(x, 16);}\nint main(void) { return 0; }" HAVE_CC_BUILTIN_ASSUME_ALIGNED)
CHECK_CXX_SOURCE_COMPILES("void *aa_test(void *x) { return __builtin_assume_aligned(x, 16);}\nint main(void) { return 0; }" HAVE_CXX_BUILTIN_ASSUME_ALIGNED)
# Clang does not use __builtin_constant_p() the same way as gcc
if (NOT CMAKE_COMPILER_IS_CLANG)
   CHECK_C_SOURCE_COMPILES("int main(void) { __builtin_constant_p(0); }" HAVE__BUILTIN_CONSTANT_P)
endif()

set(C_FLAGS_TO_CHECK
# Variable length arrays are way bad, most especially at run time
"-Wvla"
# Pointer arith on void pointers is doing it wrong.
 "-Wpointer-arith"
# Build our C code with -Wstrict-prototypes -Wmissing-prototypes
 "-Wstrict-prototypes"
 "-Wmissing-prototypes"
)
foreach (FLAG ${C_FLAGS_TO_CHECK})
    # munge the name so it doesn't break things
    string(REPLACE "-" "_" FNAME C_FLAG${FLAG})
    CHECK_C_COMPILER_FLAG("${FLAG}" ${FNAME})
    if (${FNAME})
        set(EXTRA_C_FLAGS "${EXTRA_C_FLAGS} ${FLAG}")
    endif()
endforeach()

# self-assign should be thrown away, but clang whinges
CHECK_C_COMPILER_FLAG("-Wself-assign" CC_SELF_ASSIGN)
if (CC_SELF_ASSIGN)
    set(EXTRA_C_FLAGS "${EXTRA_C_FLAGS} -Wno-self-assign")
endif()
CHECK_CXX_COMPILER_FLAG("-Wself-assign" CXX_SELF_ASSIGN)
if (CXX_SELF_ASSIGN)
    set(EXTRA_CXX_FLAGS "${EXTRA_CXX_FLAGS} -Wno-self-assign")
endif()

# clang gets up in our face for going paren crazy with macros
CHECK_C_COMPILER_FLAG("-Wparentheses-equality" CC_PAREN_EQUALITY)
if (CC_PAREN_EQUALITY)
    set(EXTRA_C_FLAGS "${EXTRA_C_FLAGS} -Wno-parentheses-equality")
endif()

# clang complains about unused const vars in our Ragel-generated code.
CHECK_CXX_COMPILER_FLAG("-Wunused-const-variable" CXX_UNUSED_CONST_VAR)
if (CXX_UNUSED_CONST_VAR)
    set(EXTRA_CXX_FLAGS "${EXTRA_CXX_FLAGS} -Wno-unused-const-variable")
endif()

# clang-14 complains about unused-but-set variable.
CHECK_CXX_COMPILER_FLAG("-Wunused-but-set-variable" CXX_UNUSED_BUT_SET_VAR)
if (CXX_UNUSED_BUT_SET_VAR)
    set(EXTRA_CXX_FLAGS "${EXTRA_CXX_FLAGS} -Wno-unused-but-set-variable")
endif()

# clang-14 complains about using bitwise operator instead of logical ones.
CHECK_CXX_COMPILER_FLAG("-Wbitwise-instead-of-logical" CXX_BITWISE_INSTEAD_OF_LOGICAL)
if (CXX_BITWISE_INSTEAD_OF_LOGICAL)
    set(EXTRA_CXX_FLAGS "${EXTRA_CXX_FLAGS} -Wno-bitwise-instead-of-logical")
endif()

# clang-14 complains about using bitwise operator instead of logical ones.
CHECK_CXX_COMPILER_FLAG("-Wbitwise-instead-of-logical" CXX_BITWISE_INSTEAD_OF_LOGICAL)
if (CXX_BITWISE_INSTEAD_OF_LOGICAL)
    set(EXTRA_CXX_FLAGS "${EXTRA_CXX_FLAGS} -Wno-bitwise-instead-of-logical")
endif()

CHECK_CXX_COMPILER_FLAG("-Wignored-attributes" CXX_IGNORED_ATTR)
if (CXX_IGNORED_ATTR)
    set(EXTRA_CXX_FLAGS "${EXTRA_CXX_FLAGS} -Wno-ignored-attributes")
endif()

# gcc 9 complains about redundant move for returned variable
CHECK_CXX_COMPILER_FLAG("-Wredundant-move" CXX_REDUNDANT_MOVE)
if (CXX_REDUNDANT_MOVE)
    set(EXTRA_CXX_FLAGS "${EXTRA_CXX_FLAGS} -Wno-redundant-move")
endif()

# note this for later, g++ doesn't have this flag but clang does
CHECK_CXX_COMPILER_FLAG("-Wweak-vtables" CXX_WEAK_VTABLES)
if (CXX_WEAK_VTABLES)
    set(EXTRA_CXX_FLAGS "${EXTRA_CXX_FLAGS} -Wweak-vtables")
endif()

CHECK_CXX_COMPILER_FLAG("-Wmissing-declarations" CXX_MISSING_DECLARATIONS)
if (CXX_MISSING_DECLARATIONS)
    set(EXTRA_CXX_FLAGS "${EXTRA_CXX_FLAGS} -Wmissing-declarations")
endif()

CHECK_CXX_COMPILER_FLAG("-Wunused-local-typedefs" CXX_UNUSED_LOCAL_TYPEDEFS)

CHECK_CXX_COMPILER_FLAG("-Wunused-variable" CXX_WUNUSED_VARIABLE)

# gcc 10 complains about this
CHECK_C_COMPILER_FLAG("-Wstringop-overflow" CC_STRINGOP_OVERFLOW)
CHECK_CXX_COMPILER_FLAG("-Wstringop-overflow" CXX_STRINGOP_OVERFLOW)
if(CC_STRINGOP_OVERFLOW OR CXX_STRINGOP_OVERFLOW)
    set(EXTRA_C_FLAGS "${EXTRA_C_FLAGS} -Wno-stringop-overflow")
    set(EXTRA_CXX_FLAGS "${EXTRA_CXX_FLAGS} -Wno-stringop-overflow")
endif()
