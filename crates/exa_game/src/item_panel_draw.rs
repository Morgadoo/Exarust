//! The item panel's icons through the GUI draw path.
//!
//! The panel draws each visible entry's icon at its cell from the item
//! manager's icon atlas, tinted by the entry's flags (green or yellow for a
//! highlight, black otherwise, half alpha when the yellow mask matches),
//! through the item panel's own highlight program: the vertex colour's red
//! mixes the highlight colour towards white, its green scales a ring of
//! neighbouring alphas into a glow, and its alpha scales the fragment. The
//! program is [`exa_ui::skin::SkinMode::HltCol`]; its shader and the same
//! arithmetic are in `exa_items::item_gui` and
//! [`exa_ui::gui_draw::item_icon_fragment`].
//!
//! The icons themselves are rendered off screen by the item manager (see
//! `exa_items::item_renderer`); this module only takes the atlas they were
//! packed into. When no atlas or no icon is available the panel draws the
//! entry not at all, as the original skips an entry whose icon is 0.
//!
//! Evidence: the panel's paint routine and the icon program.

use std::collections::VecDeque;

use bevy::prelude::*;

use exa_items::catalogue::ItemCatalogue;
use exa_items::icon_cache::{IconCache, key, streamed_key};
use exa_items::item_icon::IconScene;
use exa_ui::gui_draw::{GuiLayer, GuiTexture, LayerTexture};
use exa_ui::icon_atlas::{IconAtlas, IconImage};
use exa_ui::skin::{SkinBatch, SkinMode};

use crate::item_container::{Container, tint};

/// The blend the item panel's icons draw with.
pub const PANEL_BLEND: u16 = 0x203;

/// An icon waiting for the off-screen renderer. `scene` is baked by the
/// render pass (or filled by a headless test); a slot without one is still
/// reserved, so the panel's entry keeps a handle.
#[derive(Debug, Clone)]
pub struct PendingIcon {
    pub id: u32,
    /// The cache key, from the item's model flags.
    pub key: u32,
    /// The render variant: the key plus the framing and condition bits.
    pub variant: u32,
    /// The index in [`IconCache::entries`].
    pub entry: usize,
    /// The model flags the render plan reads.
    pub model_flags: u32,
    /// The baked scene, once the model is loaded.
    pub scene: Option<IconScene>,
    /// How often the loader missed the model.
    pub attempts: u32,
    /// The frames the prepared scene has waited for its textures.
    pub waited: u32,
}

/// The item manager's icon atlas and cache, and the texture the panel
/// samples. The atlas is empty until the item renderer fills it.
#[derive(Resource)]
pub struct PanelIcons {
    pub atlas: IconAtlas,
    pub cache: IconCache,
    /// The icons the render pass still has to fill.
    pub pending: VecDeque<PendingIcon>,
    texture: Option<GuiTexture>,
    /// The atlas image the texture was built from: its size and the icon
    /// version.
    revision: (u16, u16, u64),
    /// Bumped whenever an icon is added to the atlas.
    version: u64,
}

impl Default for PanelIcons {
    fn default() -> Self {
        Self::new(4)
    }
}

impl PanelIcons {
    /// An empty manager for a GUI unit.
    pub fn new(unit: i32) -> Self {
        Self {
            atlas: IconAtlas::new(40 * unit, 4),
            cache: IconCache::new(unit),
            pending: VecDeque::new(),
            texture: None,
            revision: (0, 0, 0),
            version: 0,
        }
    }

    /// The icon handle and half extents for an item with `flags`, reserving
    /// a cache slot when the renderer has not made one. The cache key is the
    /// original acquire's: a lit model keeps the framing bit and its
    /// condition byte and gains the lit bit; a streamed icon keeps the top
    /// tag bits and gains the effect bit for a model that asks for it. The
    /// slot has no atlas cell (`-1`, which the draw skips) until the
    /// off-screen renderer fills it.
    pub fn reserve(
        &mut self,
        id: u32,
        flags: u32,
        catalogue: Option<&ItemCatalogue>,
    ) -> (usize, (i32, i32)) {
        if id == 0 {
            return (0, self.icon_half());
        }
        let model = catalogue.and_then(|catalogue| catalogue.model(id));
        let name = match model {
            Some(model) => {
                let lit = model.flags & 1 != 0;
                let effect = model.flags & 0x2000 != 0;
                streamed_key(flags, lit, effect)
            }
            // A record without a model has no icon in the original; the
            // slot still holds the entry so the panel can list it.
            None => key(flags, false),
        };
        self.reserve_keyed(id, name, model.map_or(0, |m| m.flags))
    }

    /// The icon handle a description card holds, by the card's own acquire:
    /// the entry's flags less their top four bits, or with the apparel
    /// framing bit added when the model's flags have `0x1000`, keyed as
    /// [`key`] keys them (no streamed or effect bit, unlike the panel's
    /// object-database acquire). `model_flags` is the model section's
    /// flags word, whose bit 0 lights the model.
    pub fn reserve_card(&mut self, id: u32, flags: u32, model_flags: u32) -> (usize, (i32, i32)) {
        if id == 0 {
            return (0, self.icon_half());
        }
        let flags = if model_flags & 0x1000 == 0 {
            flags & 0x0FFF_FFFF
        } else {
            flags | exa_items::icon_cache::variant::APPAREL
        };
        let name = key(flags, model_flags & 1 != 0);
        self.reserve_keyed(id, name, model_flags)
    }

    /// The acquire's slot walk: a slot holding `id` under `name` gains a
    /// reference; otherwise the first freed slot (no id) is reused, else
    /// one is appended, and the new slot waits for the renderer.
    fn reserve_keyed(&mut self, id: u32, name: u32, model_flags: u32) -> (usize, (i32, i32)) {
        if let Some(at) = self.entries_slot(id, name) {
            let entry = &mut self.cache.entries[at];
            entry.refs = entry.refs.saturating_add(1);
            return (at + 1, (entry.half_width, entry.half_height));
        }
        let (half_width, half_height) = self.icon_half();
        let slot = exa_items::icon_cache::Entry {
            id,
            cell: -1,
            variant: name,
            half_width,
            half_height,
            refs: 1,
        };
        let free = self
            .cache
            .entries
            .iter()
            .position(|entry| entry.id == 0 && entry.refs == 0);
        let entry = match free {
            Some(at) => {
                self.cache.entries[at] = slot;
                at
            }
            None => {
                self.cache.entries.push(slot);
                self.cache.entries.len() - 1
            }
        };
        // The render variant is the key: the framing bit (and a lit model's
        // condition byte) pick the framing and turn, the effect bit the
        // subdivision, the streamed bit the object-database resolve.
        self.pending.push_back(PendingIcon {
            id,
            key: name,
            variant: name,
            entry,
            model_flags,
            scene: None,
            attempts: 0,
            waited: 0,
        });
        (entry + 1, (half_width, half_height))
    }

    /// A slot for `id` with the exact cache `variant`, when the cache holds
    /// one. A slot the renderer made has a real cell and is used as it is.
    fn entries_slot(&self, id: u32, variant: u32) -> Option<usize> {
        self.cache
            .entries
            .iter()
            .position(|entry| entry.id == id && entry.variant == variant && entry.refs > 0)
    }

    /// Fills a reserved slot with the rendered icon: its atlas cell, its
    /// half extents from the icon's content span, and the render-pass key
    /// it was rendered for. Returns whether the slot still existed.
    pub fn apply_rendered(
        &mut self,
        id: u32,
        key: u32,
        icon: exa_items::item_icon::IconPixels,
        width: f32,
        height: f32,
    ) -> bool {
        let Some(at) = self
            .cache
            .entries
            .iter()
            .position(|entry| entry.id == id && entry.variant == key && entry.cell < 0)
        else {
            return false;
        };
        if icon.width != self.atlas.cell as u32 || icon.height != self.atlas.cell as u32 {
            return false;
        }
        let image = IconImage {
            width: icon.width as u16,
            height: icon.height as u16,
            channels: 4,
            pixels: icon.pixels,
        };
        let cell = self.atlas.add(&image, id as i32);
        if cell <= 0 {
            return false;
        }
        let (half_width, half_height) = self.cache.halves(exa_items::icon_cache::Rendered {
            cell,
            width,
            height,
        });
        let entry = &mut self.cache.entries[at];
        entry.cell = cell;
        entry.half_width = half_width;
        entry.half_height = half_height;
        entry.variant = key;
        self.version = self.version.wrapping_add(1);
        true
    }

    /// The panel icon's half extents: the icon manager adds the GUI unit to
    /// each (`icon_cache`'s own halves; a reserved icon has no pixels).
    pub fn icon_half(&self) -> (i32, i32) {
        let unit = self.cache.unit;
        let half = self.atlas.cell / 2;
        (unit + half, half + unit)
    }

    /// Whether an icon slot's pixels are in the atlas (its render
    /// completed); a reserved slot waits for the item renderer.
    pub fn rendered(&self, handle: usize) -> bool {
        handle
            .checked_sub(1)
            .and_then(|slot| self.cache.entries.get(slot))
            .is_some_and(|entry| entry.cell >= 0)
    }

    /// The half extents of an icon slot, for the panel's clamps.
    pub fn half(&self, handle: usize) -> Option<(i32, i32)> {
        self.cache
            .entries
            .get(handle.checked_sub(1)?)
            .map(|entry| (entry.half_width, entry.half_height))
    }

    /// Drops one reference to an icon slot, as the manager's release does:
    /// the last reference frees the slot (its id and cell go, its variant
    /// and half extents stay) and gives the atlas cell back. A pending
    /// render for the slot is forgotten. False for no slot.
    pub fn release(&mut self, handle: usize) -> bool {
        let Some(at) = handle.checked_sub(1) else {
            return false;
        };
        let Some(entry) = self.cache.entries.get_mut(at) else {
            return false;
        };
        if entry.refs == 0 {
            return false;
        }
        entry.refs -= 1;
        if entry.refs > 0 {
            return true;
        }
        let mut cell = entry.cell;
        self.atlas.release(&mut cell);
        entry.cell = cell;
        entry.id = 0;
        self.pending.retain(|pending| pending.entry != at);
        self.version = self.version.wrapping_add(1);
        true
    }

    /// Releases every icon a container's entries hold, as the panel's
    /// destroy does, and leaves the entries without handles. Returns how
    /// many slots were released.
    pub fn release_container(&mut self, container: &mut Container) -> usize {
        let mut handles = Vec::new();
        container.release_all_icons(|icon| handles.push(icon));
        let mut released = 0;
        for handle in handles {
            if self.release(handle) {
                released += 1;
            }
        }
        released
    }

    /// The icon manager's own unit change: its icon size is the forty-unit
    /// base times the GUI unit, its renderer is resized (the atlas is
    /// rebuilt at the new cell side, so every cell is given back), and every
    /// cached icon is drawn again. A slot's cell is gone until the render
    /// finishes; meanwhile its half extents are the new unit plus half the
    /// icon side, and the finished render sets them from its measured size
    /// (half rounded to even, plus the unit) as the original's loop does.
    /// A freed slot (no id) has nothing to draw. Does nothing when the unit
    /// is unchanged.
    pub fn rescale(&mut self, unit: i32, catalogue: Option<&ItemCatalogue>) {
        let unit = unit.max(1);
        if unit == self.cache.unit {
            return;
        }
        self.cache.unit = unit;
        self.atlas = IconAtlas::new(40 * unit, 4);
        self.texture = None;
        self.revision = (0, 0, 0);
        self.version = self.version.wrapping_add(1);
        let (half_width, half_height) = self.icon_half();
        self.pending.clear();
        for at in 0..self.cache.entries.len() {
            let entry = &mut self.cache.entries[at];
            entry.cell = -1;
            entry.half_width = half_width;
            entry.half_height = half_height;
            if entry.id == 0 || entry.refs == 0 {
                continue;
            }
            let model_flags = catalogue
                .and_then(|catalogue| catalogue.model(entry.id))
                .map_or(0, |model| model.flags);
            self.pending.push_back(PendingIcon {
                id: entry.id,
                key: entry.variant,
                variant: entry.variant,
                entry: at,
                model_flags,
                scene: None,
                attempts: 0,
                waited: 0,
            });
        }
    }

    /// The atlas's texture, rebuilt when its image changed.
    fn texture(&mut self) -> Option<&GuiTexture> {
        let image = &self.atlas.image;
        let revision = (image.width, image.height, self.version);
        if self.texture.is_none() || revision != self.revision {
            self.texture = GuiTexture::from_bottom_up(
                u32::from(image.width),
                u32::from(image.height),
                &image.pixels,
            );
            self.revision = revision;
        }
        self.texture.as_ref()
    }

    /// The panel's icon layer: every visible entry whose cached icon and
    /// atlas cell exist, at its stored cell, scaled from the atlas's cell
    /// to the panel's icon size. `None` when nothing draws.
    pub fn layer(&mut self, container: &Container) -> Option<GuiLayer> {
        if container.entries.is_empty() {
            return None;
        }
        self.texture()?;
        let mut batch = SkinBatch::new();
        batch.mode = SkinMode::HltCol;
        batch.blend = PANEL_BLEND;
        let scale = if self.atlas.cell > 0 && container.icon_size > 0 {
            container.icon_size as f32 / self.atlas.cell as f32
        } else {
            1.0
        };
        let mut drew = false;
        for entry in &container.entries {
            let Some(ink) = tint(entry, container.green_mask, container.yellow_mask) else {
                continue;
            };
            let Some(cached) = entry
                .icon
                .checked_sub(1)
                .and_then(|slot| self.cache.entries.get(slot))
            else {
                continue;
            };
            // A reserved slot has no atlas cell until the item renderer
            // fills it.
            if cached.cell < 0 {
                continue;
            }
            let Some(quad) =
                self.atlas
                    .draw_batched(cached.cell, entry.cell.x as f32, entry.cell.y as f32)
            else {
                continue;
            };
            batch.set_ink(ink);
            batch.begin();
            for corner in quad.corners {
                let u = corner[2];
                let v = corner[3];
                let x = entry.cell.x as f32 + (corner[0] - entry.cell.x as f32) * scale;
                let y = entry.cell.y as f32 + (corner[1] - entry.cell.y as f32) * scale;
                batch.set_uv(u, v);
                batch.vertex(x, y);
            }
            batch.end_quads();
            drew = true;
        }
        // The atlas texture is copied into the layer only when an icon drew.
        let texture = self.texture.as_ref().filter(|_| drew)?.clone();
        Some(GuiLayer::new(batch, LayerTexture::Owned(texture)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_items::icon_cache::Entry as IconSlot;
    use exa_ui::icon_atlas::IconImage;
    use exa_ui::skin::GuiDraw;

    fn icon(fill: u8) -> IconImage {
        let mut image = IconImage::new(160, 160, 4);
        image.pixels.fill(fill);
        image
    }

    fn manager() -> PanelIcons {
        let mut manager = PanelIcons::new(4);
        let handle = manager.atlas.add(&icon(255), 7);
        assert_eq!(handle, 1);
        manager.cache.entries = vec![IconSlot {
            id: 7,
            cell: handle,
            variant: 0,
            half_width: 84,
            half_height: 84,
            refs: 1,
        }];
        manager
    }

    fn panel(flags: u32) -> Container {
        let mut container = Container {
            entries: vec![crate::item_container::Entry {
                cell: crate::item_container::Cell::new(10, 20),
                id: 7,
                icon: 1,
                flags,
                drag: 0,
            }],
            icon_size: 160,
            unit: 4,
            green_mask: 0x400,
            yellow_mask: 0x200,
            ..Default::default()
        };
        container.entries[0].drag = 0;
        container
    }

    #[test]
    fn a_visible_entry_draws_through_the_highlight_program() {
        let mut manager = manager();
        let layer = manager.layer(&panel(0)).unwrap();
        assert_eq!(layer.batch.mode, SkinMode::HltCol);
        assert_eq!(layer.batch.blend, PANEL_BLEND);
        assert_eq!(layer.batch.primitives.len(), 1);
        assert_eq!(layer.batch.primitives[0].draw, GuiDraw::Quads);
        // The ink is the panel's black tint; the quad spans the icon size.
        let primitive = &layer.batch.primitives[0];
        let vertices = layer.batch.vertices_of(primitive);
        assert_eq!(vertices.len(), 4);
        assert_eq!(vertices[0].ink, [0, 0, 0, 255]);
        assert_eq!((vertices[0].x, vertices[0].y), (10.0, 20.0));
        assert_eq!((vertices[2].x, vertices[2].y), (170.0, 180.0));
        assert!(matches!(&layer.texture, LayerTexture::Owned(t) if t.width == 256));
        // A green highlight draws green ink.
        let green = manager.layer(&panel(0x400)).unwrap();
        assert_eq!(
            green.batch.vertices_of(&green.batch.primitives[0])[0].ink,
            [0, 255, 0, 255]
        );
    }

    #[test]
    fn held_and_iconless_entries_draw_nothing() {
        let mut manager = manager();
        let mut container = panel(0);
        container.entries[0].flags = 0x100;
        assert!(manager.layer(&container).is_none());
        let mut container = panel(0);
        container.entries[0].icon = 0;
        assert!(manager.layer(&container).is_none());
        let mut container = panel(0);
        container.entries[0].icon = 9;
        assert!(manager.layer(&container).is_none());
    }

    #[test]
    fn the_card_acquire_keys_its_slot_and_reuses_a_freed_one() {
        use exa_items::icon_cache::variant;
        let mut icons = PanelIcons::new(4);
        // An unlit model: the flags lose their top four bits, so only the
        // framing bit could stay, and it is not set here.
        let (first, _) = icons.reserve_card(7, 0x1234_5678, 0);
        assert_eq!(icons.cache.entries[first - 1].variant, 0);
        // Model flag 0x1000 adds the apparel framing bit.
        let (second, _) = icons.reserve_card(7, 0x0000_0001, 0x1000);
        assert_eq!(icons.cache.entries[second - 1].variant, variant::APPAREL);
        // A lit model keeps the condition byte and gains the lit bit; the
        // masked flags lose the framing bit, and no streamed bit is added.
        let (third, _) = icons.reserve_card(8, 0x40AB_0003, 1);
        assert_eq!(
            icons.cache.entries[third - 1].variant,
            0x00AB_0000 | variant::LIT
        );
        // The same id and key gain a reference.
        assert_eq!(icons.reserve_card(7, 0, 0).0, first);
        assert_eq!(icons.cache.entries[first - 1].refs, 2);
        // A freed slot is reused before the cache grows.
        assert!(icons.release(second));
        let (reused, _) = icons.reserve_card(9, 0, 0);
        assert_eq!(reused, second);
        assert_eq!(icons.cache.entries.len(), 3);
        assert_eq!(icons.cache.entries[reused - 1].id, 9);
        assert_eq!(icons.pending.back().map(|p| p.entry), Some(reused - 1));
    }

    #[test]
    fn releasing_a_panel_drops_its_icon_references() {
        let mut manager = PanelIcons::new(4);
        let first = manager.atlas.add(&icon(255), 7);
        let second = manager.atlas.add(&icon(128), 8);
        manager.cache.entries = vec![
            IconSlot {
                id: 7,
                cell: first,
                variant: 0,
                half_width: 84,
                half_height: 84,
                refs: 2,
            },
            IconSlot {
                id: 8,
                cell: second,
                variant: 0,
                half_width: 84,
                half_height: 84,
                refs: 1,
            },
        ];
        let mut container = Container {
            entries: vec![
                crate::item_container::Entry {
                    id: 7,
                    icon: 1,
                    ..Default::default()
                },
                crate::item_container::Entry {
                    id: 8,
                    icon: 2,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        assert_eq!(manager.release_container(&mut container), 2);
        assert!(container.entries.iter().all(|entry| entry.icon == 0));
        // The first slot keeps one reference; the second is freed.
        assert_eq!(manager.cache.entries[0].refs, 1);
        assert_eq!(manager.cache.entries[0].cell, first);
        assert_eq!(manager.cache.entries[1].refs, 0);
        assert_eq!(manager.cache.entries[1].id, 0);
        // The last reference frees the first slot too.
        assert!(manager.release(1));
        assert_eq!(manager.cache.entries[0].refs, 0);
        assert_eq!(manager.cache.entries[0].id, 0);
        // A missing or already-freed slot is not released twice.
        assert!(!manager.release(1));
        assert!(!manager.release(9));
        assert!(!manager.release(0));
    }

    #[test]
    fn the_panel_scales_an_atlas_cell_to_its_icon_size() {
        let mut manager = manager();
        let mut container = panel(0);
        container.icon_size = 80;
        let layer = manager.layer(&container).unwrap();
        let vertices = layer.batch.vertices_of(&layer.batch.primitives[0]);
        assert_eq!((vertices[2].x, vertices[2].y), (90.0, 100.0));
    }

    #[test]
    fn a_unit_change_redraws_every_icon_at_forty_units() {
        let mut manager = manager();
        manager.rescale(5, None);
        // The renderer is resized to forty units a side.
        assert_eq!(manager.atlas.cell, 200);
        assert_eq!(manager.cache.unit, 5);
        // The drawn icon lost its cell and waits for the render, with the
        // new unit added to half the icon's own side.
        let slot = manager.cache.entries[0];
        assert_eq!((slot.id, slot.refs, slot.cell), (7, 1, -1));
        assert_eq!((slot.half_width, slot.half_height), (105, 105));
        assert_eq!(manager.pending.len(), 1);
        assert_eq!(manager.pending[0].id, 7);
        assert_eq!(manager.pending[0].entry, 0);
        // A freed slot is not drawn again, and an unchanged unit is a no-op.
        manager.pending.clear();
        manager.cache.entries[0].refs = 0;
        manager.rescale(4, None);
        assert_eq!(manager.atlas.cell, 160);
        assert!(manager.pending.is_empty());
    }
}
