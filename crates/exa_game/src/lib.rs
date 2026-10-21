//! Game modes and session flow: Story, Arena (company management, matches, seasons), Practice; save games; ties all plugins together.
//!
//! Planned in Phase 4–7; see `README.md` next to this crate and `docs/PLAN.md`.

pub mod arena;
pub mod arena_board;
pub mod arena_hub;
pub mod arena_ids;
pub mod arena_log;
pub mod arena_match;
pub mod arena_outfit;
pub mod arena_outfit_data;
pub mod arena_spawn;
pub mod arsenal;
pub mod balance;
pub mod bindings;
pub mod body_overlay;
pub mod career;
pub mod char_view;
pub mod commands;
pub mod create;
pub mod cursor;
pub mod destination;
pub mod drag;
pub mod effects;
pub mod emblem;
pub mod experience;
pub mod fall;
pub mod flow;
pub mod footprints;
pub mod frame;
pub mod game_event;
pub mod gamepad;
pub mod gui_screens;
pub mod hand;
pub mod hirelings;
pub mod hotkeys;
pub mod hud;
pub mod hw_config;
pub mod idle_orbit;
pub mod item_container;
pub mod item_effects;
pub mod joint_trace;
pub mod limp;
pub mod loadout;
pub mod look_overlay;
pub mod manage;
pub mod manage_icons;
pub mod match_layout;
pub mod menu;
pub mod menu_pages;
pub mod menu_screen;
pub mod mind;
pub mod nav_camera;
pub mod nav_pick;
pub mod pair_trace;
pub mod path_probe;
pub mod pick;
pub mod player;
pub mod player_behaviour;
pub mod player_controls;
pub mod player_light;
pub mod power_screen;
pub mod practice;
pub mod prelude;
pub mod report;
pub mod session;
pub mod shell_draw;
pub mod shop_window;
pub mod sim_control;
pub mod slots;
pub mod splash;
pub mod staff;
pub mod stages;
pub mod steering;
pub mod techniques;
pub mod teleporters;
pub mod thaum;
pub mod torch;
pub mod tournament;
pub mod trace;
pub mod trade;
pub mod window;

use bevy::prelude::*;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            exa_assets::AssetsPlugin,
            player::PlayerPlugin,
            gamepad::GamepadPlugin,
            exa_render::RenderPlugin,
            exa_physics::PhysicsPlugin,
            exa_anim::AnimPlugin,
            exa_character::CharacterPlugin,
            exa_items::ItemsPlugin,
            exa_combat::CombatPlugin,
            exa_ai::AiPlugin,
            exa_world::WorldPlugin,
            exa_audio::AudioPlugin,
            exa_script::ScriptPlugin,
            exa_ui::UiPlugin,
        ))
        .init_resource::<prelude::Tips>()
        .add_plugins((
            limp::LimpPlugin,
            balance::BalancePlugin,
            fall::FallPlugin,
            footprints::FootprintPlugin,
            (cursor::CursorPlugin, char_view::CharViewPlugin),
            (
                hand::HandPlugin,
                idle_orbit::IdleOrbitPlugin,
                hotkeys::HotkeyPlugin,
                player_light::PlayerLightPlugin,
                commands::CommandPlugin,
            ),
            hud::HudPlugin,
            torch::TorchPlugin,
            trace::TracePlugin,
            effects::EffectsPlugin,
            thaum::live::ThaumEffectsPlugin,
            teleporters::TeleportPlugin,
            hw_config::HardwareConfigPlugin,
            destination::DestinationPlugin,
            (
                menu_screen::MenuScreenPlugin,
                arena_match::ArenaMatchPlugin,
                gui_screens::GameScreensPlugin,
                power_screen::PowerScreenPlugin,
            thaum::visuals::VisualPlugin,
            ),
        ))
        .add_plugins(thaum::cast::CastPlugin)
        .add_plugins((flow::FlowPlugin, splash::SplashPlugin))
        // Debug tools: idle unless their flags or keys turn them on.
        .add_plugins((
            body_overlay::BodyOverlayPlugin,
            joint_trace::JointTracePlugin,
            look_overlay::LookOverlayPlugin,
            pair_trace::PairTracePlugin,
            path_probe::PathProbePlugin,
            sim_control::SimControlPlugin,
        ));
    }
}
