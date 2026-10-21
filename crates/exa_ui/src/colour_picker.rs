//! The colour-picker logic (`RF_GUI_Classes`): one picker per colour-swatch
//! control, re-focused when the same swatch is clicked again, with its RGB
//! and HSV sliders, channel gradients and hex field kept in sync.
//!
//! A colour is three floats in 0–1 (plus alpha, unused) multiplied by 255
//! and converted with the processor's round-to-even and the low byte, as the
//! original does. The picker keeps both an RGB and an HSV representation: a
//! channel slider writes its own representation and refreshes the other
//! (the original guards the recursion with its `sync` byte). Refreshing
//! from RGB builds the six channel-gradient ends (the colour with one
//! channel forced to 0 and to 255), updates the sliders, converts to HSV
//! for the HSV sliders, writes the colour back to the swatch control and
//! calls its owner. Refreshing from HSV also builds the four corner colours
//! of the colour field: `(h, 0, v)`, `(h, 1, v)`, `(h, s, 0)` and
//! `(h, s, 1)`.
//!
//! Evidence: the picker's open routine, the six slider callbacks, the RGB
//! refresh and the HSV update, read against their assembly.

/// The colour swatch control class id.
pub const COLOUR_SWATCH_CLASS: u16 = 0x17;
/// The update word the picker leaves after a refresh.
pub const REFRESH_WORD: u32 = 0x2000;
/// The word set when the picker is re-focused.
pub const FOCUS_WORD: u32 = 0x1000;

/// A colour as the picker keeps it: red, green, blue and alpha floats.
pub type Rgba = [f32; 4];
/// A hue, saturation and value triple (hue in turns, 0–1).
pub type Hsv = [f32; 4];

/// One channel-gradient end pair: the three ends with the channel at 0 and
/// at 255, alpha 255.
pub type Ramp = [[u8; 4]; 2];

/// An RGB float to a byte the original's way: multiply by 255 in single
/// precision, convert with round-to-nearest-even, keep the low byte.
pub fn to_byte(value: f32) -> u8 {
    let scaled = value * 255.0;
    let rounded = scaled.round_ties_even();
    let converted = if rounded.is_nan() || rounded < -9.223_372e18 || rounded > 9.223_372e18 {
        i64::MIN
    } else {
        rounded as i64
    };
    converted as u8
}

/// The RGB bytes of a colour.
pub fn rgb_bytes(colour: Rgba) -> [u8; 3] {
    [to_byte(colour[0]), to_byte(colour[1]), to_byte(colour[2])]
}

/// The six channel-gradient ends of an RGB colour, in the original's order:
/// red at 0 then 255, green at 0 then 255, blue at 0 then 255.
pub fn channel_ramps(bytes: [u8; 3]) -> [Ramp; 3] {
    let [r, g, b] = bytes;
    [
        [[0, g, b, 255], [255, g, b, 255]],
        [[r, 0, b, 255], [r, 255, b, 255]],
        [[r, g, 0, 255], [r, g, 255, 255]],
    ]
}

/// The RGB-to-HSV conversion the picker's `RGB→HSV` helper performs: the
/// largest channel is the value, the spread over it the saturation, and the
/// hue a sixth of a turn per sector. A zero spread gives hue and saturation
/// 0 (the original's colour helper does the same). Unordered values fall
/// through the branches as written. [guess: the helper itself is outside
/// the pack; the standard sector form matches its callers' expectations]
pub fn rgb_to_hsv(rgb: [f32; 3]) -> [f32; 3] {
    let (r, g, b) = (rgb[0], rgb[1], rgb[2]);
    let mut max = r;
    let mut min = r;
    if g > max {
        max = g;
    }
    if b > max {
        max = b;
    }
    if g < min {
        min = g;
    }
    if b < min {
        min = b;
    }
    let delta = max - min;
    let mut h = 0.0;
    if delta != 0.0 {
        if max == r {
            h = (g - b) / delta;
            if h < 0.0 {
                h += 6.0;
            }
        } else if max == g {
            h = (b - r) / delta + 2.0;
        } else {
            h = (r - g) / delta + 4.0;
        }
        h /= 6.0;
    }
    let s = if max == 0.0 { 0.0 } else { delta / max };
    [h, s, max]
}

/// The HSV-to-RGB conversion, hue in turns. A hue outside 0–1 wraps in
/// sixths, as the sector arithmetic does. [guess: helper outside the pack]
pub fn hsv_to_rgb(hsv: [f32; 3]) -> [f32; 3] {
    let (h, s, v) = (hsv[0], hsv[1], hsv[2]);
    let sector = h * 6.0;
    let index = sector.floor();
    let f = sector - index;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    let i = ((index as i64) % 6 + 6) % 6;
    match i {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        _ => [v, p, q],
    }
}

/// The four corner colours of the picker's colour field, as bytes: the hue
/// with no and full saturation at the value, then at no and full value.
pub fn field_corners(hsv: Hsv) -> [[u8; 4]; 4] {
    let h = hsv[0];
    let s = hsv[1];
    let v = hsv[2];
    let mut corners = [[0u8; 4]; 4];
    for (corner, colour) in corners.iter_mut().zip([
        hsv_to_rgb([h, 0.0, v]),
        hsv_to_rgb([h, 1.0, v]),
        hsv_to_rgb([h, s, 0.0]),
        hsv_to_rgb([h, s, 1.0]),
    ]) {
        *corner = [
            to_byte(colour[0]),
            to_byte(colour[1]),
            to_byte(colour[2]),
            255,
        ];
    }
    corners
}

/// A colour-swatch control as the picker sees it: its class, its displayed
/// name, its colour and whether its owner listens for a change.
#[derive(Debug, Clone, PartialEq)]
pub struct SwatchControl {
    pub class: u16,
    pub name: Vec<u8>,
    pub colour: Rgba,
    /// Whether the control has an owner callback (the original's +0x10).
    pub callback: bool,
}

impl SwatchControl {
    pub fn new(colour: Rgba) -> Self {
        Self {
            class: COLOUR_SWATCH_CLASS,
            name: Vec::new(),
            colour,
            callback: true,
        }
    }
}

/// What a refresh told the world.
#[derive(Debug, Clone, PartialEq)]
pub struct Refresh {
    /// The colour written back to the swatch control.
    pub swatch: Rgba,
    /// The owner callback fired with this colour.
    pub callback: Option<Rgba>,
}

/// One colour picker.
#[derive(Debug, Clone, PartialEq)]
pub struct ColourPicker {
    /// The swatch the picker edits.
    pub swatch_id: usize,
    pub name: Vec<u8>,
    /// Red, green, blue and alpha.
    pub colour: Rgba,
    /// Hue, saturation, value and an unused fourth word.
    pub hsv: Hsv,
    /// The recursion guard: a refresh in progress does not convert back.
    pub syncing: bool,
    /// The six channel-gradient ends.
    pub ramps: [Ramp; 3],
    /// The four field corner colours.
    pub corners: [[u8; 4]; 4],
    /// The slider thumb colours, one per channel.
    pub thumbs: [u8; 3],
    /// The last update word.
    pub word: u32,
    /// Whether the picker was shown.
    pub shown: bool,
    pub is_open: bool,
}

impl ColourPicker {
    /// A picker on a swatch: its name, its colour, HSV of that colour and
    /// the first refresh.
    pub fn new(swatch_id: usize, swatch: &SwatchControl) -> Self {
        let colour = swatch.colour;
        let hsv = rgb_to_hsv([colour[0], colour[1], colour[2]]);
        let bytes = rgb_bytes(colour);
        let mut picker = Self {
            swatch_id,
            name: swatch.name.clone(),
            colour,
            hsv: [hsv[0], hsv[1], hsv[2], 0.0],
            syncing: false,
            ramps: channel_ramps(bytes),
            corners: field_corners([hsv[0], hsv[1], hsv[2], 0.0]),
            thumbs: bytes,
            word: 0,
            shown: false,
            is_open: true,
        };
        let _ = picker.refresh_rgb();
        picker
    }

    /// The RGB slider callback, the slider in 0–255: the channel takes the
    /// value over 255 and the picker refreshes from RGB.
    pub fn set_rgb_slider(&mut self, channel: usize, value_255: f32) -> Option<Refresh> {
        let channel = channel.min(2);
        self.colour[channel] = value_255 / 255.0;
        Some(self.refresh_rgb())
    }

    /// The RGB slider callback with the slider value already in 0–1.
    pub fn set_rgb_slider_raw(&mut self, channel: usize, value: f32) -> Option<Refresh> {
        let channel = channel.min(2);
        self.colour[channel] = value;
        Some(self.refresh_rgb())
    }

    /// The HSV slider callback, the slider in 0–255: the component takes
    /// the value over 255 and the picker converts from HSV.
    pub fn set_hsv_slider(&mut self, channel: usize, value_255: f32) -> Option<Refresh> {
        let channel = channel.min(2);
        self.hsv[channel] = value_255 / 255.0;
        Some(self.refresh_hsv())
    }

    /// The HSV slider callback with the value already in 0–1.
    pub fn set_hsv_slider_raw(&mut self, channel: usize, value: f32) -> Option<Refresh> {
        let channel = channel.min(2);
        self.hsv[channel] = value;
        Some(self.refresh_hsv())
    }

    /// The hue-and-saturation slider pair, both already in 0–1.
    pub fn set_hue_saturation(&mut self, hue: f32, saturation: f32) -> Option<Refresh> {
        self.hsv[0] = hue;
        self.hsv[1] = saturation;
        Some(self.refresh_hsv())
    }

    /// The picker applies its own stored colour again.
    pub fn set_value(&mut self) -> Option<Refresh> {
        Some(self.refresh_rgb())
    }

    /// The picker is re-focused: its update word becomes 0x1000.
    pub fn focus(&mut self) {
        self.word = FOCUS_WORD;
        self.shown = true;
    }

    /// A new colour from outside (the caller's floats): refresh from it.
    pub fn set_colour(&mut self, colour: [f32; 3]) -> Option<Refresh> {
        self.colour[0] = colour[0];
        self.colour[1] = colour[1];
        self.colour[2] = colour[2];
        Some(self.refresh_rgb())
    }

    /// Refreshes from the RGB colour: the bytes, the six ramps, the slider
    /// thumbs, the HSV sliders (unless a sync is in progress), the write
    /// back and the callback.
    pub fn refresh_rgb(&mut self) -> Refresh {
        let bytes = rgb_bytes(self.colour);
        self.ramps = channel_ramps(bytes);
        self.thumbs = bytes;
        if !self.syncing {
            let hsv = rgb_to_hsv([self.colour[0], self.colour[1], self.colour[2]]);
            self.hsv = [hsv[0], hsv[1], hsv[2], 0.0];
        }
        self.word = REFRESH_WORD;
        Refresh {
            swatch: self.colour,
            callback: Some(self.colour),
        }
    }

    /// Refreshes from the HSV triple: the sliders, the corner colours and
    /// the RGB colour, which refreshes the rest once without converting
    /// back.
    pub fn refresh_hsv(&mut self) -> Refresh {
        let corners = field_corners(self.hsv);
        self.corners = corners;
        let sync = self.syncing;
        self.syncing = true;
        let rgb = hsv_to_rgb([self.hsv[0], self.hsv[1], self.hsv[2]]);
        self.colour = [rgb[0], rgb[1], rgb[2], self.colour[3]];
        let mut refresh = self.refresh_rgb();
        self.syncing = sync;
        refresh.swatch = self.colour;
        refresh.callback = Some(self.colour);
        refresh
    }

    /// The HSV sliders' byte values (the three components times 255).
    pub fn hsv_slider_bytes(&self) -> [u8; 3] {
        [
            to_byte(self.hsv[0]),
            to_byte(self.hsv[1]),
            to_byte(self.hsv[2]),
        ]
    }
}

/// Why a picker was opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opened {
    /// An existing picker for the swatch was re-focused.
    Refocused(usize),
    /// A new picker was made.
    Created(usize),
}

/// The unit's list of pickers, one per swatch control.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PickerSet {
    pub pickers: Vec<ColourPicker>,
}

impl PickerSet {
    /// Opens the picker of a swatch control: an existing picker for that
    /// swatch is brought to the front and focused, else one is made, put on
    /// the list and refreshed from the swatch's colour.
    pub fn open(&mut self, swatch_id: usize, swatch: &SwatchControl) -> Opened {
        if let Some(index) = self
            .pickers
            .iter_mut()
            .position(|picker| picker.swatch_id == swatch_id)
        {
            self.pickers[index].focus();
            return Opened::Refocused(index);
        }
        let picker = ColourPicker::new(swatch_id, swatch);
        self.pickers.push(picker);
        Opened::Created(self.pickers.len() - 1)
    }

    /// The picker of a swatch, if one is open.
    pub fn picker_for(&self, swatch_id: usize) -> Option<&ColourPicker> {
        self.pickers.iter().find(|p| p.swatch_id == swatch_id)
    }

    /// The unit's finalisation: the picker list is freed.
    pub fn finalize(&mut self) {
        self.pickers.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn colour() -> Rgba {
        [0.2, 0.4, 0.6, 1.0]
    }

    #[test]
    fn a_colour_becomes_bytes_with_round_to_even_and_the_low_byte() {
        assert_eq!(to_byte(0.0), 0);
        assert_eq!(to_byte(1.0), 255);
        assert_eq!(to_byte(0.5), 128);
        // 0.3 × 255 = 76.5 exactly ties to 76 (even).
        assert_eq!(0.3f32 * 255.0, 76.5);
        assert_eq!(to_byte(0.3), 76);
        // 0.7 × 255 = 178.5 ties to 178 (even).
        assert_eq!(to_byte(0.7), 178);
        // The conversion keeps the low byte of the rounded value.
        assert_eq!(to_byte(2.0), 254);
        assert_eq!(rgb_bytes(colour()), [51, 102, 153]);
        assert_eq!(rgb_bytes([-0.0, 1.0 / 255.0, 1.5, 0.0]), [0, 1, 126]);
    }

    #[test]
    fn the_six_ramps_force_one_channel_at_a_time() {
        assert_eq!(
            channel_ramps([51, 102, 153]),
            [
                [[0, 102, 153, 255], [255, 102, 153, 255]],
                [[51, 0, 153, 255], [51, 255, 153, 255]],
                [[51, 102, 0, 255], [51, 102, 255, 255]],
            ]
        );
    }

    #[test]
    fn rgb_and_hsv_round_trip_the_colour() {
        let hsv = rgb_to_hsv([0.2, 0.4, 0.6]);
        assert_eq!(hsv[0], 210.0 / 360.0);
        assert!((hsv[1] - 2.0 / 3.0).abs() < 1e-6);
        assert!((hsv[2] - 0.6).abs() < 1e-6);
        let back = hsv_to_rgb(hsv);
        for (a, b) in back.iter().zip([0.2, 0.4, 0.6]) {
            assert!((a - b).abs() < 1e-6, "{back:?}");
        }
        assert_eq!(hsv_to_rgb([0.0, 0.0, 0.5]), [0.5, 0.5, 0.5]);
        assert_eq!(hsv_to_rgb([0.0, 1.0, 1.0]), [1.0, 0.0, 0.0]);
        assert_eq!(hsv_to_rgb([1.0 / 3.0, 1.0, 1.0]), [0.0, 1.0, 0.0]);
        assert_eq!(hsv_to_rgb([2.0 / 3.0, 1.0, 1.0]), [0.0, 0.0, 1.0]);
        // Grey has no hue or saturation.
        assert_eq!(rgb_to_hsv([0.5, 0.5, 0.5]), [0.0, 0.0, 0.5]);
        // A red with the green channel higher wraps its hue up a turn.
        let hsv = rgb_to_hsv([0.5, 0.75, 0.25]);
        let back = hsv_to_rgb(hsv);
        for (a, b) in back.iter().zip([0.5, 0.75, 0.25]) {
            assert!((a - b).abs() < 1e-5, "{back:?}");
        }
    }

    #[test]
    fn the_corners_are_the_hue_at_no_and_full_saturation_and_value() {
        let corners = field_corners([0.0, 0.5, 1.0, 0.0]);
        assert_eq!(
            corners,
            [
                [255, 255, 255, 255],
                [255, 0, 0, 255],
                [0, 0, 0, 255],
                [255, 128, 128, 255],
            ]
        );
    }

    #[test]
    fn a_picker_refreshes_the_other_representation_without_recursing() {
        let swatch = SwatchControl::new(colour());
        let mut picker = ColourPicker::new(0, &swatch);
        assert_eq!(picker.ramps[0], [[0, 102, 153, 255], [255, 102, 153, 255]]);
        assert_eq!(picker.thumbs, [51, 102, 153]);
        assert_eq!(picker.word, REFRESH_WORD);
        assert_eq!(picker.hsv_slider_bytes(), [149, 170, 153]);
        // A red slider of 255 turns the colour red and the hue 0.
        let refresh = picker.set_rgb_slider(0, 255.0).unwrap();
        assert_eq!(refresh.swatch, [1.0, 0.4, 0.6, 1.0]);
        assert_eq!(picker.hsv[0], 340.0 / 360.0);
        // A hue slider at half a turn makes the colour cyan at the value
        // the RGB refresh left (1) and its saturation (0.6).
        let refresh = picker.set_hsv_slider(0, 127.5).unwrap();
        assert_eq!(refresh.swatch, [0.39999998, 1.0, 1.0, 1.0]);
        assert_eq!(picker.corners[1], [0, 255, 255, 255]);
        // The guard is released afterwards.
        assert!(!picker.syncing);
        // Raw slider values pass straight through.
        let refresh = picker.set_rgb_slider_raw(1, 0.5).unwrap();
        assert_eq!(refresh.swatch, [0.39999998, 0.5, 1.0, 1.0]);
        picker.set_hsv_slider_raw(2, 0.25);
        assert_eq!(picker.colour[2], 0.25);
        picker.set_hue_saturation(0.0, 1.0);
        assert_eq!(picker.colour, [0.25, 0.0, 0.0, 1.0]);
        picker.set_colour([0.0, 0.0, 0.0]);
        assert_eq!(picker.hsv_slider_bytes(), [0, 0, 0]);
        let refresh = picker.set_value().unwrap();
        assert_eq!(refresh.callback, Some([0.0, 0.0, 0.0, 1.0]));
        picker.focus();
        assert_eq!(picker.word, FOCUS_WORD);
        assert!(picker.shown);
    }

    #[test]
    fn a_second_open_refocuses_the_same_swatch() {
        let swatch = SwatchControl::new(colour());
        let mut set = PickerSet::default();
        assert_eq!(set.open(7, &swatch), Opened::Created(0));
        assert_eq!(set.open(8, &swatch), Opened::Created(1));
        assert_eq!(set.pickers.len(), 2);
        set.pickers[0].word = 0;
        assert_eq!(set.open(7, &swatch), Opened::Refocused(0));
        assert_eq!(set.pickers[0].word, FOCUS_WORD);
        assert_eq!(set.picker_for(7).unwrap().swatch_id, 7);
        assert_eq!(set.picker_for(9), None);
        set.finalize();
        assert!(set.pickers.is_empty());
        // A swatch that is not a colour swatch is still a swatch key here;
        // the original only reads its colour for class 0x17.
        let mut other = SwatchControl::new([1.0, 0.0, 0.0, 1.0]);
        other.class = 1;
        let picker = ColourPicker::new(3, &other);
        assert_eq!(picker.name, Vec::<u8>::new());
        assert_eq!(picker.thumbs, [255, 0, 0]);
    }
}
