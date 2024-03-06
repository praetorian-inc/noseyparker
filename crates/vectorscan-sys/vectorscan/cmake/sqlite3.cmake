#
# a lot of noise to find sqlite
#

option(SQLITE_PREFER_STATIC "Build sqlite3 statically instead of using an installed lib" OFF)

if(NOT SQLITE_PREFER_STATIC)
find_package(PkgConfig QUIET)

# first check for sqlite on the system
pkg_check_modules(SQLITE3 sqlite3)
endif()

# now do version checks
if (SQLITE3_FOUND)
    list(INSERT CMAKE_REQUIRED_INCLUDES 0 "${SQLITE3_INCLUDE_DIRS}")
    if (SQLITE_VERSION LESS "3.8.10")
        message(FATAL_ERROR "sqlite3 is broken from 3.8.7 to 3.8.10 - please find a working version")
    endif()
endif()

if (NOT SQLITE3_BUILD_SOURCE)
    set(_SAVED_FLAGS ${CMAKE_REQUIRED_FLAGS})
    list(INSERT CMAKE_REQUIRED_LIBRARIES 0 ${SQLITE3_LDFLAGS})
    CHECK_SYMBOL_EXISTS(sqlite3_open_v2 sqlite3.h HAVE_SQLITE3_OPEN_V2)
    list(REMOVE_ITEM CMAKE_REQUIRED_INCLUDES "${SQLITE3_INCLUDE_DIRS}")
    list(REMOVE_ITEM CMAKE_REQUIRED_LIBRARIES ${SQLITE3_LDFLAGS})
else()
    if (NOT TARGET sqlite3_static)
    # build sqlite as a static lib to compile into our test programs
    add_library(sqlite3_static STATIC "${PROJECT_SOURCE_DIR}/sqlite3/sqlite3.c")
    set_target_properties(sqlite3_static PROPERTIES COMPILE_FLAGS "-Wno-error -Wno-extra -Wno-unused -Wno-cast-qual -DSQLITE_OMIT_LOAD_EXTENSION")
    endif()
endif()

# that's enough about sqlite
