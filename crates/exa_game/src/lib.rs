//! Game modes and session flow: Story, Arena (company management, matches, seasons), Practice; save games; ties all plugins together.
//!
//! Planned in Phase 4–7; see `README.md` next to this crate and `docs/PLAN.md`.

pub mod archive_pass;
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
pub mod arsenal_view;
pub mod balance;
pub mod base_form;
pub mod bindings;
pub mod body_inventory;
pub mod body_overlay;
pub mod career;
pub mod char_view;
pub mod character_use;
pub mod character_voice;
pub mod commands;
pub mod container_fill;
pub mod content_requests;
pub mod control_nodes;
pub mod controls_live;
pub mod create;
pub mod creator_screen;
pub mod creator_view;
pub mod crystal_drain;
pub mod cursor;
pub mod cut_away_live;
pub mod debug_shot;
pub mod destination;
pub mod dialogue_live;
pub mod div_card_screen;
pub mod doll_view;
pub mod drag;
pub mod effects;
pub mod emblem;
pub mod end_menu_screen;
pub mod engine_log;
pub mod equip;
pub mod experience;
pub mod experience_live;
pub mod fall;
pub mod figure_pick;
pub mod fire_sound;
pub mod flow;
pub mod follower_travel;
pub mod footprints;
pub mod footwork;
pub mod form_input;
pub mod frame;
pub mod game_event;
pub mod game_event_live;
pub mod gamepad;
pub mod glow_pulse;
pub mod golem_alarm;
pub mod golem_body;
pub mod gui_drop;
pub mod gui_paint;
pub mod gui_screens;
pub mod hand;
pub mod help_live;
pub mod hirelings;
pub mod hooks;
pub mod hotkeys;
pub mod hub_forms;
pub mod hub_windows;
pub mod hud;
pub mod hw_config;
pub mod idle_orbit;
pub mod item_ability;
pub mod item_card;
pub mod item_container;
pub mod item_cursor;
pub mod item_effects;
pub mod item_icon_render;
pub mod item_panel_draw;
pub mod joint_trace;
pub mod level_change;
pub mod level_save;
pub mod level_transition;
pub mod light_probe;
pub mod limp;
pub mod loadout;
pub mod loadout_kit;
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
pub mod option_forms;
pub mod os_clipboard;
pub mod os_pointer;
pub mod pair_trace;
pub mod panel_live;
pub mod parchment;
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
pub mod revive_pass;
pub mod rise;
pub mod role_scripts;
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
pub mod shop_panels;
pub mod shop_preview;
pub mod shop_stock;
pub mod shop_window;
pub mod sim_control;
pub mod skill_screen;
pub mod slots;
pub mod socket_drag;
pub mod spectate;
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
pub mod view_overlay;
pub mod weather;
pub mod window;
pub mod world_characters;
pub mod world_clock;
pub mod zone_scripts;

use bevy::prelude::*;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        bridge_game_types(app);
        // The save unit's importer hook for a level read from the archive.
        exa_world::set_archive_pass(archive_pass::archive_pass);
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
            golem_body::GolemBodyPlugin,
            use_cursor::UseCursorPlugin,
            npc_search::NpcSearchPlugin,
            role_scripts::RoleScriptsPlugin,
            zone_scripts::ZoneScriptsPlugin,
            character_use::CharacterUsePlugin,
            cut_away_live::CutAwayPlugin,
        ))
        .init_resource::<prelude::Tips>()
        .init_resource::<os_clipboard::SystemClipboard>()
        .add_systems(Update, prelude::sync_tips_setting)
        .add_plugins(prelude::TipsScreenPlugin)
        .add_plugins((
            limp::LimpPlugin,
            balance::BalancePlugin,
            (fall::FallPlugin, level_change::LevelChangeBridgePlugin),
            (footprints::FootprintPlugin, character_voice::CharacterVoicePlugin),
            (
                cursor::CursorPlugin,
                char_view::CharViewPlugin,
                doll_view::DollViewPlugin,
                figure_pick::FigurePickPlugin,
            ),
            (
                hand::HandPlugin,
                idle_orbit::IdleOrbitPlugin,
                hotkeys::HotkeyPlugin,
                equip::EquipPlugin,
                player_light::PlayerLightPlugin,
                spectate::SpectatePlugin,
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
            (
                teleporters::TeleportPlugin,
                glow_pulse::GlowPulsePlugin,
                crystal_drain::CrystalDrainPlugin,
                item_ability::ItemAbilityPlugin,
                socket_drag::SocketDragPlugin,
            ),
            (
                hw_config::HardwareConfigPlugin,
                os_pointer::OsPointerPlugin,
                engine_log::EngineLogPlugin,
                view_overlay::ViewOverlayPlugin,
            ),
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
                arsenal_view::ArsenalViewPlugin,
                power_screen::PowerScreenPlugin,
                skill_screen::SkillScreenPlugin,
                controls_live::ControlsLivePlugin,
            thaum::visuals::VisualPlugin,
            ),
        ))
        .add_plugins(thaum::cast::CastPlugin)
        .add_plugins(mind::draw::MindSightPlugin)
        .add_plugins(mind::runtime::MindRuntimePlugin)
        .add_plugins(fire_sound::FireSoundPlugin)
        .add_plugins(weather::WeatherPlugin)
        .add_plugins(world_clock::WorldClockPlugin)
        .add_plugins(save_live::SaveLivePlugin)
        .add_plugins(content_requests::ContentRequestsPlugin)
        .add_plugins(help_live::HelpLivePlugin)
        .add_plugins((
            flow::FlowPlugin,
            splash::SplashPlugin,
            div_card_screen::DivinationPlugin,
            loadout_screen::LoadoutScreenPlugin,
            experience_live::ExperienceLivePlugin,
            game_event_live::GameEventLivePlugin,
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

/// The data this plugin's fixed-step systems touch, for the simulation's
/// bridge (`exa_physics::sim_bridge`).
fn bridge_game_types(app: &mut App) {
    use exa_physics::sim_bridge::SimBridgeAppExt;
    app.bridge_copy::<crate::item_ability::ItemArcs>()
        .bridge_move_resource::<crate::joint_trace::JointTrace>()
        .bridge_move_resource::<crate::pair_trace::PairTrace>()
        .bridge_snapshot_resource::<crate::path_probe::PathProbe>()
        .bridge_move_resource::<crate::trace::BodyTrace>();
}
