//! The skill tree screen's draw list.
//!
//! The screen's draw body is one long run of immediate-mode vertices: the
//! page-2 background, the dimming quad of the ring page, a soft plate under
//! every ring node, the five background spokes of each node, the node's own
//! group icon, the small level icons around it, the orbit bars, and last the
//! ring tooltip, the learning row or the forget dialog. This module builds
//! those vertices in the original's order, with its exact single-precision
//! sums, texture coordinates, shader colours (the ring shader's `Colour`)
//! and texture selection.
//!
//! A [`SkillDraw`] is a list of batches. Each batch is one program run with
//! the texture it samples and the shader colour it sets. Geometry is in
//! window pixels. The ring and spoke passes use the two shaders
//! [`crate::skill_gui::skill_programs`] builds; the icons sample the two
//! skill atlases; the tooltips are glow boxes ([`SkillTip`]) and text
//! anchors ([`SkillLabel`]) for the GUI font, whose measurement the caller
//! supplies through [`SkillDrawInput::text_width`].
//!
//! The original's reveal value turns and spreads the ring (the node plates,
//! spokes, icons and orbit bars multiply their offsets by it) and the
//! opening scale scales the shader alphas, the dim and the tooltips; the
//! hide clock scales the opening scale. [`crate::skill::skill_screen_fades`]
//! computes both.
//!
//! Evidence: the skill screen's draw body, the group-icon and orbit-bar
//! draws, the three tooltip bodies, and the two shader sources in the
//! constants table. Guesses are labelled.

use crate::skill::{
    NODES_PER_GROUP, PAGE_GROUPS, REQUIREMENT_MET, RING_GROUPS, STATUS_CHOSEN, STATUS_REQUIREMENTS,
    STATUS_UNAVAILABLE, SkillLearningRow, SkillRing, SkillScreen, SkillTipBox, forget_dialog_place,
    learning_row, pentagon_offset, ring_group_tooltip, ring_node_title, ring_node_tooltip,
    selection_tip_slide, skill_group_icons, skill_icon_quad, skill_orbit_bars, skill_popup_fade,
    skill_ring_position, skill_screen_fades,
};
use crate::skill_gui::{ICON_UNITS, SINGLE_ICON_UNITS, SkillIconHandles};
use exa_formats::characters::{SKILLS, Skill};

/// The program a batch draws with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillProgram {
    /// The skin's flat solid program (the dim quad).
    Solid,
    /// The screen's rounded ring program, source [`crate::skill_gui::RING_SHADER`].
    Rounded,
    /// The bar-stroke program, source [`crate::skill_gui::BAR_SHADER`].
    Bar,
    /// The skin's textured program (icons).
    Textured,
}

/// The texture a batch samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillTexture {
    None,
    /// The `skillgui.rfi` background.
    Background,
    /// The small group-icon atlas (`sgsklicons.rfi`).
    GroupIcons,
    /// The six large single-skill icons (`skillicon.rfi`).
    SingleIcons,
}

/// One vertex: place, texture coordinate and ink.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkillVertex {
    pub x: f32,
    pub y: f32,
    pub u: f32,
    pub v: f32,
    pub ink: [f32; 4],
}

impl SkillVertex {
    fn new(x: f32, y: f32, u: f32, v: f32, ink: [f32; 4]) -> Self {
        Self { x, y, u, v, ink }
    }
}

/// One submitted program run.
#[derive(Debug, Clone, PartialEq)]
pub struct SkillBatch {
    pub program: SkillProgram,
    pub texture: SkillTexture,
    /// The shader's `Colour`, when the program takes one.
    pub colour: Option<[f32; 4]>,
    /// The bar program's `Prog`, when it takes one.
    pub prog: Option<f32>,
    pub vertices: Vec<SkillVertex>,
}

impl SkillBatch {
    fn new(program: SkillProgram, texture: SkillTexture, colour: Option<[f32; 4]>) -> Self {
        Self {
            program,
            texture,
            colour,
            prog: None,
            vertices: Vec::new(),
        }
    }

    /// Appends one quad in the original's vertex order: top right, top
    /// left, bottom left, bottom right.
    fn quad(&mut self, corners: [(f32, f32); 4], uvs: [(f32, f32); 4], ink: [f32; 4]) {
        for ((x, y), (u, v)) in corners.into_iter().zip(uvs) {
            self.vertices.push(SkillVertex::new(x, y, u, v, ink));
        }
    }

    /// Appends the same quad in white; the program's `Colour` supplies the
    /// shader's colour.
    fn flat_quad(&mut self, corners: [(f32, f32); 4], uvs: [(f32, f32); 4]) {
        self.quad(corners, uvs, [1.0; 4]);
    }
}

/// A shader glow box: its rectangle and alpha.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkillTip {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
    pub alpha: f32,
}

impl From<SkillTipBox> for SkillTip {
    fn from(tip: SkillTipBox) -> Self {
        Self {
            left: tip.left,
            top: tip.top,
            width: tip.width,
            height: tip.height,
            alpha: tip.alpha,
        }
    }
}

/// A text anchor for the GUI font.
#[derive(Debug, Clone, PartialEq)]
pub struct SkillLabel {
    pub x: f32,
    pub y: f32,
    pub ink: [u8; 4],
    pub text: String,
    /// The label is centred on `x` rather than left-aligned.
    pub centred: bool,
}

/// One frame's skill-screen geometry and labels.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SkillDraw {
    pub batches: Vec<SkillBatch>,
    pub tips: Vec<SkillTip>,
    pub labels: Vec<SkillLabel>,
    /// The opening scale is below 0.999, so the draw ends with the skin's
    /// full-screen fade call (both arguments 1).
    pub closing: bool,
}

impl SkillDraw {
    fn batch(&mut self, program: SkillProgram, texture: SkillTexture, colour: Option<[f32; 4]>) {
        self.batches.push(SkillBatch::new(program, texture, colour));
    }

    fn last(&mut self) -> Option<&mut SkillBatch> {
        self.batches.last_mut()
    }
}

/// The draw's inputs beyond the screen itself.
pub struct SkillDrawInput<'a> {
    pub screen: &'a SkillScreen,
    /// The character's skills, or `None` for the screen's preview record.
    pub skills: Option<&'a [Skill; SKILLS]>,
    pub icons: &'a SkillIconHandles,
    /// The two atlases' pixel sizes, `(small, single)`.
    pub group_atlas: (u16, u16),
    pub single_atlas: (u16, u16),
    pub window: i32,
    /// The screen's own origin and its parent's, for the tooltip clamp.
    pub offsets: [i32; 2],
    pub clock: i32,
    /// When the hovered code last changed.
    pub hover_since: i32,
    /// A level is ready to spend.
    pub ready: bool,
    /// The ink the page-2 background's textured quad takes (guess: the
    /// skin's own title ink, white in the shipped scheme).
    pub background_ink: [f32; 4],
    /// Measures a string in the GUI font.
    pub text_width: &'a dyn Fn(&str) -> i32,
}

/// `CVTSS2SI` into an `int`: half to even, NaN and overflow the indefinite
/// value. The caller keeps the low byte where the original does.
fn cvt(value: f32) -> i32 {
    let rounded = value.round_ties_even();
    if rounded.is_nan() || !(-2_147_483_648.0..2_147_483_648.0).contains(&rounded) {
        i32::MIN
    } else {
        rounded as i32
    }
}

/// The low byte of a `CVTSS2SI` result.
fn low_byte(value: f32) -> u8 {
    cvt(value) as u8
}

/// A byte colour over 255.
fn ink_of(bytes: [u8; 4]) -> [f32; 4] {
    bytes.map(|byte| f32::from(byte) / 255.0)
}

/// One ring node: its slot, group and position at the reveal value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkillNodePlace {
    pub slot: i32,
    /// The 0-based group the slot shows.
    pub group: usize,
    pub x: f32,
    pub y: f32,
}

/// The ring's node places at the reveal value `turn`, in slot order (the
/// draw walks slots 1 to the ring's count).
pub fn node_places(screen: &SkillScreen, turn: f32) -> Vec<SkillNodePlace> {
    let ring = screen.ring(turn);
    (1..=screen.count)
        .filter_map(|slot| {
            let group = screen.slot_group(slot)?;
            let (x, y) = skill_ring_position(slot, &ring);
            Some(SkillNodePlace { slot, group, x, y })
        })
        .collect()
}

/// Four screen corners.
type Corners = [(f32, f32); 4];
/// Four texture coordinates.
type Uvs = [(f32, f32); 4];

/// The corners of a node's soft plate: 30 units either side, spread by
/// `alpha`, texture coordinates the whole square.
fn plate_corners(x: f32, y: f32, half: f32) -> (Corners, Uvs) {
    (
        [
            (x + half, y - half),
            (x - half, y - half),
            (x - half, y + half),
            (x + half, y + half),
        ],
        [(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0)],
    )
}

/// The icon atlas quad: `icon` (1-based) from a `cell`-pixel grid in an
/// atlas `size` pixels across, with its top-left at `(x, y)`.
pub fn atlas_quad(
    icon: i32,
    cell: i32,
    size: (u16, u16),
    x: f32,
    y: f32,
) -> Option<(Corners, Uvs)> {
    let quad = skill_icon_quad(icon, cell, size.0, size.1, x, y)?;
    let side = quad.size;
    Some((
        [(x + side, y), (x, y), (x, y + side), (x + side, y + side)],
        [
            (quad.uv[2], quad.uv[1]),
            (quad.uv[0], quad.uv[1]),
            (quad.uv[0], quad.uv[3]),
            (quad.uv[2], quad.uv[3]),
        ],
    ))
}

/// Builds the skill screen's draw list. Returns an empty list when the
/// screen is hidden.
pub fn skill_draw(input: &SkillDrawInput<'_>) -> SkillDraw {
    let screen = input.screen;
    let fades = skill_screen_fades(
        screen.state,
        screen.reveal_deadline,
        screen.hide_deadline,
        input.clock,
        screen.page,
    );
    if fades.hidden {
        return SkillDraw::default();
    }
    let alpha = fades.alpha;
    let scale = fades.scale;
    // The compare is ordered: a NaN skips the closing call.
    let closing = scale < 0.999;
    let width = (screen.bounds.right - screen.bounds.left) as f32;
    let height = (screen.bounds.bottom - screen.bounds.top) as f32;
    let unit = screen.unit as f32;
    let mut draw = SkillDraw {
        closing,
        ..SkillDraw::default()
    };

    // The page-2 background: the texture's lower-left cell over a 12 × 8
    // unit rectangle at the screen's centre. The UV pair is the constant
    // the original's texture-coordinate helper loads.
    if screen.page == 2 {
        let cx = (screen.bounds.right - screen.bounds.left) / 2;
        let cy = (screen.bounds.bottom - screen.bounds.top) / 2 - 3 * screen.unit;
        let (x, y) = (cx as f32, cy as f32);
        draw.batch(
            SkillProgram::Textured,
            SkillTexture::Background,
            Some(input.background_ink),
        );
        if let Some(batch) = draw.last() {
            // The page background's four vertices and their texture
            // coordinates, in the original's order.
            batch.quad(
                [
                    (x - 6.0 * unit, y + 4.0 * unit),
                    (x - 6.0 * unit, y - 4.0 * unit),
                    (x + 6.0 * unit, y - 4.0 * unit),
                    (x + 6.0 * unit, y + 4.0 * unit),
                ],
                [
                    (0.785_714_3, 0.6),
                    (0.785_714_3, 1.0),
                    (1.0, 1.0),
                    (1.0, 0.6),
                ],
                [1.0; 4],
            );
        }
    }

    // The ring page dims the game's view: black at 77 × scale, rounded
    // to a byte.
    if screen.page == 0 {
        let ink = [0.0, 0.0, 0.0, f32::from(low_byte(scale * 77.0)) / 255.0];
        draw.batch(SkillProgram::Solid, SkillTexture::None, None);
        if let Some(batch) = draw.last() {
            batch.quad(
                [(width, 0.0), (0.0, 0.0), (0.0, height), (width, height)],
                [(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0)],
                ink,
            );
        }
    }

    let nodes = node_places(screen, alpha);

    // The soft plate under every node: 30 units of a black glow, its
    // shader alpha 0.6 × scale.
    draw.batch(
        SkillProgram::Rounded,
        SkillTexture::Background,
        Some([0.0, 0.0, 0.0, 0.6 * scale]),
    );
    let half = 30.0 * unit * alpha;
    for node in &nodes {
        let (corners, uvs) = plate_corners(node.x, node.y, half);
        if let Some(batch) = draw.last() {
            batch.flat_quad(corners, uvs);
        }
    }

    // The five spokes behind every node: 12.3 to 26.3 units out, 18 units
    // either side, the sine and cosine scaled by the reveal value.
    draw.batch(
        SkillProgram::Rounded,
        SkillTexture::Background,
        Some([1.0, 0.98, 0.9, 0.4 * scale]),
    );
    let spoke_half = 18.0 * unit;
    let spoke_inner = -(unit as f64 * 12.3) as f32;
    let spoke_outer = spoke_inner - 14.0 * unit;
    for node in &nodes {
        for slot in 0..5 {
            let angle = slot as f32 * std::f32::consts::TAU * 0.2;
            let sin = angle.sin() * alpha;
            let cos = angle.cos() * alpha;
            let corners = [
                (
                    (node.x - spoke_half * cos) - spoke_inner * sin,
                    spoke_inner * cos + (node.y - spoke_half * sin),
                ),
                (
                    (node.x - spoke_half * cos) - spoke_outer * sin,
                    spoke_outer * cos + (node.y - spoke_half * sin),
                ),
                (
                    (spoke_half * cos + node.x) - spoke_outer * sin,
                    spoke_outer * cos + spoke_half * sin + node.y,
                ),
                (
                    (spoke_half * cos + node.x) - spoke_inner * sin,
                    spoke_inner * cos + spoke_half * sin + node.y,
                ),
            ];
            if let Some(batch) = draw.last() {
                // The spoke's per-vertex texture coordinates.
                batch.flat_quad(
                    corners,
                    [
                        (-0.071_428_6, 0.0),
                        (-0.071_428_6, 0.7),
                        (0.571_428_6, 0.7),
                        (0.571_428_6, 0.0),
                    ],
                );
            }
        }
    }

    // The node's own group icon: 24 units a side, a unit above the node's
    // top half. Its ink is 255, 248, 240 at 120 × scale² × 0.75.
    let node_ink = ink_of([0xff, 0xf8, 0xf0, low_byte(120.0 * scale * scale * 0.75)]);
    draw.batch(SkillProgram::Textured, SkillTexture::SingleIcons, None);
    let single_cell = (SINGLE_ICON_UNITS as f32 * unit) as i32;
    for node in &nodes {
        let x = node.x - (single_cell >> 1) as f32;
        let y = node.y - (single_cell >> 1) as f32 - unit;
        if let Some((corners, uvs)) =
            atlas_quad(node.group as i32 + 1, single_cell, input.single_atlas, x, y)
            && let Some(batch) = draw.last()
        {
            batch.quad(corners, uvs, node_ink);
        }
    }

    // The small level icons around the node, eight units a side. A chosen
    // technique ahead of the points draws its last icon at half alpha; the
    // hovered one takes 255, 204, 153.
    draw.batch(SkillProgram::Textured, SkillTexture::GroupIcons, None);
    let group_cell = ICON_UNITS as f32 * unit;
    let hover_ink = ink_of([0xff, 0xcc, 0x99, low_byte(255.0 * scale)]);
    let plain_ink = ink_of([0xff, 0xf8, 0xf0, low_byte(120.0 * scale)]);
    for node in &nodes {
        let icons = skill_group_icons(
            input.skills,
            node.group,
            node.x,
            node.y,
            screen.unit,
            alpha,
            input.ready,
            screen.hover,
        );
        for icon in icons {
            let icon_ink = if icon.hovered {
                hover_ink
            } else if icon.half {
                let base = low_byte(120.0 * scale) >> 1;
                ink_of([0xff, 0xf8, 0xf0, base])
            } else {
                plain_ink
            };
            let index = if icon.technique == 0 {
                input.icons.first
            } else {
                input.icons.groups[node.group][usize::from(icon.technique) - 1]
            };
            if let Some((corners, uvs)) =
                atlas_quad(index, group_cell as i32, input.group_atlas, icon.x, icon.y)
                && let Some(batch) = draw.last()
            {
                batch.quad(corners, uvs, icon_ink);
            }
        }
    }

    // The orbit bars: 20.3 to 26.3 units out, 14 either side, the shader
    // colour 1, 0.79, 0.286 at 0.5 × scale.
    let bar_uvs = [(0.0, 0.7), (0.0, 1.0), (0.5, 1.0), (0.5, 0.7)];
    for node in &nodes {
        let bars = skill_orbit_bars(input.skills, node.group, node.x, node.y, screen.unit, alpha);
        if bars.is_empty() {
            continue;
        }
        let last = bars.len() - 1;
        let partial = bars[last].fill < 0.99;
        // The full bars share `Prog` 1; a partial last bar is its own run
        // with `Prog` set to its fill.
        if !partial || last > 0 {
            draw.batch(
                SkillProgram::Bar,
                SkillTexture::None,
                Some([1.0, 0.79, 0.286, 0.5 * scale]),
            );
            if let Some(batch) = draw.last() {
                batch.prog = Some(1.0);
                for bar in &bars[..if partial { last } else { bars.len() }] {
                    for (at, corner) in bar.corners.iter().enumerate() {
                        batch.vertices.push(SkillVertex::new(
                            corner[0],
                            corner[1],
                            bar_uvs[at].0,
                            bar_uvs[at].1,
                            [1.0; 4],
                        ));
                    }
                }
            }
        }
        if partial {
            draw.batch(
                SkillProgram::Bar,
                SkillTexture::None,
                Some([1.0, 0.79, 0.286, 0.5 * scale]),
            );
            if let Some(batch) = draw.last() {
                batch.prog = Some(bars[last].fill);
                for (at, corner) in bars[last].corners.iter().enumerate() {
                    batch.vertices.push(SkillVertex::new(
                        corner[0],
                        corner[1],
                        bar_uvs[at].0,
                        bar_uvs[at].1,
                        [1.0; 4],
                    ));
                }
            }
        }
    }

    // The hover tooltip, then the learning row or the forget dialog.
    if screen.hover > 0
        && let Some(opacity) = selection_tip_slide(screen.hover, input.clock - input.hover_since)
    {
        let ink = [0xff, 0xf8, 0xf0, low_byte(255.0 * scale * opacity)];
        let code = screen.hover;
        if code & 0xff == 0 {
            let group = ((code >> 8) & 0xff) as usize;
            let name = crate::skill::SKILL_GROUP_NAMES
                .get(group.wrapping_sub(1))
                .copied()
                .unwrap_or("");
            let text_width = (input.text_width)(name) as u32;
            let (tip, name_at) = ring_group_tooltip(
                node_x(&nodes, code),
                node_y(&nodes, code),
                text_width,
                screen.unit,
                opacity * scale,
            );
            draw.tips.push(tip.into());
            draw.labels.push(SkillLabel {
                x: name_at.0,
                y: name_at.1,
                ink,
                text: name.to_string(),
                centred: false,
            });
        } else {
            let tip = ring_node_tooltip(
                node_x(&nodes, code),
                node_y(&nodes, code),
                screen.unit,
                input.window,
                &input.offsets,
                opacity * scale,
            );
            draw.tips.push(tip.tip.into());
            if let Some(skills) = input.skills
                && let Some((title, chosen)) = ring_node_title(skills, code)
            {
                draw.labels.push(SkillLabel {
                    x: tip.x,
                    y: tip.title_y,
                    ink,
                    text: title.to_string(),
                    centred: true,
                });
                if !chosen {
                    let status = node_status(skills, code, screen.ready);
                    if !status.is_empty() {
                        draw.labels.push(SkillLabel {
                            x: tip.text.0,
                            y: tip.text.1,
                            ink,
                            text: status.to_string(),
                            centred: false,
                        });
                    }
                }
            }
        }
    }

    let popup = skill_popup_fade(screen.popup, screen.popup_deadline, input.clock);
    let popup_ink = [0xff, 0xf8, 0xf0, low_byte(255.0 * scale * popup.alpha)];
    if screen.learning != 0 {
        let place = learning_node(&nodes, screen.learning);
        if let Some((x, y)) = place {
            let row: SkillLearningRow = learning_row(
                x,
                y,
                screen.unit,
                popup.scale,
                input.window,
                &input.offsets,
                popup.alpha * scale * scale,
                &screen.requirements,
                screen.hover,
            );
            draw.tips.push(row.tip.into());
            let row_group = ((screen.learning >> 8) & 0xff) as usize;
            let handles = row_group
                .checked_sub(1)
                .and_then(|group| input.icons.groups.get(group))
                .copied()
                .unwrap_or([0; NODES_PER_GROUP]);
            for icon in row.icons {
                let handle = handles[usize::from(icon.slot as u8).saturating_sub(1)];
                if handle == 0 {
                    continue;
                }
                if let Some((corners, uvs)) =
                    atlas_quad(handle, group_cell as i32, input.group_atlas, icon.x, icon.y)
                {
                    draw.batch(SkillProgram::Textured, SkillTexture::GroupIcons, None);
                    let icon_ink = if icon.hovered {
                        ink_of([0xff, 0xcc, 0x99, low_byte(255.0 * scale * popup.alpha)])
                    } else {
                        ink_of([0xff, 0xf8, 0xf0, low_byte(120.0 * scale * popup.alpha)])
                    };
                    if let Some(batch) = draw.last() {
                        batch.quad(corners, uvs, icon_ink);
                    }
                }
            }
            if screen.hover > 0 && screen.hover < 9 {
                let title = crate::skill::technique_name_for(screen.learning, screen.hover as u8);
                draw.labels.push(SkillLabel {
                    x: row.x,
                    y: row.title_y,
                    ink: popup_ink,
                    text: title,
                    centred: true,
                });
                if let Some(status) = row.status {
                    draw.labels.push(SkillLabel {
                        x: row.text.0,
                        y: row.text.1,
                        ink: popup_ink,
                        text: status.to_string(),
                        centred: false,
                    });
                }
            }
        }
    } else if screen.forgetting != 0 {
        let place = learning_node(&nodes, screen.forgetting);
        if let Some((x, y)) = place {
            let dialog = forget_dialog_place(
                x,
                y,
                screen.unit,
                popup.scale,
                input.window,
                &input.offsets,
                popup.alpha * scale,
            );
            draw.tips.push(dialog.tip.into());
            draw.labels.push(SkillLabel {
                x: dialog.text_left,
                y: dialog.title_y,
                ink: popup_ink,
                text: crate::skill::FORGET_TITLE.to_string(),
                centred: false,
            });
            draw.labels.push(SkillLabel {
                x: dialog.text_left,
                y: dialog.question_y,
                ink: popup_ink,
                text: crate::skill::FORGET_QUESTION.to_string(),
                centred: false,
            });
            if dialog.choices {
                let choice_ink = [
                    0xff,
                    0xcc,
                    0x99,
                    low_byte(255.0 * scale * dialog.choice_alpha),
                ];
                draw.labels.push(SkillLabel {
                    x: dialog.yes_x,
                    y: dialog.choice_y,
                    ink: choice_ink,
                    text: "YES".to_string(),
                    centred: true,
                });
                draw.labels.push(SkillLabel {
                    x: dialog.no_x,
                    y: dialog.choice_y,
                    ink: choice_ink,
                    text: "NO".to_string(),
                    centred: true,
                });
            }
        }
    }

    draw
}

/// The x of the node a code names, clamped to the ring if it is not on it.
fn node_x(nodes: &[SkillNodePlace], code: i32) -> f32 {
    nodes
        .iter()
        .find(|node| node.group + 1 == ((code >> 8) & 0xff) as usize)
        .map(|node| node.x)
        .unwrap_or(0.0)
}

fn node_y(nodes: &[SkillNodePlace], code: i32) -> f32 {
    nodes
        .iter()
        .find(|node| node.group + 1 == ((code >> 8) & 0xff) as usize)
        .map(|node| node.y)
        .unwrap_or(0.0)
}

/// The node place a learning-row or forget-dialog code names.
fn learning_node(nodes: &[SkillNodePlace], code: i32) -> Option<(f32, f32)> {
    nodes
        .iter()
        .find(|node| node.group + 1 == ((code >> 8) & 0xff) as usize)
        .map(|node| (node.x, node.y))
}

/// The status line of a ring node the record does not hold, from the
/// learning row's requirement mask of its group.
fn node_status(skills: &[Skill; SKILLS], code: i32, ready: bool) -> &'static str {
    let group = ((code >> 8) & 0xff) as usize;
    let level = (code & 0xff) as usize;
    if group == 0 || level == 0 {
        return STATUS_UNAVAILABLE;
    }
    let requirements = crate::skill::skill_requirements(group - 1, skills);
    let _ = ready;
    match requirements.get(level - 1) {
        Some(&crate::skill::REQUIREMENT_CHOSEN) => STATUS_CHOSEN,
        Some(&crate::skill::REQUIREMENT_MISSING) => STATUS_REQUIREMENTS,
        Some(&crate::skill::REQUIREMENT_ABSENT) => STATUS_UNAVAILABLE,
        _ => "",
    }
}

/// The ring slot order a page uses, for the draw's callers.
pub fn page_groups(page: i32) -> &'static [usize] {
    if page > 0 { &PAGE_GROUPS } else { &RING_GROUPS }
}

/// The learning row's slot count and the empty-slot mark, re-exported so a
/// caller can index the mask without importing `skill`.
pub const ROW_SLOTS: usize = NODES_PER_GROUP;
/// The clear requirement mark.
pub const SLOT_FREE: u8 = REQUIREMENT_MET;

/// A node's ring layout from its bounds.
pub fn ring_of(screen: &SkillScreen, turn: f32) -> SkillRing {
    screen.ring(turn)
}

/// The corner of a pentagon point, exposed for tests of the orbit bars.
pub fn pentagon_point(slot: i32, unit: i32) -> (f32, f32) {
    pentagon_offset(slot, unit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::{SkillBounds, SkillScreen};

    fn bounds() -> SkillBounds {
        SkillBounds {
            left: 0,
            right: 200,
            top: 0,
            bottom: 100,
        }
    }

    fn screen() -> SkillScreen {
        SkillScreen::open(bounds(), 4, false, 0, 0)
    }

    fn icons() -> SkillIconHandles {
        let mut groups = [[0i32; 8]; 6];
        for (group, row) in groups.iter_mut().enumerate() {
            for (at, cell) in row.iter_mut().enumerate() {
                *cell = (group * 8 + at + 2) as i32;
            }
        }
        SkillIconHandles { first: 1, groups }
    }

    fn measure(text: &str) -> i32 {
        text.len() as i32 * 8
    }

    #[test]
    fn a_four_state_draws_only_degenerate_geometry() {
        // The original keeps drawing with both fades at zero; the geometry
        // collapses to zero-sized quads and zero alphas.
        let mut screen = screen();
        screen.state = 4;
        let input = SkillDrawInput {
            screen: &screen,
            skills: None,
            icons: &icons(),
            group_atlas: (0x300, 0x2a0),
            single_atlas: (0x240, 0x360),
            window: 1200,
            offsets: [0, 0],
            clock: 1000,
            hover_since: 0,
            ready: true,
            background_ink: [1.0; 4],
            text_width: &measure,
        };
        let draw = skill_draw(&input);
        let plates = draw
            .batches
            .iter()
            .find(|batch| batch.program == SkillProgram::Rounded)
            .unwrap();
        assert!(
            plates
                .vertices
                .iter()
                .all(|vertex| vertex.x == plates.vertices[0].x)
        );
        assert_eq!(plates.colour.unwrap()[3], 0.0);
    }

    #[test]
    fn a_finished_hide_on_the_ring_closes_the_draw() {
        let mut screen = screen();
        screen.state = 2;
        screen.hide_deadline = 100;
        let input = SkillDrawInput {
            screen: &screen,
            skills: None,
            icons: &icons(),
            group_atlas: (0x300, 0x2a0),
            single_atlas: (0x240, 0x360),
            window: 1200,
            offsets: [0, 0],
            clock: 200,
            hover_since: 0,
            ready: true,
            background_ink: [1.0; 4],
            text_width: &measure,
        };
        let draw = skill_draw(&input);
        assert!(draw.batches.is_empty());
    }

    #[test]
    fn the_ring_page_dims_and_places_six_nodes() {
        let mut screen = screen();
        screen.reveal_deadline = 0;
        let input = SkillDrawInput {
            screen: &screen,
            skills: None,
            icons: &icons(),
            group_atlas: (0x300, 0x2a0),
            single_atlas: (0x240, 0x360),
            window: 1200,
            offsets: [0, 0],
            clock: 100,
            hover_since: 0,
            ready: true,
            background_ink: [1.0; 4],
            text_width: &measure,
        };
        let draw = skill_draw(&input);
        // Solid dim, rounded plates, rounded spokes, textured node icons,
        // textured group icons and the orbit bars.
        assert_eq!(draw.batches[0].program, SkillProgram::Solid);
        assert_eq!(draw.batches[0].vertices.len(), 4);
        assert_eq!(draw.batches[0].vertices[0].ink[3], 77.0 / 255.0);
        let nodes = node_places(&screen, 1.0);
        assert_eq!(nodes.len(), 6);
        assert_eq!(nodes[0].group, 0);
        let plates = draw
            .batches
            .iter()
            .find(|batch| {
                batch.program == SkillProgram::Rounded && batch.colour == Some([0.0, 0.0, 0.0, 0.6])
            })
            .unwrap();
        assert_eq!(plates.vertices.len(), 24);
        // The first node is at the ring's first slot.
        assert_eq!(plates.vertices[0].x, nodes[0].x + 120.0);
    }

    #[test]
    fn a_page_two_draws_its_background_and_plate_quads() {
        let mut screen = screen();
        screen.page = 2;
        screen.count = 4;
        screen.reveal_deadline = 0;
        let input = SkillDrawInput {
            screen: &screen,
            skills: None,
            icons: &icons(),
            group_atlas: (0x300, 0x2a0),
            single_atlas: (0x240, 0x360),
            window: 1200,
            offsets: [0, 0],
            clock: 100,
            hover_since: 0,
            ready: true,
            background_ink: [1.0; 4],
            text_width: &measure,
        };
        let draw = skill_draw(&input);
        assert_eq!(draw.batches[0].texture, SkillTexture::Background);
        // No dim on a page.
        assert!(
            !draw
                .batches
                .iter()
                .any(|batch| batch.program == SkillProgram::Solid)
        );
    }

    #[test]
    fn the_icon_atlas_quad_uses_the_cell_row() {
        let (corners, uvs) = atlas_quad(6, 32, (0x300, 0x2a0), 10.0, 20.0).unwrap();
        assert_eq!(
            corners,
            [(42.0, 20.0), (10.0, 20.0), (10.0, 52.0), (42.0, 52.0)]
        );
        // Icon 6: a 24-cell row (width 0x300 / 32 = 24), row 0.
        assert!((uvs[1].0 - 5.0 * (32.0 / 768.0)).abs() < 1e-6);
        assert_eq!(atlas_quad(0, 32, (0x300, 0x2a0), 0.0, 0.0), None);
    }

    #[test]
    fn a_hovered_node_draws_its_tip_and_title() {
        let mut screen = screen();
        screen.reveal_deadline = 0;
        screen.hover = 0x0101;
        let mut skills = [Skill::default(); SKILLS];
        skills[0] = Skill {
            points: 5_000,
            techniques: [1, 0, 0, 0, 0, 0],
        };
        let input = SkillDrawInput {
            screen: &screen,
            skills: Some(&skills),
            icons: &icons(),
            group_atlas: (0x300, 0x2a0),
            single_atlas: (0x240, 0x360),
            window: 1200,
            offsets: [0, 0],
            clock: 1000,
            hover_since: 0,
            ready: true,
            background_ink: [1.0; 4],
            text_width: &measure,
        };
        let draw = skill_draw(&input);
        assert_eq!(draw.tips.len(), 1);
        assert_eq!(draw.labels.len(), 1);
        assert!(!draw.labels[0].text.is_empty());
        assert!(draw.labels[0].centred);
    }

    #[test]
    fn the_node_window_is_clamped_and_the_slot_order_official() {
        assert_eq!(page_groups(0), &RING_GROUPS);
        assert_eq!(page_groups(2), &PAGE_GROUPS);
        assert_eq!(row_slots(), 8);
    }

    fn row_slots() -> usize {
        ROW_SLOTS
    }
}
