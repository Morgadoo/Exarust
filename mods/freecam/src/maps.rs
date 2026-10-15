//! Level names, read from the game's own `Resource.rpk`.
//!
//! An entry counts as a level when its scene file carries a tile grid or a
//! list of placed props. The game directory is where the executable lives.

use std::path::PathBuf;

use windows_sys::Win32::System::LibraryLoader::GetModuleFileNameW;

pub struct Map {
    pub name: String,
}

/// Levels in the installation, in the order the pack stores them.
pub fn levels() -> Vec<Map> {
    let Some(root) = install_dir() else {
        return Vec::new();
    };
    let Ok(file) = std::fs::File::open(root.join("Resource.rpk")) else {
        return Vec::new();
    };
    names(&file)
        .into_iter()
        .filter(|name| name.ends_with(".rfc") && is_level(&file, name))
        .map(|name| Map {
            name: name.trim_end_matches(".rfc").to_string(),
        })
        .collect()
}

fn install_dir() -> Option<PathBuf> {
    let mut buffer = [0u16; 260];
    let length = unsafe { GetModuleFileNameW(std::ptr::null_mut(), buffer.as_mut_ptr(), 260) };
    if length == 0 {
        return None;
    }
    let path = String::from_utf16_lossy(&buffer[..length as usize]);
    PathBuf::from(path).parent().map(PathBuf::from)
}

/// Entry names of an RPK. The table is 32 bytes per entry.
fn names(file: &std::fs::File) -> Vec<String> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = file;
    let mut header = [0u8; 8];
    if file.read_exact(&mut header).is_err() {
        return Vec::new();
    }
    let magic = u32::from_le_bytes(header[0..4].try_into().unwrap());
    let table = u32::from_le_bytes(header[4..8].try_into().unwrap()) as usize;
    if magic != 0xAFBF_0C01 || !table.is_multiple_of(32) {
        return Vec::new();
    }
    let mut raw = vec![0u8; table];
    if file.read_exact(&mut raw).is_err() {
        return Vec::new();
    }
    let _ = file.seek(SeekFrom::Start(0));
    raw.chunks_exact(32)
        .map(entry_name)
        .filter(|name| !name.is_empty())
        .collect()
}

/// A scene file is a level when it holds a tile grid (`0x3DED`) or props (`0x3DE1`).
fn is_level(file: &std::fs::File, name: &str) -> bool {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = file;
    let mut header = [0u8; 8];
    if file.seek(SeekFrom::Start(0)).is_err() || file.read_exact(&mut header).is_err() {
        return false;
    }
    let table = u32::from_le_bytes(header[4..8].try_into().unwrap()) as u64;
    let mut raw = vec![0u8; table as usize];
    if file.read_exact(&mut raw).is_err() {
        return false;
    }
    let Some(entry) = raw.chunks_exact(32).find(|entry| entry_name(entry) == name) else {
        return false;
    };
    let offset = u32::from_le_bytes(entry[16..20].try_into().unwrap()) as u64;
    let size = u32::from_le_bytes(entry[20..24].try_into().unwrap()) as usize;
    let mut body = vec![0u8; size.min(64 * 1024)];
    if file.seek(SeekFrom::Start(8 + table + offset)).is_err() || file.read_exact(&mut body).is_err()
    {
        return false;
    }
    body.windows(4).any(|word| {
        let tag = u32::from_le_bytes(word.try_into().unwrap());
        tag == 0x3DED || tag == 0x3DE1
    })
}

fn entry_name(entry: &[u8]) -> String {
    let end = entry[..16].iter().position(|b| *b == 0).unwrap_or(16);
    String::from_utf8_lossy(&entry[..end]).into_owned()
}
