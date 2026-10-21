//! The 3D viewport control (`T3DViewport`): the view's size and camera
//! scale, the progressive still-image supersampling and its jitter table,
//! the version and frame-rate overlays, and how pointer and key events are
//! routed to the view's cursor actions.
//!
//! The running app draws the world into a full-window view; this unit holds
//! the viewport's own rules, independent of the renderer. Rules:
//! `docs/UI.md` (Forms), `docs/RENDERING.md`.

/// The viewport form's title.
pub const TITLE: &str = "Viewport";
/// Its style word (a wide frame with a title).
pub const STYLE: u32 = 0x88;
/// Its state word at creation.
pub const STATE: u32 = 0x4c010;
/// The view id before a view is attached.
pub const NO_VIEW: i32 = -1;
/// The form's border (8 pixels).
pub const BORDER: i32 = 8;
/// The frame-rate field's line within the form.
pub const FPS_LINE: i32 = 1;
/// The version overlay's text.
pub const VERSION: &str = "V 0.9.5.2g";

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

/// The level a supersampling setting names, clamped to the recorded eight.
pub fn level(setting: i32) -> SupersampleLevel {
    LEVELS[setting.clamp(0, LEVELS.len() as i32 - 1) as usize]
}

/// The view's progressive still-image supersampling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Still {
    /// The level's setting (an index into [`LEVELS`]).
    pub setting: i32,
    /// The sample being sought next.
    pub index: i32,
    /// A still image is being accumulated.
    pub active: bool,
    /// The view's supersampling counter: a level leaving or entering zero
    /// moves it by one.
    pub counter: i32,
    /// The view's sample count, zero when the level has a single sample.
    pub count: i32,
}

impl Still {
    /// Changes the level: a move away from level 0 bumps the view's sample
    /// counter and clears its still flag; a move back clears the counter.
    /// Returns whether the setting changed, as the original does.
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
        self.active = false;
        self.setting = setting;
        let count = LEVELS[setting as usize].count;
        self.count = if count == 1 { 0 } else { count };
        true
    }

    /// Restarts the still accumulation.
    pub fn reset(&mut self) {
        self.index = 0;
        self.active = false;
    }

    /// The next sample's normalized jitter for a view of `size` drawn into
    /// `viewport` pixels. `None` once the level's samples are done.
    pub fn next(&mut self, size: (i32, i32), viewport: (i32, i32)) -> Option<[f32; 2]> {
        let level = LEVELS[self.setting as usize];
        if self.index >= level.count {
            return None;
        }
        let offset = SAMPLE_OFFSETS[level.base + self.index as usize];
        self.active = true;
        self.index += 1;
        Some(jitter(offset, size, viewport))
    }

    /// The sample index the still accumulation is on.
    pub fn progress(&self) -> StillProgress {
        let level = LEVELS[self.setting as usize];
        let index = self.index;
        StillProgress {
            level: self.setting,
            index,
            count: level.count,
            table: level.base as i32 + index,
            fraction: (index + 1) as f32 / level.count as f32,
            half: 0.5 - (index as f32 + 0.5) / level.count as f32,
            fade: 1.0 - 1.0 / (index as f32 + 1.0),
            done: index >= level.count,
        }
    }
}

/// What one still sample contributes to the overlay and the blend.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StillProgress {
    pub level: i32,
    pub index: i32,
    pub count: i32,
    pub table: i32,
    /// The share accumulated so far, (index + 1) / count.
    pub fraction: f32,
    /// 0.5 − (index + 0.5) / count.
    pub half: f32,
    /// 1 − 1 / (index + 1).
    pub fade: f32,
    pub done: bool,
}

/// The normalized jitter of one sample: twice the point over the view's
/// size, its x over the viewport's width and its y over its height.
pub fn jitter(offset: (f32, f32), size: (i32, i32), viewport: (i32, i32)) -> [f32; 2] {
    let x = if viewport.0 == 0 {
        0.0
    } else {
        size.0 as f32 * 2.0 * offset.0 / viewport.0 as f32
    };
    let y = if viewport.1 == 0 {
        0.0
    } else {
        size.1 as f32 * 2.0 * offset.1 / viewport.1 as f32
    };
    [x, y]
}

/// The camera's distance scale: twice the field of view over the target's
/// distance, clamped to 100..3000 cm.
pub fn camera_scale(field_of_view: f32, distance: f32) -> f32 {
    let distance = distance.clamp(100.0, 3000.0);
    if distance == 0.0 {
        return 0.0;
    }
    field_of_view * 2.0 * 0.5 / distance
}

/// The version overlay's top-left corner: the unit in from the left, the
/// unit up from the client's bottom.
pub fn version_place(client: (i32, i32), unit: i32) -> (i32, i32) {
    (unit, client.1 - unit)
}

/// The frame-rate readout's top-left corner: the line's left is the client
/// width less a unit and the text's advances; the baseline is a line down
/// from a unit times [`FPS_LINE`].
pub fn fps_place(client_width: i32, text_width: i32, unit: i32, line_height: i32) -> (i32, i32) {
    (
        (client_width - unit) - text_width,
        unit * FPS_LINE + line_height,
    )
}

/// The frame-rate readout's text, two decimals and " FPS".
pub fn fps_text(rate: f32) -> String {
    format!("{rate:.2} FPS")
}

/// A view-space point projected into viewport pixels: x and y from the
/// centre over the projection scales, z negated.
pub fn project(point: [f32; 3], size: (i32, i32), projection: (f32, f32)) -> [f32; 3] {
    let [x, y, z] = point;
    let sx = projection.0 * size.0 as f32 * 0.5;
    let sy = projection.1 * size.1 as f32 * 0.5;
    [
        if sx == 0.0 {
            0.0
        } else {
            (x - size.0 as f32 * 0.5) * z / sx
        },
        if sy == 0.0 {
            0.0
        } else {
            (y - size.1 as f32 * 0.5) * z / sy
        },
        -z,
    ]
}

/// A viewport point unprojected into view space; the y is counted from the
/// viewport's bottom, as the original's does.
pub fn unproject(point: [f32; 3], size: (i32, i32), projection: (f32, f32)) -> [f32; 3] {
    let [x, y, z] = point;
    if z >= 0.0 {
        let sx = if x >= 0.0 { size.0 as f32 } else { 0.0 };
        let sy = if y >= 0.0 { size.1 as f32 } else { 0.0 };
        return [sx, size.1 as f32 - sy, -z];
    }
    let sx = size.0 as f32 * 0.5 + projection.0 * size.0 as f32 * 0.5 * x / -z;
    let sy = size.1 as f32 * 0.5 + projection.1 * size.1 as f32 * 0.5 * y / -z;
    [sx, size.1 as f32 - sy, -z]
}

/// A cursor action: the key mask it answers and its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorAction {
    pub mask: u32,
    pub name: &'static str,
}

/// Picks the action with the most mask bits that the word contains (the
/// original ranks by population count and takes the first best).
pub fn pick_action(actions: &[CursorAction], word: u32) -> Option<usize> {
    let mut best = None;
    let mut best_bits = 0;
    for (index, action) in actions.iter().enumerate().rev() {
        if word & action.mask == action.mask {
            let bits = action.mask.count_ones();
            if best_bits < bits {
                best_bits = bits;
                best = Some(index);
            }
        }
    }
    best
}

/// The viewport's key routing state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KeyRoute {
    /// The cursor action a key began, to be suspended at its end.
    pub action: Option<usize>,
    /// The action was closed by the key (bits 5).
    pub close: bool,
}

/// The outcome of routing a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KeyOutcome {
    pub result: u32,
    /// The cursor action to invoke, with the key's word.
    pub invoke: Option<usize>,
    pub close: bool,
    pub beep: bool,
}

impl KeyRoute {
    /// Routes a key down: a suspended action gets the release word first;
    /// if it answers with bit 0 the form's base handler runs; an action
    /// matching the word begins when no control has the focus.
    pub fn key(
        &mut self,
        actions: &[CursorAction],
        word: u32,
        base: u32,
        control_focus: bool,
        suspended: Option<u32>,
    ) -> KeyOutcome {
        if self.action.is_some()
            && let Some(reply) = suspended
        {
            self.action = None;
            if reply & 1 != 0 {
                return KeyOutcome {
                    result: 1,
                    ..Default::default()
                };
            }
        }
        let mut out = KeyOutcome {
            result: base,
            ..Default::default()
        };
        if !control_focus && let Some(index) = pick_action(actions, word) {
            self.action = Some(index);
            out.invoke = Some(index);
            if out.result & 5 == 5 {
                self.close = true;
                out.close = true;
            }
        }
        out
    }

    /// Routes a key up: while an action is active its reply decides; a
    /// reply of 0x40 keeps it suspended, else it ends and the base handler
    /// runs.
    pub fn key_up(&mut self, reply: Option<u32>, base: u32) -> KeyOutcome {
        if self.action.is_none() {
            return KeyOutcome {
                result: base,
                ..Default::default()
            };
        }
        match reply {
            Some(0x40) => KeyOutcome {
                result: 0x40,
                ..Default::default()
            },
            Some(reply) => {
                self.action = None;
                KeyOutcome {
                    result: if reply & 0x80 != 0 { 0x80 } else { base },
                    beep: reply & 3 != 0,
                    ..Default::default()
                }
            }
            None => KeyOutcome {
                result: base,
                ..Default::default()
            },
        }
    }

    /// Escape while an action is active: its reply runs; a redraw beeps and
    /// a bit-0x80 reply ends the action.
    pub fn escape(&mut self, reply: Option<u32>, base: u32) -> KeyOutcome {
        if self.action.is_none() {
            return KeyOutcome {
                result: base,
                ..Default::default()
            };
        }
        let Some(reply) = reply else {
            return KeyOutcome {
                result: 0x80,
                ..Default::default()
            };
        };
        let beep = reply & 3 == 3;
        if reply & 0x80 == 0x80 {
            self.action = None;
        }
        KeyOutcome {
            result: reply,
            beep,
            ..Default::default()
        }
    }

    /// A view event's result asks for a beep when it has the low bits.
    pub fn event_beep(result: u32) -> bool {
        result & 3 != 0
    }

    /// The base pointer handler runs when no button is down; an active
    /// action's callback runs first.
    pub fn pointer(_action: bool, _buttons: u16, base: u32) -> u32 {
        base
    }
}

/// A viewport's size state: its client, the view's target size and whether
/// it is in still mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewSize {
    pub client: (i32, i32),
    pub target: (i32, i32),
    pub still: bool,
}

impl ViewSize {
    pub fn new(client: (i32, i32)) -> Self {
        Self {
            client,
            target: client,
            still: false,
        }
    }

    /// The resize hook: outside still mode the target follows the client;
    /// in still mode the view is resized and its size copied back.
    pub fn resized(&mut self, client: (i32, i32)) -> bool {
        if client == self.client {
            return false;
        }
        self.client = client;
        self.target = client;
        true
    }

    /// Enters still mode and resizes the view.
    pub fn enter_still(&mut self, client: (i32, i32)) {
        self.still = true;
        self.client = client;
        self.target = client;
    }
}

/// The viewport's hit-test fallback: a point the form's own test missed is
/// turned into the view's coordinates and offered to its callback.
pub fn extra_hit(base: i32, translated: Option<i32>) -> (i32, i32) {
    match (base, translated) {
        (-1, Some(code)) => (-1, code),
        (code, _) => (code, 0),
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
        assert_eq!(level(-2).count, 1);
        assert_eq!(level(99).count, 66);
        assert_eq!(SAMPLE_OFFSETS[0], (0.0, 0.0));
        assert_eq!(SAMPLE_OFFSETS[1], (0.246490002, 0.249999002));
        assert_eq!(SAMPLE_OFFSETS[56], (0.297997, -0.417965));
    }

    #[test]
    fn the_still_walks_every_sample_of_its_level() {
        let mut still = Still::default();
        assert!(still.set_setting(1));
        assert!(!still.set_setting(1));
        let size = (100, 100);
        let viewport = (200, 100);
        let first = still.next(size, viewport).unwrap();
        assert_eq!(
            first,
            [
                100.0 * 2.0 * 0.246490002 / 200.0,
                100.0 * 2.0 * 0.249999002 / 100.0
            ]
        );
        assert!(still.next(size, viewport).is_some());
        assert!(still.next(size, viewport).is_none());
        assert_eq!(still.index, 2);
        still.reset();
        assert_eq!(still.index, 0);
    }

    #[test]
    fn the_still_counter_follows_zero() {
        let mut still = Still {
            index: 5,
            ..Default::default()
        };
        still.set_setting(4);
        still.set_setting(0);
        assert_eq!((still.setting, still.index, still.counter), (0, 5, 0));
        still.set_setting(2);
        assert_eq!((still.setting, still.index, still.counter), (2, 5, 1));
    }

    #[test]
    fn a_sample_reports_its_share() {
        let mut still = Still::default();
        still.set_setting(2);
        still.index = 2;
        let p = still.progress();
        assert_eq!((p.index, p.count, p.table, p.done), (2, 3, 5, false));
        assert!((p.fraction - 1.0).abs() < 1e-6);
        assert!((p.half - (0.5 - 2.5 / 3.0)).abs() < 1e-6);
        still.index = 3;
        assert!(still.progress().done);
    }

    #[test]
    fn the_camera_scale_clamps_the_distance() {
        assert!((camera_scale(30.0, 400.0) - 30.0 / 400.0).abs() < 1e-6);
        assert!((camera_scale(30.0, 10.0) - 30.0 / 100.0).abs() < 1e-6);
        assert!((camera_scale(30.0, 9000.0) - 30.0 / 3000.0).abs() < 1e-6);
    }

    #[test]
    fn the_overlays_sit_in_the_unit_grid() {
        assert_eq!(version_place((800, 600), 5), (5, 595));
        assert_eq!(fps_place(800, 60, 5, 16), (735, 21));
        assert_eq!(fps_text(57.25), "57.25 FPS");
    }

    #[test]
    fn projection_and_unprojection_round_trip_the_centre() {
        let size = (200, 100);
        let projection = (1.0, 2.0);
        let p = project([0.0, 0.0, 10.0], size, projection);
        assert_eq!(p[0], -10.0);
        assert_eq!(p[1], -5.0);
        assert_eq!(p[2], -10.0);
        let u = unproject([100.0, 50.0, -10.0], size, projection);
        assert_eq!(u, [1100.0, -450.0, 10.0]);
        assert_eq!(
            unproject([-1.0, 1.0, 0.0], size, projection),
            [0.0, 0.0, -0.0]
        );
    }

    #[test]
    fn the_action_with_the_most_bits_wins() {
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
        ];
        assert_eq!(pick_action(&actions, 0x3), Some(1));
        assert_eq!(pick_action(&actions, 0x2), Some(2));
        assert_eq!(pick_action(&actions, 0x4), None);
    }

    #[test]
    fn keys_begin_and_end_cursor_actions() {
        let actions = [
            CursorAction {
                mask: 0x1,
                name: "a",
            },
            CursorAction {
                mask: 0x101,
                name: "b",
            },
        ];
        let mut route = KeyRoute::default();
        let out = route.key(&actions, 0x101, 0, false, None);
        assert_eq!((out.invoke, route.action), (Some(1), Some(1)));
        assert!(!out.close);
        // The key up with 0x40 keeps it suspended.
        let out = route.key_up(Some(0x40), 0);
        assert_eq!(out.result, 0x40);
        assert_eq!(route.action, Some(1));
        // A key down whose suspended action answers bit 0 runs the base.
        let out = route.key(&actions, 0x1, 7, true, Some(0x80));
        assert_eq!(out.result, 7);
        assert_eq!(route.action, None);
        assert!(KeyRoute::event_beep(3) && !KeyRoute::event_beep(0x80));
    }

    #[test]
    fn escape_and_resize_rules() {
        let mut route = KeyRoute {
            action: Some(0),
            close: false,
        };
        let out = route.escape(Some(3), 9);
        assert!(out.beep);
        assert_eq!(out.result, 3);
        let out = route.escape(Some(0x80), 9);
        assert_eq!(route.action, None);
        assert_eq!(out.result, 0x80);
        let mut size = ViewSize::new((100, 50));
        assert!(!size.resized((100, 50)));
        size.enter_still((200, 100));
        assert!(size.resized((300, 150)));
        assert_eq!(size.target, (300, 150));
        assert_eq!(extra_hit(-1, Some(4)), (-1, 4));
        assert_eq!(extra_hit(3, Some(4)), (3, 0));
    }
}
