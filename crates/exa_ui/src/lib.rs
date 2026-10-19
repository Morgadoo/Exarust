//! User interface: main menu, character creation, inventory, arena management, HUD, manual pages, tooltips, cursor.
//!
//! Planned in Phase 4–7; see `README.md` next to this crate and `docs/PLAN.md`.
//! So far: the game's bitmap font ([`font`]), the interface scale
//! ([`scale`]), the frame-rate readout ([`fps`]), the loading screen
//! ([`loading`]), the in-game world cursor ([`cursor`]), the health and
//! focus bars ([`hud`]), the low-health heartbeat's border
//! ([`heartbeat`]) and the fade to black of a fall to death ([`fade`]),
//! painted with gamma-space blending ([`blend`]).

pub mod blend;
pub mod cursor;
pub mod fade;
pub mod font;
pub mod fps;
pub mod heartbeat;
pub mod hud;
pub mod loading;
pub mod menu;
pub mod scale;
pub mod skin;

use bevy::prelude::*;

pub use cursor::{CursorShape, DrawWorldCursor, ScriptedPointer, WorldCursor};
pub use fade::DeathFade;
pub use fps::ShowFps;
pub use heartbeat::Heartbeat;
pub use hud::{DrawHud, HudBars};
pub use loading::LoadingStatus;
pub use menu::{MenuKeyClose, menu_clock_scale, menu_key_close};
pub use scale::GuiScale;
pub use skin::{
    BEVEL_INK, BackdropCorner, BevelArm, BevelBox, BevelInk, BevelPlace, CappedSpan, CentredIcon,
    CloseEdge, ControlPlace, DispatchPlace, DrawSkin, FlatSlider, FormDispatch, FrameDraw,
    FramePlace, HitPlace, IconPlace, InkLine, InkQuad, LINE_CHIP, LINE_CHIP_ALT, LINE_HEIGHTS,
    LINE_RAIL, LabelMetrics, LabelPlace, LineChip, LineLabel, LineOpen, LineRail, LineScale,
    LineShade, LineShadeInks, LineShadePlace, LineShadeQuad, LineShadeRect, RailPlace,
    SKIN_ROUNDED_SHADER, SKIN_SOLID_SHADER, SKIN_TEXTURED_SHADER, SPAN_BAR, SPAN_CAP, SPAN_END,
    SPAN_SQUARE, SPAN_VIEW_UNSET, ScrollBar, ShadePlace, SkinAlpha, SkinCross, SkinDetail,
    SkinDetailPlace, SkinFrameDraw, SkinFramePlace, SkinHitPlace, SkinImageDraw, SkinImageInks,
    SkinImagePlace, SkinInsetArm, SkinInsetBevel, SkinInsetBevelPlace, SkinInsetPlain, SkinLabel,
    SkinMetrics, SkinPrepared, SkinProgram, SkinPrograms, SkinRect, SkinRectPlace, SkinRegion,
    SkinRegionPlace, SkinShade, SkinSlider, SkinSliderMode, SkinSliderPlace, SkinStrip,
    SkinUniform, SkinUniformValue, SliderDraw, SliderPlace, SliderQuad, SpanBar, SpanSquare,
    SpanView, TitleDraw, control_rect, dispatch_form, draw_bevel, draw_capped_span,
    draw_centred_icon, draw_flat_slider, draw_frame, draw_inset_bevel, draw_line_chip,
    draw_line_rail, draw_line_shade, draw_skin_bevel, draw_skin_cross, draw_skin_detail,
    draw_skin_image, draw_skin_rect, draw_skin_region, draw_skin_scroll, draw_skin_shade,
    draw_skin_slider, draw_skin_strip, draw_span_bar, draw_span_square, hit_line, hit_skin,
    open_line, open_skin, place_line_label, place_skin_label, prepare_skin_programs,
    prepare_skin_renderer, scale_line, scale_skin_alpha, skin_frames, skin_measures,
};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            scale::plugin,
            fps::plugin,
            loading::plugin,
            cursor::plugin,
            hud::plugin,
            heartbeat::plugin,
            fade::plugin,
        ));
    }
}
