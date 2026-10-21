//! Gamepad control, as the original's Steam Input action sets describe it:
//! the menu set and the two gameplay sets (interact and combat), the actions
//! each holds (digital or analog), how a frame of pad input becomes the
//! menu's navigation commands and the player's command word, the look and
//! aim sticks, the camera's zoom, rumble, and the focus stack that picks the
//! active set. Steam itself is not used: Bevy's gamepads are read directly
//! and the action names are kept only as the original's manifest names.
//!
//! Rules and the default binding table: `docs/PLAYER.md` (Gamepad).

use bevy::input::gamepad::{AxisSettings, Gamepad, GamepadAxis, GamepadButton, GamepadSettings};
use bevy::prelude::*;

/// The action sets the original registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Set {
    /// The menu set, `Set_Menus`: one focused form's navigation and buttons.
    Menus,
    /// The gameplay set used while the game is not in combat mode,
    /// `Set_Interact` (its actions end in `I`).
    Interact,
    /// The gameplay set used in combat mode, `Set_Combat` (its actions end
    /// in `C`).
    Combat,
}

/// Which of the two gameplay sets drives the pad's gameplay actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Variant {
    /// `Set_Interact`: `MoveI`, `CameraI`, and so on.
    #[default]
    Interact,
    /// `Set_Combat`: `MoveC`, `CameraC`, and so on.
    Combat,
}

impl Variant {
    /// The set this variant names.
    pub fn set(self) -> Set {
        match self {
            Variant::Interact => Set::Interact,
            Variant::Combat => Set::Combat,
        }
    }
}

impl From<Variant> for Set {
    fn from(variant: Variant) -> Self {
        variant.set()
    }
}

/// Whether a Steam Input action is digital or analog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// A button: pressed or not, with an "active" flag in the action data.
    Digital,
    /// A pair of axes, `x` and `y`, each in -1 to 1.
    Analog,
}

/// The original's gamepad actions. Actions with an `I` and a `C` name are
/// one Exarust action read from whichever gameplay set is active; the
/// manifest name follows the set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Id {
    /// `MenuNav`: the menu's navigation stick (directional steps).
    MenuNav,
    /// `MenuScr`: the menu's scroll stick.
    MenuScreen,
    /// `MenuSel`: confirm the focused control.
    MenuSelect,
    /// `MenuBak`: go forward/back in the form's own sense.
    MenuBack,
    /// `MenuCur`: the menu cursor's movement (resolved by the original but
    /// never read by its menu frame).
    MenuCursor,
    /// `MenuLMB`: a left click in the menu (resolved, never read here).
    MenuLeftButton,
    /// `MenuRMB`: a right click in the menu (resolved, never read here).
    MenuRightButton,
    /// `MenuExit`: leave the menu.
    MenuExit,
    /// `Move`: the movement stick.
    Move,
    /// `Camera`: the camera's look stick.
    Camera,
    /// `Aim`: the aiming direction, turned into the camera's frame.
    Aim,
    /// `Sprint`: run and sprint.
    Sprint,
    /// `Combat`: the primary combat action (Exarust uses it as the attack).
    Combat,
    /// `WepSwp`: switch equipped.
    SwapWeapon,
    /// `Powers`: the power bar.
    Powers,
    /// `UsePower`: use the selected power.
    UsePower,
    /// `PowerTree`: the power tree.
    PowerTree,
    /// `Talk`: talk to the character under the cursor.
    Talk,
    /// `Select`: use or select what the cursor is over.
    Select,
    /// `Look`: Cursor Look.
    Look,
    /// `Zoom_In`: zoom in while held.
    ZoomIn,
    /// `Zoom_Out`: zoom out while held.
    ZoomOut,
    /// `CamMode`: the camera mode.
    CameraMode,
    /// `pause_menu`: open the pause menu.
    PauseMenu,
    /// `Inventory`: the inventory (interact set only).
    Inventory,
    /// `Skills`: the skill screen (interact set only).
    Skills,
    /// `CrouchC`: crouch (combat set only).
    Crouch,
    /// `AltAction`: the alternate attack (combat set only).
    Alternate,
}

impl Id {
    /// Every action, in the order the original resolves them.
    pub const ALL: [Id; 28] = [
        Id::MenuNav,
        Id::MenuScreen,
        Id::MenuSelect,
        Id::MenuBack,
        Id::MenuCursor,
        Id::MenuLeftButton,
        Id::MenuRightButton,
        Id::MenuExit,
        Id::Move,
        Id::Camera,
        Id::Aim,
        Id::Sprint,
        Id::Combat,
        Id::SwapWeapon,
        Id::Powers,
        Id::UsePower,
        Id::PowerTree,
        Id::Talk,
        Id::Select,
        Id::Look,
        Id::ZoomIn,
        Id::ZoomOut,
        Id::CameraMode,
        Id::PauseMenu,
        Id::Inventory,
        Id::Skills,
        Id::Crouch,
        Id::Alternate,
    ];
    /// How many actions there are.
    pub const COUNT: usize = Self::ALL.len();

    /// The action's index in [`Id::ALL`].
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Whether the action is digital or analog.
    pub const fn kind(self) -> Kind {
        match self {
            Id::MenuNav | Id::MenuScreen | Id::MenuCursor | Id::Move | Id::Camera | Id::Aim => {
                Kind::Analog
            }
            _ => Kind::Digital,
        }
    }

    /// Whether the action belongs to `set`.
    pub fn in_set(self, set: Set) -> bool {
        match self {
            Id::MenuNav
            | Id::MenuScreen
            | Id::MenuSelect
            | Id::MenuBack
            | Id::MenuCursor
            | Id::MenuLeftButton
            | Id::MenuRightButton
            | Id::MenuExit => set == Set::Menus,
            Id::Inventory | Id::Skills => set == Set::Interact,
            Id::Crouch | Id::Alternate => set == Set::Combat,
            _ => matches!(set, Set::Interact | Set::Combat),
        }
    }

    /// The action's name in the original's manifest, as `set` holds it.
    /// `MenuCur`, `MenuLMB` and `MenuRMB` are the original's names.
    pub const fn manifest_name(self, set: Set) -> &'static str {
        let variant = matches!(set, Set::Combat);
        match self {
            Id::MenuNav => "MenuNav",
            Id::MenuScreen => "MenuScr",
            Id::MenuSelect => "MenuSel",
            Id::MenuBack => "MenuBak",
            Id::MenuCursor => "MenuCur",
            Id::MenuLeftButton => "MenuLMB",
            Id::MenuRightButton => "MenuRMB",
            Id::MenuExit => "MenuExit",
            Id::Move => {
                if variant {
                    "MoveC"
                } else {
                    "MoveI"
                }
            }
            Id::Camera => {
                if variant {
                    "CameraC"
                } else {
                    "CameraI"
                }
            }
            Id::Aim => {
                if variant {
                    "AimC"
                } else {
                    "AimI"
                }
            }
            Id::Sprint => {
                if variant {
                    "SprintC"
                } else {
                    "SprintI"
                }
            }
            Id::Combat => {
                if variant {
                    "CombatC"
                } else {
                    "CombatI"
                }
            }
            Id::SwapWeapon => {
                if variant {
                    "WepSwpC"
                } else {
                    "WepSwpI"
                }
            }
            Id::Powers => {
                if variant {
                    "PowersC"
                } else {
                    "PowersI"
                }
            }
            Id::UsePower => {
                if variant {
                    "UsePowerC"
                } else {
                    "UsePowerI"
                }
            }
            Id::PowerTree => {
                if variant {
                    "PowerTreeC"
                } else {
                    "PowerTreeI"
                }
            }
            Id::Talk => {
                if variant {
                    "TalkC"
                } else {
                    "TalkI"
                }
            }
            Id::Select => {
                if variant {
                    "SelectC"
                } else {
                    "SelectI"
                }
            }
            Id::Look => {
                if variant {
                    "LookC"
                } else {
                    "LookI"
                }
            }
            Id::ZoomIn => {
                if variant {
                    "Zoom_InC"
                } else {
                    "Zoom_InI"
                }
            }
            Id::ZoomOut => {
                if variant {
                    "Zoom_OutC"
                } else {
                    "Zoom_OutI"
                }
            }
            Id::CameraMode => {
                if variant {
                    "CamModeC"
                } else {
                    "CamModeI"
                }
            }
            Id::PauseMenu => {
                if variant {
                    "pause_menuC"
                } else {
                    "pause_menuI"
                }
            }
            Id::Inventory => "Inventory",
            Id::Skills => "Skills",
            Id::Crouch => "CrouchC",
            Id::Alternate => "AltAction",
        }
    }
}

/// A control a default binding reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Control {
    /// A digital button.
    Button(GamepadButton),
    /// A stick's horizontal and vertical axes, in that order.
    Stick(GamepadAxis, GamepadAxis),
}

/// The default mapping from the original's actions to Bevy's gamepad
/// controls, one entry per action ([`Id::ALL`] order). The Steam manifest's
/// physical bindings are not in the game's code, so this is Exarust's own
/// table; it is documented in `docs/PLAYER.md`. The two gameplay sets share
/// it.
pub const DEFAULT_CONTROLS: [(Id, Control); Id::COUNT] = [
    (
        Id::MenuNav,
        Control::Stick(GamepadAxis::LeftStickX, GamepadAxis::LeftStickY),
    ),
    (
        Id::MenuScreen,
        Control::Stick(GamepadAxis::RightStickX, GamepadAxis::RightStickY),
    ),
    (Id::MenuSelect, Control::Button(GamepadButton::South)),
    (Id::MenuBack, Control::Button(GamepadButton::East)),
    (
        Id::MenuCursor,
        Control::Stick(GamepadAxis::RightStickX, GamepadAxis::RightStickY),
    ),
    (
        Id::MenuLeftButton,
        Control::Button(GamepadButton::RightTrigger2),
    ),
    (
        Id::MenuRightButton,
        Control::Button(GamepadButton::LeftTrigger2),
    ),
    (Id::MenuExit, Control::Button(GamepadButton::Start)),
    (
        Id::Move,
        Control::Stick(GamepadAxis::LeftStickX, GamepadAxis::LeftStickY),
    ),
    (
        Id::Camera,
        Control::Stick(GamepadAxis::RightStickX, GamepadAxis::RightStickY),
    ),
    (
        Id::Aim,
        Control::Stick(GamepadAxis::RightStickX, GamepadAxis::RightStickY),
    ),
    (Id::Sprint, Control::Button(GamepadButton::LeftThumb)),
    (Id::Combat, Control::Button(GamepadButton::RightTrigger2)),
    (Id::SwapWeapon, Control::Button(GamepadButton::North)),
    (Id::Powers, Control::Button(GamepadButton::DPadUp)),
    (Id::UsePower, Control::Button(GamepadButton::West)),
    (Id::PowerTree, Control::Button(GamepadButton::DPadDown)),
    (Id::Talk, Control::Button(GamepadButton::DPadLeft)),
    (Id::Select, Control::Button(GamepadButton::South)),
    (Id::Look, Control::Button(GamepadButton::RightThumb)),
    (Id::ZoomIn, Control::Button(GamepadButton::RightTrigger)),
    (Id::ZoomOut, Control::Button(GamepadButton::LeftTrigger)),
    (Id::CameraMode, Control::Button(GamepadButton::Mode)),
    (Id::PauseMenu, Control::Button(GamepadButton::Start)),
    (Id::Inventory, Control::Button(GamepadButton::Select)),
    (Id::Skills, Control::Button(GamepadButton::DPadRight)),
    (Id::Crouch, Control::Button(GamepadButton::East)),
    (Id::Alternate, Control::Button(GamepadButton::LeftTrigger2)),
];

/// The default control bound to `id`.
pub fn default_control(id: Id) -> Control {
    DEFAULT_CONTROLS[id.index()].1
}

/// A frame of the pad's action data: each action's digital state and analog
/// value, and whether the controller is active (Steam's "active" flag; for
/// Bevy, a gamepad is connected).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PadSample {
    /// A gamepad is connected and its actions are active.
    pub connected: bool,
    digital: [bool; Id::COUNT],
    analog: [Vec2; Id::COUNT],
}

impl Default for PadSample {
    fn default() -> Self {
        Self {
            connected: false,
            digital: [false; Id::COUNT],
            analog: [Vec2::ZERO; Id::COUNT],
        }
    }
}

impl PadSample {
    /// Whether `id` is pressed.
    pub fn digital(&self, id: Id) -> bool {
        self.digital[id.index()]
    }

    /// `id`'s analog axes.
    pub fn analog(&self, id: Id) -> Vec2 {
        self.analog[id.index()]
    }

    /// Sets `id`'s digital state.
    pub fn with_digital(mut self, id: Id, pressed: bool) -> Self {
        self.digital[id.index()] = pressed;
        self
    }

    /// Sets `id`'s analog value.
    pub fn with_analog(mut self, id: Id, value: Vec2) -> Self {
        self.analog[id.index()] = value;
        self
    }

    /// Sets `id`'s digital state in place.
    pub fn set_digital(&mut self, id: Id, pressed: bool) {
        self.digital[id.index()] = pressed;
    }

    /// Sets `id`'s analog value in place.
    pub fn set_analog(&mut self, id: Id, value: Vec2) {
        self.analog[id.index()] = value;
    }
}

/// The GUI's directional key codes the menu reader emits: left, right, down
/// and up, with 1 added to each. The scroll stick uses the same directions
/// with 2 added instead.
pub mod gui_key {
    /// Up: the previous row.
    pub const UP: u32 = 0x4001;
    /// Down: the next row.
    pub const DOWN: u32 = 0x8001;
    /// Left.
    pub const LEFT: u32 = 0x1001;
    /// Right.
    pub const RIGHT: u32 = 0x2001;
    /// Up, from the scroll stick.
    pub const SCROLL_UP: u32 = 0x4002;
    /// Down, from the scroll stick.
    pub const SCROLL_DOWN: u32 = 0x8002;
    /// Left, from the scroll stick.
    pub const SCROLL_LEFT: u32 = 0x1002;
    /// Right, from the scroll stick.
    pub const SCROLL_RIGHT: u32 = 0x2002;
    /// A navigation step came from the stick (the original's analog bit).
    pub const ANALOG: u32 = 0x10;
    /// A scroll step came from the stick.
    pub const SCROLL_ANALOG: u32 = 0x20;
    /// The confirm key: `MenuSel`.
    pub const SELECT: u32 = 0x100;
    /// The forward key: `MenuBak`.
    pub const FORWARD: u32 = 0x200;
    /// The back key: `MenuExit`.
    pub const BACK: u32 = 0x400;
}

/// The menu navigation stick's radial dead zone: the sum of the squared
/// axes must be above 0.25.
pub const MENU_DEAD_ZONE: f32 = 0.25;
/// The first direction of a stick repeats after 400 ms of the real clock.
pub const MENU_FIRST_REPEAT_MS: i64 = 400;
/// Further directions repeat every 150 ms...
pub const MENU_REPEAT_MS: i64 = 150;
/// ...or every 30 ms in a form that asks for fast repeat.
pub const MENU_FAST_REPEAT_MS: i64 = 30;
/// Input is ignored for this long after a gameplay set is activated, while
/// the original primes its held states.
pub const PRIME_MS: i64 = 3;
/// The gameplay stick's "fast" threshold: the squared magnitude above which
/// the move is a run (the original's `0.8 < x² + y²`).
pub const MOVE_FAST: f32 = 0.8;
/// The step modes' axis dead zone.
pub const STEP_DEAD_ZONE: f32 = 0.1;
/// A step repeats this often (ms) while its stick stays pushed.
pub const STEP_REPEAT_MS: i64 = 300;
/// A held `Powers` adds its second command after this long (ms).
pub const POWERS_HOLD_MS: i64 = 500;

/// What the menu reader knows about the focused form.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MenuContext {
    /// A form has the focus (the focus stack is not empty).
    pub focused: bool,
    /// The menu action set is active.
    pub ready: bool,
    /// The focused form wants the navigation stick's repeats fast (its
    /// saved set word has bit 0x2000).
    pub fast_nav: bool,
    /// The same for the scroll stick (bit 0x4000).
    pub fast_scroll: bool,
    /// The form has an exit hook, which takes `MenuExit` instead of the
    /// normal dispatch.
    pub exit_hook: bool,
}

/// One menu frame's output.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MenuFrame {
    /// The key word a form's key method receives: the newly pressed
    /// `MenuSel`/`MenuBak`/`MenuExit` bits and this frame's direction.
    pub key: u32,
    /// The menu cursor's movement, `(dx, dy)` with `y` down.
    pub cursor: Vec2,
    /// The scroll stick's movement.
    pub scroll_cursor: Vec2,
}

/// The menu set's reader, kept between frames: the real clock stamp, the
/// pressed-key latch, the repeat deadlines and the last cursors.
#[derive(Resource, Debug, Clone, Default)]
pub struct MenuPad {
    last_ms: i64,
    primed: bool,
    prime_at: i64,
    mask: u32,
    repeat_at: i64,
    scroll_repeat_at: i64,
    cursor: Vec2,
    scroll_cursor: Vec2,
}

impl MenuPad {
    /// One frame of the menu set at `now_ms` (the real clock).
    pub fn frame(&mut self, now_ms: i64, sample: &PadSample, ctx: MenuContext) -> MenuFrame {
        if !ctx.focused || !ctx.ready {
            self.last_ms = now_ms;
            return self.out();
        }
        if self.primed {
            if now_ms < self.prime_at + PRIME_MS {
                return self.out();
            }
            self.prime(sample);
            self.primed = false;
        }
        if !sample.connected {
            self.primed = true;
            self.prime_at = now_ms;
            return self.out();
        }
        let previous = self.mask;
        self.mask = 0;
        if sample.digital(Id::MenuSelect) {
            self.mask |= gui_key::SELECT;
        }
        self.cursor = Vec2::ZERO;
        let dt = (now_ms - self.last_ms) as f32 * 0.0625;
        self.last_ms = now_ms;
        let mut direction = 0;
        let nav = sample.analog(Id::MenuNav);
        let nav_mag = nav.x * nav.x + nav.y * nav.y;
        if nav_mag > MENU_DEAD_ZONE {
            if now_ms > self.repeat_at {
                direction = Self::direction(nav, false);
                direction |= gui_key::ANALOG;
                self.cursor = Vec2::new(nav.x, -nav.y) * dt;
                if self.repeat_at == 0 {
                    self.repeat_at = now_ms + MENU_FIRST_REPEAT_MS;
                } else if ctx.fast_nav {
                    self.scroll_repeat_at = now_ms + MENU_FAST_REPEAT_MS;
                } else {
                    self.repeat_at = now_ms + MENU_REPEAT_MS;
                }
            }
        } else {
            self.repeat_at = 0;
        }
        if direction & gui_key::ANALOG == 0 {
            self.scroll_cursor = Vec2::ZERO;
            let scroll = sample.analog(Id::MenuScreen);
            let scroll_mag = scroll.x * scroll.x + scroll.y * scroll.y;
            if scroll_mag > MENU_DEAD_ZONE {
                if now_ms > self.scroll_repeat_at {
                    direction = Self::direction(scroll, true);
                    direction |= gui_key::SCROLL_ANALOG;
                    self.scroll_cursor = Vec2::new(scroll.x, -scroll.y) * dt;
                    if self.scroll_repeat_at == 0 {
                        self.scroll_repeat_at = now_ms + MENU_FIRST_REPEAT_MS;
                    } else if ctx.fast_scroll {
                        self.scroll_repeat_at = now_ms + MENU_FAST_REPEAT_MS;
                    } else {
                        self.scroll_repeat_at = now_ms + MENU_REPEAT_MS;
                    }
                }
            } else {
                self.scroll_repeat_at = 0;
            }
        }
        if sample.digital(Id::MenuBack) {
            self.mask |= gui_key::FORWARD;
        }
        if sample.digital(Id::MenuExit) {
            self.mask |= gui_key::BACK;
            if ctx.exit_hook {
                return if previous & gui_key::BACK == 0 {
                    MenuFrame {
                        key: gui_key::BACK,
                        ..MenuFrame::default()
                    }
                } else {
                    MenuFrame::default()
                };
            }
        }
        let key = (!previous & self.mask) | direction;
        MenuFrame {
            key,
            cursor: self.cursor,
            scroll_cursor: self.scroll_cursor,
        }
    }

    /// The directional key for a stick, `scrolled` for the scroll stick's
    /// codes: the dominant axis, vertical on a tie, with the raw axis's
    /// sign.
    fn direction(axes: Vec2, scrolled: bool) -> u32 {
        let (left, right, down, up) = if scrolled {
            (
                gui_key::SCROLL_LEFT,
                gui_key::SCROLL_RIGHT,
                gui_key::SCROLL_DOWN,
                gui_key::SCROLL_UP,
            )
        } else {
            (gui_key::LEFT, gui_key::RIGHT, gui_key::DOWN, gui_key::UP)
        };
        if axes.x.abs() <= axes.y.abs() {
            if axes.y <= 0.0 { down } else { up }
        } else if axes.x <= 0.0 {
            left
        } else {
            right
        }
    }

    /// The held keys a newly active set starts from, so a button already
    /// down is not a press: `MenuExit`, `MenuSel` and `MenuBak`.
    fn prime(&mut self, sample: &PadSample) {
        self.mask = 0;
        if sample.digital(Id::MenuExit) {
            self.mask |= gui_key::BACK;
        }
        if sample.digital(Id::MenuSelect) {
            self.mask |= gui_key::SELECT;
        }
        if sample.digital(Id::MenuBack) {
            self.mask |= gui_key::FORWARD;
        }
    }

    /// The last frame's output, unchanged.
    fn out(&self) -> MenuFrame {
        MenuFrame {
            key: 0,
            cursor: self.cursor,
            scroll_cursor: self.scroll_cursor,
        }
    }

    /// Makes the next frame wait for the three milliseconds after
    /// `now_ms` and prime the held keys, as activating the menu set does.
    pub fn activate(&mut self, now_ms: i64) {
        self.primed = true;
        self.prime_at = now_ms;
    }
}

/// A control a directional menu pick can land on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PickTarget {
    /// The control's id, which the form's focus call receives with 0x10000
    /// set when the form is not full screen.
    pub id: i32,
    /// The control's centre in the focused form's client pixels.
    pub centre: IVec2,
    /// The control takes a pick (its record's flags allow it).
    pub reachable: bool,
    /// The control is the focused one, which a pick skips.
    pub current: bool,
}

/// The lateral reach of a vertical pick, as the original's `4 * 80` client
/// pixels: a control further sideways than this is not straight above or
/// below.
pub const PICK_REACH: i64 = 320;
/// The minimum alignment a horizontal pick needs (the dot product of the
/// normalized offset and the stick's direction).
pub const PICK_ALIGN: f32 = 0.3;
/// The vertical offset of a horizontal pick is stretched by this, so the
/// pick prefers controls across the row rather than down a column.
pub const PICK_STRETCH: f32 = 8.0;
/// The original's smallest accepted direction product: the `f32` whose bit
/// pattern is 4, a denormal, so the test is "same sign as the stick".
const PICK_SIGN_BIT: f32 = f32::from_bits(4);

/// The control under a menu stick's direction, as the original's pick
/// walks the focused form: the nearest control in the stick's direction
/// from `origin` (the current control's centre). Vertical picks take the
/// nearest control within [`PICK_REACH`] pixels sideways; horizontal picks
/// score the offset with its `y` stretched by [`PICK_STRETCH`] and take the
/// smallest length divided by the alignment, at [`PICK_ALIGN`] or more.
/// The current control and unreachable controls are skipped.
pub fn pick_control(cursor: Vec2, origin: IVec2, targets: &[PickTarget]) -> Option<i32> {
    let vertical = cursor.x.abs() <= cursor.y.abs();
    let length = (cursor.x * cursor.x + cursor.y * cursor.y).sqrt();
    let dir = if length > 0.0 {
        Vec2::new(cursor.x / length, cursor.y / length)
    } else {
        Vec2::ZERO
    };
    let mut best_x = 100_000_i64;
    let mut best_y = 100_000_i64;
    let mut best_score = 1_000_000.0_f32;
    let mut best = None;
    for target in targets {
        if target.current || !target.reachable {
            continue;
        }
        let dx = (target.centre.x - origin.x) as f32;
        let dy = (target.centre.y - origin.y) as f32;
        let (along, stick) = if vertical {
            (dy, cursor.y)
        } else {
            (dx, cursor.x)
        };
        if stick * along <= PICK_SIGN_BIT {
            continue;
        }
        if vertical {
            let ax = dx.abs().round_ties_even() as i64;
            let ay = dy.abs().round_ties_even() as i64;
            if ax <= PICK_REACH && ay <= best_y && (ay < best_y || ax < best_x) {
                best_x = ax;
                best_y = ay;
                best = Some(target.id);
            }
        } else {
            let offset = Vec2::new(dx, dy * PICK_STRETCH);
            let length = offset.length();
            let score = if length > 0.0 {
                offset.dot(dir) / length
            } else {
                0.0
            };
            if score >= PICK_ALIGN {
                let ratio = length / score;
                if ratio < best_score {
                    best_score = ratio;
                    best = Some(target.id);
                }
            }
        }
    }
    best
}

/// The active action set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveSet {
    /// The menu set.
    Menus,
    /// Registered gameplay set `index` (zero-based) with its variant.
    Gameplay { index: usize, variant: Variant },
}

/// The registered gameplay sets and the active set, as the original's set
/// table holds them: the interact set and the combat set, registered in that
/// order, with the interact set active at start-up. Switching a set primes
/// the input reader, as the original's activation callback does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetRegistry {
    names: Vec<&'static str>,
    active: ActiveSet,
}

impl Default for SetRegistry {
    fn default() -> Self {
        Self {
            names: vec!["Set_Interact", "Set_Combat"],
            active: ActiveSet::Gameplay {
                index: 0,
                variant: Variant::Interact,
            },
        }
    }
}

impl SetRegistry {
    /// The registered set names, in index order.
    pub fn names(&self) -> &[&'static str] {
        &self.names
    }

    /// The active set.
    pub fn active(&self) -> ActiveSet {
        self.active
    }

    /// The gameplay set's variant, or `None` for the menus.
    pub fn variant(&self) -> Option<Variant> {
        match self.active {
            ActiveSet::Menus => None,
            ActiveSet::Gameplay { variant, .. } => Some(variant),
        }
    }

    /// Registers a gameplay set with `name`; returns its index, or `None`
    /// when the table is full (the original's registration fails when its
    /// eight-entry table is full).
    pub fn register(&mut self, name: &'static str) -> Option<usize> {
        if self.names.len() >= 8 {
            return None;
        }
        self.names.push(name);
        Some(self.names.len() - 1)
    }

    /// Activates set `index`: 0 is the menus; 1 and up are the registered
    /// gameplay sets. Returns whether the active set changed.
    pub fn activate(&mut self, index: usize) -> bool {
        let next = if index == 0 {
            ActiveSet::Menus
        } else {
            let at = index - 1;
            let variant = match self.names.get(at) {
                Some(&"Set_Combat") => Variant::Combat,
                _ => Variant::Interact,
            };
            ActiveSet::Gameplay { index: at, variant }
        };
        let changed = next != self.active;
        self.active = next;
        changed
    }

    /// Activates the set a focus change names.
    pub fn apply(&mut self, change: FocusChange) -> bool {
        match change {
            FocusChange::Menus => self.activate(0),
            FocusChange::Gameplay(index) => self.activate(index + 1),
            FocusChange::None => false,
        }
    }

    /// Switches to the variant's gameplay set.
    pub fn set_variant(&mut self, variant: Variant) -> bool {
        let index = self
            .names
            .iter()
            .position(|name| *name == variant.set_name())
            .map_or(0, |at| at + 1);
        self.activate(index)
    }
}

impl Variant {
    /// The original's manifest name for the set.
    pub const fn set_name(self) -> &'static str {
        match self {
            Variant::Interact => "Set_Interact",
            Variant::Combat => "Set_Combat",
        }
    }
}

/// The key a picked control receives: its id, or its id with 0x10000 set
/// when the form is not full screen (the original's send-key rule).
pub fn picked_key(id: i32, full_screen: bool) -> i32 {
    if full_screen { id } else { id | 0x1_0000 }
}

/// The raw gamepad fallback's move, from a 16-bit stick pair: each axis over
/// 32768, turned into the camera's frame, with the squared magnitude over
/// 0.8 marked fast. The original uses this only when Steam Input is not
/// available; Exarust's Bevy reader replaces it, but the rule is kept.
pub fn raw_move(x: i16, y: i16, camera_right: Vec3, camera_forward: Vec3) -> (Vec3, bool) {
    const SCALE: f32 = 3.051_757_8e-05;
    let x = f32::from(x) * SCALE;
    let y = f32::from(y) * SCALE;
    let right = camera_right.normalize_or_zero();
    let forward = camera_forward.normalize_or_zero();
    let move_world = (right * x + forward * y).with_y(0.0);
    (move_world, 0.8 < f64::from(x * x + y * y))
}

/// Whether the raw fallback's button word has the one button the original
/// maps (bit 0x40).
pub fn raw_button(buttons: u32) -> bool {
    buttons & 0x40 != 0
}

/// One form on the focus stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusEntry {
    /// The form's id.
    pub form: u64,
    /// The set word the form pushed with (its low byte names a registered
    /// gameplay set when bit 0x8000 is set).
    pub flags: u32,
    /// Whether the form is full screen.
    pub full_screen: bool,
}

/// What a focus change asks of the action sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusChange {
    /// The menus set.
    Menus,
    /// Registered gameplay set `index` (zero-based; 0 is the interact set
    /// when both sets registered in the original's order).
    Gameplay(usize),
    /// No change.
    None,
}

/// The GUI form focus stack: up to eight forms, each with the set it wants.
/// Pushing a form already on the stack moves it to the top. The bottom
/// form's push activates the menus set; another form's push activates the
/// set its flags name (or keeps the menus). Popping restores the new top's
/// set.
#[derive(Resource, Debug, Clone, Default)]
pub struct FocusStack {
    entries: Vec<FocusEntry>,
}

impl FocusStack {
    /// The most forms the original keeps.
    pub const LIMIT: usize = 8;

    /// The forms, bottom first.
    pub fn entries(&self) -> &[FocusEntry] {
        &self.entries
    }

    /// Whether a form has the focus.
    pub fn focused(&self) -> bool {
        !self.entries.is_empty()
    }

    /// How many forms are stacked.
    pub fn depth(&self) -> usize {
        self.entries.len()
    }

    /// The set the top form asks for, if any.
    pub fn top_change(&self) -> FocusChange {
        self.entries.last().map_or(FocusChange::None, Self::change)
    }

    fn change(entry: &FocusEntry) -> FocusChange {
        if entry.flags & 0x8000 != 0 && entry.flags & 0xff != 0 {
            FocusChange::Gameplay((entry.flags & 0xff) as usize - 1)
        } else {
            FocusChange::Menus
        }
    }

    /// Pushes `entry`: a duplicate moves to the top, and a ninth form is
    /// refused. Returns the set the push activates: the set its flags name,
    /// or the menus.
    pub fn push(&mut self, entry: FocusEntry) -> FocusChange {
        if self.entries.len() >= Self::LIMIT {
            return FocusChange::None;
        }
        if let Some(at) = self.entries.iter().position(|e| e.form == entry.form) {
            self.entries.remove(at);
        }
        self.entries.push(entry);
        Self::change(&entry)
    }

    /// Pops `form`, wherever it is; returns the set the new top asks for
    /// (or the menus when none is left), or none when it was not on the
    /// stack.
    pub fn pop(&mut self, form: u64) -> FocusChange {
        let Some(at) = self.entries.iter().position(|e| e.form == form) else {
            return FocusChange::None;
        };
        self.entries.remove(at);
        self.top_change_or_menus()
    }

    fn top_change_or_menus(&self) -> FocusChange {
        self.entries.last().map_or(FocusChange::Menus, Self::change)
    }
}

/// The gameplay stick's mode: the original's `0x10056a370` flag reads the
/// move stick's horizontal axis as left/right steps, and its `0x100569fe0`
/// flag reads the vertical axis as forward/back steps; with neither set the
/// stick is analog. Their owners are outside this component; Exarust leaves
/// both off and uses the analog stick everywhere.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StepModes {
    /// `0x10056a370`: the horizontal axis steps left and right.
    pub across: bool,
    /// `0x100569fe0`: the vertical axis steps forward and back.
    pub along: bool,
}

/// The player command word's bits, as the original's player controls read
/// them.
pub mod command {
    /// The primary combat action (`CombatI`/`CombatC`).
    pub const ATTACK: u32 = 0x1;
    /// Switch equipped (`WepSwp`).
    pub const SWAP_WEAPON: u32 = 0x2;
    /// Step left (the across mode's negative axis).
    pub const STEP_LEFT: u32 = 0x10;
    /// Step right (the across mode's positive axis).
    pub const STEP_RIGHT: u32 = 0x20;
    /// Step forward (the along mode's positive axis).
    pub const STEP_FORWARD: u32 = 0x40;
    /// Step back (the along mode's negative axis).
    pub const STEP_BACK: u32 = 0x80;
    /// The inventory (`Inventory`, interact set only).
    pub const INVENTORY: u32 = 0x100;
    /// The skill screen (`Skills`, interact set only).
    pub const SKILLS: u32 = 0x200;
    /// Use the selected power (`UsePower`).
    pub const USE_POWER: u32 = 0x1000;
    /// The power bar (`Powers`).
    pub const POWER_BAR: u32 = 0x2000;
    /// The power bar's second command after 500 ms held.
    pub const POWER_HOLD: u32 = 0x4000;
    /// Use or select (`Select`).
    pub const USE: u32 = 0x10000;
    /// Talk (`Talk`).
    pub const TALK: u32 = 0x20000;
    /// Cursor Look (`Look`), held.
    pub const LOOK: u32 = 0x0010_0000;
    /// The camera mode (`CamMode`), held.
    pub const CAMERA_MODE: u32 = 0x0020_0000;
    /// The pause menu (`pause_menu`).
    pub const PAUSE: u32 = 0x1000_0000;
    /// The bits the gamepad reader owns in the command word: the original
    /// clears bits 20 to 23 (the look and camera-mode bits) and keeps every
    /// other bit for other input.
    pub const GAMEPAD_MASK: u32 = 0xff0f_ffff;
}

/// One frame of gameplay pad input, with the original's dead zones and
/// arithmetic already applied.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayFrame {
    /// The move stick's raw axes.
    pub move_raw: Vec2,
    /// The move stick turned into the world by the camera's basis, level.
    pub move_world: Vec3,
    /// The squared move magnitude is over 0.8 (the original's run threshold).
    pub move_fast: bool,
    /// The aim stick's raw axes.
    pub aim_raw: Vec2,
    /// The aim stick turned into the world by the camera's basis.
    pub aim_world: Vec3,
    /// The camera stick's processed axes: `x` squared past its 0.1 dead
    /// zone, `y` past its 0.5 dead zone, before the 30 and 10 gains.
    pub look: Vec2,
    /// The camera's turn this frame (rad): yaw and pitch.
    pub look_delta: Vec2,
    /// The camera's zoom change this frame (Exarust's 800–2000 scale).
    pub zoom: f32,
    /// The player command word after this frame.
    pub commands: u32,
    /// The digital state bits the pad set this frame.
    pub state: u32,
    /// The bits newly pressed this frame (a held button is not a press).
    pub pressed: u32,
    /// The step bit a step mode repeated this frame, if any.
    pub step: u32,
    /// `Sprint` is held.
    pub sprint: bool,
    /// `AltAction` is held (combat set).
    pub alternate: bool,
    /// `CrouchC` is held (combat set).
    pub crouch: bool,
    /// `Look` is held.
    pub look_held: bool,
    /// `Look` went down this frame.
    pub look_pressed: bool,
    /// `CamMode` is held.
    pub camera_mode_held: bool,
    /// `CamMode` went down this frame.
    pub camera_mode_pressed: bool,
}

impl Default for PlayFrame {
    fn default() -> Self {
        Self {
            move_raw: Vec2::ZERO,
            move_world: Vec3::ZERO,
            move_fast: false,
            aim_raw: Vec2::ZERO,
            aim_world: Vec3::ZERO,
            look: Vec2::ZERO,
            look_delta: Vec2::ZERO,
            zoom: 0.0,
            commands: 0,
            state: 0,
            pressed: 0,
            step: 0,
            sprint: false,
            alternate: false,
            crouch: false,
            look_held: false,
            look_pressed: false,
            camera_mode_held: false,
            camera_mode_pressed: false,
        }
    }
}

/// The camera stick's response per frame: the original's `0.0025` times the
/// camera's scale field, which Exarust takes as the aim speed (a guess: the
/// field's owner is outside this component; the default aim speed 0.29
/// turns a full stick 0.02175 rad a frame). Positive `x` turns right
/// (negative yaw) and positive `y` looks up (negative pitch, Exarust's pitch
/// being below the horizon).
pub const LOOK_RATE: f32 = 0.0025;
/// A full camera stick's yaw gain.
pub const LOOK_YAW_GAIN: f32 = 30.0;
/// A full camera stick's pitch gain.
pub const LOOK_PITCH_GAIN: f32 = 10.0;
/// The zoom's travel on Exarust's 800–2000 scale.
pub const ZOOM_RANGE: f32 = 1200.0;
/// The zoom action's share of the range a step of 10 moves: the original's
/// `10 × 0.008333334 × range × 0.2`, 20 on a range of 1200.
pub const ZOOM_STEP_SHARE: f32 = 0.008_333_334 * 0.2;

/// The processed camera stick: `y` is zero below a 0.5 dead zone; `x` is
/// squared past a 0.1 dead zone; a magnitude over 0.01 (of the squared
/// pair) gives `(x·30, y·10)`, else nothing.
pub fn look_input(axes: Vec2) -> Vec2 {
    let y = if axes.y.abs() < 0.5 { 0.0 } else { axes.y };
    let x = if f64::from(axes.x.abs()) >= 0.1 {
        axes.x.abs() * axes.x
    } else {
        0.0
    };
    if 0.01 < f64::from(y * y + x * x) {
        Vec2::new(x * LOOK_YAW_GAIN, y * LOOK_PITCH_GAIN)
    } else {
        Vec2::ZERO
    }
}

/// The camera's turn from processed look input at `aim_speed`: the
/// original's `0.0025` rate, with the aim speed as its scale. Returns
/// `(yaw, pitch)` in radians, both negative for right and up.
pub fn look_delta(input: Vec2, aim_speed: f32) -> Vec2 {
    Vec2::new(
        -LOOK_RATE * aim_speed * input.x,
        -LOOK_RATE * aim_speed * input.y,
    )
}

/// The zoom change one frame of the zoom actions makes on a range of
/// [`ZOOM_RANGE`]: `input × 0.008333334 × range × 0.2`.
pub fn zoom_step(input: f32) -> f32 {
    input * ZOOM_STEP_SHARE * ZOOM_RANGE
}

/// What the gameplay reader knows about the game this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayContext {
    /// The gameplay set in use.
    pub variant: Variant,
    /// The step modes.
    pub modes: StepModes,
    /// The original's `0x1004efd00`: the aim stick is kept as a raw pair
    /// instead of being turned into the camera's frame.
    pub aim_in_world: bool,
    /// The live aim speed (0 to 1 and beyond; the setting's default 0.29).
    pub aim_speed: f32,
    /// The camera's world right axis.
    pub camera_right: Vec3,
    /// The camera's world forward axis.
    pub camera_forward: Vec3,
}

impl Default for PlayContext {
    fn default() -> Self {
        Self {
            variant: Variant::Interact,
            modes: StepModes::default(),
            aim_in_world: false,
            aim_speed: 0.29,
            camera_right: Vec3::X,
            camera_forward: Vec3::Z,
        }
    }
}

/// One motor's intensity, 0 to 1, as the original's vibration request
/// carries it.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Rumble {
    /// The first motor.
    pub first: f32,
    /// The second motor.
    pub second: f32,
}

impl Rumble {
    /// The decay a frame leaves: each motor keeps 0.6 of itself.
    pub const DECAY: f32 = 0.6;

    /// Adds `amount` to both motors, each clamped at 1.
    pub fn add(&mut self, amount: f32) {
        self.first += amount;
        if self.first > 1.0 {
            self.first = 1.0;
        }
        self.second += amount;
        if self.second > 1.0 {
            self.second = 1.0;
        }
    }

    /// One frame's decay. The original multiplies in double precision.
    pub fn decay(&mut self) {
        self.first = (f64::from(self.first) * f64::from(Self::DECAY)) as f32;
        self.second = (f64::from(self.second) * f64::from(Self::DECAY)) as f32;
    }

    /// The 16-bit motor speeds the original asks for: the intensity times
    /// 65535, rounded to even and kept to 16 bits. They are computed even
    /// though the original's vibration call is a stub that sends nothing.
    pub fn motors(&self) -> (u16, u16) {
        let word = |value: f32| {
            let scaled = 65535.0_f32 * value;
            (scaled.round_ties_even() as i64 as u32 & 0xffff) as u16
        };
        (word(self.first), word(self.second))
    }
}

/// The gameplay set's reader, kept between frames.
#[derive(Resource, Debug, Clone)]
pub struct PlayPad {
    /// Last frame's digital state.
    state: u32,
    /// The player command word the game reads.
    command: u32,
    /// The step repeat's deadline (ms of the real clock).
    repeat_at: i64,
    /// The `Powers` hold's deadline.
    powers_at: i64,
    /// A gameplay set was activated at this time and its held keys still
    /// need priming.
    prime_at: Option<i64>,
    /// The gameplay set the reader last ran.
    variant: Variant,
    /// The first frame has been taken.
    started: bool,
    /// The attack press latched for the next fixed tick.
    attack_latched: bool,
    /// The frame's result.
    frame: PlayFrame,
    /// The last frame's rumble.
    pub rumble: Rumble,
}

impl Default for PlayPad {
    fn default() -> Self {
        Self {
            state: 0,
            command: 0,
            repeat_at: 0,
            powers_at: 0,
            prime_at: None,
            variant: Variant::Interact,
            started: false,
            attack_latched: false,
            frame: PlayFrame::default(),
            rumble: Rumble::default(),
        }
    }
}

impl PlayPad {
    /// The current frame.
    pub fn frame(&self) -> &PlayFrame {
        &self.frame
    }

    /// The player command word.
    pub fn commands(&self) -> u32 {
        self.command
    }

    /// Clears command bits a consumer has taken.
    pub fn consume(&mut self, bits: u32) {
        self.command &= !bits;
    }

    /// Whether the primary combat action is held.
    pub fn attack_held(&self) -> bool {
        self.frame.state & command::ATTACK != 0
    }

    /// Takes the latched attack press, if any (for a click between ticks).
    pub fn take_attack_press(&mut self) -> bool {
        let pressed = self.attack_latched;
        self.attack_latched = false;
        pressed
    }

    /// A gameplay set was activated at `now_ms`: the next [`Self::frame`]
    /// waits [`PRIME_MS`] and primes the held keys.
    pub fn activate_set(&mut self, now_ms: i64) {
        self.prime_at = Some(now_ms);
    }

    /// One frame of gameplay input at `now_ms` (the real clock).
    pub fn update(&mut self, now_ms: i64, sample: &PadSample, ctx: PlayContext) -> &PlayFrame {
        if !self.started {
            self.started = true;
            self.variant = ctx.variant;
            self.prime_at = Some(now_ms);
            return &self.frame;
        }
        if ctx.variant != self.variant {
            self.variant = ctx.variant;
            self.prime_at = Some(now_ms);
        }
        if let Some(at) = self.prime_at {
            if now_ms < at + PRIME_MS {
                return &self.frame;
            }
            self.prime(sample, ctx.variant);
            self.prime_at = None;
        }
        let mut previous = self.state;
        self.state = 0;
        self.rumble.decay();
        let mut frame = PlayFrame::default();
        let mut direction = 0;
        let move_axes = sample.analog(Id::Move);
        let right = ctx.camera_right.normalize_or_zero();
        let forward = ctx.camera_forward.normalize_or_zero();
        if ctx.modes.across || ctx.modes.along {
            let (axes, bits) = if ctx.modes.across {
                (move_axes.x, (command::STEP_LEFT, command::STEP_RIGHT))
            } else {
                (move_axes.y, (command::STEP_BACK, command::STEP_FORWARD))
            };
            if axes > STEP_DEAD_ZONE {
                if self.repeat_at < now_ms {
                    previous &= !bits.1;
                    self.repeat_at = now_ms + STEP_REPEAT_MS;
                }
                self.state |= bits.1;
                direction = bits.1;
            } else if axes < -STEP_DEAD_ZONE {
                if self.repeat_at < now_ms {
                    previous &= !bits.0;
                    self.repeat_at = now_ms + STEP_REPEAT_MS;
                }
                self.state |= bits.0;
                direction = bits.0;
            } else {
                self.repeat_at = 0;
            }
        } else {
            frame.move_raw = move_axes;
            frame.move_fast =
                0.8 < f64::from(move_axes.x * move_axes.x + move_axes.y * move_axes.y);
            frame.move_world = (right * move_axes.x + forward * move_axes.y).with_y(0.0);
        }
        frame.step = direction;
        let look = look_input(sample.analog(Id::Camera));
        frame.look = look;
        frame.look_delta = look_delta(look, ctx.aim_speed);
        let aim = sample.analog(Id::Aim);
        frame.aim_raw = aim;
        frame.aim_world = if ctx.aim_in_world {
            Vec3::new(aim.x, 0.0, aim.y)
        } else {
            right * aim.x + forward * aim.y
        };
        let mut zoom = 0.0;
        if sample.digital(Id::ZoomIn) {
            zoom += zoom_step(10.0);
        }
        if sample.digital(Id::ZoomOut) {
            zoom -= zoom_step(10.0);
        }
        frame.zoom = zoom;
        frame.alternate = ctx.variant == Variant::Combat && sample.digital(Id::Alternate);
        frame.crouch = ctx.variant == Variant::Combat && sample.digital(Id::Crouch);
        frame.sprint = sample.digital(Id::Sprint);
        if sample.digital(Id::Combat) {
            self.state |= command::ATTACK;
        }
        if ctx.variant == Variant::Interact {
            if sample.digital(Id::Inventory) {
                self.state |= command::INVENTORY;
            }
            if sample.digital(Id::Skills) {
                self.state |= command::SKILLS;
            }
        }
        if sample.digital(Id::SwapWeapon) {
            self.state |= command::SWAP_WEAPON;
        }
        if sample.digital(Id::Talk) {
            self.state |= command::TALK;
        }
        if sample.digital(Id::Powers) {
            let before = self.state;
            self.state |= command::POWER_BAR;
            if previous & command::POWER_BAR == 0 {
                self.powers_at = now_ms + POWERS_HOLD_MS;
            } else if self.powers_at < now_ms {
                self.state = before | command::POWER_BAR | command::POWER_HOLD;
            }
        }
        if sample.digital(Id::UsePower) {
            self.state |= command::USE_POWER;
        }
        if sample.digital(Id::Select) {
            self.state |= command::USE;
        }
        let mut held = 0;
        let look = sample.digital(Id::Look);
        if look {
            held |= command::LOOK;
        }
        let camera_mode = sample.digital(Id::CameraMode);
        if camera_mode {
            held |= command::CAMERA_MODE;
        }
        if sample.digital(Id::PauseMenu) {
            self.state |= command::PAUSE;
        }
        let pressed = !previous & self.state;
        if pressed & command::ATTACK != 0 {
            self.attack_latched = true;
        }
        let was_look = self.command & command::LOOK != 0;
        let was_camera_mode = self.command & command::CAMERA_MODE != 0;
        self.command =
            (self.command & command::GAMEPAD_MASK) | (pressed & command::GAMEPAD_MASK) | held;
        frame.state = self.state;
        frame.pressed = pressed;
        frame.commands = self.command;
        frame.look_held = held & command::LOOK != 0;
        frame.look_pressed = look && !was_look;
        frame.camera_mode_held = held & command::CAMERA_MODE != 0;
        frame.camera_mode_pressed = camera_mode && !was_camera_mode;
        self.frame = frame;
        &self.frame
    }

    /// The held keys a newly active gameplay set starts from: the original
    /// primes `Combat`, `WepSwp`, `Talk`, `Powers`, `UsePower`, `PowerTree`,
    /// `Select`, `pause_menu`, `Inventory` and `Skills`, and nothing else.
    fn prime(&mut self, sample: &PadSample, variant: Variant) {
        self.state = 0;
        for (id, bit) in [
            (Id::Combat, command::ATTACK),
            (Id::SwapWeapon, command::SWAP_WEAPON),
            (Id::Talk, command::TALK),
            (Id::Powers, command::POWER_BAR),
            (Id::UsePower, command::USE_POWER),
            (Id::PowerTree, command::POWER_HOLD),
            (Id::Select, command::USE),
            (Id::PauseMenu, command::PAUSE),
        ] {
            if sample.digital(id) {
                self.state |= bit;
            }
        }
        if variant == Variant::Interact {
            if sample.digital(Id::Inventory) {
                self.state |= command::INVENTORY;
            }
            if sample.digital(Id::Skills) {
                self.state |= command::SKILLS;
            }
        }
    }

    /// Reads an initial digital state word, as the original primes it after
    /// its three milliseconds; exposed for tests and set changes.
    pub fn prime_now(&mut self, sample: &PadSample, variant: Variant) {
        self.prime(sample, variant);
    }
}

/// Rumble a hit asks for, as the original's motile dynamics call it: both
/// motors rise by the amount, clamped at 1.
pub fn rumble_hit(pad: &mut PlayPad, amount: f32) {
    pad.rumble.add(amount);
}

// ---- Bevy wiring ----------------------------------------------------------

/// Whether a gamepad is connected and drives the player.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct PadStatus {
    /// A gamepad is connected.
    pub connected: bool,
    /// `Bindings::controller` is set (the controls page's controller mode).
    pub controller: bool,
}

impl PadStatus {
    /// The player reads the pad when a gamepad is connected in controller
    /// mode.
    pub fn active(&self) -> bool {
        self.connected && self.controller
    }
}

/// The pad plugin: reads the first connected gamepad into the menu and
/// gameplay readers each frame, and keeps the controller-mode flag.
pub struct GamepadPlugin;

impl Plugin for GamepadPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuPad>()
            .init_resource::<PlayPad>()
            .init_resource::<PadStatus>()
            .init_resource::<FocusStack>()
            .init_resource::<crate::bindings::Bindings>()
            .add_systems(First, raw_axis_settings)
            .add_systems(
                PreUpdate,
                (
                    raw_axis_settings,
                    read_pad.run_if(resource_exists::<crate::bindings::Bindings>),
                    sync_controller_mode,
                )
                    .chain(),
            );
    }
}

/// Gives every gamepad's sticks and triggers the identity response, so the
/// original's own dead zones (0.1, 0.25, 0.5, 0.8) see the raw values
/// rather than Bevy's default 0.05 rescaling.
fn raw_axis_settings(mut pads: Query<&mut GamepadSettings, With<Gamepad>>) {
    let Ok(identity) = AxisSettings::new(-1.0, 0.0, 0.0, 1.0, 0.0) else {
        return;
    };
    for mut settings in &mut pads {
        if settings.default_axis_settings != identity {
            settings.default_axis_settings = identity.clone();
        }
        if !settings.axis_settings.is_empty() {
            settings.axis_settings.clear();
        }
    }
}

/// Reads the first connected gamepad into [`PadSample`] and runs both
/// readers. The gameplay variant follows the player's combat mode; the
/// camera's basis comes from the chase camera.
#[allow(clippy::too_many_arguments)]
fn read_pad(
    time: Res<Time<Real>>,
    pads: Query<&Gamepad>,
    players: Query<&crate::player::PlayerBody, With<crate::player::Player>>,
    cameras: Query<&GlobalTransform, With<crate::player::ChaseCamera>>,
    config: Option<Res<crate::hw_config::HardwareConfig>>,
    bindings: Res<crate::bindings::Bindings>,
    focus: Res<FocusStack>,
    mut menu: ResMut<MenuPad>,
    mut play: ResMut<PlayPad>,
    mut status: ResMut<PadStatus>,
) {
    let now = time.elapsed().as_millis() as i64;
    status.controller = bindings.controller;
    let Some(pad) = pads.iter().next() else {
        status.connected = false;
        menu.frame(
            now,
            &PadSample::default(),
            MenuContext {
                focused: focus.focused(),
                ready: false,
                ..MenuContext::default()
            },
        );
        return;
    };
    status.connected = true;
    let sample = sample_pad(pad);
    let variant = if players.iter().next().is_some_and(|body| body.combat) {
        Variant::Combat
    } else {
        Variant::Interact
    };
    let top = focus.entries().last();
    menu.frame(
        now,
        &sample,
        MenuContext {
            focused: focus.focused(),
            ready: status.connected,
            fast_nav: top.is_some_and(|e| e.flags & 0x2000 != 0),
            fast_scroll: top.is_some_and(|e| e.flags & 0x4000 != 0),
            exit_hook: false,
        },
    );
    let (right, forward) = cameras.iter().next().map_or((Vec3::X, Vec3::Z), |t| {
        (
            t.right().with_y(0.0).normalize_or(Vec3::X),
            t.forward().with_y(0.0).normalize_or(Vec3::Z),
        )
    });
    let ctx = PlayContext {
        variant,
        aim_speed: config.as_ref().map_or(0.29, |c| c.live.aim_speed),
        camera_right: right,
        camera_forward: forward,
        ..PlayContext::default()
    };
    play.update(now, &sample, ctx);
}

/// The first connected gamepad's action data, read through the default
/// controls.
fn sample_pad(pad: &Gamepad) -> PadSample {
    let mut sample = PadSample {
        connected: true,
        ..PadSample::default()
    };
    for id in Id::ALL {
        match default_control(id) {
            Control::Button(button) => sample.set_digital(id, pad.pressed(button)),
            Control::Stick(x, y) => sample.set_analog(
                id,
                Vec2::new(pad.get(x).unwrap_or(0.0), pad.get(y).unwrap_or(0.0)),
            ),
        }
    }
    sample
}

/// Copies the controls page's controller mode into the bindings, and turns
/// the controller on once a gamepad is connected when no settings file
/// exists (Exarust's own convenience: without a Controls page there is no
/// other switch to press).
fn sync_controller_mode(
    status: Res<PadStatus>,
    config: Option<Res<crate::hw_config::HardwareConfig>>,
    mut bindings: ResMut<crate::bindings::Bindings>,
) {
    let enabled = match config.as_ref() {
        Some(config) => config.live.controller,
        None => status.connected,
    };
    if bindings.controller != enabled && (!bindings.controller || config.is_some()) {
        bindings.controller = enabled;
    }
}

#[cfg(test)]
#[path = "gamepad_test.rs"]
mod tests;
