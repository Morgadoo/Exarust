//! Placement markers: the scene nodes a lifted item can snap onto.
//!
//! When the original attaches a node to its sector it looks the node's
//! object id up in the object database. A node with no record (a model's
//! own child node, whose id word is zero) whose name starts with `[` goes
//! into the sector's marker list; so does an object whose volume section's
//! flags word reads `0x22` in its second byte. The item cursor later walks
//! those lists: the eight name characters after the first, read as
//! hexadecimal, name the object (or its template) the marker takes, and
//! the ninth character `L` makes the marker give the item back.
//!
//! Shipped markers are child nodes of object models: the torch holders'
//! `[0000001F]` and the teleporter's and resurrection device's
//! `[000001F4L]` and `[000001AFL]`. No shipped volume section carries the
//! `0x22` flags (every one reads `0x00`, `0x10`, `0x11` or `0x12`), so the
//! object branch is kept as the rule ([`registers_as_marker`]) and the
//! level loader registers the named nodes only.

use bevy::prelude::*;

/// The first character of a marker node's name.
pub const MARKER_LEAD: u8 = b'[';
/// The volume flags' second byte that makes an object a marker.
pub const MARKER_VOLUME: u32 = 0x2200;
/// The mask of the volume flags' second byte.
pub const VOLUME_KIND_MASK: u32 = 0xFF00;

/// On a scene node the original puts in its sector's marker list: the
/// eight name characters after the lead (zero past the name's end) and the
/// character after them.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacementMarkerNode {
    pub name: [u8; 8],
    pub link: u8,
}

impl PlacementMarkerNode {
    /// The marker fields of a node name: the bytes at 1..9 and the byte at
    /// 9, zero where the name is shorter (the original reads them from its
    /// fixed, zero-filled name field).
    pub fn of_name(name: &str) -> Self {
        let bytes = name.as_bytes();
        let at = |i: usize| bytes.get(i).copied().unwrap_or(0);
        Self {
            name: std::array::from_fn(|i| at(i + 1)),
            link: at(9),
        }
    }
}

/// On a placed object's model root: the template its record names (the
/// record header's parent word), which the item cursor compares with a
/// marker's id beside the object's own id.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacedTemplate(pub u32);

/// Whether a node attached to a sector goes into the sector's marker list:
/// with no object record, when its name starts with `[`; with a record,
/// when the record's volume section flags read [`MARKER_VOLUME`] in their
/// second byte (no volume section, no marker).
pub fn registers_as_marker(has_record: bool, name: &str, volume_flags: Option<u32>) -> bool {
    if has_record {
        volume_flags.is_some_and(|f| f & VOLUME_KIND_MASK == MARKER_VOLUME)
    } else {
        name.as_bytes().first() == Some(&MARKER_LEAD)
    }
}

/// The marker component of a model node, when it registers: a model's
/// nodes carry no object id, so only the name decides.
pub fn model_node_marker(name: &str) -> Option<PlacementMarkerNode> {
    registers_as_marker(false, name, None).then(|| PlacementMarkerNode::of_name(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A node without a record registers by its `[` lead; one with a record
    /// by its volume flags' second byte alone.
    #[test]
    fn which_nodes_register_as_markers() {
        assert!(registers_as_marker(false, "[0000001F]", None));
        assert!(!registers_as_marker(false, "TrcHdA01 L", None));
        assert!(!registers_as_marker(false, "", None));
        // A record ignores the name.
        assert!(!registers_as_marker(true, "[0000001F]", None));
        assert!(registers_as_marker(true, "Crate", Some(0x2240)));
        assert!(registers_as_marker(true, "Crate", Some(0x0001_22FF)));
        assert!(!registers_as_marker(true, "Crate", Some(0x1140)));
        assert!(!registers_as_marker(true, "Crate", Some(0x1200)));
    }

    /// The fields are the eight bytes after the lead and the ninth; a short
    /// name leaves zeros.
    #[test]
    fn a_marker_name_gives_its_id_characters_and_link() {
        let torch = model_node_marker("[0000001F]").unwrap();
        assert_eq!(&torch.name, b"0000001F");
        assert_eq!(torch.link, b']');
        let device = model_node_marker("[000001F4L]").unwrap();
        assert_eq!(&device.name, b"000001F4");
        assert_eq!(device.link, b'L');
        let short = model_node_marker("[RC").unwrap();
        assert_eq!(short.name, [b'R', b'C', 0, 0, 0, 0, 0, 0]);
        assert_eq!(short.link, 0);
        assert_eq!(model_node_marker("Torch"), None);
    }

    /// The shipped torch holder carries one marker, for the torch's id
    /// 0x1F, and its bounding sphere (skipped without the game).
    #[test]
    fn the_torch_holder_carries_a_marker() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let pack = game.pack(exa_assets::PackKind::Objlib).unwrap();
        let scene = exa_formats::rfc::Scene::parse(pack.get("torch holder a1").unwrap()).unwrap();
        let markers: Vec<_> = scene
            .nodes
            .iter()
            .filter_map(|n| model_node_marker(&n.name).map(|m| (m, n.bounds_radius)))
            .collect();
        assert_eq!(markers.len(), 1);
        assert_eq!(&markers[0].0.name, b"0000001F");
        assert!(markers[0].1 > 0.0, "{}", markers[0].1);
    }
}
