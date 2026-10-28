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

use bevy::prelude::*;

use exa_items::icon_cache::IconCache;
use exa_ui::gui_draw::{GuiLayer, GuiTexture, LayerTexture};
use exa_ui::icon_atlas::IconAtlas;
use exa_ui::skin::{SkinBatch, SkinMode};

use crate::item_container::{Container, tint};

/// The blend the item panel's icons draw with.
pub const PANEL_BLEND: u16 = 0x203;

/// The item manager's icon atlas and cache, and the texture the panel
/// samples. The atlas is empty until the item renderer fills it.
#[derive(Resource)]
pub struct PanelIcons {
    pub atlas: IconAtlas,
    pub cache: IconCache,
    texture: Option<GuiTexture>,
    /// The atlas image the texture was built from.
    revision: (u16, u16, bool),
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
            texture: None,
            revision: (0, 0, false),
        }
    }

    /// The atlas's texture, rebuilt when its image changed.
    fn texture(&mut self) -> Option<&GuiTexture> {
        let image = &self.atlas.image;
        let revision = (image.width, image.height, self.atlas.dirty);
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
    fn the_item_program_agrees_in_both_crates() {
        let ring = [0.25, 0.5, 1.0, 0.0, 0.75, 0.125, 0.5, 1.0];
        let texel = [0.1, 0.2, 0.3, 0.6];
        let ink = [0.2, 0.7, 0.4, 0.9];
        assert_eq!(
            exa_items::item_gui::item_icon_pixel(ring, texel, ink, [1.0, 0.75, 0.45]),
            exa_ui::gui_draw::item_icon_fragment(ring, texel, ink, [1.0, 0.75, 0.45])
        );
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
}
