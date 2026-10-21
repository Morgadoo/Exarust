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
//! background, page order and labels. The arena match board's tabs, match
//! and event tables, tournament and season rows, fighter assignment, layout
//! and drawing order live in [`match_board`].

pub mod blend;
pub mod compact_skill;
pub mod controls;
pub mod creation;
pub mod cursor;
pub mod div_card;
pub mod emblem_form;
pub mod engine_math;
pub mod fade;
pub mod font;
pub mod form;
pub mod fps;
pub mod gui_controls;
pub mod gui_core;
pub mod gui_font;
pub mod gui_glyphs;
pub mod gui_level;
pub mod gui_rescale;
pub mod gui_text;
pub mod heartbeat;
pub mod hud;
pub mod icon_atlas;
pub mod item_view;
pub mod loading;
pub mod loadout;
pub mod match_board;
pub mod menu;
pub mod scale;
pub mod selection;
pub mod settings;
pub mod skill;
pub mod skin;
pub mod table_view;
pub mod technique_selector;
pub mod text_box;
pub mod text_edit;
pub mod text_layout;

use bevy::prelude::*;

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
    CARDS, CardDraw, CardVertex, Reading, SHEET, SOUND, ShadowVertex, back_quad, card_place,
    card_size, clock_step, deal_cards, draw_order as div_draw_order, ease, face_quad, fade_alpha,
    mouse as div_mouse, shadow_mesh, shadow_outline, turn, turn_share,
};
pub use emblem_form::{
    BACKGROUND_MAX, BACKGROUND_STEM, COLOUR_THREE_TABLE, COLOUR_UNIFORMS, DESIGN_MAX, DESIGN_STEM,
    EMBLEM_SHADER, EmblemDraw, EmblemEditor, EmblemRandom, EmblemRecord, MIRROR_UNIFORM,
    PictureSelection, draw_order as emblem_draw_order, picture, preview_quad, select_background,
    select_design,
};
pub use fade::DeathFade;
pub use fps::ShowFps;
pub use gui_controls::{
    ClassRegistry, LabelDraw, LabelInk, append_records, check_tick_shown, label_draw,
    layout_records, register_class, register_default_classes,
};
pub use gui_core::{
    CursorState, DragState, FormHost, FormId, GuiMetrics, GuiRect, GuiTask, HitResult,
    MESSAGE_FRAME, MESSAGE_HIGH, MESSAGE_MODAL, MESSAGE_NO_CLOSE, MessageBoxPlan, PlaceRequest,
    PlainForm, PointerState, TOOLTIP_DELAY, Tooltip, anchored, drag_threshold, edge_rect,
    message_box_plan, place_form, set_modal,
};
pub use gui_level::GuiLevelState;
pub use gui_text::{
    ELLIPSIS, FontBinding, MENU_FONT, MenuFont, TextQuad, WrappedDraw, add_gap, count_lines,
    draw_line, draw_line_centred, draw_line_plain, draw_span, draw_tail, draw_truncated,
    draw_wrapped, draw_wrapped_at, ellipsis_fit, fit_menu_width, fit_width, font_uv, glyph_uv,
    measure, measure_in, menu_font_scale, needs_rescale,
};
pub use heartbeat::Heartbeat;
pub use hud::{DrawHud, HudBars};
pub use loading::LoadingStatus;
pub use loadout::{
    DropResult, DropSource, EquipFacts, EquipKind, FORM_HEIGHT, FORM_OPACITY, FORM_WIDTH,
    HeldWeapon, LoadoutEditor, LoadoutItem, LoadoutLayout, LoadoutMode, LoadoutSave, OFFHAND_FLAG,
    Slot, apply_drop, can_equip, copies, encumbrance, item_cost, layout as loadout_layout,
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
    CharLootShow, CharLootShowInput, ClassCardHit, ClassCardHitInput, ClassConfirm, ClassNav,
    ClassNavAction, ClassNavInput, ClassSelect, EndSessionAction, EndSessionDispatch,
    EndSessionInput, InventoryDetach, InventoryDetachInput, InventoryKeyClose, InventoryPage,
    JournalShow, MENU_NAME_CODE, MainMenuKey, MainMenuKeyAction, MainMenuKeyInput, MenuConfirm,
    MenuForward, MenuHover, MenuKeyClose, MenuName, MenuSelect, SettingsOpen, SkillKey,
    SkillSelection, SkillSelectorApplied, SkillSelectorDraw, SkillSelectorDrawPlace,
    SkillSelectorMode, SkillSelectorShow, SkillTipDraw, SkillTipPlace, SkillTipShow,
    SkillTipShowInput, SplashBounds, SplashDestroy, SplashDestroyInput, TRANSITION_TEXT,
    TransitionFade, TransitionFadePlace, ViewClear, ViewDestroy, ViewNode, ViewNodeRecord,
    clear_character_view, close_settings_menu, confirm_class, confirm_main_menu,
    destroy_character_view, detach_inventory, dispatch_end_session, draw_skill_selector,
    draw_skill_tip, draw_transition_fade, first_settled_view_node, forward_main_menu,
    hit_class_cards, inventory_key_close, inventory_page, main_menu_key, menu_clock_scale,
    menu_key_close, navigate_class, open_settings_menu, reset_settings_menu, select_main_menu,
    select_menu_class, settle_skill_selection, show_char_loot, show_journal, show_skill_selector,
    show_skill_tip, skill_selector_key, splash_bounds, type_menu_name, update_menu_hover,
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
    REQUIREMENT_CHOSEN, REQUIREMENT_MET, REQUIREMENT_MISSING, RING_GROUPS, SkillBar, SkillBounds,
    SkillGroupIcon, SkillHit, SkillHover, SkillHoverInput, SkillIconQuad, SkillInputInput,
    SkillInputOutput, SkillKeyInput, SkillKeyOutput, SkillLearningRow, SkillLeave, SkillLink,
    SkillLinkInput, SkillNodeTip, SkillOrbitBar, SkillPointInput, SkillPopupFade, SkillRing,
    SkillRowIcon, SkillScreen, SkillShow, SkillTipBox, clamp_tooltip, forget_dialog_place,
    handle_skill_input, hit_skill_point, learning_row, leave_skill_ui, pentagon_offset,
    pick_skill_satellite, ring_group_tooltip, ring_node_title, ring_node_tooltip,
    selection_tip_slide, show_skill_ui, skill_group_icons, skill_hover, skill_icon_quad,
    skill_node_hit, skill_node_spokes, skill_node_state, skill_orbit_bars, skill_popup_fade,
    skill_position, skill_requirements, skill_ring_position, skill_satellite, skill_screen_key,
    spend_skill_experience, unlearn_skill_level,
};
pub use skin::{
    BEVEL_INK, BackdropCorner, BevelArm, BevelBox, BevelInk, BevelPlace, CappedSpan, CentredIcon,
    CloseEdge, ControlLabelDraw, ControlLabelPlace, ControlPlace, ControlScrollPlace,
    ControlSliderPlace, DispatchPlace, DrawSkin, FORM_ALT, FORM_CAP, FORM_CLOSE_INK, FORM_CORNER,
    FORM_EDGE, FRAME_CORNER, FRAME_EDGE, FadedLabel, FlatSlider, FormBackground,
    FormBackgroundPlace, FormChrome, FormChromePlace, FormDispatch, FormPage, FormPagePlace,
    FormRectPlace, FormScroll, FormTitle, FrameDraw, FramePlace, GlowText, GuiDraw, GuiPrimitive,
    GuiRect, GuiVertex, HELD_CAP, HELD_END, HELD_FLIP, HELD_OVER, HELD_POST, HELD_RIGHT, HELD_TAB,
    HitPlace, IconPlace, InkLine, InkQuad, LINE_CHIP, LINE_CHIP_ALT, LINE_HEIGHTS, LINE_RAIL,
    LabelMetrics, LabelPlace, LabelVariant, LineChip, LineLabel, LineOpen, LineRail, LineScale,
    LineShade, LineShadeInks, LineShadePlace, LineShadeQuad, LineShadeRect, MarkIcon, RailPlace,
    SCROLL_BAR, SCROLL_EDGE, SKIN_INK, SKIN_ROUNDED_SHADER, SKIN_SOLID_SHADER,
    SKIN_TEXTURED_SHADER, SLIDER_CAP, SLIDER_THUMB, SLIDER_TRACK, SPAN_BAR, SPAN_CAP, SPAN_END,
    SPAN_SQUARE, SPAN_VIEW_UNSET, ScrollBar, ShadePlace, SkinAlpha, SkinBatch, SkinControlSlider,
    SkinCross, SkinDetail, SkinDetailPlace, SkinGroup, SkinHitPlace, SkinImageDraw, SkinImageInks,
    SkinImagePlace, SkinInk, SkinInsetArm, SkinInsetBevel, SkinInsetBevelPlace, SkinInsetPlain,
    SkinLabel, SkinLayout, SkinMetrics, SkinMode, SkinPalette, SkinPrepared, SkinProgram,
    SkinPrograms, SkinRect, SkinRectPlace, SkinRegion, SkinRegionPlace, SkinScheme, SkinScrollBar,
    SkinShade, SkinSlider, SkinSliderMode, SkinSliderPlace, SkinStrip, SkinUniform,
    SkinUniformValue, SliderDraw, SliderPlace, SliderQuad, SpanBar, SpanSquare, SpanView,
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
    gradient_box, gradient_frame, hit_line, hit_skin, load_scheme, mark_active, open_line,
    open_skin, outline_rect, pair_rect, place_line_label, place_skin_label, prepare_skin_programs,
    prepare_skin_renderer, rounded_fill, rounded_fill_fixed, rounded_grow, rounded_scale,
    rounded_textured_frame, scale_line, scale_palette_alpha, scale_skin_alpha,
    scroll_arrow_horizontal, scroll_arrow_vertical, selection_band, set_skin_metrics, shadow_ring,
    skin_constructor, skin_measures, solid_rect, stripe_bar, sunken_ring, two_ink_band,
    two_ink_outline, vertical_gradient, vertical_gradient_frame, with_alpha,
};
pub use technique_selector::{
    CONFIRM_TEXT, FORGET_WORDS, LEVEL_XP, NodeState, SelectorDraw, SelectorKey, SkillColumn,
    SkillRecord, TechniqueChange, TechniqueNode, TechniqueSelector, TechniqueText,
    TechniqueTraining, TechniqueTree, assign_technique, clamp_index, close_pick,
    draw_order as technique_draw_order, enter_forget, layout as technique_layout, refresh_tree,
    selector_key, text_places,
};
pub use text_edit::{
    CARET, EditAction, EditDraw, EditView, MarkupPicture, MarkupState, SPAN, TextureCache,
    apply_key, apply_markup_state, apply_markup_tag, backspace, caret_from_x, caret_position,
    delete_forward, delete_selection, draw_edit, key, navigate_key, paste, picture_rect,
    scroll_view, selected_text, selection,
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
        ));
    }
}
