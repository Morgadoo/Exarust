//! The 3D viewport control (`T3DViewport`): the view's size and camera
//! scale, the supersampling levels with their jitter table and the
//! one-frame multi-sample still, the stereo eye offset, the version and
//! frame-rate overlays, the conversions between the view's pixels and its
//! space, and the cursor actions the view's press, release and drag run.
//!
//! The running app draws the world into a full-window view
//! (`exa_render`'s supersampling is the running path); this unit holds the
//! viewport's own rules, independent of the renderer. Rules: `docs/UI.md`
//! (3D viewport), `docs/RENDERING.md`.

/// The viewport form's title.
pub const TITLE: &str = "Viewport";
/// Its style word.
pub const STYLE: u32 = 0x88;
/// The state bits its constructor sets (bit 0x20 is cleared first).
pub const STATE_BITS: u32 = 0x4c010;
/// The form's border.
pub const BORDER: i32 = 8;
/// The selected control meaning "no control", at which the view's cursor
/// actions take the press.
pub const NO_CONTROL_SELECTED: i32 = -1;
/// The frame-rate readout's line, in units from the top.
pub const FPS_LINE: i32 = 1;
/// The version overlay's text.
pub const VERSION: &str = "V 0.9.5.2g";
/// The frame-rate readout's suffix.
pub const FPS_SUFFIX: &str = " FPS";
/// The overlays' shadow, drawn one pixel up and left: black at half alpha.
pub const OVERLAY_SHADOW: [u8; 4] = [0, 0, 0, 0x80];
/// The frame-rate readout's alpha over the skin's ink.
pub const FPS_ALPHA: u8 = 0xa0;

/// One supersampling level: its sample count and the first sample in
/// [`SAMPLE_OFFSETS`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SupersampleLevel {
    pub count: i32,
    pub base: usize,
}

/// The eight levels the original records.
pub const LEVELS: [SupersampleLevel; 8] = [
    SupersampleLevel { count: 1, base: 0 },
    SupersampleLevel { count: 2, base: 1 },
    SupersampleLevel { count: 3, base: 3 },
    SupersampleLevel { count: 4, base: 6 },
    SupersampleLevel { count: 8, base: 10 },
    SupersampleLevel {
        count: 15,
        base: 18,
    },
    SupersampleLevel {
        count: 24,
        base: 33,
    },
    SupersampleLevel {
        count: 66,
        base: 57,
    },
];

/// The jitter points, in the view's units, in the order the levels use.
/// The first point is the centre and the higher levels refine a square.
pub const SAMPLE_OFFSETS: [(f32, f32); 123] = [
    (0.0, 0.0),
    (0.246490002, 0.249999002),
    (-0.246490002, -0.249999002),
    (0.117147997, -0.117569998),
    (0.256262988, 0.368119001),
    (-0.373411, -0.250550002),
    (-0.292625993, -0.149945006),
    (0.296923995, 0.149994001),
    (0.203849003, -0.353780001),
    (-0.208147004, 0.353729993),
    (0.164215997, -0.0543989986),
    (-0.183789998, 0.0821020007),
    (0.102254003, 0.299133003),
    (-0.0792630017, -0.317382991),
    (0.286437988, -0.393494993),
    (-0.334818006, 0.435330987),
    (0.459461987, 0.141540006),
    (-0.414498001, -0.192828998),
    (0.105354004, -0.0100990003),
    (-0.154180005, 0.0217940006),
    (-0.0559180006, -0.215196997),
    (0.285560995, 0.188437),
    (-0.370135009, -0.116425),
    (0.360175997, -0.0656879991),
    (-0.111750998, 0.27501899),
    (0.179270998, -0.33119601),
    (-0.370609999, 0.387504011),
    (0.451635987, -0.300013006),
    (-0.454968005, 0.134087995),
    (-0.307049006, -0.364926994),
    (0.384119987, 0.458499998),
    (-0.0802310035, -0.470964998),
    (0.138721004, 0.409168005),
    (0.0302450005, 0.136383995),
    (0.0188650005, -0.348866999),
    (-0.350113988, -0.472308993),
    (0.222181007, 0.149524003),
    (-0.393669993, -0.266873002),
    (0.404567987, 0.230435997),
    (0.0983809978, 0.465337008),
    (0.462671012, 0.442115992),
    (0.400373012, -0.212720007),
    (-0.409987986, 0.263345003),
    (-0.115878001, -0.00198099995),
    (0.348425001, -0.00923699979),
    (-0.46401599, 0.066467002),
    (-0.138674006, -0.468006015),
    (0.144932002, -0.0227799993),
    (-0.250194997, 0.150160998),
    (-0.181400001, -0.264218986),
    (0.196097001, -0.234138995),
    (-0.311082006, -0.0788149983),
    (0.268379003, 0.366777986),
    (-0.0406010002, 0.327109009),
    (-0.234392002, 0.354658991),
    (-0.0031020001, -0.154402003),
    (0.297996998, -0.417964995),
    (0.00914600026, 0.0150979999),
    (0.124155998, -0.0168590005),
    (-0.108006001, -0.00247099996),
    (-0.0839409977, -0.121440001),
    (0.0834750012, 0.117027998),
    (0.0412059985, -0.106205001),
    (-0.0406429991, 0.123452999),
    (0.206511006, -0.104667999),
    (-0.200362995, 0.070863001),
    (0.124757998, -0.197108999),
    (-0.121527001, 0.210223004),
    (0.0187030006, -0.261792004),
    (0.0132099995, 0.234401003),
    (-0.109319001, -0.242379993),
    (0.145087004, 0.221726),
    (-0.224595994, -0.189548001),
    (0.266377002, -0.218171),
    (-0.209349006, -0.0653830022),
    (0.247881994, 0.152009994),
    (-0.24244, 0.186933994),
    (-0.0730360001, -0.355064005),
    (0.0762479976, 0.478538007),
    (-0.170919001, -0.429367989),
    (-0.189296007, 0.438962996),
    (0.234646007, 0.0345490016),
    (-0.301609993, 0.00931899995),
    (-0.221637994, -0.313767999),
    (0.0792270005, 0.345021009),
    (0.0473559983, -0.387134999),
    (-0.361249, -0.104548998),
    (0.328972995, -0.101914003),
    (0.341369003, 0.0814900026),
    (0.174257994, -0.311854005),
    (-0.380809993, 0.130035996),
    (0.294061005, 0.263363987),
    (-0.114836, 0.328188986),
    (-0.339083999, -0.35570699),
    (-0.0217429996, 0.403898001),
    (0.295962006, -0.334699005),
    (-0.233015999, 0.305202991),
    (0.200905994, 0.333292991),
    (-0.286709011, -0.470214009),
    (0.31013599, 0.419699997),
    (-0.344844013, -0.230813995),
    (-0.345773011, 0.253778994),
    (0.394102007, -0.210901007),
    (-0.312570989, 0.397655994),
    (0.452998012, 0.0253970008),
    (-0.461320996, -0.176661),
    (0.194420993, 0.453509986),
    (-0.0555399992, -0.477952003),
    (0.447544008, -0.0960040018),
    (-0.426789999, 0.00497699995),
    (0.404051006, 0.224974006),
    (-0.453521013, -0.299888998),
    (0.158474997, -0.430761993),
    (0.486972004, 0.139163002),
    (0.28224501, -0.458671004),
    (-0.466210008, 0.248839006),
    (0.482046008, 0.443890005),
    (-0.477605999, -0.424878001),
    (0.408659995, 0.345826),
    (-0.430063009, 0.363413006),
    (0.425004989, -0.332397014),
    (-0.400020987, 0.48219499),
    (0.408216, -0.457699001),
];

/// The level a supersampling setting names, clamped to the recorded eight
/// (the original indexes without a bound).
pub fn level(setting: i32) -> SupersampleLevel {
    LEVELS[setting.clamp(0, LEVELS.len() as i32 - 1) as usize]
}

/// The viewport's supersampling setting and the view's counters it moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Still {
    /// The level (an index into [`LEVELS`]).
    pub setting: i32,
    /// The level's sample count, 0 when it has a single sample.
    pub count: i32,
    /// The view's count of viewports that supersample.
    pub counter: i32,
    /// The view's ready flag, cleared by a change.
    pub ready: bool,
}

impl Still {
    /// Changes the level: leaving level 0 raises the view's counter, coming
    /// back to it lowers it; the view's ready flag clears and the sample
    /// count follows the level. Returns whether the level changed.
    pub fn set_setting(&mut self, setting: i32) -> bool {
        let setting = setting.clamp(0, LEVELS.len() as i32 - 1);
        if setting == self.setting {
            return false;
        }
        if setting == 0 {
            if self.setting > 0 {
                self.counter -= 1;
            }
        } else if self.setting == 0 {
            self.counter += 1;
        }
        self.ready = false;
        self.setting = setting;
        let count = LEVELS[setting as usize].count;
        self.count = if count == 1 { 0 } else { count };
        true
    }
}

/// One sample of a supersampled frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StillSample {
    /// The jitter point and its index in [`SAMPLE_OFFSETS`].
    pub offset: (f32, f32),
    pub table: i32,
    /// The view's normalized jitter ([`jitter`]).
    pub jitter: [f32; 2],
    /// The share accumulated with this sample (computed in double
    /// precision).
    pub fraction: f32,
    /// 0.5 less the sample's centre over the count.
    pub half: f32,
    /// The weight the earlier samples keep when this one is blended in;
    /// `None` for the first sample.
    pub blend: Option<f32>,
}

/// The samples the one-frame still render takes for a level: sample `i`
/// (0-based) uses the level's point `i`, the share `(i + 1) / count`, the
/// half `0.5 − (i + 0.5) / count` and, after the first, the blend
/// `1 − 1 / (i + 1)`. `scale` is the view's projection scales and `view`
/// its pixel size.
pub fn still_samples(setting: i32, scale: (f32, f32), view: (i32, i32)) -> Vec<StillSample> {
    let level = level(setting);
    (0..level.count)
        .map(|i| {
            let table = level.base as i32 + i;
            let offset = SAMPLE_OFFSETS[table as usize];
            StillSample {
                offset,
                table,
                jitter: jitter(offset, scale, view),
                fraction: (f64::from(i + 1) / f64::from(level.count)) as f32,
                half: 0.5 - (i as f32 + 0.5) / level.count as f32,
                blend: (i > 0).then_some(1.0 - 1.0 / (i + 1) as f32),
            }
        })
        .collect()
}

/// A progressive draw's sample `index` (1-based, from the frame after a
/// refresh): the level's point `index − 1`, the share `index / count`, the
/// half `0.5 − (index + 0.5) / count` and, from the second, the blend
/// `1 − 1 / index`. The 0.9.5 draw sets its refresh flag on every frame, so
/// the progressive path never runs there; the rule is kept for its fields.
pub fn progressive_sample(
    setting: i32,
    index: i32,
    scale: (f32, f32),
    view: (i32, i32),
) -> Option<StillSample> {
    let level = level(setting);
    if index < 1 || level.count < index {
        return None;
    }
    let table = level.base as i32 + index - 1;
    let offset = SAMPLE_OFFSETS[table as usize];
    Some(StillSample {
        offset,
        table,
        jitter: jitter(offset, scale, view),
        fraction: (f64::from(index) / f64::from(level.count)) as f32,
        half: 0.5 - (index as f32 + 0.5) / level.count as f32,
        blend: (index > 1).then_some(1.0 - 1.0 / index as f32),
    })
}

/// The normalized jitter of one point: `2 · scale · point / size` per axis,
/// in that order (a zero size divides to an infinity, as the original's).
pub fn jitter(offset: (f32, f32), scale: (f32, f32), view: (i32, i32)) -> [f32; 2] {
    [
        2.0 * scale.0 * offset.0 / view.0 as f32,
        2.0 * scale.1 * offset.1 / view.1 as f32,
    ]
}

/// The stereo eye's shift for a frame without jitter: the camera scale to
/// the right for eye 1, to the left for eye 2, none otherwise.
pub fn eye_offset(eye: i32, camera_scale: f32) -> [f32; 2] {
    match eye {
        1 => [camera_scale, 0.0],
        2 => [-camera_scale, 0.0],
        _ => [0.0, 0.0],
    }
}

/// The camera's scale: `(2 · fov) · 0.5` over the camera's distance to its
/// target, the distance held to 100 through 3000 (a not-a-number distance
/// passes both tests and stays one).
pub fn camera_scale(field_of_view: f32, distance: f32) -> f32 {
    // Both compares are unordered in the original, as `clamp`'s are: a
    // not-a-number distance stays one.
    let distance = distance.clamp(100.0, 3000.0);
    2.0 * field_of_view * 0.5 / distance
}

/// The version overlay's place: a unit in from the left, a unit up from the
/// form's bottom.
pub fn version_place(bottom: i32, unit: i32) -> (i32, i32) {
    (unit, bottom - unit)
}

/// The frame-rate readout's place: its right end a unit inside the client
/// (the form's inner right less the border), its width the main font's gap
/// times the text's length; its baseline [`FPS_LINE`] units and a line
/// height down.
pub fn fps_place(
    inner_right: i32,
    border: i32,
    unit: i32,
    gap: i32,
    line_height: i32,
    text_len: usize,
) -> (i32, i32) {
    (
        ((inner_right - border) - unit) - gap * text_len as i32,
        unit * FPS_LINE + line_height,
    )
}

/// The frame-rate readout's text: two decimals and [`FPS_SUFFIX`].
pub fn fps_text(rate: f32) -> String {
    format!("{rate:.2}{FPS_SUFFIX}")
}

/// The frame-rate counter: frames since the last readout, refreshed once
/// more than 250 clock units (milliseconds) have passed.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FpsCounter {
    pub frames: i32,
    pub since: u32,
    pub text: String,
}

impl FpsCounter {
    /// Counts a frame at `now`; past 250 the rate is the frames over the
    /// elapsed seconds (in double precision, shown as a single) and the
    /// count restarts.
    pub fn frame(&mut self, now: u32) {
        self.frames += 1;
        let elapsed = i64::from(now) - i64::from(self.since);
        if 250 < elapsed {
            let rate = f64::from(self.frames) / (elapsed as f64 * 0.001);
            self.text = fps_text(rate as f32);
            self.frames = 0;
            self.since = now;
        }
    }
}

/// A view pixel and depth into view space: x and y from the view's centre,
/// times the depth over the projection scale times half the size; the
/// depth negated.
pub fn screen_to_view(point: [f32; 3], size: (i32, i32), projection: (f32, f32)) -> [f32; 3] {
    let [x, y, z] = point;
    let (w, h) = (size.0 as f32, size.1 as f32);
    [
        (x - w * 0.5) * z / (projection.0 * w * 0.5),
        (y - h * 0.5) * z / (projection.1 * h * 0.5),
        -z,
    ]
}

/// A view-space point into view pixels, y counted from the bottom: a point
/// at or behind the eye (depth 0 or more) goes to the edge its signs point
/// to; otherwise half the size plus the projection scale times half the
/// size times the coordinate over the negated depth.
pub fn view_to_screen(point: [f32; 3], size: (i32, i32), projection: (f32, f32)) -> [f32; 3] {
    let [x, y, z] = point;
    let (w, h) = (size.0 as f32, size.1 as f32);
    let (sx, sy) = if z >= 0.0 {
        (
            if x >= 0.0 { w } else { 0.0 },
            if y >= 0.0 { h } else { 0.0 },
        )
    } else {
        (
            w * 0.5 + projection.0 * w * 0.5 * x / -z,
            h * 0.5 + projection.1 * h * 0.5 * y / -z,
        )
    };
    [sx, h - sy, -z]
}

/// A cursor action: the press word bits it answers and its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorAction {
    pub mask: u32,
    pub name: &'static str,
}

/// The action whose mask the word holds with the most bits set in the
/// mask's low 16 bits; the table is scanned from its end, so among equals
/// the last wins, and a mask without low bits is never picked.
pub fn pick_action(actions: &[CursorAction], word: u32) -> Option<usize> {
    let mut best = None;
    let mut best_bits = 0;
    for (index, action) in actions.iter().enumerate().rev() {
        if word & action.mask == action.mask {
            let bits = (action.mask as u16).count_ones();
            if best_bits < bits {
                best_bits = bits;
                best = Some(index);
            }
        }
    }
    best
}

/// The view's cursor-action state: the action a press picked, whether an
/// action is running, and whether the press became a view drag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ActionRoute {
    pub picked: Option<usize>,
    pub running: bool,
    pub view_drag: bool,
}

/// A press's routing: the result and the action to begin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ActionPress {
    pub result: u32,
    pub begin: Option<usize>,
}

impl ActionRoute {
    /// The view's press. A running action is ended first (`end` is its
    /// answer to the end code 0x80); an answer with bit 0 takes the press
    /// (result 1). Otherwise the result is the form's own press `base` with
    /// that answer; when no control is selected (-1), the action matching
    /// the word is picked and `begin` runs it (it returns the result it
    /// leaves and whether the action keeps running), and a result with
    /// bits 1 and 4 makes the press a view drag.
    pub fn press(
        &mut self,
        actions: &[CursorAction],
        word: u32,
        end: impl FnOnce() -> u32,
        base: u32,
        selected: i32,
        begin: impl FnOnce(usize, u32) -> (u32, bool),
    ) -> ActionPress {
        let ended = if self.running {
            self.running = false;
            end()
        } else {
            0
        };
        self.picked = None;
        if ended & 1 != 0 {
            return ActionPress {
                result: 1,
                begin: None,
            };
        }
        let mut result = base | ended;
        let mut begun = None;
        if selected == -1 {
            if let Some(index) = pick_action(actions, word) {
                self.picked = Some(index);
                let (left, running) = begin(index, result);
                result = left;
                self.running = running;
                begun = Some(index);
            }
            if result & 5 == 5 {
                self.view_drag = true;
            }
        }
        ActionPress {
            result,
            begin: begun,
        }
    }

    /// The view's release: a running action hears the end code (`end`); an
    /// answer of 0x40 keeps it running and is the result, otherwise (or
    /// with no action) the form's own release `base` answers.
    pub fn release(&mut self, end: impl FnOnce() -> u32, base: impl FnOnce() -> u32) -> u32 {
        self.picked = None;
        if !self.running {
            return base();
        }
        self.running = false;
        let answer = end();
        if answer == 0x40 {
            self.running = true;
            return 0x40;
        }
        base()
    }

    /// The view's drag move. In a view drag a running action gets the move
    /// (`step` answers code 0) and stops on 0x80; with none running the
    /// result is 0x80. Outside a view drag the form's own move answers.
    pub fn drag(&mut self, step: impl FnOnce() -> u32, base: impl FnOnce() -> u32) -> u32 {
        if !self.view_drag {
            return base();
        }
        if !self.running {
            return 0x80;
        }
        let answer = step();
        if answer & 0x80 == 0x80 {
            self.running = false;
        }
        answer
    }
}

/// Whether a view callback's answer asks the root for a redraw (any of its
/// two low bits).
pub fn requests_redraw(answer: u32) -> bool {
    answer & 3 != 0
}

/// The viewport's view size: the client (the form's inner right less the
/// border by its bottom less its top) unless the size is fixed, when the
/// view is resized to the client and its own size is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ViewSize {
    pub size: (i32, i32),
    pub fixed: bool,
    /// The next draw restarts its frame.
    pub refresh: bool,
}

impl ViewSize {
    /// The resize: the size follows `client`, or the view's size `view`
    /// once fixed; the camera scale is recomputed and the next draw
    /// refreshes.
    pub fn resized(&mut self, client: (i32, i32), view: (i32, i32)) {
        self.size = if self.fixed { view } else { client };
        self.refresh = true;
    }

    /// Fixes the size: the view is resized to the client from now on.
    pub fn fix(&mut self) {
        self.fixed = true;
    }
}

/// The counters the viewport's constructor moves: its instance number and
/// its place in the global viewport list, both counting from one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ViewportRegistry {
    pub created: i32,
    pub listed: i32,
}

impl ViewportRegistry {
    /// Makes a viewport: returns its instance number and list place.
    pub fn add(&mut self) -> (i32, i32) {
        self.created += 1;
        self.listed += 1;
        (self.created, self.listed)
    }
}

/// The viewport's hit test: where the form's own test finds no control
/// (-1), the point belongs to the view, whose cursor shape is returned.
pub fn view_cursor(base: i32, view_cursor: i32) -> Option<i32> {
    (base == -1).then_some(view_cursor)
}

/// The view-port's delegate rules for messages the hosted view can take
/// over from the form. Each delegate is a callback the view installs; when
/// it is absent the form's own handler answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ViewDelegates {
    /// The view takes the press (its callback at the form's press slot).
    pub press: bool,
    /// The view takes the release.
    pub release: bool,
    /// The view takes keys.
    pub key: bool,
    /// The view has a begin-frame callback.
    pub begin: bool,
    /// The view has a redraw-request callback.
    pub redraw: bool,
    /// The view has an extra hit callback.
    pub hit: bool,
    /// The view has a cursor-shape callback.
    pub shape: bool,
    /// The view has an extra key-up callback.
    pub extra_key: bool,
}

/// The press's route: the view's callback answers when it has one, else the
/// form's own press. `base` and `view` are the two answers, each run only
/// when its route is taken.
pub fn press_route(
    delegates: ViewDelegates,
    base: impl FnOnce() -> u32,
    view: impl FnOnce() -> u32,
) -> u32 {
    if delegates.press { view() } else { base() }
}

/// The release's route: the view's callback hears the release when it has
/// one; the answer is 0 either way and the form's own release is not run.
pub fn release_route(delegates: ViewDelegates, view: impl FnOnce() -> u32) -> u32 {
    if delegates.release {
        view();
    }
    0
}

/// The key's route and its forward: a key-goes-to-the-view answer is the
/// view's when it has a key callback, else the form's. With the view
/// answering and the key's low bits clear, the key is also handed to the
/// focused control (`forward`).
pub fn key_route(
    delegates: ViewDelegates,
    mask: u32,
    base: impl FnOnce() -> u32,
    view: impl FnOnce() -> u32,
) -> (u32, bool) {
    if delegates.key {
        (view(), mask & 3 == 0)
    } else {
        (base(), false)
    }
}

/// The begin-frame callback's answer, 0 without one.
pub fn begin_route(delegates: ViewDelegates, begin: impl FnOnce() -> u32) -> u32 {
    if delegates.begin { begin() } else { 0 }
}

/// The redraw-request the draw slot sends the task: 7 when the view's
/// callback answered with either redraw bit, none without a callback.
pub fn redraw_message(delegates: ViewDelegates, answer: u32) -> Option<u32> {
    (delegates.redraw && requests_redraw(answer)).then_some(7)
}

/// The viewport's hit test when the form finds no control (-1): the view's
/// hit callback, when it has one, hears the point in the form's
/// coordinates, and the port's cursor code is stored with or without it
/// ([`view_cursor`]). Elsewhere no code is stored.
pub fn hit_route(
    delegates: ViewDelegates,
    hit: i32,
    port_code: i32,
    view: impl FnOnce() -> i32,
) -> Option<i32> {
    if hit == -1 && delegates.hit {
        view();
    }
    view_cursor(hit, port_code)
}

/// The cursor-shape route: the form's own shape answer, and the view's
/// callback hears the shape when it has one.
pub fn shape_route(
    delegates: ViewDelegates,
    base: impl FnOnce() -> i32,
    view: impl FnOnce() -> i32,
) -> i32 {
    let shape = base();
    if delegates.shape {
        view();
    }
    shape
}

/// The extra key-up callback's answer, 0 without one.
pub fn extra_key_route(delegates: ViewDelegates, key: impl FnOnce() -> u32) -> u32 {
    if delegates.extra_key { key() } else { 0 }
}

/// The view the port hosts: attaching one stores it and answers it back
/// (the original sets the view's target and keeps its pointer).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ViewSlot {
    pub view: Option<u32>,
}

impl ViewSlot {
    /// Attaches `view` to the port, returning it.
    pub fn attach(&mut self, view: u32) -> u32 {
        self.view = Some(view);
        view
    }
}

/// The fields the viewport's draw slot reads for one frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrawInput {
    /// The form's client rectangle in the view's coordinates (left, top,
    /// right, bottom).
    pub client: [i32; 4],
    /// The view's own bounds (left, top, right, bottom).
    pub view: [i32; 4],
    /// The view's pixel size, for the jitter.
    pub view_size: (i32, i32),
    /// The supersampling setting and its sample count ([`Still::count`],
    /// 0 for a single-sample level).
    pub setting: i32,
    pub count: i32,
    /// The view's sample counter: 0 means the frame renders whole.
    pub index: i32,
    /// The refresh flag the counter sees. The 0.9.5 slot sets it on every
    /// frame after its bounds test, so the running game always passes
    /// `true`; `false` models the progressive branch the flag hides.
    pub refresh: bool,
    /// The task's pointer is busy (any of its 0xf0 state bits) and the port
    /// does not draw through it: the counter holds instead of advancing.
    pub held: bool,
    /// The view has already accumulated its whole still.
    pub accumulated: bool,
    /// The stereo mode: 0 none, 1 the right eye, 2 the left.
    pub stereo: i32,
    /// The camera's scale (its projection's reach).
    pub camera_scale: f32,
    /// The view's projection scales.
    pub scale: (f32, f32),
    pub unit: i32,
    /// The form's inner right, its border, its bottom and its line index.
    pub inner_right: i32,
    pub border: i32,
    pub bottom: i32,
    pub line: i32,
    /// The small font's gap and line height, for the frame-rate readout.
    pub gap: i32,
    pub line_height: i32,
    /// The readouts the form shows.
    pub fps: bool,
    pub fps_len: usize,
    pub version: bool,
}

/// What the viewport's draw slot does this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrawPlan {
    /// The client has a size, so the slot runs at all.
    pub visible: bool,
    /// The client's left, right or bottom edge passes the view's (its top
    /// is not compared); the refresh flag is then set.
    pub exceeds: bool,
    /// The view renders a new frame this time (otherwise its last image is
    /// shown).
    pub render: bool,
    /// The sample counter after the frame.
    pub index: i32,
    /// The task message the counter's step sends: 0x22 for its first
    /// sample, 0x23 for the later ones.
    pub message: Option<u32>,
    /// The jittered sample a progressive frame accumulates (none on a
    /// whole frame).
    pub sample: Option<StillSample>,
    /// The stereo eye's shift, applied only to a whole frame.
    pub eye: [f32; 2],
    /// The frame-rate readout's place.
    pub fps: Option<(i32, i32)>,
    /// The version overlay's place.
    pub version: Option<(i32, i32)>,
}

/// The viewport's draw-slot rules. A client at least a pixel each way runs
/// the slot. A client whose left edge is left of the view's, or whose right
/// or bottom edge passes the view's, sets the refresh flag (the 0.9.5 slot
/// then sets it again unconditionally, so `input.refresh` is `true` in the
/// running game). On a refresh the counter restarts and the frame renders
/// whole, shifted by the stereo eye. Without one, a counter below the
/// sample count renders: it holds while the task's pointer is busy and the
/// port does not draw through it, else it advances, sending 0x22 at its
/// first sample and 0x23 after, and the frame takes that progressive
/// sample. A counter at the count renders again only while the view has
/// not accumulated its still. The frame-rate readout and the version take
/// their places whenever the slot runs.
pub fn plan_draw(input: DrawInput) -> DrawPlan {
    let [left, top, right, bottom] = input.client;
    let visible =
        i64::from(right) - i64::from(left) >= 1 && i64::from(bottom) - i64::from(top) >= 1;
    let [view_left, _, view_right, view_bottom] = input.view;
    let exceeds = left < view_left || right > view_right || bottom > view_bottom;
    if !visible {
        return DrawPlan {
            visible: false,
            exceeds: false,
            render: false,
            index: input.index,
            message: None,
            sample: None,
            eye: [0.0, 0.0],
            fps: None,
            version: None,
        };
    }
    let mut index = input.index;
    let mut message = None;
    let render = if input.refresh || exceeds {
        index = 0;
        true
    } else if index < input.count {
        if !input.held {
            index += 1;
            message = Some(if index == 1 { 0x22 } else { 0x23 });
        }
        true
    } else {
        !input.accumulated
    };
    let (sample, eye) = match (render, index > 0) {
        (true, true) => (
            progressive_sample(input.setting, index, input.scale, input.view_size),
            [0.0, 0.0],
        ),
        (true, false) => (None, eye_offset(input.stereo, input.camera_scale)),
        (false, _) => (None, [0.0, 0.0]),
    };
    DrawPlan {
        visible: true,
        exceeds,
        render,
        index,
        message,
        sample,
        eye,
        fps: input.fps.then(|| {
            let (x, _) = fps_place(
                input.inner_right,
                input.border,
                input.unit,
                input.gap,
                input.line_height,
                input.fps_len,
            );
            (x, input.unit * input.line + input.line_height)
        }),
        version: input
            .version
            .then(|| version_place(input.bottom, input.unit)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_levels_record_their_counts_and_offsets() {
        assert_eq!(LEVELS[0], SupersampleLevel { count: 1, base: 0 });
        assert_eq!(
            LEVELS[7],
            SupersampleLevel {
                count: 66,
                base: 57
            }
        );
        let total: i32 = LEVELS.iter().map(|l| l.count).sum();
        assert_eq!(total as usize, SAMPLE_OFFSETS.len());
        assert_eq!(level(-2).count, 1);
        assert_eq!(level(99).count, 66);
        assert_eq!(SAMPLE_OFFSETS[0], (0.0, 0.0));
        assert_eq!(SAMPLE_OFFSETS[1], (0.246490002, 0.249999002));
        assert_eq!(SAMPLE_OFFSETS[56], (0.297997, -0.417965));
    }

    #[test]
    fn the_still_render_walks_every_sample_of_its_level() {
        let samples = still_samples(2, (1.0, 0.5), (200, 100));
        assert_eq!(samples.len(), 3);
        let first = samples[0];
        assert_eq!(first.table, 3);
        assert_eq!(first.offset, SAMPLE_OFFSETS[3]);
        assert_eq!(
            first.jitter,
            [
                2.0 * 1.0 * SAMPLE_OFFSETS[3].0 / 200.0,
                2.0 * 0.5 * SAMPLE_OFFSETS[3].1 / 100.0
            ]
        );
        assert_eq!(first.blend, None);
        assert_eq!(first.fraction, (1.0f64 / 3.0) as f32);
        assert_eq!(first.half, 0.5 - 0.5 / 3.0);
        assert_eq!(samples[2].blend, Some(1.0 - 1.0 / 3.0));
        assert_eq!(samples[2].fraction, 1.0);
        // Level 0 is the centre alone.
        let one = still_samples(0, (1.0, 1.0), (10, 10));
        assert_eq!((one.len(), one[0].jitter), (1, [0.0, 0.0]));
    }

    #[test]
    fn a_progressive_sample_counts_from_one() {
        let p = progressive_sample(2, 2, (1.0, 1.0), (100, 100)).unwrap();
        assert_eq!((p.table, p.offset), (4, SAMPLE_OFFSETS[4]));
        assert_eq!(p.fraction, (2.0f64 / 3.0) as f32);
        assert_eq!(p.half, 0.5 - 2.5 / 3.0);
        assert_eq!(p.blend, Some(0.5));
        assert_eq!(
            progressive_sample(2, 1, (1.0, 1.0), (100, 100))
                .unwrap()
                .blend,
            None
        );
        assert!(progressive_sample(2, 0, (1.0, 1.0), (100, 100)).is_none());
        assert!(progressive_sample(2, 4, (1.0, 1.0), (100, 100)).is_none());
    }

    #[test]
    fn the_still_counter_follows_zero() {
        let mut still = Still::default();
        assert!(still.set_setting(4));
        assert_eq!((still.count, still.counter), (8, 1));
        assert!(!still.set_setting(4));
        still.set_setting(0);
        assert_eq!((still.setting, still.count, still.counter), (0, 0, 0));
        still.set_setting(2);
        still.set_setting(3);
        assert_eq!((still.setting, still.count, still.counter), (3, 4, 1));
    }

    #[test]
    fn the_camera_scale_holds_the_distance() {
        assert_eq!(camera_scale(30.0, 400.0), 30.0 / 400.0);
        assert_eq!(camera_scale(30.0, 10.0), 30.0 / 100.0);
        assert_eq!(camera_scale(30.0, 9000.0), 30.0 / 3000.0);
        assert!(camera_scale(30.0, f32::NAN).is_nan());
        assert_eq!(eye_offset(1, 0.25), [0.25, 0.0]);
        assert_eq!(eye_offset(2, 0.25), [-0.25, 0.0]);
        assert_eq!(eye_offset(0, 0.25), [0.0, 0.0]);
    }

    #[test]
    fn the_overlays_sit_in_the_unit_grid() {
        assert_eq!(version_place(600, 5), (5, 595));
        // "57.25 FPS" is nine bytes of gap 6: 800 − 8 − 5 − 54.
        let text = fps_text(57.25);
        assert_eq!(text, "57.25 FPS");
        assert_eq!(fps_place(800, 8, 5, 6, 16, text.len()), (733, 21));
    }

    #[test]
    fn the_frame_rate_refreshes_after_250() {
        let mut fps = FpsCounter::default();
        for now in [100, 200, 250] {
            fps.frame(now);
        }
        assert_eq!((fps.frames, fps.text.as_str()), (3, ""));
        fps.frame(251);
        // Four frames over 0.251 s.
        assert_eq!(fps.text, fps_text((4.0f64 / (251.0 * 0.001)) as f32));
        assert_eq!((fps.frames, fps.since), (0, 251));
    }

    #[test]
    fn view_pixels_and_space_convert() {
        let size = (200, 100);
        let projection = (1.0, 2.0);
        let p = screen_to_view([0.0, 0.0, 10.0], size, projection);
        assert_eq!(p, [-10.0, -5.0, -10.0]);
        let u = view_to_screen([100.0, 50.0, -10.0], size, projection);
        assert_eq!(u, [1100.0, -450.0, 10.0]);
        assert_eq!(
            view_to_screen([-1.0, 1.0, 0.0], size, projection),
            [0.0, 0.0, -0.0]
        );
    }

    #[test]
    fn the_action_with_the_most_low_bits_wins() {
        let actions = [
            CursorAction {
                mask: 0x1,
                name: "a",
            },
            CursorAction {
                mask: 0x3,
                name: "b",
            },
            CursorAction {
                mask: 0x2,
                name: "c",
            },
            CursorAction {
                mask: 0x3_0000,
                name: "high",
            },
            CursorAction {
                mask: 0x1_0001,
                name: "d",
            },
        ];
        assert_eq!(pick_action(&actions, 0x3), Some(1));
        assert_eq!(pick_action(&actions, 0x2), Some(2));
        assert_eq!(pick_action(&actions, 0x4), None);
        // High mask bits do not count: d (one low bit) ties a, and the
        // later one wins; a high-only mask is never picked.
        assert_eq!(pick_action(&actions, 0x1_0001), Some(4));
        assert_eq!(pick_action(&actions, 0x3_0000), None);
    }

    #[test]
    fn a_press_begins_an_action_and_a_release_ends_it() {
        let actions = [CursorAction {
            mask: 0x1,
            name: "a",
        }];
        let mut route = ActionRoute::default();
        let out = route.press(&actions, 0x1001, || 0, 0x10, -1, |_, r| (r | 5, true));
        assert_eq!(
            out,
            ActionPress {
                result: 0x15,
                begin: Some(0)
            }
        );
        assert!(route.running && route.view_drag);
        // A drag move reaches the action until it answers 0x80.
        assert_eq!(route.drag(|| 3, || 9), 3);
        assert!(requests_redraw(3));
        assert_eq!(route.drag(|| 0x80, || 9), 0x80);
        assert!(!route.running);
        assert_eq!(route.drag(|| 3, || 9), 0x80);
        // A selected control keeps the press for the form.
        let mut route = ActionRoute::default();
        let out = route.press(&actions, 0x1001, || 0, 7, 2, |_, r| (r, true));
        assert_eq!(
            out,
            ActionPress {
                result: 7,
                begin: None
            }
        );
        // A running action ended with bit 0 takes the press.
        route.running = true;
        let out = route.press(&actions, 0x1001, || 1, 7, -1, |_, r| (r, true));
        assert_eq!(out.result, 1);
        // The release: 0x40 keeps the action, anything else ends it.
        route.running = true;
        assert_eq!(route.release(|| 0x40, || 5), 0x40);
        assert!(route.running);
        assert_eq!(route.release(|| 0, || 5), 5);
        assert!(!route.running);
    }

    #[test]
    fn the_draw_slot_plans_a_whole_or_a_sampled_frame() {
        let whole = DrawInput {
            client: [0, 0, 100, 80],
            view: [0, 0, 100, 80],
            view_size: (100, 80),
            setting: 2,
            count: 3,
            index: 0,
            refresh: true,
            held: false,
            accumulated: false,
            stereo: 1,
            camera_scale: 0.5,
            scale: (1.0, 0.5),
            unit: 5,
            inner_right: 800,
            border: 8,
            bottom: 600,
            line: 1,
            gap: 6,
            line_height: 16,
            fps: true,
            fps_len: 9,
            version: true,
        };
        // The running game's refresh: a whole frame, the eye shifted.
        let plan = plan_draw(whole);
        assert!(plan.visible && plan.render && !plan.exceeds);
        assert_eq!((plan.index, plan.message, plan.sample), (0, None, None));
        assert_eq!(plan.eye, [0.5, 0.0]);
        // 800 − 8 − 5 − 6·9 = 733; 5·1 + 16 = 21.
        assert_eq!(plan.fps, Some((733, 21)));
        assert_eq!(plan.version, Some((5, 595)));
        // The readout's line is the form's: line 2 is 5·2 + 16 = 26 down.
        assert_eq!(
            plan_draw(DrawInput { line: 2, ..whole }).fps,
            Some((733, 26))
        );
        let still = DrawInput {
            refresh: false,
            ..whole
        };
        // The first progressive frame: sample 1, message 0x22, no eye.
        let first = plan_draw(still);
        assert_eq!((first.index, first.message), (1, Some(0x22)));
        let sample = first.sample.expect("a progressive frame samples");
        assert_eq!(sample.table, 3);
        assert_eq!(sample.fraction, (1.0f64 / 3.0) as f32);
        assert_eq!(first.eye, [0.0, 0.0]);
        // The second continues with 0x23 and blends at a half.
        let second = plan_draw(DrawInput { index: 1, ..still });
        let sample = second.sample.expect("the second sample");
        assert_eq!(
            (second.index, second.message, sample.table),
            (2, Some(0x23), 4)
        );
        assert_eq!(sample.blend, Some(0.5));
        // A busy pointer holds the counter but still renders its sample.
        let held = plan_draw(DrawInput {
            index: 1,
            held: true,
            ..still
        });
        assert!(held.render);
        assert_eq!((held.index, held.message), (1, None));
        assert_eq!(held.sample.map(|s| s.table), Some(3));
        // At the count the still renders only until it has accumulated.
        let done = DrawInput { index: 3, ..still };
        assert!(plan_draw(done).render);
        let kept = plan_draw(DrawInput {
            accumulated: true,
            ..done
        });
        assert!(!kept.render);
        assert_eq!((kept.index, kept.sample, kept.eye), (3, None, [0.0, 0.0]));
        // A right or bottom edge past the view's, or a left edge before
        // it, refreshes; a top edge above it does not.
        for client in [[-1, 0, 100, 80], [0, 0, 101, 80], [0, 0, 100, 81]] {
            let moved = plan_draw(DrawInput {
                client,
                index: 2,
                ..still
            });
            assert!(moved.exceeds && moved.render);
            assert_eq!((moved.index, moved.sample), (0, None));
        }
        let above = plan_draw(DrawInput {
            client: [0, -5, 100, 80],
            index: 1,
            ..still
        });
        assert!(!above.exceeds);
        assert_eq!(above.index, 2);
        // A client with no height runs nothing and shows no readout.
        let empty = plan_draw(DrawInput {
            client: [0, 0, 100, 0],
            ..whole
        });
        assert!(!empty.visible && !empty.render);
        assert_eq!((empty.fps, empty.version, empty.sample), (None, None, None));
        // Stereo 2 shifts the other way, and no stereo does not shift.
        assert_eq!(plan_draw(DrawInput { stereo: 2, ..whole }).eye, [-0.5, 0.0]);
        assert_eq!(plan_draw(DrawInput { stereo: 0, ..whole }).eye, [0.0, 0.0]);
    }

    #[test]
    fn the_constructor_numbers_each_viewport() {
        let mut registry = ViewportRegistry::default();
        assert_eq!(registry.add(), (1, 1));
        assert_eq!(registry.add(), (2, 2));
        assert_eq!(TITLE, "Viewport");
        assert_eq!((STYLE, STATE_BITS, BORDER), (0x88, 0x4c010, 8));
        assert_eq!(NO_CONTROL_SELECTED, -1);
    }

    #[test]
    fn the_view_delegates_take_their_messages_from_the_form() {
        let none = ViewDelegates::default();
        let view = ViewDelegates {
            press: true,
            release: true,
            key: true,
            begin: true,
            ..none
        };
        // A view callback answers in place of the form's handler.
        assert_eq!(press_route(view, || 1, || 2), 2);
        assert_eq!(press_route(none, || 1, || 2), 1);
        // The release answers 0 either way; only the view's callback runs.
        let mut heard = false;
        assert_eq!(
            release_route(view, || {
                heard = true;
                2
            }),
            0
        );
        assert!(heard);
        assert_eq!(release_route(none, || unreachable!()), 0);
        // A key's low bits clear with a view callback forwards too.
        assert_eq!(key_route(view, 0x1000, || 1, || 2), (2, true));
        assert_eq!(key_route(view, 0x1002, || 1, || 2), (2, false));
        assert_eq!(key_route(none, 0x1000, || 1, || 2), (1, false));
        assert_eq!(begin_route(view, || 9), 9);
        assert_eq!(begin_route(none, || 9), 0);
        // The draw's redraw request is the task's message 7.
        assert_eq!(redraw_message(view, 3), None);
        let redraw = ViewDelegates {
            redraw: true,
            ..none
        };
        assert_eq!(redraw_message(redraw, 3), Some(7));
        assert_eq!(redraw_message(redraw, 0), None);
        // A view hit callback gives its own code where the form misses.
        let hit = ViewDelegates {
            hit: true,
            shape: true,
            extra_key: true,
            ..none
        };
        assert_eq!(hit_route(hit, -1, 4, || 0), Some(4));
        assert_eq!(hit_route(hit, 3, 4, || 0), None);
        // Without a callback the port's cursor code is still stored.
        assert_eq!(hit_route(none, -1, 4, || unreachable!()), Some(4));
        assert_eq!(shape_route(hit, || 5, || 0), 5);
        assert_eq!(shape_route(none, || 5, || 0), 5);
        assert_eq!(extra_key_route(hit, || 9), 9);
        assert_eq!(extra_key_route(none, || 9), 0);
        // Attaching a view keeps it.
        let mut slot = ViewSlot::default();
        assert_eq!(slot.attach(7), 7);
        assert_eq!(slot.view, Some(7));
    }

    #[test]
    fn the_view_size_follows_the_client_until_fixed() {
        let mut size = ViewSize::default();
        size.resized((100, 50), (64, 64));
        assert_eq!(size.size, (100, 50));
        assert!(size.refresh);
        size.fix();
        size.resized((300, 150), (256, 128));
        assert_eq!(size.size, (256, 128));
        assert_eq!(view_cursor(-1, 4), Some(4));
        assert_eq!(view_cursor(3, 4), None);
    }
}
