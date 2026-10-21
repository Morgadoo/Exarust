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
        // The resource pack: the original opens `Resource` in the project
        // folder (the project file's own copy of the name is the same).
        let resource_file =
            Project::resolve_path(&root, &format!("{}.rpk", PackKind::Resource.file_stem()));
        let resource = MappedArchive::open(&resource_file)?;
        packs.insert(PackKind::Resource, resource);
        // The object and texture libraries the project names must be there;
        // a missing one is fatal in the original ("Unable to locate data
        // files."). A name Exarust has no pack kind for is skipped: nothing
        // here could look anything up in it.
        for name in project
            .object_libraries
            .iter()
            .chain(&project.texture_libraries)
        {
            let Some(kind) = PackKind::from_name(name) else {
                continue;
            };
            let path = Project::resolve_path(&root, &format!("{}.rpk", kind.file_stem()));
            if !path.exists() {
                return Err(Error::Malformed(format!(
                    "{} has no {}",
                    root.display(),
                    path.display()
                )));
            }
            packs.insert(kind, MappedArchive::open(&path)?);
        }
        // Exarust also uses the packs the original opens on demand later
        // (items, characters, sounds, …); they are optional here.
        for kind in PackKind::ALL {
            if packs.contains_key(&kind) {
                continue;
            }
            let path = root.join(format!("{}.rpk", kind.file_stem()));
            if path.exists() {
                packs.insert(kind, MappedArchive::open(&path)?);
            }
        }
        Ok(Self {
            root,
            project,
            packs,
        })
    }

    /// The original's user folder: `<root>/Userdata` when it exists, else
    /// `<APPDATA>/<project name>` (see [`Project::user_folder`]).
    pub fn user_folder(&self) -> PathBuf {
        let appdata = std::env::var_os("APPDATA").map(PathBuf::from);
        Project::user_folder(&self.root, &self.project.name, appdata.as_deref())
    }

    /// A project path resolved against the installation folder.
    pub fn resolve_path(&self, path: &str) -> PathBuf {
        Project::resolve_path(&self.root, path)
    }

    /// The object libraries the project names, as opened packs, in the
    /// project's order.
    pub fn object_libraries(&self) -> impl Iterator<Item = &MappedArchive> {
        self.project
            .object_libraries
            .iter()
            .filter_map(|name| self.pack(PackKind::from_name(name)?))
    }

    /// The texture libraries the project names, as opened packs, in the
    /// project's order.
    pub fn texture_libraries(&self) -> impl Iterator<Item = &MappedArchive> {
        self.project
            .texture_libraries
            .iter()
            .filter_map(|name| self.pack(PackKind::from_name(name)?))
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

    /// Finds an image by bare name (no extension), searching the texture
    /// packs first and the resource pack (where entries keep `.rfi`) last.
    pub fn texture(&self, name: &str) -> Option<&[u8]> {
        const PACKS: [PackKind; 5] = [
            PackKind::Textures,
            PackKind::Apparel,
            PackKind::ComponentTex,
            PackKind::Emblems,
            PackKind::Resource,
        ];
        let with_ext = format!("{name}.rfi");
        PACKS.iter().find_map(|&kind| {
            let pack = self.pack(kind)?;
            pack.get(name).or_else(|| pack.get(&with_ext))
        })
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
