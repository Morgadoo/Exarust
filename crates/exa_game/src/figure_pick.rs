//! The inventory figure's pointer actions: the pick of the item under the
//! pointer, the drag that takes an item off the figure into a pack, and the
//! item card a right press opens.
//!
//! **The pick** ([`exa_ui::character_view::hover_pick`]) casts the screen
//! point's ray from the view's camera (its unit direction, so distances are
//! in metres). It first meets the merged body mesh: every face of the
//! figure's merged runs, skinned to the current pose on the CPU as the
//! original skins the merged mesh before the test, in the meshes' one-sided
//! face test ([`exa_ui::character_view::ray_face`]), the nearest face
//! answering the worn item its garment is (the original's per-face item
//! table; Exarust's faces carry their garment, [`FaceGarments`], whose model
//! names the worn entry). Then every other drawn mesh of the figure is a
//! scene node met only when nearer: a held item's meshes answer the hand's
//! item and its slot, anything else (the hair) answers no item. The hand's
//! record entry gives a held item's flags, the worn entry a garment's.
//!
//! **The drag** starts on a left press over the view through
//! [`drag_start_plan`]: the item's icon and flags, half the icon size as the
//! grab. The player's inventory view is not frozen, locked or charging mind
//! costs. The drag is a [`PanelDrag`] from [`Panel::Figure`], ended by the
//! panel input's release ([`drop_from_figure`]): a release on a pack panel
//! runs the panel's accept test and drop (the player's pack is linked to
//! the view, so a held weapon's own pack entry loses its held mark and moves
//! under the pointer, and a garment is added without its bulk), then the
//! view's release ([`release_on_figure`]): a hand's item is taken out by the
//! scheduled take-out (the off hand's weapon or light moving up into the
//! main hand), the player's pack dropping its held entry when the item went
//! elsewhere; a garment comes off the worn list (the underwear rule putting
//! the braies back on), the player's pack is charged its bulk and the
//! outfit is rebuilt. A release anywhere else (outside every panel, on the
//! view itself, on a body's equipment panel) cancels: nothing moves.
//!
//! **The card**: a right press over the view runs the pick and opens the
//! item's card at once ([`crate::item_card::FigureCard`]).
//!
//! Debug tools: `--figure-drag FROM>TO@FRAME` presses on the figure at
//! FROM at that frame and releases at TO two frames later;
//! `--figure-card AT@FRAME` runs the card action at AT. A point is `vX,Y`
//! (fractions of the view), `pX,Y` (fractions of the player's pack panel)
//! or `X,Y` (window pixels).
//!
//! Evidence: the character view's hover pick, drag-over, card, release and
//! the meshes' ray callback; the skin appends' per-face table
//! (`docs/UI.md`, Character view).

use bevy::ecs::system::SystemParam;
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_character::FaceGarments;
use exa_character::record::CharacterData;
use exa_combat::attack::Slot;
use exa_combat::block::ShieldHold;
use exa_formats::characters::ItemRef;
use exa_items::catalogue::ItemCatalogue;
use exa_items::doll::{HeldPlace, TakeOut, take_out};
use exa_items::possession::{self, HELD, Possessions};
use exa_ui::character_view::{
    BodyHit, FIELD_OF_VIEW, Hover, PICK_REQUIRED, PickNode, ReleaseFacts, drag_start_plan,
    hover_pick, nearest_face, release_on_figure,
};
use exa_ui::gui_core::GuiRect;

use crate::body_inventory;
use crate::doll_view::{DOLL_PANEL, DollItems, InventoryDoll, camera_transform, open_view};
use crate::gui_screens::GameScreens;
use crate::item_container::{Cell, Drag, DropDrag, to_list_pos};
use crate::item_panel_draw::PanelIcons;
use crate::panel_live::{self, Panel, PanelDrag, container, container_mut, open_panels, panel_at};

/// The view camera's near and far planes, metres ([`crate::doll_view`]'s
/// camera): the ray's reach when the body is missed is its near-to-far
/// length, as the original bounds the node tests by its ray's length.
const NEAR: f32 = 0.1;
const FAR: f32 = 50.0;

/// The search distance the body mesh's test starts from: the original's
/// largest-float stand-in, 3.4e38.
pub const BODY_FAR: f32 = f32::from_bits(0x7f7f_c99e);

/// Marks the root of a held item drawn on the figure with its 1-based item
/// slot (1 the main hand, 2 the off hand), for a model the figure carries
/// that is neither a striker nor a shield (a torch). The figure's weapons
/// are found by their [`exa_combat::Blade`] slot and its shield by the
/// figure's [`ShieldHold`].
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FigureHeld(pub usize);

/// The drag off the figure in flight: the slot its pick found (the view's
/// stored slot, which the release reads) and the payload's icon.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FigureDrag {
    pub slot: Option<usize>,
    pub icon: usize,
}

/// A drag off the figure ended: where it landed (the panel, the dropped
/// entry's cell and the panel's unit and vertical scale) or `None` for a
/// cancelled drag, with the drag itself.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct FigureRelease {
    pub drag: PanelDrag,
    pub landed: Option<(Panel, Cell, i32, f32)>,
    /// The panel's drop appended a new entry, which keeps the payload's
    /// icon reference.
    pub added: bool,
}

/// The ray of a point of the view (physical pixels from the view's
/// top-left): the camera's position and the unit direction through the
/// point, at the view's vertical field of view.
pub fn view_ray(camera: &Transform, at: Vec2, width: f32, height: f32) -> (Vec3, Vec3) {
    let half = (FIELD_OF_VIEW.to_radians() * 0.5).tan();
    let aspect = width / height.max(1.0);
    let x = (at.x / width.max(1.0)) * 2.0 - 1.0;
    let y = 1.0 - (at.y / height.max(1.0)) * 2.0;
    let aim = camera.forward().as_vec3()
        + camera.right().as_vec3() * (x * aspect * half)
        + camera.up().as_vec3() * (y * half);
    (camera.translation, aim.normalize_or_zero())
}

/// The 1-based item slot of a held item's striker slot.
fn blade_slot(slot: Slot) -> usize {
    match slot {
        Slot::First => 1,
        Slot::Second => 2,
    }
}

/// A drawn mesh of the figure: its mesh, whether it shows, where it is,
/// its skin, and its faces' garments when it is a merged run.
type FigurePart = (
    &'static Mesh3d,
    &'static InheritedVisibility,
    &'static GlobalTransform,
    Option<&'static SkinnedMesh>,
    Option<&'static FaceGarments>,
);

/// The figure's parts the pick walks.
#[derive(SystemParam)]
pub struct FigurePicker<'w, 's> {
    children: Query<'w, 's, &'static Children>,
    parents: Query<'w, 's, &'static ChildOf>,
    parts: Query<'w, 's, FigurePart>,
    transforms: Query<'w, 's, &'static GlobalTransform>,
    meshes: Res<'w, Assets<Mesh>>,
    bindposes: Res<'w, Assets<SkinnedMeshInverseBindposes>>,
    blades: Query<'w, 's, &'static exa_combat::Blade>,
    shields: Query<'w, 's, &'static ShieldHold>,
    held: Query<'w, 's, &'static FigureHeld>,
}

/// The pick's result and what it walked, for the log.
#[derive(Debug, Clone, PartialEq)]
pub struct Picked {
    pub hover: Hover,
    pub body: Option<BodyHit>,
    /// The other meshes as nodes: item, slot and distance.
    pub nodes: Vec<(u32, Option<usize>, Option<f32>)>,
    /// The figure's meshes: drawn ones, and the merged runs among them.
    pub drawn: usize,
    pub runs: usize,
    /// Meshes whose triangles could not be read (no asset, no skin yet).
    pub unread: usize,
}

impl FigurePicker<'_, '_> {
    /// A mesh's triangles where it is drawn now: skinned through its joints,
    /// or placed by its transform.
    fn faces(
        &self,
        mesh: &Mesh,
        skin: Option<&SkinnedMesh>,
        at: &GlobalTransform,
    ) -> Option<Vec<[Vec3; 3]>> {
        let positions: Vec<Vec3> = match skin {
            Some(skin) => exa_character::blood::skinned_positions(
                mesh,
                skin,
                &self.bindposes,
                &self.transforms,
            )?,
            None => mesh
                .attribute(Mesh::ATTRIBUTE_POSITION)?
                .as_float3()?
                .iter()
                .map(|p| at.transform_point(Vec3::from(*p)))
                .collect(),
        };
        let indices: Vec<usize> = mesh.indices()?.iter().collect();
        Some(
            indices
                .as_chunks::<3>()
                .0
                .iter()
                .filter_map(|tri| {
                    Some([
                        *positions.get(tri[0])?,
                        *positions.get(tri[1])?,
                        *positions.get(tri[2])?,
                    ])
                })
                .collect(),
        )
    }

    /// The 1-based item slot a figure mesh's held item fills: the first
    /// ancestor below the figure that is a striker (by its slot), the
    /// figure's shield or a [`FigureHeld`] root.
    fn slot_of(&self, figure: Entity, mesh: Entity) -> Option<usize> {
        let shield = self.shields.get(figure).ok().map(|hold| hold.item);
        let mut at = mesh;
        while at != figure {
            if let Ok(held) = self.held.get(at) {
                return Some(held.0);
            }
            if let Ok(blade) = self.blades.get(at) {
                return Some(blade_slot(blade.slot));
            }
            if Some(at) == shield {
                return Some(2);
            }
            at = self.parents.get(at).ok()?.parent();
        }
        None
    }

    /// The pick at a ray: the merged body's nearest face, then every other
    /// drawn mesh of the figure as a node ([`hover_pick`]).
    pub fn pick(
        &self,
        figure: Entity,
        (origin, dir): (Vec3, Vec3),
        p: &Possessions,
        items: &DollItems,
    ) -> Picked {
        let mut body: Option<(f32, &FaceGarments, usize)> = None;
        let mut nodes = Vec::new();
        let (mut drawn, mut runs, mut unread) = (0, 0, 0);
        for entity in self.children.iter_descendants(figure) {
            let Ok((mesh3d, visible, at, skin, garments)) = self.parts.get(entity) else {
                continue;
            };
            if !visible.get() {
                continue;
            }
            drawn += 1;
            runs += usize::from(garments.is_some());
            let Some(faces) = self
                .meshes
                .get(&mesh3d.0)
                .and_then(|mesh| self.faces(mesh, skin, at))
            else {
                unread += 1;
                continue;
            };
            match garments {
                Some(garments) => {
                    let within = body.map_or(BODY_FAR, |(t, _, _)| t);
                    if let Some((face, t)) = nearest_face(origin, dir, faces, within) {
                        body = Some((t, garments, face));
                    }
                }
                None => {
                    let slot = self.slot_of(figure, entity);
                    let item = slot
                        .and_then(|slot| p.out.get(slot - 1))
                        .map_or(0, |hand| hand.id);
                    nodes.push(PickNode {
                        node: entity.index().index(),
                        flags: PICK_REQUIRED,
                        item,
                        item_flags: 0,
                        item_settle: -1,
                        distance: nearest_face(origin, dir, faces, BODY_FAR).map(|(_, t)| t),
                        slot,
                    });
                }
            }
        }
        let body = body.map(|(distance, garments, face)| BodyHit {
            item: garments
                .model(face)
                .and_then(|model| worn_item(p, items, model))
                .unwrap_or(0),
            distance,
        });
        let worn: Vec<(u32, u32)> = p.worn.iter().map(|w| (w.id, w.flags)).collect();
        let hand_flags: Vec<u32> = p.out.iter().map(|hand| hand.flags).collect();
        let hover = hover_pick(None, body, FAR - NEAR, &nodes, &hand_flags, &worn);
        Picked {
            hover,
            body,
            nodes: nodes
                .iter()
                .map(|node| (node.item, node.slot, node.distance))
                .collect(),
            drawn,
            runs,
            unread,
        }
    }
}

/// The worn item a garment model is: the first worn entry whose record is
/// worn as that model.
fn worn_item(p: &Possessions, items: &DollItems, model: &str) -> Option<u32> {
    p.worn
        .iter()
        .find(|worn| {
            items
                .get(worn.id)
                .is_some_and(|item| item.worn_model.eq_ignore_ascii_case(model))
        })
        .map(|worn| worn.id)
}

/// A point of a debug script.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScriptPoint {
    /// Fractions of the view.
    View(f32, f32),
    /// Fractions of the player's pack panel.
    Pack(f32, f32),
    /// Window pixels (physical).
    Window(i32, i32),
}

impl ScriptPoint {
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        let pair = |t: &str| -> Option<(f32, f32)> {
            let (x, y) = t.split_once(',')?;
            Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
        };
        if let Some(rest) = text.strip_prefix('v') {
            let (x, y) = pair(rest)?;
            return Some(Self::View(x, y));
        }
        if let Some(rest) = text.strip_prefix('p') {
            let (x, y) = pair(rest)?;
            return Some(Self::Pack(x, y));
        }
        let (x, y) = pair(text)?;
        Some(Self::Window(x as i32, y as i32))
    }

    /// The window point (physical pixels), when its panel is open.
    pub fn resolve(self, screens: &GameScreens) -> Option<IVec2> {
        let within = |rect: GuiRect, x: f32, y: f32| {
            IVec2::new(
                rect.left + (rect.width() as f32 * x) as i32,
                rect.top + (rect.height() as f32 * y) as i32,
            )
        };
        match self {
            Self::View(x, y) => Some(within(open_view(screens)?, x, y)),
            Self::Pack(x, y) => {
                let panels = open_panels(screens);
                let (_, rect) = panels.iter().find(|(p, _)| *p == Panel::PlayerPack)?;
                Some(within(*rect, x, y))
            }
            Self::Window(x, y) => Some(IVec2::new(x, y)),
        }
    }
}

/// The figure's debug actions (`--figure-drag`, `--figure-card`).
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct FigureScript {
    /// A drag: from, to, and the frame of the press.
    pub drag: Option<(ScriptPoint, ScriptPoint, u32)>,
    /// A card action: where, and the frame.
    pub card: Option<(ScriptPoint, u32)>,
}

impl FigureScript {
    /// Reads `FROM>TO@FRAME`.
    pub fn parse_drag(text: &str) -> Option<(ScriptPoint, ScriptPoint, u32)> {
        let (points, frame) = text.rsplit_once('@')?;
        let (from, to) = points.split_once('>')?;
        Some((
            ScriptPoint::parse(from)?,
            ScriptPoint::parse(to)?,
            frame.trim().parse().ok()?,
        ))
    }

    /// Reads `AT@FRAME`.
    pub fn parse_card(text: &str) -> Option<(ScriptPoint, u32)> {
        let (at, frame) = text.rsplit_once('@')?;
        Some((ScriptPoint::parse(at)?, frame.trim().parse().ok()?))
    }
}

/// What a press over the figure asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FigureAction {
    Drag,
    Card,
}

/// The shared state of a press on the figure.
#[derive(SystemParam)]
struct FigurePress<'w, 's> {
    picker: FigurePicker<'w, 's>,
    doll: Res<'w, InventoryDoll>,
    screens: Res<'w, GameScreens>,
    drag: ResMut<'w, PanelDrag>,
    figure_drag: ResMut<'w, FigureDrag>,
    icons: ResMut<'w, PanelIcons>,
    catalogue: Option<Res<'w, ItemCatalogue>>,
    items: ResMut<'w, DollItems>,
    game: Option<Res<'w, exa_assets::Game>>,
    players: Query<'w, 's, &'static Possessions, With<crate::player::PlayerBody>>,
    cards: MessageWriter<'w, crate::item_card::FigureCard>,
}

impl FigurePress<'_, '_> {
    /// Runs a press over the figure at a window point: the pick, then the
    /// drag start or the card.
    fn run(&mut self, action: FigureAction, at: IVec2) -> bool {
        let Some(view) = open_view(&self.screens) else {
            return false;
        };
        if !view.contains(at.x, at.y) {
            return false;
        }
        let Some(figure) = self.doll.scene.as_ref().and_then(|scene| scene.figure) else {
            return false;
        };
        let Ok(p) = self.players.single() else {
            return false;
        };
        self.items.ensure(self.game.as_deref());
        let local = Vec2::new((at.x - view.left) as f32, (at.y - view.top) as f32);
        let ray = view_ray(
            &camera_transform(),
            local,
            view.width() as f32,
            view.height() as f32,
        );
        let picked = self.picker.pick(figure, ray, p, &self.items);
        let hover = picked.hover;
        info!(
            "figure pick at ({}, {}) in the view: item {:#x} flags {:#x} slot {:?} \
             (body face {:?}; {} meshes drawn, {} merged runs, {} unread; other meshes \
             {:?}; worn {:?}, hands {:#x}/{:#x})",
            local.x,
            local.y,
            hover.item,
            hover.flags,
            hover.slot,
            picked.body.map(|hit| (hit.item, hit.distance)),
            picked.drawn,
            picked.runs,
            picked.unread,
            picked.nodes,
            p.worn.iter().map(|w| w.id).collect::<Vec<_>>(),
            p.out[0].id,
            p.out[1].id
        );
        match action {
            FigureAction::Card => {
                // The card action opens the item's card when the pick
                // found one.
                if let Some((item, flags)) =
                    exa_ui::character_view::current_card(hover.item, hover.flags)
                {
                    self.cards.write(crate::item_card::FigureCard {
                        item,
                        flags,
                        pointer: (at.x, at.y),
                    });
                }
                true
            }
            FigureAction::Drag => {
                if self.drag.source.is_some() {
                    return false;
                }
                let catalogue = self.catalogue.as_deref();
                let icon = if hover.item == 0 {
                    0
                } else {
                    self.icons.reserve(hover.item, hover.flags, catalogue).0
                };
                // The player's own view: not frozen, not locked, no mind
                // cost.
                let plan = drag_start_plan(
                    false,
                    hover.slot.unwrap_or(0),
                    icon,
                    false,
                    hover.flags,
                    None,
                    false,
                    self.screens.pack.icon_size.max(0) as u32,
                );
                let Some(payload) = plan.payload else {
                    if icon != 0 {
                        self.icons.release(icon);
                    }
                    return true;
                };
                let half = payload.half as i32;
                *self.drag = PanelDrag {
                    source: Some(Panel::Figure),
                    icon,
                    grab: Cell::new(half, half),
                    id: hover.item,
                    flags: payload.flags,
                    bulk: catalogue
                        .and_then(|c| c.bulk(hover.item))
                        .map_or(0, i32::from),
                };
                *self.figure_drag = FigureDrag {
                    slot: hover.slot,
                    icon,
                };
                info!(
                    "figure drag: {:#x} (flags {:#x}, slot {:?}) off the figure, payload {:#x}",
                    hover.item, payload.flags, hover.slot, payload.kind
                );
                true
            }
        }
    }
}

/// The live presses over the figure: a left press starts a drag off it, a
/// right press opens the card.
fn figure_press(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    use_cursor: Option<Res<crate::player_controls::UseCursor>>,
    mut press: FigurePress,
) {
    let action = if buttons.just_pressed(MouseButton::Left) {
        FigureAction::Drag
    } else if buttons.just_pressed(MouseButton::Right) {
        FigureAction::Card
    } else {
        return;
    };
    if use_cursor.is_some_and(|c| c.took_press) {
        return;
    }
    let Some(at) = windows.single().ok().and_then(|window| {
        window
            .cursor_position()
            .map(|p| (p * window.scale_factor()).as_ivec2())
    }) else {
        return;
    };
    press.run(action, at);
}

/// The figure's debug actions at their frames after the load.
fn scripted_figure(
    script: Res<FigureScript>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    mut frames: Local<u32>,
    mut press: FigurePress,
    mut release: ResMut<panel_live::ScriptedRelease>,
) {
    if script.drag.is_none() && script.card.is_none() {
        return;
    }
    if loading.is_some() {
        return;
    }
    *frames += 1;
    let frame = *frames;
    if let Some((from, to, at)) = script.drag {
        if frame == at {
            match from.resolve(&press.screens) {
                Some(point) => {
                    info!("figure drag script: press at {point} (frame {frame})");
                    if !press.run(FigureAction::Drag, point) {
                        info!("figure drag script: the press missed the open figure");
                    }
                }
                None => info!("figure drag script: the view is not open at frame {frame}"),
            }
        }
        if frame == at + 2 {
            match to.resolve(&press.screens) {
                Some(point) => {
                    info!("figure drag script: release at {point} (frame {frame})");
                    release.0 = Some(point);
                }
                None => info!("figure drag script: no release point at frame {frame}"),
            }
        }
    }
    if let Some((point, at)) = script.card
        && frame == at
    {
        match point.resolve(&press.screens) {
            Some(point) => {
                info!("figure card script: press at {point} (frame {frame})");
                press.run(FigureAction::Card, point);
            }
            None => info!("figure card script: the view is not open at frame {frame}"),
        }
    }
}

/// Ends a drag off the figure at a window point: a release on a pack panel
/// (the player's, a body's or a container's) runs the panel's accept test
/// and drop; anywhere else (outside the panels, the view itself, a body's
/// equipment panel) cancels. The drop of a payload from the panel's linked
/// view ([`DOLL_PANEL`]) shows its held entry of the item again, or adds the
/// item without its bulk.
pub fn drop_from_figure(
    screens: &mut GameScreens,
    icons: &mut PanelIcons,
    catalogue: Option<&ItemCatalogue>,
    drag: PanelDrag,
    at: Option<IVec2>,
) -> FigureRelease {
    let cancel = FigureRelease {
        drag,
        landed: None,
        added: false,
    };
    let Some(at) = at else {
        return cancel;
    };
    if open_view(screens).is_some_and(|view| view.contains(at.x, at.y)) {
        return cancel;
    }
    let panels = open_panels(screens);
    let Some((destination, local)) = panel_at(&panels, at) else {
        return cancel;
    };
    if matches!(destination, Panel::BodyEquipment(_) | Panel::Figure) {
        return cancel;
    }
    let model_found = drag.id & 0xC000_0000 != 0
        || catalogue.is_none_or(|c| c.kind(drag.id).is_some() || c.bulk(drag.id).is_some());
    let accepted = container(screens, destination).is_some_and(|panel| {
        panel.accept(
            &Drag {
                src: Some(DOLL_PANEL),
                dst: Some(destination.number()),
                objid: drag.id,
                objflags: drag.flags,
                icon: drag.icon as u32,
                bulk: drag.bulk.clamp(0, 255) as u8,
                model_found,
            },
            destination.number(),
        )
    });
    if !accepted {
        info!(
            "figure drop: {:?} refuses {:#x}; the drag is cancelled",
            destination, drag.id
        );
        return cancel;
    }
    let half = icons.half(drag.icon).unwrap_or((0, 0));
    let Some(panel) = container_mut(screens, destination) else {
        return cancel;
    };
    let linked = panel.linked == Some(DOLL_PANEL);
    let Some(dropped) = panel.drop_entry(
        DropDrag {
            same_panel: false,
            linked,
            id: drag.id,
            icon: drag.icon,
            flags: drag.flags,
            bulk: drag.bulk,
        },
        drag.grab,
        local,
        half,
    ) else {
        return cancel;
    };
    let cell = panel.entries[dropped.index].cell;
    FigureRelease {
        drag,
        landed: Some((destination, cell, panel.unit, panel.vscale)),
        added: dropped.added.is_some(),
    }
}

/// The off hand's item once the main hand's has left, as the promotion reads
/// it.
fn off_item(
    p: &Possessions,
    catalogue: Option<&ItemCatalogue>,
) -> Option<(u32, possession::ItemKind)> {
    let off = p.out[1].id;
    (off != 0).then(|| (off, catalogue.and_then(|c| c.kind(off)).unwrap_or_default()))
}

/// Where a hand's item sits for the take-out: the off hand's light or
/// shield, else one of the attached weapons (the first when in the main
/// hand).
fn held_place(hand: usize, kind: possession::ItemKind) -> HeldPlace {
    if hand == 1 && kind.shield {
        HeldPlace::Shield
    } else if hand == 1 && kind.light && kind.weapon.is_none() {
        HeldPlace::Light
    } else {
        HeldPlace::Weapon { first: hand == 0 }
    }
}

/// The view's release for a drag off the figure, and the record side: the
/// [`release_on_figure`] plan run on the player's possessions, outfit and
/// hands, and the item given to the panel it landed on.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn release_figure(
    mut commands: Commands,
    mut releases: MessageReader<FigureRelease>,
    mut screens: ResMut<GameScreens>,
    mut icons: ResMut<PanelIcons>,
    mut figure_drag: ResMut<FigureDrag>,
    catalogue: Option<Res<ItemCatalogue>>,
    mut items: ResMut<DollItems>,
    game: Option<Res<exa_assets::Game>>,
    mut players: Query<
        (
            Entity,
            &mut Possessions,
            &mut exa_character::Outfit,
            &mut exa_character::held::Hands,
            Has<crate::torch::CarriesTorch>,
            Option<&CharacterData>,
        ),
        With<crate::player::PlayerBody>,
    >,
    mut bodies: Query<(Entity, &mut Possessions), Without<crate::player::PlayerBody>>,
) {
    for release in releases.read().copied() {
        let drag = release.drag;
        let slot = std::mem::take(&mut *figure_drag).slot;
        let plan = release_on_figure(ReleaseFacts {
            slot: if release.landed.is_some() {
                slot.map_or(0, |s| s as i32)
            } else {
                i32::MIN
            },
            locked: false,
            payload_flags: drag.flags,
            mind_cost: false,
            hand: slot.map(|s| s - 1),
            shown: true,
            shown_is_dragged: false,
            other_panel: true,
            // The payload's source is the view itself, never the pack.
            from_panel: false,
            player_shown: true,
            keep_doll: false,
        });
        // A drop that showed the pack's own entry again (or no drop at all)
        // leaves the payload's icon reference unused.
        if !release.added && drag.icon != 0 {
            icons.release(drag.icon);
        }
        let Some((destination, cell, unit, vscale)) = release.landed else {
            info!("figure release: {:#x} cancelled, nothing moves", drag.id);
            continue;
        };
        items.ensure(game.as_deref());
        let catalogue = catalogue.as_deref();
        let Ok((player, mut p, mut outfit, mut hands, torch, data)) = players.single_mut() else {
            continue;
        };
        if let Some(hand) = plan.hand {
            // The hand's item comes out by the scheduled take-out.
            let kind = catalogue.and_then(|c| c.kind(drag.id)).unwrap_or_default();
            let mut after = p.clone();
            after.out[hand] = possession::Hand::default();
            let off = if hand == 0 {
                off_item(&after, catalogue)
            } else {
                None
            };
            let released = take_out(
                &mut p,
                &TakeOut::scheduled(hand),
                Some((drag.id, held_place(hand, kind))),
                off,
            );
            // The record side of the pack: the held copy of the item.
            let at = p
                .carried
                .iter()
                .position(|c| c.id == drag.id && c.flags & HELD != 0);
            if destination == Panel::PlayerPack {
                // The linked pack showed its held entry again under the
                // pointer: the listed copy loses its mark and moves there.
                let (x, y) = to_list_pos(cell, unit, vscale);
                match at {
                    Some(at) => {
                        let carried = &mut p.carried[at];
                        carried.flags &= !HELD;
                        carried.x = x;
                        carried.y = y;
                    }
                    None => body_inventory::give(&mut p, drag.id, drag.flags, cell, unit, vscale),
                }
            } else {
                // Elsewhere: the player's pack drops its held entry (its
                // panel and its listed copy) and the destination takes it.
                if plan.remove_held_icon {
                    let bulk = catalogue.and_then(|c| c.bulk(drag.id)).map_or(0, i32::from);
                    if let Some(entry) = screens
                        .pack
                        .entries
                        .iter()
                        .position(|e| e.id == drag.id && e.flags & HELD != 0)
                    {
                        screens.pack.remove_at(entry, bulk);
                    }
                }
                if let Some(at) = at {
                    p.carried.remove(at);
                }
                if let Panel::BodyPack(body) = destination
                    && let Some((_, mut b)) = bodies
                        .iter_mut()
                        .find(|(entity, _)| entity.index().index() as i32 == body)
                {
                    body_inventory::give(&mut b, drag.id, drag.flags & !HELD, cell, unit, vscale);
                }
            }
            info!(
                "figure release: {:#x} taken out of hand {} into {:?} (promoted {:?}), \
                 hands now {:#x}/{:#x}",
                drag.id, hand, destination, released.promoted, p.out[0].id, p.out[1].id
            );
            let mut next = crate::equip::hands_of(&p, |id| catalogue.and_then(|c| c.kind(id)));
            if torch {
                next = crate::equip::keep_torch(next, &hands);
            }
            hands.set_if_neq(next);
            let main = p.out[0].id;
            match catalogue.and_then(|c| c.model(main)).filter(|_| main != 0) {
                Some(model) => {
                    commands.entity(player).insert(exa_character::Wielded {
                        model: model.model.clone(),
                        weapon: model.weapon,
                    });
                }
                None => {
                    commands.entity(player).remove::<exa_character::Wielded>();
                }
            }
            continue;
        }
        if !plan.take_off {
            continue;
        }
        // A garment: off the worn list, the underwear rule after it, the
        // outfit rebuilt, the player's pack charged its bulk.
        let Some(at) = p.worn.iter().position(|w| w.id == drag.id) else {
            continue;
        };
        let worn = p.worn.remove(at);
        let mut record = CharacterData::of_record(exa_formats::characters::CharacterRecord {
            worn: p.worn.clone(),
            race: data.map_or(0, |d| d.record.race),
            ..Default::default()
        });
        let underwear = record.ensure_underwear(|id| items.get(id));
        if underwear && let Some(braies) = record.record.worn.last().copied() {
            p.worn.push(ItemRef::new(braies.id, braies.flags));
        }
        if plan.charge_panel {
            screens.pack.used += drag.bulk;
        }
        if let Some(item) = items.get(drag.id) {
            let model = item.worn_model.clone();
            if let Some(at) = outfit
                .0
                .iter()
                .position(|garment| garment.model.eq_ignore_ascii_case(&model))
            {
                outfit.0.remove(at);
            }
        }
        match destination {
            Panel::PlayerPack => {
                body_inventory::give(&mut p, drag.id, worn.flags, cell, unit, vscale);
            }
            Panel::BodyPack(body) => {
                if let Some((_, mut b)) = bodies
                    .iter_mut()
                    .find(|(entity, _)| entity.index().index() as i32 == body)
                {
                    body_inventory::give(&mut b, drag.id, worn.flags, cell, unit, vscale);
                }
            }
            _ => {}
        }
        info!(
            "figure release: {:#x} taken off into {:?} at {:?}{}; worn now {:?}",
            drag.id,
            destination,
            (cell.x, cell.y),
            if underwear {
                ", the braies back on"
            } else {
                ""
            },
            p.worn.iter().map(|w| w.id).collect::<Vec<_>>()
        );
    }
}

/// The inventory figure's drag, pick and card.
pub struct FigurePickPlugin;

impl Plugin for FigurePickPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FigureDrag>()
            .init_resource::<FigureScript>()
            .add_message::<FigureRelease>()
            .add_message::<crate::item_card::FigureCard>()
            .add_systems(
                Update,
                (figure_press, scripted_figure)
                    .chain()
                    .in_set(panel_live::PanelInput)
                    .after(panel_live::press)
                    .before(panel_live::release),
            )
            .add_systems(Update, release_figure.after(panel_live::release));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_view_ray_runs_through_the_figure_point_at_the_centre() {
        let camera = camera_transform();
        let (origin, dir) = view_ray(&camera, Vec2::new(112.0, 190.0), 224.0, 380.0);
        assert_eq!(origin, camera.translation);
        assert!((dir.length() - 1.0).abs() < 1e-5);
        assert!(dir.dot(camera.forward().as_vec3()) > 0.9999);
        // A point higher in the view aims higher.
        let (_, up) = view_ray(&camera, Vec2::new(112.0, 20.0), 224.0, 380.0);
        assert!(up.y > dir.y);
    }

    #[test]
    fn script_points_read_views_packs_and_pixels() {
        assert_eq!(
            ScriptPoint::parse("v0.5,0.25"),
            Some(ScriptPoint::View(0.5, 0.25))
        );
        assert_eq!(
            ScriptPoint::parse("p0.4, 0.6"),
            Some(ScriptPoint::Pack(0.4, 0.6))
        );
        assert_eq!(
            ScriptPoint::parse("300,200"),
            Some(ScriptPoint::Window(300, 200))
        );
        assert_eq!(ScriptPoint::parse("q1,2"), None);
        assert_eq!(
            FigureScript::parse_drag("v0.5,0.1>p0.5,0.5@120"),
            Some((
                ScriptPoint::View(0.5, 0.1),
                ScriptPoint::Pack(0.5, 0.5),
                120
            ))
        );
        assert_eq!(
            FigureScript::parse_card("v0.5,0.6@90"),
            Some((ScriptPoint::View(0.5, 0.6), 90))
        );
        assert_eq!(FigureScript::parse_drag("v0.5,0.1@120"), None);
    }

    #[test]
    fn a_drop_on_the_linked_pack_shows_the_held_entry_or_adds_without_bulk() {
        use crate::item_container::Entry;
        let mut screens = GameScreens::default();
        screens.task.screen = (1600, 900);
        screens.open_inventory(true, true, 4);
        assert_eq!(screens.pack.linked, Some(DOLL_PANEL));
        // The held weapon's own entry, marked held.
        screens.pack.entries = vec![Entry {
            cell: Cell::new(0, 0),
            id: 12,
            icon: 1,
            flags: HELD,
            drag: 0,
        }];
        screens.pack.used = 3;
        let pack = open_panels(&screens)
            .into_iter()
            .find(|(panel, _)| *panel == Panel::PlayerPack)
            .unwrap()
            .1;
        let at = IVec2::new(pack.left + 60, pack.top + 50);
        let mut icons = PanelIcons::default();
        let drag = PanelDrag {
            source: Some(Panel::Figure),
            icon: 2,
            grab: Cell::new(20, 20),
            id: 12,
            flags: 0,
            bulk: 2,
        };
        let release = drop_from_figure(&mut screens, &mut icons, None, drag, Some(at));
        let (panel, cell, _, _) = release.landed.expect("the pack takes it");
        assert_eq!(panel, Panel::PlayerPack);
        assert!(!release.added);
        assert_eq!(screens.pack.entries.len(), 1);
        assert_eq!(screens.pack.entries[0].flags & HELD, 0);
        assert_eq!(screens.pack.entries[0].cell, cell);
        assert_eq!(screens.pack.used, 3);
        // A garment has no held entry: it is added, without its bulk.
        let helm = PanelDrag { id: 409, ..drag };
        let release = drop_from_figure(&mut screens, &mut icons, None, helm, Some(at));
        assert!(release.added);
        assert_eq!(screens.pack.entries.len(), 2);
        assert_eq!(screens.pack.used, 3);
        // Outside every panel, and on the view itself, the drag is cancelled.
        let away = drop_from_figure(
            &mut screens,
            &mut icons,
            None,
            drag,
            Some(IVec2::new(1500, 800)),
        );
        assert_eq!(away.landed, None);
        let view = open_view(&screens).unwrap();
        let back = drop_from_figure(
            &mut screens,
            &mut icons,
            None,
            drag,
            Some(IVec2::new(view.left + 5, view.top + 5)),
        );
        assert_eq!(back.landed, None);
        assert_eq!(
            drop_from_figure(&mut screens, &mut icons, None, drag, None).landed,
            None
        );
    }

    #[test]
    fn a_held_item_is_placed_by_its_hand_and_kind() {
        let shield = possession::ItemKind {
            shield: true,
            ..Default::default()
        };
        let torch = possession::ItemKind {
            light: true,
            ..Default::default()
        };
        let sword = possession::ItemKind {
            weapon: Some(0x11),
            ..Default::default()
        };
        assert_eq!(held_place(1, shield), HeldPlace::Shield);
        assert_eq!(held_place(1, torch), HeldPlace::Light);
        assert_eq!(held_place(0, torch), HeldPlace::Weapon { first: true });
        assert_eq!(held_place(1, sword), HeldPlace::Weapon { first: false });
        assert_eq!(blade_slot(Slot::First), 1);
        assert_eq!(blade_slot(Slot::Second), 2);
    }
}
