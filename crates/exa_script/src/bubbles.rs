//! Speech bubbles: what is said, by whom, and for how long.
//!
//! A line shows for 1000 ms + 50 ms per byte; the narrator's lines under
//! 10 s are stretched by (2 − d/10 000), the player's are halved. A
//! speaker who already has a bubble up closes it early and the new one
//! waits behind it, due 375 ms after the old one's end; a character's line
//! with a CR is cut there and the rest (past CRs and LFs) follows as a
//! further waiting bubble. A bubble fades in over 300 ms (smoothstep),
//! holds for its duration and fades out over 1.5 s, four times as fast
//! when closed early. Spoken lines are also heard: each side's AI contact
//! of the other gets a hearing stimulus of len/2 + 20.
//!
//! Evidence: the bubble queue, its per-frame fade and the narrator holds
//! (REA on `Exanima.exe` 0.9.5).

/// Who is speaking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Speaker {
    Narrator,
    /// A character id.
    Character(u32),
}

/// Bubble state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Showing.
    Showing,
    /// Waiting behind an earlier bubble of the same speaker.
    Waiting,
    /// Closing early.
    Closing,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bubble {
    pub speaker: Speaker,
    pub text: Vec<u8>,
    /// Clock (ms) the bubble started showing.
    pub start: u32,
    pub duration: u32,
    pub phase: Phase,
    /// 0..1, from the last [`Bubbles::update`].
    pub alpha: f32,
    /// The wrapped width and height (pixels), as the text measure gives
    /// them; placement and drawing read them.
    pub width: i32,
    pub height: i32,
    /// The wrapped line count (0 until [`Bubbles::layout`] measured it).
    pub lines: i32,
    /// The screen position of the box's top-left corner.
    pub x: i32,
    pub y: i32,
}

/// Fade-in time, ms.
pub const FADE_IN: u32 = 300;
/// Fade-out time, ms.
pub const FADE_OUT: u32 = 1500;
/// Gap before a waiting bubble of the same speaker, ms.
pub const QUEUE_GAP: u32 = 375;
/// A bubble's initial wrap width: 80 times the GUI unit (4).
pub const BUBBLE_WIDTH: i32 = 320;
/// The margin a bubble keeps from the screen's edges, in GUI units.
pub const MARGIN: i32 = 4;
/// A character line is raised this far above the head (cm) before it is
/// projected.
pub const HEAD_LIFT: f32 = 120.0;

/// The viewport a bubble is placed in: its size, the main font's glyph
/// height, and (for character lines) the camera.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    pub width: i32,
    pub height: i32,
    /// The font's glyph height (the quad it draws for one byte).
    pub glyph_height: i32,
    pub camera: Option<Camera>,
}

/// The camera as the original's bubble placement reads it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    /// The rigid inverse view: three basis rows, then the eye position.
    /// A point becomes `(p − eye)` dotted with each row.
    pub inverse_view: [f32; 12],
    /// The projection's x and y scales.
    pub projection: [f32; 2],
}

/// The screen point of a world point, as the original's viewport projects
/// it: camera space first (a rigid inverse view), then a perspective
/// divide. A point at or behind the camera's plane is clamped to the
/// screen's edges; an unordered component counts as positive, as the
/// original's ordered compares do.
pub fn project(camera: &Camera, view: &View, point: [f32; 3]) -> [f32; 2] {
    let m = &camera.inverse_view;
    let d = [point[0] - m[9], point[1] - m[10], point[2] - m[11]];
    // The original sums each row's products left to right (SSE pair, then
    // the third), so keep that order.
    let cam = [
        (d[0] * m[0] + d[1] * m[1]) + d[2] * m[2],
        (d[0] * m[3] + d[1] * m[4]) + d[2] * m[5],
        (d[0] * m[6] + d[1] * m[7]) + d[2] * m[8],
    ];
    let w = view.width as f32;
    let h = view.height as f32;
    if cam[2] >= 0.0 || cam[2].is_nan() {
        // At or behind the plane (an unordered depth included): the edges.
        let x = if cam[0] >= 0.0 { w } else { 0.0 };
        let y = if cam[1] >= 0.0 { h } else { 0.0 };
        return [x, h - y];
    }
    let z = -cam[2];
    let sx = w * 0.5 + (((camera.projection[0] * w) * 0.5) * cam[0]) / z;
    let sy = h * 0.5 + (((camera.projection[1] * h) * 0.5) * cam[1]) / z;
    [sx, h - sy]
}

/// The original's `round` to an integer: nearest, ties to even (CVTSS2SI).
fn round_to_int(value: f32) -> i64 {
    if value.is_finite() {
        value.round_ties_even() as i64
    } else {
        // Out of range: the instruction leaves the integer indefinite.
        i64::MIN
    }
}

/// A hearing stimulus raised by speech: `listener`'s contact of `speaker`
/// gets `amount` (the AI scales it by 0.25).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Heard {
    pub listener: u32,
    pub speaker: u32,
    pub amount: f32,
}

/// The bubbles up, oldest first.
#[derive(Debug, Clone, Default)]
pub struct Bubbles {
    pub list: Vec<Bubble>,
    /// Hearing stimuli raised since last taken.
    pub heard: Vec<Heard>,
}

fn smoothstep(t: f32) -> f32 {
    (3.0 - t * 2.0) * t * t
}

/// How long a line shows.
pub fn duration(speaker: Speaker, len: usize, player: Option<u32>) -> u32 {
    let mut d = (len as u32).wrapping_mul(50).wrapping_add(1000);
    if speaker == Speaker::Narrator && d < 10_000 {
        // Converted with CVTSD2SI in the default rounding mode: nearest,
        // ties to even.
        d = ((2.0 - f64::from(d) * 0.0001) * f64::from(d)).round_ties_even() as u32;
    }
    if player.is_some_and(|p| speaker == Speaker::Character(p)) {
        d >>= 1;
    }
    d
}

impl Bubbles {
    /// Put up `text` for `speaker` (heard by `listener`); returns the clock
    /// time the line is due: now, or after the speaker's previous bubble.
    pub fn say(
        &mut self,
        speaker: Speaker,
        listener: Option<Speaker>,
        text: &[u8],
        now: u32,
        player: Option<u32>,
    ) -> u32 {
        let mut due = now;
        let mut phase = Phase::Showing;
        if let Some(prev) = self.list.iter_mut().find(|b| b.speaker == speaker) {
            prev.phase = Phase::Closing;
            phase = Phase::Waiting;
            due = prev
                .start
                .wrapping_add(prev.duration)
                .wrapping_add(QUEUE_GAP);
        }
        let mut shown = text;
        let mut rest = None;
        if let Speaker::Character(s) = speaker {
            if let Some(Speaker::Character(l)) = listener {
                let amount = (text.len() >> 1) as f32 + 20.0;
                self.heard.push(Heard {
                    listener: l,
                    speaker: s,
                    amount,
                });
                self.heard.push(Heard {
                    listener: s,
                    speaker: l,
                    amount,
                });
            }
            if let Some(cut) = text.iter().position(|&c| c == b'\r') {
                shown = &text[..cut];
                let mut i = cut;
                while i < text.len() && matches!(text[i], b'\r' | b'\n') {
                    i += 1;
                }
                rest = Some(&text[i..]);
            }
        }
        self.list.push(Bubble {
            speaker,
            text: shown.to_vec(),
            start: now,
            duration: duration(speaker, shown.len(), player),
            phase,
            alpha: 0.0,
            width: BUBBLE_WIDTH,
            height: 0,
            lines: 0,
            x: 0,
            y: 0,
        });
        match rest {
            Some(rest) => self.say(speaker, None, rest, now, player),
            None => due,
        }
    }

    /// The last bubble's duration (the dialogue paces itself on it).
    pub fn last_duration(&self) -> u32 {
        self.list.last().map_or(0, |b| b.duration)
    }

    /// Close the narrator's bubble: closing, 1 s left (the panel answered).
    pub fn close_narrator(&mut self, now: u32, left: u32) {
        if let Some(b) = self
            .list
            .iter_mut()
            .find(|b| b.speaker == Speaker::Narrator && b.phase != Phase::Closing)
        {
            b.phase = Phase::Closing;
            b.start = now.wrapping_sub(left);
            b.duration = left;
        }
    }

    /// Keep the narrator's last bubble up for a minute (waiting for the
    /// player's reply).
    pub fn hold_narrator(&mut self) {
        if let Some(b) = self
            .list
            .iter_mut()
            .rev()
            .find(|b| b.speaker == Speaker::Narrator)
        {
            b.duration = 60_000;
        }
    }

    /// Close every narrator bubble not already closing, `left` ms to go.
    pub fn close_all_narrator(&mut self, now: u32, left: u32) {
        for b in &mut self.list {
            if b.speaker == Speaker::Narrator && b.phase != Phase::Closing {
                b.phase = Phase::Closing;
                b.start = now.wrapping_sub(left);
                b.duration = left;
            }
        }
    }

    /// One frame: waiting bubbles start once no earlier slot holds a bubble
    /// of their speaker, alphas follow the fade, finished bubbles are
    /// dropped. The list is compacted in place as in the original, so a
    /// bubble dropped this frame still holds back a later one of its
    /// speaker until the next frame.
    pub fn update(&mut self, now: u32) {
        self.frame(now, None, |_| None);
    }

    /// Measure every bubble whose line count is not known yet: the text is
    /// wrapped at [`BUBBLE_WIDTH`] and the width narrowed to its widest
    /// line; the height is the line count times the font's line height,
    /// less half the GUI unit (2 at unit 4). `fit` is the GUI's width fit
    /// and returns the line count.
    pub fn layout(&mut self, line_height: i32, fit: impl Fn(&[u8], &mut i32) -> i32) {
        for bubble in &mut self.list {
            if bubble.lines != 0 {
                continue;
            }
            let mut width = BUBBLE_WIDTH;
            let lines = fit(&bubble.text, &mut width);
            bubble.width = width;
            bubble.lines = lines;
            bubble.height = lines.wrapping_mul(line_height).wrapping_sub(MARGIN >> 1);
        }
    }

    /// Place one bubble, as the original does after its fade step: a
    /// narrator line is centred 75 % down the screen, a character line is
    /// its raised head position projected and centred on its width; both
    /// are kept [`MARGIN`] inside the screen. (Free-standing so a frame's
    /// list borrow does not conflict.)
    pub fn place_bubble(bubble: &mut Bubble, view: &View, head: Option<[f32; 3]>) {
        if bubble.speaker == Speaker::Narrator {
            bubble.x = (view.width - bubble.width) / 2;
            bubble.y =
                view.glyph_height + round_to_int(view.height as f32 * 0.75) as i32 - bubble.height;
        } else if let Some(head) = head {
            let lifted = [head[0], head[1] + HEAD_LIFT, head[2]];
            if let Some(camera) = view.camera {
                let projected = project(&camera, view, lifted);
                bubble.x = round_to_int(projected[0] - (bubble.width / 2) as f32) as i32;
                bubble.y = view.glyph_height + round_to_int(projected[1]) as i32 - bubble.height;
            }
        }
        // The margins: below the top/left edge is pulled in, past the
        // opposite edge is pushed back.
        if bubble.y < MARGIN {
            bubble.y = MARGIN;
        } else if i64::from(view.height) - i64::from(MARGIN)
            < i64::from(bubble.height) + i64::from(bubble.y)
        {
            bubble.y = (view.height - bubble.height) - MARGIN;
        }
        if bubble.x < MARGIN {
            bubble.x = MARGIN;
        } else if i64::from(view.width) - i64::from(MARGIN)
            < i64::from(bubble.width) + i64::from(bubble.x)
        {
            bubble.x = (view.width - bubble.width) - MARGIN;
        }
    }

    /// One frame of the bubble draw: the state work of [`Bubbles::update`],
    /// then – with a view – every showing bubble is placed. Waiting bubbles
    /// are not placed or drawn. `head` answers a character's head position.
    pub fn frame(
        &mut self,
        now: u32,
        view: Option<&View>,
        mut head: impl FnMut(Speaker) -> Option<[f32; 3]>,
    ) {
        let list = &mut self.list;
        let mut write = 0;
        for read in 0..list.len() {
            let mut b = list[read].clone();
            if b.phase == Phase::Waiting {
                if list[..read].iter().any(|k| k.speaker == b.speaker) {
                    list[write] = b;
                    write += 1;
                    continue;
                }
                b.phase = Phase::Showing;
                b.start = now;
            }
            let mut t = now.wrapping_sub(b.start);
            if t < FADE_IN {
                b.alpha = smoothstep((f64::from(t) * (1.0 / 300.0)) as f32);
            } else if t <= b.duration {
                b.alpha = 1.0;
            } else {
                if b.phase == Phase::Closing {
                    t = t.wrapping_mul(4).wrapping_sub(b.duration.wrapping_mul(3));
                }
                let end = u64::from(b.duration) + u64::from(FADE_OUT);
                if u64::from(t) > end {
                    continue;
                }
                b.alpha = smoothstep(((end - u64::from(t)) as f64 * (1.0 / 1500.0)) as f32);
            }
            if let Some(view) = view {
                let speaker = b.speaker;
                Bubbles::place_bubble(&mut b, view, head(speaker));
            }
            list[write] = b;
            write += 1;
        }
        list.truncate(write);
    }
}
