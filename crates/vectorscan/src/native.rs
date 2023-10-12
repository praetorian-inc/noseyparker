use foreign_types::ForeignType;
use std::ffi::{c_int, c_uint, c_ulonglong, c_void};
use vectorscan_sys as hs;

use super::{wrapper, AsResult, Error, HyperscanErrorCode, Pattern, ScanMode};

pub enum Scan {
    Continue,
    Terminate,
}

pub struct BlockDatabase {
    db: wrapper::Database,
}

pub struct BlockScanner<'db> {
    scratch: wrapper::Scratch,
    database: &'db wrapper::Database,
}

impl BlockDatabase {
    pub fn new(patterns: Vec<Pattern>) -> Result<Self, Error> {
        let db = wrapper::Database::new(patterns, ScanMode::BLOCK)?;
        Ok(Self { db })
    }

    pub fn create_scanner(&self) -> Result<BlockScanner, Error> {
        BlockScanner::new(self)
    }
}

impl<'db> BlockScanner<'db> {
    pub fn new(db: &'db BlockDatabase) -> Result<Self, Error> {
        Ok(Self {
            database: &db.db,
            scratch: wrapper::Scratch::new(&db.db)?,
        })
    }

    pub fn scan<F>(&mut self, data: &[u8], on_match: F) -> Result<Scan, Error>
    where
        F: FnMut(u32, u64, u64, u32) -> Scan,
    {
        let mut context = Context { on_match };

        let res = unsafe {
            hs::hs_scan(
                self.database.as_ptr(),
                data.as_ptr() as *const _,
                data.len() as u32,
                0,
                self.scratch.as_ptr(),
                Some(on_match_trampoline::<F>),
                &mut context as *mut _ as *mut c_void,
            )
            .ok()
        };

        match res {
            Ok(_) => Ok(Scan::Continue),
            Err(err) => match err {
                Error::Hyperscan(HyperscanErrorCode::ScanTerminated, _) => Ok(Scan::Terminate),
                err => Err(err),
            },
        }
    }
}

/// Bundles together Rust state to be passed to a C FFI Hyperscan matching API.
///
/// This serves to wrap a Rust closure with a layer of indirection, so it can be referred to
/// through a `void *` pointer in C.
struct Context<F>
where
    F: FnMut(u32, u64, u64, u32) -> Scan,
{
    on_match: F,
}

unsafe extern "C" fn on_match_trampoline<F>(
    id: c_uint,
    from: c_ulonglong,
    to: c_ulonglong,
    flags: c_uint,
    ctx: *mut c_void,
) -> c_int
where
    F: FnMut(u32, u64, u64, u32) -> Scan,
{
    let context = (ctx as *mut Context<F>)
        .as_mut()
        .expect("context object should be set");
    match (context.on_match)(id, from, to, flags) {
        Scan::Continue => 0,
        Scan::Terminate => 1,
    }
}
