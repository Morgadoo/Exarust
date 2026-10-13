use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::{Error, MappedArchive, Project, Result};

/// The packs a retail installation ships, by role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PackKind {
    /// Levels, terrain, characters' base meshes, UI images, databases, shaders.
    Resource,
    /// Static world props and modular level pieces.
    Objlib,
    /// World and prop textures.
    Textures,
    /// Interactive items: containers, furniture, wearable and wieldable items.
    Objects,
    /// Wearable meshes fitted to the character body.
    Characters,
    /// Hair and other character extensions.
    CharExt,
    /// Weapon parts (heads, hafts, guards, pommels) assembled by factories.
    Components,
    ComponentTex,
    /// Apparel textures.
    Apparel,
    Emblems,
    /// Weapon assembly recipes over `Components`.
    Factories,
    /// RIFF/WAVE PCM sounds and music.
    Sound,
}

impl PackKind {
    pub const ALL: [PackKind; 12] = [
        PackKind::Resource,
        PackKind::Objlib,
        PackKind::Textures,
        PackKind::Objects,
        PackKind::Characters,
        PackKind::CharExt,
        PackKind::Components,
        PackKind::ComponentTex,
        PackKind::Apparel,
        PackKind::Emblems,
        PackKind::Factories,
        PackKind::Sound,
    ];

    pub fn file_stem(self) -> &'static str {
        match self {
            PackKind::Resource => "Resource",
            PackKind::Objlib => "Objlib",
            PackKind::Textures => "Textures",
            PackKind::Objects => "Objects",
            PackKind::Characters => "Characters",
            PackKind::CharExt => "CharExt",
            PackKind::Components => "Components",
            PackKind::ComponentTex => "ComponentTex",
            PackKind::Apparel => "Apparel",
            PackKind::Emblems => "Emblems",
            PackKind::Factories => "Factories",
            PackKind::Sound => "Sound",
        }
    }

    pub fn from_name(name: &str) -> Option<PackKind> {
        let stem = name.strip_suffix(".rpk").unwrap_or(name);
        Self::ALL
            .into_iter()
            .find(|k| k.file_stem().eq_ignore_ascii_case(stem))
    }
}

/// An opened installation: the project file plus every pack that is present.
pub struct GameData {
    root: PathBuf,
    project: Project,
    packs: BTreeMap<PackKind, MappedArchive>,
}

impl GameData {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let rfp = root.join("Exanima.rfp");
        let bytes = std::fs::read(&rfp).map_err(|source| Error::Io {
            path: rfp.clone(),
            source,
        })?;
        let project = Project::parse(&bytes)?;
        let mut packs = BTreeMap::new();
        for kind in PackKind::ALL {
            let path = root.join(format!("{}.rpk", kind.file_stem()));
            if path.exists() {
                packs.insert(kind, MappedArchive::open(&path)?);
            }
        }
        if !packs.contains_key(&PackKind::Resource) {
            return Err(Error::Malformed(format!(
                "{} has no Resource.rpk",
                root.display()
            )));
        }
        Ok(Self {
            root,
            project,
            packs,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    pub fn pack(&self, kind: PackKind) -> Option<&MappedArchive> {
        self.packs.get(&kind)
    }

    pub fn packs(&self) -> impl Iterator<Item = (PackKind, &MappedArchive)> {
        self.packs.iter().map(|(k, v)| (*k, v))
    }

    /// Resolves a material library path from the project (`Resource\Common.rml`)
    /// to its bytes inside the resource pack.
    pub fn resource_file(&self, path: &str) -> Option<&[u8]> {
        let name = path.rsplit(['\\', '/']).next().unwrap_or(path);
        self.pack(PackKind::Resource)?.get(name)
    }
}

/// Finds the installation: an explicit path, then `EXANIMA_DIR` from the
/// environment, then `EXANIMA_DIR` from a `.env` in the working directory.
pub fn locate_install(explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(p) = explicit {
        return Some(p.to_path_buf());
    }
    if let Ok(p) = std::env::var("EXANIMA_DIR") {
        return Some(PathBuf::from(p));
    }
    let env = std::fs::read_to_string(".env").ok()?;
    env.lines()
        .find_map(|l| l.trim().strip_prefix("EXANIMA_DIR="))
        .map(|v| PathBuf::from(v.trim().trim_matches('"')))
}
