//! The running game's skill screen: the original's `K` key opens the skill
//! tree over the play view, the pointer hovers its nodes and satellites, a
//! click opens a level's learning row or forget dialog and answers it, and
//! the right button closes it. The rules are [`exa_ui::skill`]'s
//! [`SkillScreen`] and its draw list [`exa_ui::skill_draw`].
//!
//! Opening is the skill unit's open routine: the unit's atlases are cut
//! once (and again when the interface unit changes), the reveal sound
//! `SkillShow_01` plays, a ring screen is built over the window and the
//! player's record is attached, which refreshes its ready flag. The
//! player's controls hear nothing while it is open (they are off), so the
//! skills key does not toggle it shut; the right
//! button closes it at once, as the screen's pointer method calls the open
//! routine's toggle. Closing refreshes the body's technique word from the
//! record (the close routine's technique refresh and character refit).
//! The open routine turns the player's controls off before it builds the
//! screen and the close routine turns them back on
//! ([`crate::controls_live`]).
//!
//! Every frame the draw's fades advance (the reveal clears its bit; a
//! finished hide closes the screen) and the popup's fade runs (a finished
//! close clears the pending choice). The pointer's code is the screen's
//! point hit test (the forget dialog's Yes and No, the learning row's
//! slots, else the ring's node and satellite test) and goes through the
//! hover filter, which stamps the tooltip's slide.
//!
//! The draw list is rasterised here into one window-sized image, the
//! batches in order with the screen's four programs (the skin's solid and
//! textured ones, the rounded ring program and the orbit bar program,
//! evaluated per pixel from their sources), the glow boxes as the skin's
//! rounded box (black, its scale cut from the box grown by eight units),
//! and the labels in the interface level's main font, each box and label
//! at its place among the batches. The canvas blends with the original's
//! 0x203 word and is shown with straight alpha over the HUD.
//!
//! Evidence: the skill unit's open and close routines, the screen's
//! pointer, hit, hover and satellite methods, its draw body and tooltips,
//! and the skin's rounded box, read in the instructions.

use bevy::asset::RenderAssetUsages;
use bevy::audio::PlaybackMode;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;
use exa_assets::Game;
use exa_character::record::CharacterData;
use exa_formats::characters::{SKILLS, Skill};
use exa_ui::gui_draw::{GuiTexture, GuiTextures};
use exa_ui::gui_font::{FONTBASE_24, GuiFont, font_with_fallback, gui_level, level_for_unit};
use exa_ui::icon_atlas::{IconAtlas, IconImage};
use exa_ui::skill::{
    ExperienceRates, SkillBounds, SkillHit, SkillHoverInput, SkillPointInput, SkillPointer,
    SkillScreen, clamp_tooltip, hit_skill_point, skill_hover, skill_node_hit, skill_popup_fade,
    skill_ring_position, skill_satellite,
};
use exa_ui::skill_draw::{
    SkillDraw, SkillDrawInput, SkillLabel, SkillProgram, SkillTexture, SkillTip, SkillVertex,
    skill_draw,
};
use exa_ui::skill_gui::{
    ICON_FORMAT, ICON_UNITS, SINGLE_FORMAT, SINGLE_ICON_ATLAS, SINGLE_ICON_UNITS, SKILL_GUI,
    SKILL_ICON_ATLAS, SKILL_SHOW_SOUND, SkillIconHandles, build_group_icons, build_single_icons,
    icon_sharpened,
};

use crate::experience::{global_xp_for_worth, global_xp_worth, technique_bits};
use crate::experience_live::{StoryExperience, TechniqueWord};
use crate::player::PlayerBody;
use crate::player_controls::PlayerControl;

/// The canvas's layer: a GUI form over the HUD, as the power tree's.
pub const SKILL_Z: i32 = crate::power_screen::TREE_Z;
/// The time between redraws while the screen animates.
const REDRAW_MS: i32 = 33;
/// The tooltip clamp's half width in units.
const TIP_HALF_UNITS: i32 = 41;

/// Asks for the skill screen (the skills key, through the hotkeys).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenSkills;

/// The experience system's conversions the screen spends through.
pub const RATES: ExperienceRates = ExperienceRates {
    worth: global_xp_worth,
    experience: global_xp_for_worth,
};

/// The skill unit's atlases at one interface unit.
pub struct SkillAtlases {
    pub unit: i32,
    pub icons: SkillIconHandles,
    pub group: GuiTexture,
    pub group_size: (u16, u16),
    pub single: GuiTexture,
    pub single_size: (u16, u16),
    pub background: Option<GuiTexture>,
}

impl SkillAtlases {
    /// Cuts the two atlases as the unit's builder does: the small icons
    /// 8 units a side (sharpened when that shrinks them), the six large
    /// icons 24 units a side (always sharpened).
    pub fn build(game: &Game, unit: i32) -> Option<Self> {
        let image = |name: &str| -> Option<IconImage> {
            let rgba = game
                .load_rgba(name)
                .map_err(|error| warn!("skills: {name}: {error}"))
                .ok()?;
            Some(IconImage {
                width: u16::try_from(rgba.width).ok()?,
                height: u16::try_from(rgba.height).ok()?,
                channels: 4,
                pixels: rgba.pixels,
            })
        };
        let mut group = IconAtlas::new(ICON_UNITS * unit, 4);
        group.sharpen = icon_sharpened(unit);
        let icons = build_group_icons(&image(SKILL_ICON_ATLAS)?, |cell| {
            group.add_resized(cell, ICON_FORMAT, 0, 0)
        });
        let mut single = IconAtlas::new(SINGLE_ICON_UNITS * unit, 4);
        single.sharpen = true;
        build_single_icons(&image(SINGLE_ICON_ATLAS)?, |cell| {
            single.add_resized(cell, SINGLE_FORMAT, 0, 0)
        });
        let texture = |atlas: &IconAtlas| {
            GuiTexture::from_bottom_up(
                u32::from(atlas.image.width),
                u32::from(atlas.image.height),
                &atlas.image.pixels,
            )
        };
        let background = game
            .load_rgba(SKILL_GUI)
            .ok()
            .and_then(|rgba| GuiTexture::from_bottom_up(rgba.width, rgba.height, &rgba.pixels));
        Some(Self {
            unit,
            icons,
            group: texture(&group)?,
            group_size: (group.image.width, group.image.height),
            single: texture(&single)?,
            single_size: (single.image.width, single.image.height),
            background,
        })
    }
}

/// The open skill screen and its hover state.
#[derive(Resource, Default)]
pub struct SkillUi {
    pub screen: Option<SkillScreen>,
    /// The hover filter's last code (0 for none): the tooltip slides in
    /// while it is set.
    pub hovered: i32,
    /// The last code the filter passed, which the tooltip shows.
    pub tip_code: i32,
    /// When the hovered code last changed.
    pub hover_since: i32,
    /// The reveal value the last draw stored (it turns the ring).
    pub turn: f32,
    pub atlases: Option<SkillAtlases>,
    /// The last rasterised draw, its size and when.
    last: Option<(SkillDraw, (u32, u32))>,
    last_draw: i32,
}

pub struct SkillScreenPlugin;

impl Plugin for SkillScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SkillUi>()
            .add_message::<OpenSkills>()
            .add_systems(
                Update,
                (open_skills, pointer, draw)
                    .chain()
                    .after(crate::hotkeys::HotkeySet),
            );
    }
}

/// The GUI clock in ms.
fn clock(time: &Time) -> i32 {
    time.elapsed().as_millis() as i32
}

/// The interface unit.
fn unit_of(gui: Option<&exa_ui::GuiScale>) -> i32 {
    gui.map_or(4, |gui| gui.scale.max(1) as i32)
}

#[allow(clippy::too_many_arguments)]
fn open_skills(
    mut requests: MessageReader<OpenSkills>,
    windows: Query<&Window, With<PrimaryWindow>>,
    time: Res<Time>,
    gui: Option<Res<exa_ui::GuiScale>>,
    game: Option<Res<Game>>,
    mut players: Query<&mut CharacterData, With<PlayerBody>>,
    mut ui: ResMut<SkillUi>,
    bank: Option<ResMut<exa_audio::SoundBank>>,
    mut sources: Option<ResMut<Assets<AudioSource>>>,
    (mut commands, control): (Commands, Option<ResMut<PlayerControl>>),
) {
    if requests.read().count() == 0 {
        return;
    }
    // The player's controls are off while the screen is up, so the key does
    // not reach the open routine again; the routine's own toggle closes an
    // open screen.
    if ui.screen.is_some() {
        return;
    }
    let (Ok(window), Ok(mut data)) = (windows.single(), players.single_mut()) else {
        return;
    };
    // The open routine turns the controls off before it builds the screen.
    if let Some(mut control) = control {
        crate::controls_live::switch_controls(&mut control, true, "skill screen opens");
    }
    let unit = unit_of(gui.as_deref());
    if let Some(game) = game.as_deref()
        && ui
            .atlases
            .as_ref()
            .is_none_or(|atlases| atlases.unit != unit)
    {
        ui.atlases = SkillAtlases::build(game, unit);
    }
    let now = clock(&time);
    let bounds = SkillBounds {
        left: 0,
        right: window.physical_width() as i32,
        top: 0,
        bottom: window.physical_height() as i32,
    };
    let mut screen = SkillScreen::open(bounds, unit, false, 0, now);
    screen.attach(&mut data, false, RATES);
    info!(
        "skills: the skill screen opens (ready {}, skills {:?})",
        screen.ready,
        data.record.skills.map(|skill| skill.points)
    );
    ui.screen = Some(screen);
    ui.hovered = 0;
    ui.tip_code = 0;
    ui.hover_since = now;
    ui.last = None;
    if let (Some(game), Some(mut bank), Some(sources)) = (game, bank, sources.as_mut())
        && let Some(clip) = bank.get(&game, sources, SKILL_SHOW_SOUND)
    {
        commands.spawn((
            AudioPlayer(clip),
            PlaybackSettings {
                mode: PlaybackMode::Despawn,
                ..default()
            },
        ));
    }
}

/// The skill block as the screen's hover filter reads it: one empty
/// eight-byte entry, then each skill's points and technique bytes.
fn hover_grid(skills: &[Skill; SKILLS]) -> [u8; 8 + SKILLS * 8] {
    let mut grid = [0u8; 8 + SKILLS * 8];
    for (at, skill) in skills.iter().enumerate() {
        let entry = &mut grid[8 + at * 8..16 + at * 8];
        entry[..2].copy_from_slice(&skill.points.to_le_bytes());
        entry[2..].copy_from_slice(&skill.techniques);
    }
    grid
}

/// A level code's satellite, as the screen's satellite method places it.
fn satellite(screen: &SkillScreen, code: i32, turn: f32) -> Option<(f32, f32)> {
    let group = usize::try_from((code >> 8) & 0xff).ok()?.checked_sub(1)?;
    let slot = screen.group_slot(group)?;
    Some(skill_satellite(slot - 1, code & 0xff, &screen.ring(turn)))
}

/// The screen's point hit test at `(x, y)`.
pub fn hit_code(screen: &SkillScreen, turn: f32, x: i32, y: i32) -> i32 {
    let ring = screen.ring(turn);
    let nodes: Vec<(usize, (f32, f32))> = (1..=screen.count)
        .filter_map(|slot| Some((screen.slot_group(slot)?, skill_ring_position(slot, &ring))))
        .collect();
    let fallback = skill_node_hit(
        SkillHit {
            mouse: (x, y),
            unit: screen.unit,
            turn,
            page: screen.page,
        },
        &nodes,
    );
    let width = screen.bounds.right - screen.bounds.left;
    let height = screen.bounds.bottom - screen.bounds.top;
    let half = (TIP_HALF_UNITS * screen.unit) as f32;
    let clamped = |code: i32| {
        satellite(screen, code, turn).map_or((0.0, 0.0), |(sx, sy)| {
            (clamp_tooltip(sx, &[0, 0], half, width), sy)
        })
    };
    hit_skill_point(SkillPointInput {
        state: u16::from(screen.state),
        page: screen.page,
        base: -1,
        owns: true,
        allowed: true,
        point: (x, y),
        width,
        height,
        unit: screen.unit,
        high: screen.forgetting,
        low: screen.learning,
        high_point: clamped(screen.forgetting),
        low_point: clamped(screen.learning),
        fallback,
    })
}

/// Closes the screen as the close routine does: the screen goes, the
/// player's controls come back on and the body's technique word is
/// refreshed from the record.
fn close(
    ui: &mut SkillUi,
    player: Option<(Entity, &CharacterData)>,
    story: Option<&mut StoryExperience>,
    commands: &mut Commands,
    control: Option<&mut PlayerControl>,
) {
    ui.screen = None;
    ui.last = None;
    if let Some(control) = control {
        crate::controls_live::switch_controls(control, false, "skill screen closes");
    }
    if let Some((entity, data)) = player {
        let word = technique_bits(&data.record.skills);
        if let Some(story) = story {
            story.technique_word = word;
        }
        commands.entity(entity).insert(TechniqueWord(word));
        info!("skills: the skill screen closes (technique word {word:#010x})");
    }
}

#[allow(clippy::too_many_arguments)]
fn pointer(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    time: Res<Time>,
    scripted: Option<Res<exa_ui::ScriptedPointer>>,
    mut players: Query<(Entity, &mut CharacterData), With<PlayerBody>>,
    mut story: Option<ResMut<StoryExperience>>,
    mut ui: ResMut<SkillUi>,
    mut commands: Commands,
    mut control: Option<ResMut<PlayerControl>>,
) {
    let now = clock(&time);
    let mut mask = 0u16;
    if buttons.just_released(MouseButton::Left) {
        mask |= 0x2000;
    }
    if buttons.just_pressed(MouseButton::Right) {
        mask |= 2;
    }
    let cursor = windows.single().ok().and_then(|window| {
        // A scripted pointer stands in for the mouse in unattended runs.
        let at = match scripted.as_deref() {
            Some(scripted) => scripted.position(window)?,
            None => window.cursor_position()?,
        };
        let scale = window.scale_factor();
        Some(((at.x * scale) as i32, (at.y * scale) as i32))
    });
    let player = players.single_mut().ok();
    let outcome = {
        let SkillUi {
            screen,
            hovered,
            tip_code,
            hover_since,
            turn,
            ..
        } = &mut *ui;
        let Some(screen) = screen.as_mut() else {
            return;
        };
        step(
            screen,
            StepInput { now, mask, cursor },
            HoverState {
                hovered,
                tip_code,
                hover_since,
                turn,
            },
            player.map(|(_, data)| data.into_inner()),
        )
    };
    match outcome {
        Some(SkillPointer::Close) => {
            let player = players.single().ok();
            close(
                &mut ui,
                player,
                story.as_deref_mut(),
                &mut commands,
                control.as_deref_mut(),
            );
        }
        Some(SkillPointer::Changed) => {
            // One record in the original: the experience system's copy
            // follows the screen's change.
            if let (Some(story), Ok((_, data))) = (story.as_deref_mut(), players.single()) {
                story.player.skills = data.record.skills;
                info!(
                    "skills: the record changes: {:?}",
                    data.record
                        .skills
                        .map(|skill| (skill.points, skill.techniques))
                );
            }
        }
        Some(SkillPointer::Pressed) => {
            if let Some(screen) = ui.screen.as_ref() {
                info!(
                    "skills: node {:#x} pressed (learning {:#x}, forgetting {:#x})",
                    screen.hover, screen.learning, screen.forgetting
                );
            }
        }
        _ => {}
    }
}

/// One frame's pointer input.
#[derive(Debug, Clone, Copy)]
pub struct StepInput {
    pub now: i32,
    /// The pointer's buttons: 0x2000 a left click, 2 the right button.
    pub mask: u16,
    /// The pointer in window pixels.
    pub cursor: Option<(i32, i32)>,
}

/// The screen's hover state the step updates.
pub struct HoverState<'a> {
    pub hovered: &'a mut i32,
    pub tip_code: &'a mut i32,
    pub hover_since: &'a mut i32,
    pub turn: &'a mut f32,
}

/// One frame of the screen: the draw's fades (a finished hide asks to
/// close), the popup's fade (a finished close clears the pending choice),
/// the pointer's code through the hover filter, then the buttons.
pub fn step(
    screen: &mut SkillScreen,
    input: StepInput,
    hover: HoverState<'_>,
    data: Option<&mut CharacterData>,
) -> Option<SkillPointer> {
    let now = input.now;
    let (alpha, _) = screen.fade(now);
    *hover.turn = alpha;
    if screen.state == 4 && screen.page != 2 {
        return Some(SkillPointer::Close);
    }
    let popup = skill_popup_fade(screen.popup, screen.popup_deadline, now);
    screen.popup = popup.flags;
    if popup.closed {
        screen.learning = 0;
        screen.forgetting = 0;
    }
    let data = data?;
    if let Some((x, y)) = input.cursor {
        let code = hit_code(screen, alpha, x, y);
        let grid = hover_grid(&data.record.skills);
        let out = skill_hover(SkillHoverInput {
            index: code,
            current: screen.hover,
            grid: Some(&grid),
            previous: *hover.hovered,
            last_positive: *hover.tip_code,
            stamp: *hover.hover_since,
            clock: now,
            base: 0,
        });
        if !out.called_base {
            screen.hover = out.current;
            *hover.hovered = out.previous;
            *hover.tip_code = out.last_positive;
            *hover.hover_since = out.stamp;
        }
    }
    if input.mask == 0 {
        return None;
    }
    Some(screen.pointer(data, input.mask, now, RATES))
}

/// Marks the screen's canvas node.
#[derive(Component)]
pub struct SkillCanvas;

/// The level's main font and the atlas its glyphs sample.
fn screen_font(unit: i32, textures: &GuiTextures) -> Option<(GuiFont, &GuiTexture, (u16, u16))> {
    let level = gui_level(level_for_unit(unit)).main;
    let size = |texture: &GuiTexture| {
        Some((
            u16::try_from(texture.width).ok()?,
            u16::try_from(texture.height).ok()?,
        ))
    };
    if let Some(texture) = textures.get(level.texture) {
        return Some((*level, texture, size(texture)?));
    }
    let base = textures.get(FONTBASE_24.texture)?;
    let (font, atlas) = font_with_fallback(level, None, size(base));
    Some((font, base, atlas?))
}

#[allow(clippy::too_many_arguments)]
fn draw(
    windows: Query<&Window, With<PrimaryWindow>>,
    time: Res<Time>,
    players: Query<&CharacterData, With<PlayerBody>>,
    textures: Option<Res<GuiTextures>>,
    mut ui: ResMut<SkillUi>,
    mut images: ResMut<Assets<Image>>,
    mut canvas: Query<(&mut ImageNode, &mut Node, &mut Visibility), With<SkillCanvas>>,
    mut commands: Commands,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(screen) = ui.screen.as_ref() else {
        for (_, _, mut visibility) in &mut canvas {
            *visibility = Visibility::Hidden;
        }
        return;
    };
    let now = clock(&time);
    if ui.last.is_some() && now.wrapping_sub(ui.last_draw) < REDRAW_MS {
        return;
    }
    let width = window.physical_width();
    let height = window.physical_height();
    let Some(atlases) = ui.atlases.as_ref() else {
        return;
    };
    let empty = GuiTextures::default();
    let textures = textures.as_deref().unwrap_or(&empty);
    let font = screen_font(screen.unit, textures);
    let measure = |text: &str| {
        font.as_ref()
            .map_or(0, |(font, _, _)| font.text_width(text.as_bytes()))
    };
    let skills = players.single().ok().map(|data| data.record.skills);
    let list = skill_draw(&SkillDrawInput {
        screen,
        skills: skills.as_ref(),
        icons: &atlases.icons,
        group_atlas: atlases.group_size,
        single_atlas: atlases.single_size,
        window: width as i32,
        offsets: [0, 0],
        clock: now,
        hover_since: ui.hover_since,
        hovered_node: ui.hovered,
        tip_code: ui.tip_code,
        ready: screen.ready,
        skin_grey: SKIN_GREY,
        spoke_ink: SPOKE_INK,
        text_width: &measure,
    });
    let size = (width, height);
    if ui
        .last
        .as_ref()
        .is_some_and(|(last, at)| *last == list && *at == size)
    {
        ui.last_draw = now;
        return;
    }
    let pixels = rasterise(
        &list,
        atlases,
        font.as_ref()
            .map(|(font, texture, atlas)| (font, *texture, *atlas)),
        screen.unit,
        width,
        height,
    );
    let image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    let ui_scale = window.scale_factor();
    let handle = images.add(image);
    let mut spawned = false;
    for (mut node_image, mut node, mut visibility) in &mut canvas {
        if node_image.image != handle {
            images.remove(&node_image.image);
        }
        node_image.image = handle.clone();
        node.width = Val::Px(width as f32 / ui_scale);
        node.height = Val::Px(height as f32 / ui_scale);
        *visibility = Visibility::Inherited;
        spawned = true;
    }
    if !spawned {
        commands.spawn((
            SkillCanvas,
            ImageNode::new(handle),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(width as f32 / ui_scale),
                height: Val::Px(height as f32 / ui_scale),
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                ..Node::default()
            },
            GlobalZIndex(SKILL_Z),
        ));
    }
    ui.last = Some((list, size));
    ui.last_draw = now;
}

/// The skin's grey the page-2 background takes (a skin byte outside the
/// skill unit; the ring page does not draw it) [guess].
const SKIN_GREY: u8 = 0xc0;
/// The skin's ink the background spokes take (a skin field outside the
/// skill unit) [guess: the line skin's light label ink].
const SPOKE_INK: [u8; 4] = [0xff, 0xf8, 0xf0, 0x40];

/// The canvas: colour premultiplied by coverage while drawing, then
/// straightened.
struct Canvas {
    width: u32,
    height: u32,
    pixels: Vec<[f32; 4]>,
}

impl Canvas {
    /// Blends a fragment with the 0x203 word (source alpha over one minus
    /// source alpha): the canvas keeps the colour premultiplied so the
    /// straight picture over the view equals blending onto it.
    fn blend(&mut self, x: i32, y: i32, colour: [f32; 4]) {
        let a = colour[3].clamp(0.0, 1.0);
        if a <= 0.0 || x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let at = y as usize * self.width as usize + x as usize;
        let dst = &mut self.pixels[at];
        for c in 0..3 {
            dst[c] = colour[c].clamp(0.0, 1.0) * a + dst[c] * (1.0 - a);
        }
        dst[3] = a + dst[3] * (1.0 - a);
    }

    fn straight(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.pixels.len() * 4);
        for p in &self.pixels {
            let a = p[3];
            if a <= 0.0 {
                out.extend_from_slice(&[0, 0, 0, 0]);
                continue;
            }
            for channel in &p[..3] {
                out.push(((channel / a).clamp(0.0, 1.0) * 255.0).round() as u8);
            }
            out.push((a.clamp(0.0, 1.0) * 255.0).round() as u8);
        }
        out
    }
}

/// A texel as floats.
fn texel(texture: Option<&GuiTexture>, u: f32, v: f32) -> [f32; 4] {
    texture.map_or([1.0; 4], |texture| {
        texture.sample(u, v).map(|byte| f32::from(byte) / 255.0)
    })
}

/// One fragment of a skill batch.
pub fn skill_fragment(
    program: SkillProgram,
    texture: Option<&GuiTexture>,
    colour: Option<[f32; 4]>,
    prog: Option<f32>,
    uv: [f32; 2],
    ink: [f32; 4],
) -> [f32; 4] {
    match program {
        SkillProgram::Solid => ink,
        SkillProgram::Textured => {
            let t = texel(texture, uv[0], uv[1]);
            [t[0] * ink[0], t[1] * ink[1], t[2] * ink[2], t[3] * ink[3]]
        }
        // `RING_SHADER`: the radial alpha, squared, with `Colour`.
        SkillProgram::Rounded => {
            let colour = colour.unwrap_or([1.0; 4]);
            let sx = (0.5 - uv[0]).abs();
            let sy = (0.5 - uv[1]).abs();
            let mut a = (1.0 - (sx * sx + sy * sy) * 4.0).max(0.0);
            a = (a * 4.0).min(1.0);
            a *= a;
            [colour[0], colour[1], colour[2], colour[3] * a]
        }
        // `BAR_SHADER`: the stroked cut `Prog × 8.3` across, times the
        // texture's alpha, with `Colour`.
        SkillProgram::Bar => {
            let colour = colour.unwrap_or([1.0; 4]);
            let mut a = (uv[0] * 2.0 - 0.5).abs() * 16.0;
            a = (prog.unwrap_or(0.0) * 8.3 - a).min(1.0);
            a *= texel(texture, uv[0], uv[1])[3];
            [colour[0], colour[1], colour[2], colour[3] * a]
        }
    }
}

/// Twice the signed area of `(a, b, p)`.
fn edge(a: [f32; 2], b: [f32; 2], p: [f32; 2]) -> f32 {
    (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
}

/// Whether an edge owns the pixels on it (the top-left rule), for a
/// triangle of the sign `sign`.
fn owns(a: [f32; 2], b: [f32; 2], sign: f32) -> bool {
    let dx = (b[0] - a[0]) * sign;
    let dy = (b[1] - a[1]) * sign;
    dy < 0.0 || (dy == 0.0 && dx > 0.0)
}

/// Rasterises one triangle, calling `shade` with the interpolated texture
/// coordinate and ink of every pixel centre it covers.
fn triangle(
    canvas: &mut Canvas,
    v: [SkillVertex; 3],
    shade: &dyn Fn([f32; 2], [f32; 4]) -> [f32; 4],
) {
    let p = v.map(|v| [v.x, v.y]);
    let area = edge(p[0], p[1], p[2]);
    if area == 0.0 || !area.is_finite() {
        return;
    }
    let sign = area.signum();
    let min_x = p
        .iter()
        .map(|q| q[0])
        .fold(f32::INFINITY, f32::min)
        .floor()
        .max(0.0) as i32;
    let max_x = p
        .iter()
        .map(|q| q[0])
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil()
        .min(canvas.width as f32) as i32;
    let min_y = p
        .iter()
        .map(|q| q[1])
        .fold(f32::INFINITY, f32::min)
        .floor()
        .max(0.0) as i32;
    let max_y = p
        .iter()
        .map(|q| q[1])
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil()
        .min(canvas.height as f32) as i32;
    let edges = [(1, 2, 0), (2, 0, 1), (0, 1, 2)];
    for y in min_y..max_y {
        for x in min_x..max_x {
            let c = [x as f32 + 0.5, y as f32 + 0.5];
            let mut w = [0.0f32; 3];
            let mut inside = true;
            for (i, &(a, b, _)) in edges.iter().enumerate() {
                let e = edge(p[a], p[b], c) * sign;
                if e < 0.0 || (e == 0.0 && !owns(p[a], p[b], sign)) {
                    inside = false;
                    break;
                }
                w[i] = e;
            }
            if !inside {
                continue;
            }
            let total = area.abs();
            let w = w.map(|e| e / total);
            let mix = |f: &dyn Fn(&SkillVertex) -> f32| {
                w[0] * f(&v[0]) + w[1] * f(&v[1]) + w[2] * f(&v[2])
            };
            let uv = [mix(&|v| v.u), mix(&|v| v.v)];
            let ink = [
                mix(&|v| v.ink[0]),
                mix(&|v| v.ink[1]),
                mix(&|v| v.ink[2]),
                mix(&|v| v.ink[3]),
            ];
            canvas.blend(x, y, shade(uv, ink));
        }
    }
}

/// Rasterises a run of quads (four vertices each, drawn as `GL_QUADS`).
fn quads(
    canvas: &mut Canvas,
    vertices: &[SkillVertex],
    shade: &dyn Fn([f32; 2], [f32; 4]) -> [f32; 4],
) {
    for quad in vertices.as_chunks::<4>().0 {
        triangle(canvas, [quad[0], quad[1], quad[2]], shade);
        triangle(canvas, [quad[0], quad[2], quad[3]], shade);
    }
}

/// `CVTSS2SI` keeping the low byte: half to even, the indefinite value
/// (low byte 0) for a NaN or an overflow.
fn alpha_byte(value: f32) -> u8 {
    let rounded = value.round_ties_even();
    if rounded.is_nan() || !(-2_147_483_648.0..2_147_483_648.0).contains(&rounded) {
        0
    } else {
        (rounded as i32) as u8
    }
}

/// The skin's rounded box behind a tooltip: black at the box's alpha,
/// grown by the unit times 2 × 4, its scale cut from the grown box.
fn tip_box(canvas: &mut Canvas, tip: &SkillTip, unit: i32) {
    let grow = unit as f32 * 2.0 * 4.0;
    let alpha = f32::from(alpha_byte(tip.alpha * 255.0)) / 255.0;
    let sx = (tip.width / (grow * 2.0 + tip.width)) * 0.5;
    let sy = (tip.height / (grow * 2.0 + tip.height)) * 0.5;
    let scale = [sx, sy, 1.0 / (0.5 - sx), 1.0 / (0.5 - sy)];
    let (x0, y0) = (tip.left - grow, tip.top - grow);
    let (x1, y1) = (tip.left + tip.width + grow, tip.top + tip.height + grow);
    let corner = |x: f32, y: f32, u: f32, v: f32| SkillVertex {
        x,
        y,
        u,
        v,
        ink: [0.0, 0.0, 0.0, alpha],
    };
    let shade =
        move |uv: [f32; 2], ink: [f32; 4]| exa_ui::wheel_draw::rounded_fragment(uv, ink, scale);
    quads(
        canvas,
        &[
            corner(x0, y0, 0.0, 0.0),
            corner(x0, y1, 0.0, 1.0),
            corner(x1, y1, 1.0, 1.0),
            corner(x1, y0, 1.0, 0.0),
        ],
        &shade,
    );
}

/// A label's glyph quads in the GUI font, sampled from its atlas (the
/// skin's textured program, the ink as the tint).
fn label(canvas: &mut Canvas, label: &SkillLabel, font: (&GuiFont, &GuiTexture, (u16, u16))) {
    let (font, texture, atlas) = font;
    let Some((du, dv)) = exa_ui::gui_text::font_uv(font, atlas) else {
        return;
    };
    let mut x = label.x as i32;
    if label.centred {
        x -= font.text_width(label.text.as_bytes()) >> 1;
    }
    let ink = label.ink.map(|byte| f32::from(byte) / 255.0);
    let glyphs = exa_ui::gui_text::draw_line(font, label.text.as_bytes(), x, label.y as i32);
    let shade = |uv: [f32; 2], ink: [f32; 4]| {
        let t = texel(Some(texture), uv[0], uv[1]);
        [t[0] * ink[0], t[1] * ink[1], t[2] * ink[2], t[3] * ink[3]]
    };
    for glyph in glyphs {
        let [gx, top, w, h] = glyph.rect;
        let (u0, v0) = glyph.cell;
        let corner = |x: i32, y: i32, u: f32, v: f32| SkillVertex {
            x: x as f32,
            y: y as f32,
            u,
            v,
            ink,
        };
        quads(
            canvas,
            &[
                corner(gx, top, u0, v0 + dv),
                corner(gx, top + h, u0, v0),
                corner(gx + w, top + h, u0 + du, v0),
                corner(gx + w, top, u0 + du, v0 + dv),
            ],
            &shade,
        );
    }
}

/// The draw list as one straight-alpha RGBA picture, rows top-down.
pub fn rasterise(
    list: &SkillDraw,
    atlases: &SkillAtlases,
    font: Option<(&GuiFont, &GuiTexture, (u16, u16))>,
    unit: i32,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let mut canvas = Canvas {
        width,
        height,
        pixels: vec![[0.0; 4]; width as usize * height as usize],
    };
    let overlays = |canvas: &mut Canvas, order: usize| {
        for tip in list.tips.iter().filter(|tip| tip.order == order) {
            tip_box(canvas, tip, unit);
        }
        if let Some(font) = font {
            for text in list.labels.iter().filter(|text| text.order == order) {
                label(canvas, text, font);
            }
        }
    };
    for (at, batch) in list.batches.iter().enumerate() {
        overlays(&mut canvas, at);
        let texture = match batch.texture {
            SkillTexture::None => None,
            SkillTexture::Background => atlases.background.as_ref(),
            SkillTexture::GroupIcons => Some(&atlases.group),
            SkillTexture::SingleIcons => Some(&atlases.single),
        };
        let shade = |uv: [f32; 2], ink: [f32; 4]| {
            skill_fragment(batch.program, texture, batch.colour, batch.prog, uv, ink)
        };
        quads(&mut canvas, &batch.vertices, &shade);
    }
    overlays(&mut canvas, list.batches.len());
    canvas.straight()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ring_program_fades_out_from_the_centre() {
        let colour = Some([0.0, 0.0, 0.0, 0.6]);
        let centre = skill_fragment(
            SkillProgram::Rounded,
            None,
            colour,
            None,
            [0.5, 0.5],
            [1.0; 4],
        );
        assert_eq!(centre, [0.0, 0.0, 0.0, 0.6]);
        // At the edge's middle: 1 − 0.25·4 = 0.
        let edge = skill_fragment(
            SkillProgram::Rounded,
            None,
            colour,
            None,
            [0.0, 0.5],
            [1.0; 4],
        );
        assert_eq!(edge[3], 0.0);
        // Halfway out: (1 − 0.0625·4)·4 caps at 1.
        let half = skill_fragment(
            SkillProgram::Rounded,
            None,
            colour,
            None,
            [0.25, 0.5],
            [1.0; 4],
        );
        assert_eq!(half[3], 0.6);
    }

    #[test]
    fn the_bar_program_cuts_at_its_progress() {
        let colour = Some([1.0, 0.79, 0.286, 0.5]);
        let texture = GuiTexture::solid([255, 255, 255, 255]);
        // u = 0.25 is the stroke's middle: the full bar shows.
        let full = skill_fragment(
            SkillProgram::Bar,
            Some(&texture),
            colour,
            Some(1.0),
            [0.25, 0.8],
            [1.0; 4],
        );
        assert_eq!(full[3], 0.5);
        // No progress: the alpha goes below zero (the blend clamps it).
        let none = skill_fragment(
            SkillProgram::Bar,
            Some(&texture),
            colour,
            Some(0.0),
            [0.25, 0.8],
            [1.0; 4],
        );
        assert!(none[3] <= 0.0);
    }

    #[test]
    fn a_shared_diagonal_blends_once() {
        let mut canvas = Canvas {
            width: 4,
            height: 4,
            pixels: vec![[0.0; 4]; 16],
        };
        let v = |x: f32, y: f32| SkillVertex {
            x,
            y,
            u: 0.0,
            v: 0.0,
            ink: [1.0, 1.0, 1.0, 0.5],
        };
        let shade = |_: [f32; 2], ink: [f32; 4]| ink;
        quads(
            &mut canvas,
            &[v(0.0, 0.0), v(0.0, 4.0), v(4.0, 4.0), v(4.0, 0.0)],
            &shade,
        );
        assert!(canvas.pixels.iter().all(|p| (p[3] - 0.5).abs() < 1e-6));
    }

    #[test]
    fn the_hover_grid_puts_each_skill_one_entry_on() {
        let mut skills = [Skill::default(); SKILLS];
        skills[1] = Skill {
            points: 5_000,
            techniques: [3, 0, 0, 0, 0, 0],
        };
        let grid = hover_grid(&skills);
        // Group 2's first level reads index 2·8 + 1 + 1.
        assert_eq!(grid[18], 3);
        assert_eq!(&grid[..8], &[0; 8]);
    }

    #[test]
    fn the_hit_code_finds_a_satellite() {
        let bounds = SkillBounds {
            left: 0,
            right: 1280,
            top: 0,
            bottom: 720,
        };
        let screen = SkillScreen::open(bounds, 4, false, 0, 0);
        let ring = screen.ring(1.0);
        let slot = screen.group_slot(0).unwrap();
        let (x, y) = skill_ring_position(slot, &ring);
        // The node's own centre: the group's code.
        assert_eq!(hit_code(&screen, 1.0, x as i32, y as i32), 0x100);
        // Its first satellite, straight up.
        let (sx, sy) = satellite(&screen, 0x101, 1.0).unwrap();
        assert_eq!(hit_code(&screen, 1.0, sx as i32, sy as i32), 0x101);
    }
}
