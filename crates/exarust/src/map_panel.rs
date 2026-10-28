//! The map panel (F9, Escape closes it) in the level and play views: the
//! current map, switches for point shadows, every room, the navigation grid
//! and the body overlay, and the installation's levels. Choosing a level
//! restarts Exarust on it in the same view with the same flags, less those
//! tied to a place or a script. While the panel is open the mouse belongs
//! to it. Rules: `docs/DEBUGGING.md` (Map panel).
// Bevy 0.20 deprecates the focus-based `Interaction` for its picking
// widgets, which need picking features the workspace does not enable; the
// focus system still runs and updates any node carrying it.
#![allow(deprecated)]

use std::process::Command;

use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::ui::{FocusPolicy, UiSystems};
use exa_assets::{Game, PackKind};
use exa_formats::level::is_level;
use exa_game::body_overlay::BodyOverlay;
use exa_render::cube_shadows::PointShadows;
use exa_ui::PanelPointer;
use exa_world::nav::NavOverlay;
use exa_world::room_view::RoomView;

use crate::View;

/// Opens and closes the panel.
pub const PANEL_KEY: KeyCode = KeyCode::F9;

/// Flags dropped on a restart, each with its value: they name a place, a
/// file or a scripted moment of the map being left.
const DROPPED: [&str; 21] = [
    "--at",
    "--camera",
    "--corpse",
    "--torch",
    "--screenshot",
    "--frames",
    "--press",
    "--click",
    "--cursor",
    "--debug-column",
    "--shove",
    "--shove-props",
    "--pause-at-tick",
    "--pause-after-jet",
    "--pause-on-stretch",
    "--pause-on-hit",
    "--pause-after-collapse",
    "--arena-match",
    "--arena-level",
    "--arena-tier",
    "--arena-weapons",
];

const FONT: f32 = 15.0;
const WIDTH: f32 = 300.0;
/// Pixels the list scrolls per wheel line.
const LINE: f32 = 24.0;
const TEXT: Color = Color::srgb(0.86, 0.82, 0.74);
const DIM: Color = Color::srgb(0.6, 0.57, 0.5);
const BACKGROUND: Color = Color::srgba(0.03, 0.03, 0.035, 0.88);
const BUTTON: Color = Color::srgb(0.13, 0.12, 0.11);
const HOVER: Color = Color::srgb(0.24, 0.21, 0.17);
const CURRENT: Color = Color::srgb(0.38, 0.28, 0.14);

/// Whether the panel is open.
#[derive(Resource, Debug, Default)]
pub struct MapPanel {
    pub open: bool,
}

/// The installation's levels: the `.rfc` stems of `Resource.rpk` with a
/// tile grid or placed props, sorted.
#[derive(Resource, Debug, Default)]
struct LevelList(Vec<String>);

#[derive(Component)]
struct PanelRoot;

/// The scrolling list of levels.
#[derive(Component)]
struct LevelScroll;

/// A switch of the current map's view.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum Switch {
    Shadows,
    AllRooms,
    NavGrid,
    Bodies,
}

impl Switch {
    const ALL: [Self; 4] = [Self::Shadows, Self::AllRooms, Self::NavGrid, Self::Bodies];

    fn name(self) -> &'static str {
        match self {
            Self::Shadows => "Point shadows",
            Self::AllRooms => "Every room",
            Self::NavGrid => "Navigation grid (F6)",
            Self::Bodies => "Body overlay (F3)",
        }
    }
}

/// The label of a switch button.
#[derive(Component)]
struct SwitchLabel(Switch);

/// A level's button.
#[derive(Component)]
struct LevelButton(String);

/// A clickable row of the panel: the focus system keeps its
/// [`Interaction`] and it blocks the pointer from what lies behind.
#[derive(Component, Default)]
#[require(Interaction, FocusPolicy::Block)]
struct PanelButton;

type ShadedButtons<'w, 's> = Query<
    'w,
    's,
    (
        &'static Interaction,
        &'static mut BackgroundColor,
        Option<&'static LevelButton>,
    ),
    (Changed<Interaction>, With<PanelButton>),
>;

pub(crate) struct MapPanelPlugin;

impl Plugin for MapPanelPlugin {
    fn build(&self, app: &mut App) {
        let level_views = |view: Res<View>| matches!(*view, View::Level(_) | View::Play(_));
        app.init_resource::<MapPanel>()
            .add_systems(Startup, list_levels.run_if(level_views))
            .add_systems(
                PreUpdate,
                keep_mouse
                    .after(UiSystems::Focus)
                    .run_if(|panel: Res<MapPanel>| panel.open),
            )
            .add_systems(
                Update,
                (
                    toggle_panel,
                    show_panel,
                    (press_switches, label_switches, press_levels, shade_buttons)
                        .run_if(|panel: Res<MapPanel>| panel.open),
                )
                    .chain()
                    .run_if(level_views),
            );
    }
}

/// The level a view shows, if it shows one.
fn current_level(view: &View) -> Option<&str> {
    match view {
        View::Level(name) | View::Play(name) => Some(name),
        _ => None,
    }
}

fn list_levels(mut commands: Commands, game: Res<Game>) {
    let Some(resource) = game.0.pack(PackKind::Resource) else {
        return;
    };
    let mut names: Vec<String> = resource
        .entries()
        .iter()
        .filter(|e| e.name.to_ascii_lowercase().ends_with(".rfc"))
        .filter(|e| is_level(resource.bytes(e)))
        .map(|e| e.name[..e.name.len() - 4].to_string())
        .collect();
    names.sort_by_key(|n| n.to_ascii_lowercase());
    names.dedup();
    info!("map panel: {} levels (F9)", names.len());
    commands.insert_resource(LevelList(names));
}

fn toggle_panel(
    keys: Res<ButtonInput<KeyCode>>,
    mut panel: ResMut<MapPanel>,
    mut pointer: ResMut<PanelPointer>,
) {
    if keys.just_pressed(PANEL_KEY) {
        panel.open = !panel.open;
    } else if panel.open && keys.just_pressed(KeyCode::Escape) {
        panel.open = false;
    }
    pointer.set_if_neq(PanelPointer(panel.open));
}

/// Shows or hides the panel, building it the first time it opens.
fn show_panel(
    mut commands: Commands,
    panel: Res<MapPanel>,
    levels: Option<Res<LevelList>>,
    view: Res<View>,
    mut roots: Query<&mut Visibility, With<PanelRoot>>,
) {
    let shown = if panel.open {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if let Ok(mut v) = roots.single_mut() {
        v.set_if_neq(shown);
        return;
    }
    if !panel.open {
        return;
    }
    let levels: &[String] = levels.as_deref().map_or(&[], |l| &l.0);
    spawn_panel(&mut commands, &view, levels);
}

fn text(s: impl Into<String>, color: Color) -> impl Bundle {
    (
        Text::new(s),
        TextFont::from_font_size(FONT),
        TextColor(color),
    )
}

fn button_node() -> Node {
    Node {
        padding: UiRect::axes(px(8.0), px(4.0)),
        ..default()
    }
}

fn spawn_panel(commands: &mut Commands, view: &View, levels: &[String]) {
    let current = current_level(view).unwrap_or_default();
    let mode = if matches!(view, View::Play(_)) {
        "play"
    } else {
        "level"
    };
    commands
        .spawn((
            Name::new("MapPanel"),
            PanelRoot,
            Node {
                position_type: PositionType::Absolute,
                right: px(8.0),
                // Below the frame-rate readout.
                top: px(36.0),
                width: px(WIDTH),
                max_height: percent(90.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(8.0)),
                row_gap: px(4.0),
                ..default()
            },
            BackgroundColor(BACKGROUND),
            GlobalZIndex(20),
        ))
        .with_children(|panel| {
            panel.spawn(text("Map (F9 or Escape closes)", TEXT));
            panel.spawn(text(format!("{mode} {current}"), DIM));
            panel.spawn(text("View", TEXT));
            for switch in Switch::ALL {
                panel
                    .spawn((PanelButton, button_node(), BackgroundColor(BUTTON), switch))
                    .with_children(|b| {
                        b.spawn((text(switch.name(), TEXT), SwitchLabel(switch)));
                    });
            }
            panel.spawn(text("Switch map (restarts Exarust)", TEXT));
            panel
                .spawn((
                    LevelScroll,
                    Node {
                        flex_direction: FlexDirection::Column,
                        overflow: Overflow::scroll_y(),
                        min_height: px(0.0),
                        flex_shrink: 1.0,
                        row_gap: px(2.0),
                        ..default()
                    },
                    ScrollPosition::default(),
                ))
                .with_children(|list| {
                    for name in levels {
                        let colour = if name.eq_ignore_ascii_case(current) {
                            CURRENT
                        } else {
                            BUTTON
                        };
                        list.spawn((
                            PanelButton,
                            button_node(),
                            BackgroundColor(colour),
                            LevelButton(name.clone()),
                        ))
                        .with_children(|b| {
                            b.spawn(text(name.clone(), TEXT));
                        });
                    }
                });
        });
}

/// The mouse belongs to the panel: the wheel scrolls the level list and
/// neither the wheel nor the buttons reach the game (the camera's zoom, the
/// player's attack and walk).
fn keep_mouse(
    mut buttons: ResMut<ButtonInput<MouseButton>>,
    mut scroll: ResMut<AccumulatedMouseScroll>,
    mut lists: Query<&mut ScrollPosition, With<LevelScroll>>,
) {
    let dy = match scroll.unit {
        MouseScrollUnit::Line => scroll.delta.y * LINE,
        MouseScrollUnit::Pixel => scroll.delta.y,
    };
    if dy != 0.0 {
        for mut at in &mut lists {
            at.y = (at.y - dy).max(0.0);
        }
    }
    scroll.delta = Vec2::ZERO;
    buttons.reset_all();
}

fn press_switches(
    buttons: Query<(&Interaction, &Switch), Changed<Interaction>>,
    mut shadows: Option<ResMut<PointShadows>>,
    mut rooms: Option<ResMut<RoomView>>,
    mut nav: Option<ResMut<NavOverlay>>,
    mut bodies: Option<ResMut<BodyOverlay>>,
) {
    for (interaction, switch) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match switch {
            Switch::Shadows => {
                if let Some(s) = shadows.as_mut() {
                    s.0 = !s.0;
                }
            }
            Switch::AllRooms => {
                if let Some(r) = rooms.as_mut() {
                    r.all = !r.all;
                }
            }
            Switch::NavGrid => {
                if let Some(n) = nav.as_mut() {
                    n.on = !n.on;
                }
            }
            Switch::Bodies => {
                if let Some(b) = bodies.as_mut() {
                    b.on = !b.on;
                }
            }
        }
    }
}

fn label_switches(
    shadows: Option<Res<PointShadows>>,
    rooms: Option<Res<RoomView>>,
    nav: Option<Res<NavOverlay>>,
    bodies: Option<Res<BodyOverlay>>,
    mut labels: Query<(&SwitchLabel, &mut Text)>,
) {
    for (label, mut text) in &mut labels {
        let on = match label.0 {
            Switch::Shadows => shadows.as_ref().map(|s| s.0),
            Switch::AllRooms => rooms.as_ref().map(|r| r.all),
            Switch::NavGrid => nav.as_ref().map(|n| n.on),
            Switch::Bodies => bodies.as_ref().map(|b| b.on),
        };
        let state = match on {
            Some(true) => "on",
            Some(false) => "off",
            None => "n/a",
        };
        let line = format!("{}: {state}", label.0.name());
        if text.0 != line {
            text.0 = line;
        }
    }
}

fn shade_buttons(view: Res<View>, mut buttons: ShadedButtons) {
    let current = current_level(&view).unwrap_or_default();
    for (interaction, mut colour, level) in &mut buttons {
        colour.0 = match interaction {
            Interaction::Hovered | Interaction::Pressed => HOVER,
            Interaction::None if level.is_some_and(|l| l.0.eq_ignore_ascii_case(current)) => {
                CURRENT
            }
            Interaction::None => BUTTON,
        };
    }
}

fn press_levels(
    buttons: Query<(&Interaction, &LevelButton), Changed<Interaction>>,
    view: Res<View>,
    mut exit: MessageWriter<AppExit>,
) {
    let current = current_level(&view).unwrap_or_default();
    for (interaction, level) in &buttons {
        if *interaction == Interaction::Pressed && !level.0.eq_ignore_ascii_case(current) {
            restart(&level.0, &mut exit);
        }
    }
}

/// The command line that shows `level` instead: `args` (without the program)
/// less the [`DROPPED`] flags and their values, with the name after the
/// first `play` or `level` replaced. `None` when there is no such name.
pub(crate) fn restart_args(args: &[String], level: &str) -> Option<Vec<String>> {
    let mut out = Vec::with_capacity(args.len());
    let mut swapped = false;
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        if DROPPED.contains(&a) {
            i += 2;
            continue;
        }
        out.push(args[i].clone());
        if !swapped && (a == "play" || a == "level") && i + 1 < args.len() {
            out.push(level.to_string());
            swapped = true;
            i += 2;
            continue;
        }
        i += 1;
    }
    swapped.then_some(out)
}

/// Restarts Exarust on `level`: on Unix the process replaces itself (a
/// wrapper waiting on it keeps waiting); elsewhere the new one starts and
/// this one exits.
fn restart(level: &str, exit: &mut MessageWriter<AppExit>) {
    restart_with(level, &[], exit);
}

/// Restarts on `level` with `extra` arguments appended. The map panel's
/// level buttons still restart the process (a debug view switch, which also
/// serves the model and level views); the game's own level changes run in
/// process (`exa_world::level_lifecycle`).
fn restart_with(level: &str, extra: &[String], exit: &mut MessageWriter<AppExit>) {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(mut next) = restart_args(&args, level) else {
        warn!("map panel: no level on the command line to replace");
        return;
    };
    next.extend(extra.iter().cloned());
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(e) => {
            error!("map panel: cannot find the executable: {e}");
            return;
        }
    };
    info!("map panel: restarting on {level}: {}", next.join(" "));
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = Command::new(&exe).args(&next).exec();
        error!("map panel: restart failed: {err}");
        let _ = exit;
    }
    #[cfg(not(unix))]
    match Command::new(&exe).args(&next).spawn() {
        Ok(_) => {
            exit.write(AppExit::Success);
        }
        Err(e) => error!("map panel: restart failed: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    /// The level is swapped, flags kept, and the place- and script-bound
    /// flags dropped with their values.
    #[test]
    fn a_restart_keeps_the_flags_and_swaps_the_level() {
        let before = args(
            "--no-gi --at 1,2,3 --weapon Sword01 --camera 1,2,3,4,5 --press W@40-120 \
             play exanima01 --screenshot x.png --frames 90 --opponent",
        );
        assert_eq!(
            restart_args(&before, "arenao3"),
            Some(args("--no-gi --weapon Sword01 play arenao3 --opponent"))
        );
        assert_eq!(
            restart_args(&args("--no-shadows level arena1"), "arenao2"),
            Some(args("--no-shadows level arenao2"))
        );
        assert_eq!(restart_args(&args("character Default/Walk"), "x"), None);
        assert_eq!(restart_args(&args("play"), "x"), None);
    }
}
