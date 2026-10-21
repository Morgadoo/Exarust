use crate::{Error, Result, read_u32};

/// `10 BD DF AF` on disk.
pub const PROJECT_MAGIC: u32 = 0xAFDF_BD10;

const NAME_LEN: usize = 20;

/// `Exanima.rfp`: which packs make up the game.
///
/// Layout (inferred from the shipped file): magic, `u32` zero, `name[20]`,
/// then length-prefixed strings — the resource pack, followed by three
/// counted lists: object libraries, texture libraries and material libraries.
/// Material library paths (`Resource\Common.rml`) name entries inside the
/// resource pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub name: String,
    pub resource_pack: String,
    pub object_libraries: Vec<String>,
    pub texture_libraries: Vec<String>,
    pub material_libraries: Vec<String>,
}

struct Cursor<'a> {
    b: &'a [u8],
    pos: usize,
}

impl Cursor<'_> {
    fn u32(&mut self) -> Result<u32> {
        let v = read_u32(self.b, self.pos)?;
        self.pos += 4;
        Ok(v)
    }

    fn string(&mut self) -> Result<String> {
        let len = self.u32()? as usize;
        let bytes = self
            .b
            .get(self.pos..self.pos + len)
            .ok_or(Error::Truncated {
                offset: self.pos,
                needed: len,
                len: self.b.len(),
            })?;
        self.pos += len;
        Ok(bytes.iter().map(|&c| c as char).collect())
    }

    fn list(&mut self) -> Result<Vec<String>> {
        let n = self.u32()?;
        if n > 1024 {
            return Err(Error::Malformed(format!("implausible list length {n}")));
        }
        (0..n).map(|_| self.string()).collect()
    }
}

impl Project {
    pub fn parse(b: &[u8]) -> Result<Self> {
        let magic = read_u32(b, 0)?;
        if magic != PROJECT_MAGIC {
            return Err(Error::BadMagic {
                found: magic,
                expected: PROJECT_MAGIC,
            });
        }
        let raw = b.get(8..8 + NAME_LEN).ok_or(Error::Truncated {
            offset: 8,
            needed: NAME_LEN,
            len: b.len(),
        })?;
        let end = raw.iter().position(|&c| c == 0).unwrap_or(NAME_LEN);
        let name = raw[..end].iter().map(|&c| c as char).collect();
        let mut c = Cursor {
            b,
            pos: 8 + NAME_LEN,
        };
        let resource_pack = c.string()?;
        let object_libraries = c.list()?;
        let texture_libraries = c.list()?;
        let material_libraries = c.list()?;
        if c.pos != b.len() {
            return Err(Error::Malformed(format!(
                "{} trailing bytes after project",
                b.len() - c.pos
            )));
        }
        Ok(Self {
            name,
            resource_pack,
            object_libraries,
            texture_libraries,
            material_libraries,
        })
    }

    /// Resolves a path from the project the way the original's path resolver
    /// does: a path with a drive letter in it (`C:\…`, or any `:`) is taken
    /// as it is; any other path is joined to the project's folder. Windows
    /// separators become the platform's, so a project's `Resource\Common.rml`
    /// names the same entry on every host.
    pub fn resolve_path(folder: &std::path::Path, path: &str) -> std::path::PathBuf {
        if path.contains(':') {
            return std::path::PathBuf::from(path);
        }
        let separator = std::path::MAIN_SEPARATOR;
        let relative = path.replace(['\\', '/'], &separator.to_string());
        folder.join(relative)
    }

    /// The user folder the original picks: the project's `Userdata` folder
    /// when it exists, otherwise `<appdata>/<project name>`; without an
    /// `APPDATA` (a non-Windows host) it keeps the project's `Userdata`.
    /// The original's appdata is the roaming one (`CSIDL_APPDATA`).
    pub fn user_folder(
        folder: &std::path::Path,
        name: &str,
        appdata: Option<&std::path::Path>,
    ) -> std::path::PathBuf {
        let local = folder.join("Userdata");
        if local.is_dir() {
            return local;
        }
        match appdata {
            Some(appdata) => appdata.join(name),
            None => local,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_counted_lists() {
        let mut b = PROJECT_MAGIC.to_le_bytes().to_vec();
        b.extend_from_slice(&[0; 4]);
        let mut name = [0u8; NAME_LEN];
        name[..4].copy_from_slice(b"Game");
        b.extend_from_slice(&name);
        let s = |b: &mut Vec<u8>, v: &str| {
            b.extend_from_slice(&(v.len() as u32).to_le_bytes());
            b.extend_from_slice(v.as_bytes());
        };
        s(&mut b, "Resource");
        b.extend_from_slice(&1u32.to_le_bytes());
        s(&mut b, "Objlib");
        b.extend_from_slice(&1u32.to_le_bytes());
        s(&mut b, "Textures");
        b.extend_from_slice(&2u32.to_le_bytes());
        s(&mut b, "Resource\\A.rml");
        s(&mut b, "Resource\\B.rml");
        let p = Project::parse(&b).unwrap();
        assert_eq!(p.name, "Game");
        assert_eq!(p.object_libraries, ["Objlib"]);
        assert_eq!(p.material_libraries.len(), 2);
    }

    #[test]
    fn paths_with_a_drive_are_absolute_and_others_join_the_folder() {
        let root = std::path::Path::new("/games/exanima");
        assert_eq!(
            Project::resolve_path(root, "C:\\Games\\Exanima\\Resource.rpk"),
            std::path::PathBuf::from("C:\\Games\\Exanima\\Resource.rpk")
        );
        assert_eq!(
            Project::resolve_path(root, "Resource\\Common.rml"),
            root.join("Resource").join("Common.rml")
        );
        assert_eq!(
            Project::resolve_path(root, "Resource.rpk"),
            root.join("Resource.rpk")
        );
    }

    #[test]
    fn the_user_folder_prefers_the_projects_own_and_then_appdata() {
        let dir = std::env::temp_dir().join("exarust-project-user-folder-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("Userdata")).unwrap();
        assert_eq!(
            Project::user_folder(&dir, "Exanima", Some(std::path::Path::new("C:\\AppData"))),
            dir.join("Userdata")
        );
        std::fs::remove_dir_all(dir.join("Userdata")).unwrap();
        assert_eq!(
            Project::user_folder(&dir, "Exanima", Some(std::path::Path::new("C:\\AppData"))),
            std::path::Path::new("C:\\AppData").join("Exanima")
        );
        assert_eq!(
            Project::user_folder(&dir, "Exanima", None),
            dir.join("Userdata")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
