use libgit2_sys as raw;

/// Get the maximum memory that will be mapped in total by the libgit2 C library.
///
/// See <https://libgit2.org/libgit2/#HEAD/group/libgit2/git_libgit2_opts>.
///
/// Note: This is not exposed natively by `git2`, so instead it is implemented here.
pub fn get_mwindow_mapped_limit() -> usize {
    let mut out: libc::size_t = 0;
    let rc = unsafe {
        raw::git_libgit2_opts(
            raw::GIT_OPT_GET_MWINDOW_MAPPED_LIMIT as libc::c_int,
            &mut out,
        )
    };

    // Looks like `git_libgit2_opts` can't ever fail for GIT_OPT_GET_MWINDOW_MAPPED_LIMIT:
    // https://github.com/libgit2/libgit2/blob/936b184e7494158c20e522981f4a324cac6ffa47/src/libgit2/libgit2.c#L179-L181
    assert!(rc >= 0);

    #[allow(clippy::useless_conversion)]
    out.try_into().expect("result should be convertible to usize")
}

/// Set the maximum amount of memory that can be mapped at any time by the libgit2 C library.
///
/// See <https://libgit2.org/libgit2/#HEAD/group/libgit2/git_libgit2_opts>.
///
/// Note: This is not exposed natively by `git2`, so instead it is implemented here.
pub fn set_mwindow_mapped_limit(limit: usize) {
    #[allow(clippy::useless_conversion)]
    let limit: libc::size_t = limit.try_into().expect("input should be convertible to libc::size_t");
    let rc = unsafe {
        raw::git_libgit2_opts(
            raw::GIT_OPT_SET_MWINDOW_MAPPED_LIMIT as libc::c_int,
            limit,
        )
    };

    // Looks like `git_libgit2_opts` can't ever fail for GIT_OPT_GET_MWINDOW_MAPPED_LIMIT:
    // https://github.com/libgit2/libgit2/blob/936b184e7494158c20e522981f4a324cac6ffa47/src/libgit2/libgit2.c#L175-L177
    assert!(rc >= 0);
}
