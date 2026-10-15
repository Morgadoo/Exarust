//! Finds a byte pattern in the game module. `?` matches any byte, so a
//! pattern survives a rebuild that only moves an address.

use std::ffi::c_void;

use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::SystemServices::IMAGE_DOS_HEADER;

pub struct Module {
    base: *const u8,
    size: usize,
}

impl Module {
    /// The module this thread is running in: `Exanima.exe`.
    pub fn current() -> Option<Self> {
        let base = unsafe { GetModuleHandleW(std::ptr::null()) } as *const u8;
        if base.is_null() {
            return None;
        }
        let size = unsafe { image_size(base) }?;
        Some(Self { base, size })
    }

    pub fn find(&self, pattern: &str) -> Option<*mut c_void> {
        let bytes = parse(pattern)?;
        let hay = unsafe { std::slice::from_raw_parts(self.base, self.size) };
        hay.windows(bytes.len())
            .position(|window| {
                window
                    .iter()
                    .zip(&bytes)
                    .all(|(have, want)| want.is_none_or(|b| *have == b))
            })
            .map(|at| unsafe { self.base.add(at) as *mut c_void })
    }
}

fn parse(pattern: &str) -> Option<Vec<Option<u8>>> {
    pattern
        .split_whitespace()
        .map(|token| match token {
            "?" | "??" => Some(None),
            _ => u8::from_str_radix(token, 16).ok().map(Some),
        })
        .collect()
}

/// Size of the loaded image, from its own headers. The optional header's
/// `SizeOfImage` sits at a fixed offset in a 64-bit PE.
unsafe fn image_size(base: *const u8) -> Option<usize> {
    let dos = unsafe { &*(base as *const IMAGE_DOS_HEADER) };
    if dos.e_magic != 0x5A4D {
        return None;
    }
    let nt = unsafe { base.offset(dos.e_lfanew as isize) };
    let signature = unsafe { (nt as *const u32).read() };
    if signature != 0x4550 {
        return None;
    }
    let size = unsafe { (nt.add(0x50) as *const u32).read() };
    Some(size as usize)
}
