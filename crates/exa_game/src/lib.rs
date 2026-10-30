//! Game modes and session flow: Story, Arena (company management, matches, seasons), Practice; save games; ties all plugins together.
//!
//! Planned in Phase 4–7; see `README.md` next to this crate and `docs/PLAN.md`.

pub mod arena;
pub mod arena_board;
pub mod arena_hub;
pub mod arena_hub_live;
pub mod arena_ids;
pub mod arena_live;
pub mod arena_log;
pub mod arena_match;
pub mod arena_names;
pub mod arena_outfit;
pub mod arena_outfit_data;
pub mod arena_records;
pub mod arena_session;
pub mod arena_spawn;
pub mod arena_tables;
pub mod arsenal;
pub mod balance;
pub mod bindings;
pub mod body_inventory;
pub mod body_overlay;
pub mod career;
pub mod char_view;
pub mod character_voice;
pub mod commands;
pub mod container_fill;
pub mod control_nodes;
pub mod create;
pub mod creator_screen;
pub mod creator_view;
pub mod cursor;
pub mod destination;
pub mod dialogue_live;
pub mod div_card_screen;
pub mod doll_view;
pub mod drag;
pub mod effects;
pub mod emblem;
pub mod end_menu_screen;
pub mod equip;
pub mod experience;
pub mod fall;
pub mod fire_sound;
pub mod flow;
pub mod footprints;
pub mod footwork;
pub mod frame;
pub mod game_event;
pub mod gamepad;
pub mod golem_alarm;
pub mod gui_drop;
pub mod gui_paint;
pub mod gui_screens;
pub mod hand;
pub mod hirelings;
pub mod hooks;
pub mod hotkeys;
pub mod hub_forms;
pub mod hub_windows;
pub mod hud;
pub mod hw_config;
pub mod idle_orbit;
pub mod item_card;
pub mod item_container;
pub mod item_cursor;
pub mod item_effects;
pub mod item_icon_render;
pub mod item_panel_draw;
pub mod joint_trace;
pub mod level_change;
pub mod level_transition;
pub mod light_probe;
pub mod limp;
pub mod loadout;
pub mod loadout_screen;
pub mod look_overlay;
pub mod manage;
pub mod manage_icons;
pub mod match_layout;
pub mod menu;
pub mod menu_pages;
pub mod menu_screen;
pub mod menu_text;
pub mod mind;
pub mod nav_camera;
pub mod nav_drag;
pub mod nav_pick;
pub mod npc_search;
pub mod object_click;
pub mod pair_trace;
pub mod panel_live;
pub mod path_probe;
pub mod pick;
pub mod player;
pub mod player_behaviour;
pub mod player_controls;
pub mod player_light;
pub mod player_practice;
pub mod power_screen;
pub mod practice;
pub mod prelude;
pub mod report;
pub mod rise;
pub mod save_kit;
pub mod save_level;
pub mod save_live;
pub mod save_scene;
pub mod save_screen;
pub mod scene_bodies;
pub mod scene_editor;
pub mod script_cast;
pub mod session;
pub mod settings_form;
pub mod shell_draw;
pub mod shop_stock;
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
pub mod transition_screen;
pub mod use_cursor;
pub mod window;
pub mod world_clock;

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
        .add_plugins((
            body_inventory::BodyInventoryPlugin,
            panel_live::PanelLivePlugin,
            gui_drop::GuiDropPlugin,
            golem_alarm::GolemAlarmPlugin,
            use_cursor::UseCursorPlugin,
            npc_search::NpcSearchPlugin,
        ))
        .init_resource::<prelude::Tips>()
        .add_plugins((
            limp::LimpPlugin,
            balance::BalancePlugin,
            fall::FallPlugin,
            (footprints::FootprintPlugin, character_voice::CharacterVoicePlugin),
            (
                cursor::CursorPlugin,
                char_view::CharViewPlugin,
                doll_view::DollViewPlugin,
            ),
            (
                hand::HandPlugin,
                idle_orbit::IdleOrbitPlugin,
                hotkeys::HotkeyPlugin,
                equip::EquipPlugin,
                player_light::PlayerLightPlugin,
                light_probe::LightProbePlugin,
                commands::CommandPlugin,
            ),
            (
                script_cast::ScriptCastPlugin,
                hud::HudPlugin,
                dialogue_live::DialogueLivePlugin,
            ),
            torch::TorchPlugin,
            trace::TracePlugin,
            effects::EffectsPlugin,
            thaum::live::ThaumEffectsPlugin,
            teleporters::TeleportPlugin,
            hw_config::HardwareConfigPlugin,
            destination::DestinationPlugin,
            (
                menu_screen::MenuScreenPlugin,
                end_menu_screen::EndMenuScreenPlugin,
                arena_match::ArenaMatchPlugin,
                arena_hub_live::ArenaHubPlugin,
                (scene_bodies::SceneBodiesPlugin, arena_live::ArenaLivePlugin),
                gui_screens::GameScreensPlugin,
                item_card::ItemCardPlugin,
                item_icon_render::ItemIconPlugin,
                power_screen::PowerScreenPlugin,
            thaum::visuals::VisualPlugin,
            ),
        ))
        .add_plugins(thaum::cast::CastPlugin)
        .add_plugins(fire_sound::FireSoundPlugin)
        .add_plugins(world_clock::WorldClockPlugin)
        .add_plugins(save_live::SaveLivePlugin)
        .add_plugins((
            flow::FlowPlugin,
            splash::SplashPlugin,
            div_card_screen::DivinationPlugin,
            loadout_screen::LoadoutScreenPlugin,
            transition_screen::TransitionScreenPlugin,
            settings_form::SettingsFormPlugin,
            creator_screen::CreatorScreenPlugin,
            creator_view::CreatorViewPlugin,
            save_screen::SaveScreenPlugin,
        ))
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
