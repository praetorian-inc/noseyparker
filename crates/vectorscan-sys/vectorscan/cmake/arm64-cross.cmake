set(CMAKE_SYSTEM_NAME "Linux")
set(CMAKE_SYSTEM_PROCESSOR "aarch64")

# specify the cross compiler
set(CMAKE_C_COMPILER "$ENV{CROSS}gcc")
set(CMAKE_CXX_COMPILER "$ENV{CROSS}g++")
# where is the target environment
set(CMAKE_SYSROOT $ENV{CROSS_SYS})

set(Boost_INCLUDE_DIR $ENV{BOOST_PATH})

# for libraries and headers in the target directories
set(CMAKE_FIND_ROOT_PATH_MODE_LIBRARY ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_INCLUDE ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_PACKAGE ONLY)

set(THREADS_PTHREAD_ARG "2" CACHE STRING "Result from TRY_RUN" FORCE)

set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -falign-functions=16 -falign-jumps=16 -falign-labels=16 -falign-loops=16" CACHE STRING "" FORCE)

set(GNUCC_ARCH "armv8.2-a+fp16+simd+rcpc+dotprod+crypto")
set(TUNE_FLAG "neoverse-n1")