//! Game modes and session flow: Story, Arena (company management, matches, seasons), Practice; save games; ties all plugins together.
//!
//! Planned in Phase 4–7; see `README.md` next to this crate and `docs/PLAN.md`.

pub mod arena;
pub mod arena_board;
pub mod arena_log;
pub mod balance;
pub mod bindings;
pub mod body_overlay;
pub mod career;
pub mod create;
pub mod cursor;
pub mod destination;
pub mod drag;
pub mod effects;
pub mod emblem;
pub mod experience;
pub mod fall;
pub mod footprints;
pub mod frame;
pub mod hand;
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
pub mod menu;
pub mod menu_pages;
pub mod menu_screen;
pub mod mind;
pub mod nav_camera;
pub mod nav_pick;
pub mod pair_trace;
pub mod pick;
pub mod player;
pub mod player_light;
pub mod practice;
pub mod session;
pub mod shell_draw;
pub mod sim_control;
pub mod slots;
pub mod stages;
pub mod steering;
pub mod techniques;
pub mod teleporters;
pub mod thaum;
pub mod torch;
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
        .add_plugins((
            limp::LimpPlugin,
            balance::BalancePlugin,
            fall::FallPlugin,
            footprints::FootprintPlugin,
            cursor::CursorPlugin,
            hand::HandPlugin,
            idle_orbit::IdleOrbitPlugin,
            hotkeys::HotkeyPlugin,
            player_light::PlayerLightPlugin,
            hud::HudPlugin,
            torch::TorchPlugin,
            trace::TracePlugin,
            effects::EffectsPlugin,
            thaum::live::ThaumEffectsPlugin,
            teleporters::TeleportPlugin,
            hw_config::HardwareConfigPlugin,
            destination::DestinationPlugin,
            menu_screen::MenuScreenPlugin,
        ))
        // Debug tools: idle unless their flags or keys turn them on.
        .add_plugins((
            body_overlay::BodyOverlayPlugin,
            joint_trace::JointTracePlugin,
            look_overlay::LookOverlayPlugin,
            pair_trace::PairTracePlugin,
            sim_control::SimControlPlugin,
        ));
    }
}
