//! User interface: main menu, character creation, inventory, arena management, HUD, manual pages, tooltips, cursor.
//!
//! Planned in Phase 4–7; see `README.md` next to this crate and `docs/PLAN.md`.
//! So far: the game's bitmap font ([`font`]), the interface scale
//! ([`scale`]), the frame-rate readout ([`fps`]), the loading screen
//! ([`loading`]), the in-game world cursor ([`cursor`]), the health and
//! focus bars ([`hud`]), the low-health heartbeat's border
//! ([`heartbeat`]) and the fade to black of a fall to death ([`fade`]),
//! painted with gamma-space blending ([`blend`]); and the original's
//! widgets: its GUI fonts and levels ([`gui_font`], [`gui_level`]), text
//! layout and the text box ([`text_layout`], [`text_box`]), icon atlases
//! and glyphs ([`icon_atlas`], [`gui_glyphs`]), the table and item views
//! ([`table_view`], [`item_view`]) and the basic controls
//! ([`gui_controls`]), how forms rescale with the unit
//! ([`gui_rescale`]), and the original's draw skin ([`skin`]): its palette,
//! layout metrics, immediate-mode batches, flat control parts, form chrome,
//! background, page order and labels, the thin-line subclass's overrides and
//! widget dispatcher, and the line skin's remaining methods ([`line_skin`]):
//! its unit lengths, centred icon, scroll refresh, progress strip, integer
//! span bar and maximised window frame. The arena match board's tabs, match
//! and event tables, tournament and season rows, fighter assignment, layout
//! and drawing order live in [`match_board`]. The in-game unit's screens,
//! journal, open-container registry, power-slot and counter controls, HUD
//! extras and close/save scopes live in [`game_gui`]; the form base class's
//! ordering, fade, traversal, scroll and hit rules in [`form`].

pub mod blend;
pub mod character_view;
pub mod class_select;
pub mod col_swatch;
pub mod colour_picker;
pub mod colour_swatch;
pub mod compact_skill;
pub mod controls;
pub mod creation;
pub mod cursor;
pub mod dialogs;
pub mod dialogue_choices;
pub mod div_card;
pub mod drop_down;
pub mod emblem_form;
pub mod engine_math;
pub mod fade;
pub mod font;
pub mod form;
pub mod fps;
pub mod game_gui;
pub mod game_tips;
pub mod gui_controls;
pub mod gui_core;
pub mod gui_draw;
pub mod gui_font;
pub mod gui_glyphs;
pub mod gui_level;
pub mod gui_rescale;
pub mod gui_task;
pub mod gui_text;
pub mod heartbeat;
pub mod help_screen;
pub mod help_system;
pub mod hud;
pub mod icon_atlas;
pub mod item_view;
pub mod line_skin;
pub mod loading;
pub mod loadout;
pub mod match_board;
pub mod menu;
pub mod modal_screens;
pub mod picker_fields;
pub mod pickers;
pub mod popups;
pub mod scale;
pub mod selection;
pub mod settings;
pub mod skill;
pub mod skill_draw;
pub mod skill_gui;
pub mod skin;
pub mod speech;
pub mod table_view;
pub mod task_stack;
pub mod technique_selector;
pub mod text_box;
pub mod text_edit;
pub mod text_layout;
pub mod tooltips;
pub mod transition;
pub mod tree_draw;
pub mod view_helpers;
pub mod viewport;

use bevy::prelude::*;

pub use character_view::{
    AcceptDragInput, FigureDragEnd, FigureDrop, FigureDropInput, Shown, ViewRef, accepts_drag,
    background_colours, camera_position, current_card, default_camera_position,
    drag_end as figure_drag_end, drag_start as figure_drag_start, drop_kind, ease_race_share,
    find_owner, held_index, insight_blocks, player_pack, show_item, supersample_level,
};
pub use col_swatch::{
    COLUMNS, ColourSwatch, Heard as SwatchHeard, ROWS, SwatchCell, SwatchLabel, cell_at,
    cells as swatch_cells, index_text as swatch_index_text, selection_frames,
};
pub use colour_picker::{
    COLOUR_SWATCH_CLASS, ColourPicker, FINISH_WORD, Hsv, Opened, PickerSet, REFRESH_WORD, Ramp,
    Refresh, Rgba, SwatchControl, channel_ramps, field_corners, packed_bytes, rgb_bytes, to_byte,
    unpacked,
};
pub use compact_skill::{
    CompactBase, CompactDraw, CompactMetrics, CompactRow, CompactSkillState, NodeDraw, Pip,
    cell_centre, decode_cell, draw_order as compact_draw_order, handle_key as compact_key,
    hit_cells, lay_out_pips, metrics as compact_metrics, pips, technique_nodes, technique_slots,
    wheel,
};
pub use cursor::{
    CursorCapture, CursorShape, DrawWorldCursor, PanelPointer, ScriptedPointer, WorldCursor,
    pointer_visibility,
};
pub use div_card::{
    CARDS, CardDraw, CardVertex, Reading, SHEET, SOUND, ShadowVertex, back_image, back_quad,
    card_place, card_sample, card_size, clock_step, deal_cards, draw_order as div_draw_order, ease,
    face_image, face_quad, fade_alpha, frame as div_frame, mouse as div_mouse, rotation,
    shadow_mesh, shadow_outline, turn, turn_share,
};
pub use drop_down::{
    DropCommand, DropDownList, DropDownOwner, DropLayout, DropPress, DropRelease, LIST_ATTRIBUTES,
    LIST_BORDER, LIST_STYLE, MAX_ROWS, ROW_UNITS, RowDraw, SCROLL_BIT, SelectionBand,
    layout as drop_down_layout,
};
pub use emblem_form::{
    BACKGROUND_MAX, BACKGROUND_STEM, COLOUR_THREE_TABLE, COLOUR_UNIFORMS, DESIGN_MAX, DESIGN_STEM,
    EMBLEM_SHADER, EmblemDraw, EmblemEditor, EmblemPreview, EmblemRandom, EmblemRecord,
    MIRROR_UNIFORM, PictureSelection, draw_order as emblem_draw_order, emblem_sample, picture,
    preview_colours, preview_image, preview_quad, preview_textures, select_background,
    select_design,
};
pub use fade::DeathFade;
pub use fps::ShowFps;
pub use gui_controls::{
    CheckLabelInk, ClassRegistry, LabelDraw, LabelFront, LayoutClient, append_records,
    check_label_ink, label_draw, layout_records, register_class, register_default_classes,
};
pub use gui_core::{
    DropState, EdgeForm, FormHost, FormId, GuiMetrics, GuiTask, HitResult, MessageBoxPlan,
    PlaceRequest, PlainForm, PointerState, TickResult, TooltipClasses, TooltipRequest, TooltipShow,
    clamp_form, drag_limit, drag_threshold, edge_rect, message_box_plan, place_form, set_control,
    set_control_on, set_control_text, set_modal,
};
pub use gui_draw::{
    BLEND_ADDITIVE, BLEND_ALPHA, BLEND_PREMULTIPLIED, BLEND_REPLACE, BlendFactor, GuiCanvas,
    GuiDrawPlugin, GuiFrame, GuiFrames, GuiLayer, GuiTexture, GuiTextures, HUD_STONE_MODE,
    LayerTexture, SKIN_TEXTURE, blend_factor, blend_factors, blend_word, glyph_batch, icon_batch,
    rasterise, text_batch,
};
pub use gui_level::GuiLevelState;
pub use gui_task::{
    ACTIVE_ALPHA, CallbackList, INACTIVE_ALPHA, KeyOutcome, LAYER_ATTRIBUTES, LAYER_BORDER,
    LAYER_STYLE, NO_SHORTCUT_KEY, OwnedForm, Shortcut, TaskFocus, TaskLayer, modifier,
    modifier_score,
};
pub use gui_text::{
    ELLIPSIS, FontBinding, MENU_FONT, MenuFont, TextQuad, WrappedDraw, add_gap, add_gap_to_fonts,
    count_lines, draw_line, draw_line_centred, draw_line_plain, draw_span, draw_tail,
    draw_truncated, draw_wrapped, draw_wrapped_at, ellipsis_fit, fit_menu_width, fit_width,
    font_uv, glyph_cell, measure, measure_in, menu_font_scale, needs_rescale,
};
pub use heartbeat::Heartbeat;
pub use help_system::{
    MANUAL_ATTRIBUTES, MANUAL_CONTROLLER, MANUAL_KEYBOARD, MANUAL_LIST_TOP, MANUAL_LIST_WIDTH,
    MANUAL_OPTIONS, MANUAL_SIZE, MANUAL_TITLE, Manual, ManualSource, TIP_ATTRIBUTES, TIP_CONTROLS,
    TIP_EXTRA_HEIGHT, TIP_OPTIONS, TIP_PRESET, TIP_TEXT_INSET, TIP_TITLES, TIP_WIDTH, TOPIC_INDEX,
    TOPIC_RECORD, TOPIC_TITLE, Tip, TipPlan, TipStore, TopicRef, close_manual, manual_file,
    parse_topics, show_tip, tip_index, tip_plan,
};
pub use hud::{DrawHud, HudBars};
pub use line_skin::{
    LINE_SCHEME, LINE_STRIP_CAP, LineIcon, LineIconAlpha, LineIconInk, LineMetrics, LineScroll,
    LineScrollPlace, WINDOW_CORNER, WindowFrame, WindowFrameInk, WindowFramePlace, draw_line_icon,
    draw_line_progress, draw_open_span, draw_window_frame, line_inks, line_palette,
    refresh_line_scroll, register_line_skin, set_line_metrics,
};
pub use loading::LoadingStatus;
pub use loadout::{
    CONTROLS as LOADOUT_CONTROLS, DropResult, DropSource, EquipFacts, EquipKind, FORM_HEIGHT,
    FORM_OPACITY, FORM_WIDTH, FormControl, HeldWeapon, LoadoutEditor, LoadoutItem, LoadoutLayout,
    LoadoutMode, LoadoutSave, OFFHAND_FLAG, Slot, apply_drop, can_equip,
    controls as loadout_controls, copies, encumbrance, item_cost, layout as loadout_layout,
    may_equip, opacity as loadout_opacity, points_used, save_on_close, unequip,
};
pub use match_board::{
    Board, BoardAction, BoardHit, BoardInput, BoardLayout, BoardPaint, BoardTab, CellValue,
    CloseState, CoinDraw, Column, EventEntry, Fighter, MatchBoardDestroy, MatchBoardShow,
    MatchBoardShowInput, MatchRecord, Participant, Rect, STANDARD_MATCHES, ShowState,
    StandardMatch, TableLayout, TableSlot, TournamentStatus, WeaponRow, Widget, coin_value,
    control, destroy_match_board, is_tournament_day, layout, pack_coins, rank_name, rank_of,
    scale_coins, season_index, season_picture_name, season_reward_order, season_weeks_elapsed,
    show_match_board, tournament_begins_in,
};
pub use menu::{
    CLICK_SOUND, CharLootShow, CharLootShowInput, ClassCardHit, ClassCardHitInput, ClassConfirm,
    ClassNav, ClassNavAction, ClassNavInput, ClassSelect, EndSessionAction, EndSessionDispatch,
    EndSessionInput, InventoryDetach, InventoryDetachInput, InventoryKeyClose, InventoryPage,
    JournalShow, MENU_DEFAULTS, MENU_NAME_CODE, MENU_THEME, MENU_THEME_DNB, MainMenuKey,
    MainMenuKeyAction, MainMenuKeyInput, MenuBack, MenuConfirm, MenuDefaults, MenuForward,
    MenuHover, MenuKeyClose, MenuName, MenuResume, MenuSelect, MenuShow, MenuStep, MusicToggle,
    SELECT_SOUND, SettingsOpen, SkillKey, SkillSelection, SkillSelectorApplied, SkillSelectorDraw,
    SkillSelectorDrawPlace, SkillSelectorMode, SkillSelectorShow, SkillTipDraw, SkillTipPlace,
    SkillTipShow, SkillTipShowInput, SplashBounds, SplashDestroy, SplashDestroyInput,
    TRANSITION_TEXT, TransitionFade, TransitionFadePlace, ViewClear, ViewDestroy, ViewNode,
    ViewNodeRecord, back_main_menu, class_select_place, clear_character_view, close_settings_menu,
    confirm_class, confirm_main_menu, destroy_character_view, detach_inventory,
    dispatch_end_session, draw_skill_selector, draw_skill_tip, draw_transition_fade,
    first_settled_view_node, forward_main_menu, hit_class_cards, inventory_key_close,
    inventory_page, main_menu_key, menu_clock_scale, menu_key_close, menu_row_scale,
    navigate_class, open_settings_menu, reset_settings_menu, resume_from_menu, select_main_menu,
    select_menu_class, settle_skill_selection, show_char_loot, show_journal, show_skill_selector,
    show_skill_tip, skill_selector_key, splash_bounds, toggle_menu_music, type_menu_name,
    update_menu_hover,
};
pub use pickers::{
    AttachedControl, FieldQuad, FieldVertex, HAIR_MARKER_SCALE, HAIR_TABLE, HairColPicker,
    PICKER_NOTICE, PickPoint, TonePicker, ToneTarget, pick_point,
};
pub use scale::GuiScale;
pub use selection::{
    Click as PortalClick, DestinationScreen, DestinationSlot, Frame as PortalFrame, Journey,
    JourneyGlow, SelectionDraw, SymbolQuad, TravelStep, bump_flash, crystal_charge, fill_rows,
    flash_amount, flash_fade, journey, node_mix, place_node, symbol_quad, travel_step, wobble_step,
};
pub use skill::{
    ExperienceRates, ForgetDialogPlace, GROUP_SLOTS, NODE_LEARNT, NODE_LOCKED, NODE_OPEN,
    NODE_PRESENT, NODES_PER_GROUP, PAGE_GROUPS, PAGE_SLOTS, PREVIEW_SKILL, REQUIREMENT_ABSENT,
    REQUIREMENT_CHOSEN, REQUIREMENT_MET, REQUIREMENT_MISSING, RING_GROUPS, SKILL_GROUP_NAMES,
    SkillBar, SkillBounds, SkillClose, SkillFades, SkillGroupIcon, SkillHit, SkillHover,
    SkillHoverInput, SkillIconQuad, SkillInputInput, SkillInputOutput, SkillKeyInput,
    SkillKeyOutput, SkillLearningRow, SkillLeave, SkillLink, SkillLinkInput, SkillNodeTip,
    SkillOpenPlan, SkillOrbitBar, SkillPointInput, SkillPopupFade, SkillRing, SkillRowIcon,
    SkillScreen, SkillShow, SkillTipBox, clamp_tooltip, close_skill_screen, forget_dialog_place,
    handle_skill_input, hit_skill_point, learning_row, leave_skill_ui, open_skill_screen,
    pentagon_offset, pick_skill_satellite, ring_group_tooltip, ring_node_title, ring_node_tooltip,
    selection_tip_slide, show_skill_ui, skill_group_icons, skill_hover, skill_icon_quad,
    skill_node_hit, skill_node_spokes, skill_node_state, skill_orbit_bars, skill_popup_fade,
    skill_position, skill_requirements, skill_ring_position, skill_satellite, skill_screen_fades,
    skill_screen_key, spend_skill_experience, technique_name_for, unlearn_skill_level,
};
pub use skill_draw::{
    SkillBatch, SkillDraw, SkillDrawInput, SkillLabel, SkillNodePlace, SkillProgram, SkillTexture,
    SkillTip, SkillVertex, atlas_quad as skill_atlas_quad, node_places as skill_node_places,
    skill_draw,
};
pub use skill_gui::{
    BAR_SHADER, COLOUR, FIRST_ICON, ICON_CELL, ICON_FORMAT, ICON_SETS, ICON_UNITS, M_PRJ,
    M_PRJ_VALUE, PROG, RING_SHADER, SINGLE_CELL, SINGLE_FORMAT, SINGLE_ICON_ATLAS,
    SINGLE_ICON_UNITS, SINGLE_ICONS, SKILL_GUI, SKILL_GUI_FLAGS, SKILL_GUI_UNITS, SKILL_ICON_ATLAS,
    SKILL_SHOW_FLAGS, SKILL_SHOW_GAIN, SKILL_SHOW_PITCH, SKILL_SHOW_SOUND, SkillIconHandles,
    SkillIcons, SkillPrograms, SkillResources, SkillShaderProgram, TEX, build_group_icons,
    build_single_icons, group_cells, icon_sharpened, skill_programs,
};
pub use skin::{
    BEVEL_INK, BackdropCorner, BaseSkin, BevelArm, BevelBox, BevelInk, BevelPlace, CappedSpan,
    CentredIcon, CloseEdge, ControlLabelDraw, ControlLabelPlace, ControlPlace, ControlScrollPlace,
    ControlSliderPlace, DispatchPlace, DrawSkin, FORM_ALT, FORM_CAP, FORM_CLOSE_INK, FORM_CORNER,
    FORM_EDGE, FRAME_CORNER, FRAME_EDGE, FadedLabel, FlatSlider, FormBackground,
    FormBackgroundPlace, FormChrome, FormChromePlace, FormDispatch, FormPage, FormPagePlace,
    FormRectPlace, FormScroll, FormTitle, FrameDraw, FramePlace, GUI_TEXTURE_FLAGS, GUISKIN_FILE,
    GUISKIN_TEXTURE_FLAGS, GUISKIN_UNITS, GlowText, GuiDraw, GuiPrimitive, GuiRect, GuiVertex,
    HELD_CAP, HELD_END, HELD_FLIP, HELD_OVER, HELD_POST, HELD_RIGHT, HELD_TAB, HitPlace, IconPlace,
    InkLine, InkQuad, LINE_CHIP, LINE_CHIP_ALT, LINE_HEIGHTS, LINE_RAIL, LabelMetrics, LabelPlace,
    LabelVariant, LineChip, LineLabel, LineOpen, LineRail, LineScale, LineShade, LineShadeInks,
    LineShadePlace, LineShadeQuad, LineShadeRect, MarkIcon, RailPlace, SCROLL_BAR, SCROLL_EDGE,
    SKIN_INK, SKIN_ROUNDED_SHADER, SKIN_SOLID_SHADER, SKIN_TEXTURED_SHADER, SLIDER_CAP,
    SLIDER_THUMB, SLIDER_TRACK, SPAN_BAR, SPAN_CAP, SPAN_END, SPAN_SQUARE, SPAN_VIEW_UNSET,
    ScrollBar, ShadePlace, SkinAlpha, SkinBatch, SkinControlSlider, SkinCross, SkinDetail,
    SkinDetailPlace, SkinGlobals, SkinGroup, SkinHitPlace, SkinHooks, SkinImageDraw, SkinImageInks,
    SkinImagePlace, SkinInk, SkinInsetArm, SkinInsetBevel, SkinInsetBevelPlace, SkinInsetPlain,
    SkinLabel, SkinLayout, SkinMetrics, SkinMode, SkinPalette, SkinPrepared, SkinProgram,
    SkinPrograms, SkinRect, SkinRectPlace, SkinRegion, SkinRegionPlace, SkinScheme, SkinScrollBar,
    SkinShade, SkinSlider, SkinSliderMode, SkinSliderPlace, SkinStrip, SkinSwitch, SkinTextureBind,
    SkinUniform, SkinUniformValue, SliderDraw, SliderPlace, SliderQuad, SpanBar, SpanSquare,
    SpanView, TEXTURED_SKIN_FRAGMENT, TEXTURED_SKIN_MODE, TEXTURED_SKIN_TEXTURE_FLAGS,
    TEXTURED_SKIN_VERTEX, TexturedFrame, TexturedFrameLayer, TexturedFrameState, TexturedSkinSetup,
    TitleDraw, WIDE_BAR, WIDE_CAP, bevel_rect, bevel_ring, bordered_rect, control_rect,
    dispatch_form, double_frame, draw_accent_bar, draw_bar_with_boxes, draw_bevel, draw_button_box,
    draw_button_face, draw_capped_span, draw_centred_icon, draw_control_box, draw_control_fill,
    draw_control_label, draw_control_scrollbar, draw_control_shell, draw_control_slider,
    draw_flat_slider, draw_form_background, draw_form_chrome, draw_form_page, draw_frame,
    draw_frame_control, draw_gradient_button_face, draw_inset_bevel, draw_line_chip,
    draw_line_rail, draw_line_shade, draw_mark, draw_mark_hot, draw_mark_icon, draw_mark_pressed,
    draw_panel, draw_skin_bevel, draw_skin_cross, draw_skin_detail, draw_skin_image,
    draw_skin_rect, draw_skin_region, draw_skin_scroll, draw_skin_shade, draw_skin_slider,
    draw_skin_strip, draw_span_bar, draw_span_square, draw_style_frames, edge_frame, expand_group,
    fade_label, fade_rect, faded_rect, form_rect, four_ink_frame, four_stop_rect, glow_text,
    gradient_box, gradient_frame, guiskin_size, hit_line, hit_skin, load_scheme, mark_active,
    open_line, open_skin, outline_rect, pair_rect, place_line_label, place_skin_label,
    prepare_skin_programs, prepare_skin_renderer, rounded_fill, rounded_fill_fixed, rounded_grow,
    rounded_scale, rounded_textured_frame, scale_line, scale_palette_alpha, scale_skin_alpha,
    scroll_arrow_horizontal, scroll_arrow_vertical, selection_band, set_skin_metrics, shadow_ring,
    skin_constructor, skin_measures, solid_rect, stripe_bar, sunken_ring, textured_skin_draw,
    two_ink_band, two_ink_outline, vertical_gradient, vertical_gradient_frame, with_alpha,
};
pub use technique_selector::{
    CONFIRM_TEXT, FORGET_WORDS, LEVEL_XP, NodeState, SelectorDraw, SelectorKey, SkillColumn,
    SkillRecord, TechniqueChange, TechniqueNode, TechniqueSelector, TechniqueText,
    TechniqueTraining, TechniqueTree, assign_technique, clamp_index, close_pick,
    draw_order as technique_draw_order, enter_forget, layout as technique_layout, refresh_tree,
    selector_key, text_places,
};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            scale::plugin,
            gui_level::plugin,
            fps::plugin,
            loading::plugin,
            cursor::plugin,
            hud::plugin,
            heartbeat::plugin,
            fade::plugin,
            gui_draw::GuiDrawPlugin,
        ));
    }
}
