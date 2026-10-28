//! The grid builder's set catalog: the wall sets and floor sets a level
//! names, each registered once and referred to afterwards by its index.
//!
//! The original keeps one list of wall sets and one of floor sets for the
//! whole session. A level's grid lists the set names its cells use; the
//! builder looks each up in the catalog, registering it on first use, and a
//! grid cell then only stores an index. Before a grid is built every set
//! still unloaded is loaded once; a floor set named exactly `Terrain` is
//! never loaded (the terrain is the ground of those cells), it is only
//! marked. The loaded models themselves are cached by the model loader, so
//! this catalog holds only the names and their state.
//!
//! Rules and data layout: `docs/formats/RFC.md` (Grid building).

/// Which catalog a set belongs to: wall sets (`gews_*`) or floor sets
/// (`gefs_*`). The original keeps the two lists separately and a cell's
/// plane C indexes each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SetKind {
    Wall,
    Floor,
}

/// The entry's set has been loaded (the original's flag bit 0).
pub const SET_LOADED: u32 = 1;
/// The flags a floor set named `Terrain` gets instead of loading: marked
/// loaded and as having no floor pieces.
pub const SET_TERRAIN: u32 = 0x7e01;

/// One registered set: its name and the builder's flag word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetEntry {
    /// The set's name, compared whole and case-sensitively, as the
    /// original's 16-byte name is.
    pub name: String,
    /// [`SET_LOADED`] once loaded, or [`SET_TERRAIN`] for the terrain set.
    pub flags: u32,
}

impl SetEntry {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            flags: 0,
        }
    }

    /// Whether the set has been loaded (or is the terrain set, which is
    /// never loaded).
    pub fn is_loaded(&self) -> bool {
        self.flags & SET_LOADED != 0
    }

    /// Whether this is a floor set named exactly `Terrain`.
    pub fn is_terrain(&self) -> bool {
        self.flags & SET_TERRAIN == SET_TERRAIN
    }
}

/// The catalog's two registries.
#[derive(Debug, Clone, Default)]
pub struct SetCatalog {
    walls: Vec<SetEntry>,
    floors: Vec<SetEntry>,
}

impl SetCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `name` in `kind`'s list unless it is already there and
    /// returns its index, as the original's registration does. The
    /// comparison is the whole name.
    pub fn register(&mut self, kind: SetKind, name: &str) -> usize {
        let list = self.list(kind);
        match list.iter().position(|e| e.name == name) {
            Some(index) => index,
            None => {
                list.push(SetEntry::new(name));
                list.len() - 1
            }
        }
    }

    /// Registers a wall set.
    pub fn register_wall(&mut self, name: &str) -> usize {
        self.register(SetKind::Wall, name)
    }

    /// Registers a floor set.
    pub fn register_floor(&mut self, name: &str) -> usize {
        self.register(SetKind::Floor, name)
    }

    pub fn entries(&self, kind: SetKind) -> &[SetEntry] {
        match kind {
            SetKind::Wall => &self.walls,
            SetKind::Floor => &self.floors,
        }
    }

    pub fn entry(&self, kind: SetKind, index: usize) -> Option<&SetEntry> {
        self.entries(kind).get(index)
    }

    fn list(&mut self, kind: SetKind) -> &mut Vec<SetEntry> {
        match kind {
            SetKind::Wall => &mut self.walls,
            SetKind::Floor => &mut self.floors,
        }
    }

    /// Marks a registered set loaded, as the set loader's flag write does.
    pub fn mark_loaded(&mut self, kind: SetKind, index: usize) {
        if let Some(entry) = self.list(kind).get_mut(index) {
            entry.flags |= SET_LOADED;
        }
    }

    /// Marks a registered floor set as the terrain's: [`SET_TERRAIN`], never
    /// loaded and never bringing floor pieces.
    pub fn mark_terrain(&mut self, index: usize) {
        if let Some(entry) = self.floors.get_mut(index) {
            entry.flags |= SET_TERRAIN;
        }
    }

    /// The load hook: every set still unloaded, wall sets first and then
    /// floor sets, in registration order. Each returned name is marked
    /// loaded. A floor set named exactly `Terrain` is not returned; it is
    /// marked [`SET_TERRAIN`] instead, so it never carries floor pieces.
    pub fn load_pending(&mut self) -> Vec<(SetKind, String)> {
        let mut out = Vec::new();
        for kind in [SetKind::Wall, SetKind::Floor] {
            let list = self.list(kind);
            for entry in list.iter_mut() {
                if entry.is_loaded() {
                    continue;
                }
                if kind == SetKind::Floor && entry.name == "Terrain" {
                    entry.flags |= SET_TERRAIN;
                } else {
                    entry.flags |= SET_LOADED;
                    out.push((kind, entry.name.clone()));
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_registered_once_per_kind() {
        let mut catalog = SetCatalog::new();
        assert_eq!(catalog.register_wall("GEWS_UWA01"), 0);
        assert_eq!(catalog.register_wall("GEWS_SWA01"), 1);
        assert_eq!(catalog.register_wall("GEWS_UWA01"), 0);
        // The two kinds are separate lists.
        assert_eq!(catalog.register_floor("GEWS_UWA01"), 0);
        assert_eq!(catalog.entries(SetKind::Wall).len(), 2);
        assert_eq!(catalog.entries(SetKind::Floor).len(), 1);
        assert_eq!(catalog.entry(SetKind::Wall, 1).unwrap().name, "GEWS_SWA01");
        assert!(catalog.entry(SetKind::Wall, 2).is_none());
    }

    #[test]
    fn the_terrain_floor_set_is_marked_and_never_loaded() {
        let mut catalog = SetCatalog::new();
        catalog.register_floor("GEFS_UWA01");
        catalog.register_wall("GEWS_UWA01");
        catalog.register_floor("Terrain");
        // A name that only contains the word is an ordinary set.
        catalog.register_floor("GEFS_Terrain01");
        let pending = catalog.load_pending();
        assert_eq!(
            pending,
            [
                (SetKind::Wall, "GEWS_UWA01".to_string()),
                (SetKind::Floor, "GEFS_UWA01".to_string()),
                (SetKind::Floor, "GEFS_Terrain01".to_string()),
            ]
        );
        let terrain = catalog.entry(SetKind::Floor, 1).unwrap();
        assert!(terrain.is_terrain());
        // 0x7E01 carries the loaded bit, so the set is not offered again.
        assert!(terrain.is_loaded());
        assert_eq!(terrain.flags, SET_TERRAIN);
        assert!(catalog.load_pending().is_empty());
        // Neither is a loaded set offered twice.
        assert_eq!(catalog.entry(SetKind::Wall, 0).unwrap().flags, SET_LOADED);
    }
}
