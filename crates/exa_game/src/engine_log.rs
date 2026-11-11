//! The engine's message hook, live: the original's logger as every game
//! layer reaches it.
//!
//! The original routes each message through one hook its main unit installs
//! at start-up. The hook appends `title: text` and a line break to the
//! session's log buffer, rewrites the whole log file from that buffer
//! (except for a level-5 message, and while start-up writes its system
//! information block), and then presents the message by its level
//! ([`LogPresentation`]): levels 1 to 3 go to the status history, 4 and 5 to
//! the timed overlay in the window's top-left corner, 6 and 7 to a modal
//! message box. The overlay is drawn every frame after the interface, while
//! any line waits, and its oldest line goes once its time is up.
//!
//! The log file is `<user folder>/<project>.log`, the folder the settings
//! files live in; the binary names it at start-up ([`set_log_file`]).
//! Messages also go to Exarust's own log (standard error), where the
//! original kept only its buffer. The status history is kept but, as in the
//! original, nothing draws it.
//!
//! Rules: `docs/PERFORMANCE.md` (Frame pacing, The message hook).

use std::path::PathBuf;
use std::sync::Mutex;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_ui::gui_draw::{GuiFrame, GuiFrames, GuiLayer, GuiTextures, LayerTexture, text_batch};
use exa_ui::skin::{GuiRect, SkinBatch, rounded_fill_fixed};

use crate::frame::{
    LogOverlay, LogPresentation, MessageBox, OVERLAY_PANEL_ALPHA, OVERLAY_PANEL_STYLE,
    OVERLAY_PANEL_UNITS, OVERLAY_TEXT_MAX_HEIGHT, StatusHistory, overlay_layout,
};

/// The line break the log buffer puts after every message.
pub const LOG_LINE_BREAK: &str = "\r\n";

/// The separator between a message's title and its text in the log.
pub const LOG_TITLE_SEPARATOR: &str = ": ";

/// Who shows a level-6 or level-7 message box; the binary installs one.
pub type MessageBoxHandler = fn(&MessageBox);

/// The hook's state: the log buffer and file, the overlay, the status
/// history and the clock the overlay is timed by.
#[derive(Default)]
pub struct EngineLog {
    /// Every message of the session, each followed by the line break.
    buffer: String,
    /// The log file; none until the binary names it.
    file: Option<PathBuf>,
    /// Set while the file is not to be written.
    file_held: bool,
    overlay: LogOverlay,
    status: StatusHistory,
    /// The game clock in milliseconds, as the frame tick last set it.
    clock_ms: u32,
    message_box: Option<MessageBoxHandler>,
}

impl EngineLog {
    /// Takes one message: buffers it, rewrites the log file and presents it
    /// (a message box is shown by [`log`], outside the lock).
    fn take(&mut self, level: i32, title: &str, text: &str) {
        self.buffer.push_str(title);
        self.buffer.push_str(LOG_TITLE_SEPARATOR);
        self.buffer.push_str(text);
        self.buffer.push_str(LOG_LINE_BREAK);
        if !self.file_held
            && LogPresentation::writes_file(level)
            && let Some(path) = &self.file
        {
            // The original recreates the file and writes the whole buffer;
            // a failed write is ignored.
            let _ = std::fs::write(path, self.buffer.as_bytes());
        }
        match LogPresentation::for_level(level) {
            LogPresentation::StatusLine => {
                if let Some(kind) = LogPresentation::status_kind(level) {
                    self.status.push(kind, text);
                }
            }
            LogPresentation::Overlay => {
                self.overlay.push(title, text, self.clock_ms);
            }
            LogPresentation::MessageBox | LogPresentation::FileOnly => {}
        }
    }
}

static ENGINE_LOG: Mutex<Option<EngineLog>> = Mutex::new(None);

fn with_log<R>(f: impl FnOnce(&mut EngineLog) -> R) -> R {
    let mut guard = ENGINE_LOG.lock().unwrap_or_else(|e| e.into_inner());
    f(guard.get_or_insert_with(EngineLog::default))
}

/// The engine's message hook: logs `title: text` at `level` and presents it
/// as the original's logger does. Also writes the message to Exarust's own
/// log (an error at level 6, a warning at 4, 5 and 7, information else).
pub fn log(level: i32, title: &str, text: &str) {
    match level {
        6 => error!("{title}{LOG_TITLE_SEPARATOR}{text}"),
        4 | 5 | 7 => warn!("{title}{LOG_TITLE_SEPARATOR}{text}"),
        _ => info!("{title}{LOG_TITLE_SEPARATOR}{text}"),
    }
    // The message box waits for the player; it is shown outside the lock.
    let show = with_log(|log| {
        log.take(level, title, text);
        log.message_box
    });
    if let (Some(show), Some(message)) = (show, MessageBox::for_level(level, title, text)) {
        show(&message);
    }
}

/// Names the log file (`None` stops writing it). The file is rewritten
/// from the session's buffer at the next message.
pub fn set_log_file(path: Option<PathBuf>) {
    with_log(|log| log.file = path);
}

/// The log file for a user folder and project: `<folder>/<project>.log`.
pub fn log_file_path(folder: &std::path::Path, project: &str) -> PathBuf {
    folder.join(format!("{project}.log"))
}

/// Holds or releases the file write, as the original does while start-up
/// logs its system information block (those lines reach the file with the
/// next message written).
pub fn hold_file(held: bool) {
    with_log(|log| log.file_held = held);
}

/// Installs who shows the level-6 and level-7 message boxes.
pub fn set_message_box_handler(handler: MessageBoxHandler) {
    with_log(|log| log.message_box = Some(handler));
}

/// The status history's entries, newest first.
pub fn status_entries() -> Vec<crate::frame::StatusEntry> {
    with_log(|log| log.status.entries().to_vec())
}

/// The overlay's live lines, oldest first.
pub fn overlay_lines() -> Vec<String> {
    with_log(|log| log.overlay.lines().map(|l| l.text.clone()).collect())
}

/// Sets the hook's game clock and drops the overlay's oldest line once its
/// time is up (one a tick), as the frame tick does before it draws.
pub fn tick(now_ms: u32) {
    with_log(|log| {
        log.clock_ms = now_ms;
        log.overlay.expire(now_ms);
    });
}

/// Runs the hook's clock and draws the overlay.
pub struct EngineLogPlugin;

impl Plugin for EngineLogPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(First, tick_engine_log).add_systems(
            PostUpdate,
            draw_overlay.before(exa_ui::gui_draw::render_frames),
        );
        // The probe is captured by its system, not kept as a resource, so a
        // probed run spawns the same entities as any other.
        if let Some(probe) = LogProbe::from_env() {
            app.add_systems(Update, move |frames: Res<bevy::diagnostic::FrameCount>| {
                if frames.0 == probe.frame {
                    log(probe.level, &probe.title, &probe.text);
                }
            });
        }
    }
}

/// A debug probe: `EXA_LOG_PROBE=<level>:<title>:<text>` logs that message
/// through the hook on frame `EXA_LOG_PROBE_FRAME` (default 30), so the
/// overlay, the status history, the log file and the boxes can be seen live.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogProbe {
    pub level: i32,
    pub title: String,
    pub text: String,
    pub frame: u32,
}

impl LogProbe {
    /// Parses `<level>:<title>:<text>`; the text may hold colons.
    pub fn parse(spec: &str, frame: u32) -> Option<Self> {
        let mut parts = spec.splitn(3, ':');
        let level = parts.next()?.trim().parse().ok()?;
        let title = parts.next()?.to_string();
        let text = parts.next()?.to_string();
        Some(Self {
            level,
            title,
            text,
            frame,
        })
    }

    fn from_env() -> Option<Self> {
        let spec = std::env::var("EXA_LOG_PROBE").ok()?;
        let frame = std::env::var("EXA_LOG_PROBE_FRAME")
            .ok()
            .and_then(|f| f.parse().ok())
            .unwrap_or(30);
        Self::parse(&spec, frame)
    }
}

/// The game clock is the time the app has run (the original's real clock
/// less the time its frame tick stood still; Exarust's window keeps
/// updating while it is in the background).
fn tick_engine_log(time: Res<Time<Real>>) {
    tick(time.elapsed().as_millis() as u32);
}

/// Draws the timed overlay over the interface: a soft dark panel (fill
/// style 3, alpha 0x80) one GUI unit in from the window's corner, eighty
/// units wide, and each line wrapped at that width two units in, in the
/// skin's eighth label ink. Nothing is drawn while no line waits.
fn draw_overlay(
    textures: Option<Res<GuiTextures>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut frames: ResMut<GuiFrames>,
) {
    let lines = overlay_lines();
    if lines.is_empty() {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let unit = (exa_ui::scale::gui_scale(window.physical_height(), 0) as i32).max(1);
    let empty = GuiTextures::default();
    let textures = textures.as_deref().unwrap_or(&empty);
    let level = exa_ui::gui_font::level_for_unit(unit);
    let (font, atlas) =
        crate::gui_screens::screen_font(exa_ui::gui_font::gui_level(level).main, textures);
    let Some(atlas) = atlas else {
        return;
    };
    let width = OVERLAY_PANEL_UNITS * unit;
    let wrapped: Vec<i32> = lines
        .iter()
        .map(|l| exa_ui::gui_text::count_lines(&font, l.as_bytes(), width))
        .collect();
    let layout = overlay_layout(unit, font.line_height, &wrapped);
    let ((px, py), (pw, ph)) = layout.panel;
    let mut frame = GuiFrame::new();
    let mut panel = SkinBatch::new();
    rounded_fill_fixed(
        &mut panel,
        GuiRect::sized(px as f32, py as f32, pw as f32, ph as f32),
        unit,
        OVERLAY_PANEL_STYLE,
        OVERLAY_PANEL_ALPHA,
    );
    frame.push(GuiLayer::new(panel, LayerTexture::None));
    let ink = exa_ui::line_skin::shown_palette(1.0).text[OVERLAY_TEXT_INK];
    let mut quads = Vec::new();
    for (text, &((x, y), _)) in lines.iter().zip(&layout.lines) {
        let drawn = exa_ui::gui_text::draw_wrapped(
            &font,
            text.as_bytes(),
            x,
            y,
            width,
            OVERLAY_TEXT_MAX_HEIGHT,
            0,
            false,
        );
        quads.extend(drawn.quads);
    }
    if !quads.is_empty() {
        frame.push(GuiLayer::new(
            text_batch(&font, &quads, ink, atlas),
            LayerTexture::Named(font.texture),
        ));
    }
    frames.push(frame);
}

/// The label ink the overlay's text takes: the skin's eighth.
pub const OVERLAY_TEXT_INK: usize = 7;

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> EngineLog {
        EngineLog::default()
    }

    #[test]
    fn every_message_goes_to_the_buffer_with_a_line_break() {
        let mut log = fresh();
        log.take(8, "Version", "V 0.9.5");
        log.take(5, "Windows Error", "Access is denied.");
        assert_eq!(
            log.buffer,
            "Version: V 0.9.5\r\nWindows Error: Access is denied.\r\n"
        );
    }

    #[test]
    fn the_file_is_rewritten_whole_except_for_level_five_and_while_held() {
        let dir = std::env::temp_dir().join(format!("exarust-engine-log-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = log_file_path(&dir, "Exanima");
        let _ = std::fs::remove_file(&path);
        let mut log = fresh();
        log.file = Some(path.clone());
        log.file_held = true;
        log.take(8, "Version", "V");
        assert!(!path.exists());
        log.file_held = false;
        log.take(5, "Windows Error", "denied");
        assert!(!path.exists());
        log.take(4, "Script", "bad role");
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "Version: V\r\nWindows Error: denied\r\nScript: bad role\r\n"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn levels_present_as_the_original_does() {
        let mut log = fresh();
        log.clock_ms = 1000;
        log.take(1, "a", "one");
        log.take(3, "b", "three");
        log.take(4, "Script", "bad");
        log.take(9, "c", "file only");
        assert_eq!(log.status.entries()[0].text, "three");
        assert_eq!(log.status.entries()[0].kind, 1);
        assert_eq!(log.status.entries()[1].kind, 0);
        let lines: Vec<_> = log.overlay.lines().collect();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "Script : bad");
        // Two seconds plus fifty milliseconds a byte.
        assert_eq!(lines[0].expires_ms, 1000 + 2000 + 12 * 50);
    }

    #[test]
    fn the_probe_reads_level_title_and_text() {
        assert_eq!(
            LogProbe::parse("4:Script:bad role: x", 30),
            Some(LogProbe {
                level: 4,
                title: "Script".into(),
                text: "bad role: x".into(),
                frame: 30,
            })
        );
        assert_eq!(LogProbe::parse("x:a:b", 1), None);
    }

    #[test]
    fn the_log_file_sits_beside_the_settings() {
        assert_eq!(
            log_file_path(std::path::Path::new("/u"), "Exanima"),
            std::path::Path::new("/u/Exanima.log")
        );
    }
}
