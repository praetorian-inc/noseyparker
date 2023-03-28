use crate::error::{AsResult, Error};
use bitflags::bitflags;
use foreign_types::{foreign_type, ForeignType};
use vectorscan_sys as hs;
use std::{ffi::CString, mem::MaybeUninit, ptr};

foreign_type! {
    unsafe type CompileError {
        type CType = hs::hs_compile_error_t;
        fn drop = hs::hs_free_compile_error;
    }

    pub unsafe type Database: Send + Sync {
        type CType = hs::hs_database_t;
        fn drop = hs::hs_free_database;
    }

    pub unsafe type Scratch {
        type CType = hs::hs_scratch_t;
        fn drop = hs::hs_free_scratch;
    }

    /*
    pub unsafe type Stream {
        type CType = hs::hs_stream_t;
        fn drop = stream_drop;
    }
    */
}

/*
unsafe fn stream_drop(stream: *mut hs::hs_stream_t) {
    let _ = hs::hs_close_stream(stream, ptr::null_mut(), None, ptr::null_mut());
}
*/

bitflags! {
    #[derive(Default)]
    pub struct Flag: u32 {
        const CASELESS = hs::HS_FLAG_CASELESS;
        const DOTALL = hs::HS_FLAG_DOTALL;
        const MULTILINE = hs::HS_FLAG_MULTILINE;
        const SINGLEMATCH = hs::HS_FLAG_SINGLEMATCH;
        const ALLOWEMPTY = hs::HS_FLAG_ALLOWEMPTY;
        const UTF8 = hs::HS_FLAG_UTF8;
        const UCP = hs::HS_FLAG_UCP;
        const PREFILTER = hs::HS_FLAG_PREFILTER;
        const SOM_LEFTMOST = hs::HS_FLAG_SOM_LEFTMOST;
        const COMBINATION = hs::HS_FLAG_COMBINATION;
        const QUIET = hs::HS_FLAG_QUIET;
    }
}

pub struct Pattern {
    expression: Vec<u8>,
    flags: Flag,
    id: Option<u32>,
}

impl Pattern {
    pub fn new(expression: Vec<u8>, flags: Flag, id: Option<u32>) -> Self {
        Self {
            expression,
            flags,
            id,
        }
    }
}

impl Database {
    pub fn new(patterns: Vec<Pattern>, mode: ScanMode) -> Result<Self, Error> {
        let mut c_exprs = Vec::with_capacity(patterns.len());
        let mut c_flags = Vec::with_capacity(patterns.len());
        let mut c_ids = Vec::with_capacity(patterns.len());
        for Pattern {
            expression,
            flags,
            id,
        } in patterns
        {
            // have to keep the original strings until the db is created
            let c_expr = CString::new(expression)?;
            c_exprs.push(c_expr);
            c_flags.push(flags.bits());
            c_ids.push(id.unwrap_or(0));
        }

        let mut db = MaybeUninit::uninit();
        let mut err = MaybeUninit::uninit();
        unsafe {
            hs::hs_compile_ext_multi(
                c_exprs
                    .iter()
                    .map(|expr| expr.as_ptr())
                    .collect::<Vec<_>>()
                    .as_ptr(),
                c_flags.as_ptr(),
                c_ids.as_ptr(),
                ptr::null(),
                c_exprs.len() as u32,
                mode.bits(),
                ptr::null(),
                db.as_mut_ptr(),
                err.as_mut_ptr(),
            )
            .ok()
            .map_err(|_| err.assume_init())?;
            Ok(Database::from_ptr(db.assume_init()))
        }
    }

    pub fn try_clone(&self) -> Result<Self, Error> {
        let mut buf = MaybeUninit::uninit();
        let mut len = 0usize;
        unsafe {
            hs::hs_serialize_database(self.as_ptr(), buf.as_mut_ptr(), &mut len).ok()?;
            let buf = buf.assume_init();
            let mut copy = MaybeUninit::uninit();
            hs::hs_deserialize_database(buf, len, copy.as_mut_ptr()).ok()?;
            let copy = copy.assume_init();
            Ok(Self::from_ptr(copy))
        }
    }
}

impl Scratch {
    pub fn new(database: &Database) -> Result<Self, Error> {
        let mut scratch = MaybeUninit::zeroed();
        unsafe {
            hs::hs_alloc_scratch(database.as_ptr(), scratch.as_mut_ptr())
                .ok()
                .map(|_| Scratch::from_ptr(scratch.assume_init()))
        }
    }
}
/*
impl Stream {
    pub fn new(database: &Database) -> Result<Self, Error> {
        let mut stream = MaybeUninit::uninit();
        unsafe {
            hs::hs_open_stream(database.as_ptr(), 0, stream.as_mut_ptr())
                .ok()
                .map(|_| Stream::from_ptr(stream.assume_init()))
        }
    }
}
*/

impl CompileError {
    fn message(&self) -> String {
        unsafe {
            let err = self.0.as_ptr();

            std::ffi::CStr::from_ptr((*err).message)
                .to_str()
                .unwrap()
                .into()
        }
    }
    fn expression(&self) -> i32 {
        unsafe { (*self.0.as_ptr()).expression }
    }
}

impl From<*mut hs::hs_compile_error> for Error {
    fn from(err: *mut hs::hs_compile_error) -> Self {
        unsafe {
            let err = CompileError::from_ptr(err);
            Self::HypercanCompile(err.message(), err.expression())
        }
    }
}

bitflags! {
pub struct ScanMode: u32 {
    const BLOCK = hs::HS_MODE_BLOCK;
    const VECTORED = hs::HS_MODE_VECTORED;
    const STREAM = hs::HS_MODE_STREAM;
    const SOM_SMALL = hs::HS_MODE_SOM_HORIZON_SMALL;
    const SOM_MEDIUM = hs::HS_MODE_SOM_HORIZON_MEDIUM;
    const SOM_LARGE = hs::HS_MODE_SOM_HORIZON_LARGE;
}
}
