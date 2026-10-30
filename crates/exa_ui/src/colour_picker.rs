//! The colour-picker logic (`RF_GUI_Classes`): one picker per colour-swatch
//! control, raised when the same swatch opens it again, with its RGB and HSV
//! sliders, channel gradients and colour field kept in sync.
//!
//! The picker keeps three colours of four floats: the current RGB colour,
//! its HSV form and the swatch's colour when it opened (the revert source).
//! A refresh from RGB takes a source colour, turns it into bytes (× 255,
//! the processor's round-to-even, the low byte), builds the six
//! channel-gradient ends (the colour with one channel forced to 0 and to
//! 255), sets the three RGB sliders and their number fields (× 255), shows
//! the bytes in the preview, stores the colour and, unless a sync is in
//! progress, converts it to HSV and refreshes from HSV under the sync
//! guard. It then writes the colour back to a colour-swatch control (as
//! saturated bytes) and calls the control's owner when it has one, and
//! leaves the update word 0x2000. A refresh from HSV stores the triple, sets
//! the three HSV sliders and fields, converts to RGB unless a sync is in
//! progress, builds the colour field's four corners `(h, 0, v)`,
//! `(h, 1, v)`, `(h, s, 0)` and `(h, s, 1)` and, unless syncing, refreshes
//! from RGB under the guard. The conversions are the engine's colour
//! helpers ([`exa_formats::palette::rgb_to_hsv`] and
//! [`exa_formats::palette::hsv`]).
//!
//! Evidence: the picker's open routine, the slider, field and button
//! callbacks, the RGB refresh and the HSV refresh, read from their
//! assembly. The picker form itself (`TColourPicker`), its controls'
//! layout and the number-field text format are outside this unit.

use exa_formats::palette::{hsv, rgb_to_hsv};
use exa_formats::pixel::{BYTE_UNIT, round_f32};

/// The colour swatch control class id.
pub const COLOUR_SWATCH_CLASS: u16 = 0x17;
/// The update word a refresh from RGB leaves.
pub const REFRESH_WORD: u32 = 0x2000;
/// The update word the picker's finish button leaves before it calls on.
pub const FINISH_WORD: u32 = 0x1000;

/// A colour as the picker keeps it: red, green, blue and alpha floats.
pub type Rgba = [f32; 4];
/// A hue (in turns), saturation and value, and the fourth float carried
/// along with them.
pub type Hsv = [f32; 4];

/// One channel-gradient end pair: the colour with the channel at 0 and at
/// 255, alpha 255.
pub type Ramp = [[u8; 4]; 2];

/// An RGB float to a byte the refresh's way: × 255 in single precision,
/// converted with round-to-nearest-even and the low byte kept (no
/// saturation; a NaN or out-of-range value gives 0).
pub fn to_byte(value: f32) -> u8 {
    round_f32(value * 255.0) as u8
}

/// The RGB bytes of a colour.
pub fn rgb_bytes(colour: Rgba) -> [u8; 3] {
    [to_byte(colour[0]), to_byte(colour[1]), to_byte(colour[2])]
}

/// A colour written back to a swatch control: all four floats × 255,
/// rounded to even and saturated to bytes (the engine's packed store).
pub fn packed_bytes(colour: Rgba) -> [u8; 4] {
    colour.map(|c| round_f32(c * 255.0).clamp(0, 255) as u8)
}

/// A swatch control's bytes as the picker reads them: × 1/255.
pub fn unpacked(bytes: [u8; 4]) -> Rgba {
    bytes.map(|b| f32::from(b) * BYTE_UNIT)
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

/// The four corner colours of the picker's colour field, as bytes: the hue
/// with no and full saturation at the value, then the saturation at no and
/// full value.
pub fn field_corners(colour: Hsv) -> [[u8; 4]; 4] {
    let [h, s, v, _] = colour;
    [[h, 0.0, v], [h, 1.0, v], [h, s, 0.0], [h, s, 1.0]].map(|[h, s, v]| {
        let [r, g, b] = hsv(h, s, v);
        [to_byte(r), to_byte(g), to_byte(b), 255]
    })
}

/// A colour-swatch control as the picker sees it: its class, its caption,
/// its colour bytes and whether its owner listens for a change.
#[derive(Debug, Clone, PartialEq)]
pub struct SwatchControl {
    pub class: u16,
    pub name: Vec<u8>,
    /// Red, green, blue and alpha bytes.
    pub colour: [u8; 4],
    /// Whether the control has an owner callback.
    pub callback: bool,
}

impl SwatchControl {
    pub fn new(colour: [u8; 4]) -> Self {
        Self {
            class: COLOUR_SWATCH_CLASS,
            name: Vec::new(),
            colour,
            callback: true,
        }
    }
}

/// What a refresh from RGB did to the swatch control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Refresh {
    /// The colour was written back to a colour-swatch control.
    pub wrote_back: bool,
    /// The control's owner callback ran.
    pub called_owner: bool,
}

/// One colour picker.
#[derive(Debug, Clone, PartialEq)]
pub struct ColourPicker {
    /// The swatch the picker edits.
    pub swatch_id: usize,
    /// The caption, the swatch's name.
    pub name: Vec<u8>,
    /// The current colour.
    pub colour: Rgba,
    /// The HSV form of the colour.
    pub hsv: Hsv,
    /// The swatch's colour when the picker opened (the revert source).
    pub original: Rgba,
    /// The recursion guard: a refresh in progress does not convert back.
    pub syncing: bool,
    /// The six channel-gradient ends.
    pub ramps: [Ramp; 3],
    /// The four colour-field corners.
    pub corners: [[u8; 4]; 4],
    /// The preview swatch's bytes (alpha 255).
    pub preview: [u8; 4],
    /// The RGB sliders' values and their number fields' values (× 255).
    pub rgb_sliders: [f32; 3],
    pub rgb_fields: [f32; 3],
    /// The HSV sliders' values and their number fields' values (× 255).
    pub hsv_sliders: [f32; 3],
    pub hsv_fields: [f32; 3],
    /// The last update word.
    pub word: u32,
}

impl ColourPicker {
    /// A picker on a swatch. A colour-swatch control (class 0x17) gives its
    /// caption and colour, which becomes the revert source and the first
    /// refresh; any other control leaves the picker blank.
    pub fn new(swatch_id: usize, swatch: &mut SwatchControl) -> Self {
        let mut picker = Self {
            swatch_id,
            name: Vec::new(),
            colour: [0.0; 4],
            hsv: [0.0; 4],
            original: [0.0; 4],
            syncing: false,
            ramps: [[[0; 4]; 2]; 3],
            corners: [[0; 4]; 4],
            preview: [0; 4],
            rgb_sliders: [0.0; 3],
            rgb_fields: [0.0; 3],
            hsv_sliders: [0.0; 3],
            hsv_fields: [0.0; 3],
            word: 0,
        };
        if swatch.class == COLOUR_SWATCH_CLASS {
            picker.name = swatch.name.clone();
            picker.preview = swatch.colour;
            picker.original = unpacked(swatch.colour);
            picker.refresh_rgb(picker.original, Some(swatch));
        }
        picker
    }

    /// An RGB slider (0–255): the channel takes the value over 255 and the
    /// picker refreshes from RGB.
    pub fn set_rgb_slider(
        &mut self,
        channel: usize,
        value_255: f32,
        swatch: Option<&mut SwatchControl>,
    ) -> Refresh {
        let mut colour = self.colour;
        colour[channel.min(2)] = value_255 / 255.0;
        self.refresh_rgb(colour, swatch)
    }

    /// An RGB control whose value is already 0–1.
    pub fn set_rgb_slider_raw(
        &mut self,
        channel: usize,
        value: f32,
        swatch: Option<&mut SwatchControl>,
    ) -> Refresh {
        let mut colour = self.colour;
        colour[channel.min(2)] = value;
        self.refresh_rgb(colour, swatch)
    }

    /// An HSV slider (0–255): the component takes the value over 255 and
    /// the picker refreshes from HSV.
    pub fn set_hsv_slider(
        &mut self,
        channel: usize,
        value_255: f32,
        swatch: Option<&mut SwatchControl>,
    ) -> Refresh {
        let mut colour = self.hsv;
        colour[channel.min(2)] = value_255 / 255.0;
        self.refresh_hsv(colour, swatch)
    }

    /// An HSV control whose value is already 0–1.
    pub fn set_hsv_slider_raw(
        &mut self,
        channel: usize,
        value: f32,
        swatch: Option<&mut SwatchControl>,
    ) -> Refresh {
        let mut colour = self.hsv;
        colour[channel.min(2)] = value;
        self.refresh_hsv(colour, swatch)
    }

    /// The colour field's point: its two coordinates become the hue and
    /// the saturation.
    pub fn set_hue_saturation(
        &mut self,
        hue: f32,
        saturation: f32,
        swatch: Option<&mut SwatchControl>,
    ) -> Refresh {
        let mut colour = self.hsv;
        colour[0] = hue;
        colour[1] = saturation;
        self.refresh_hsv(colour, swatch)
    }

    /// The revert button: the colour the swatch had when the picker opened.
    pub fn revert(&mut self, swatch: Option<&mut SwatchControl>) -> Refresh {
        self.refresh_rgb(self.original, swatch)
    }

    /// A colour control's bytes (as a swatch stores them) taken as the
    /// colour.
    pub fn set_bytes(&mut self, bytes: [u8; 4], swatch: Option<&mut SwatchControl>) -> Refresh {
        self.refresh_rgb(unpacked(bytes), swatch)
    }

    /// The finish button: the update word becomes 0x1000; the form's next
    /// step (a virtual slot outside this unit) follows.
    pub fn finish(&mut self) {
        self.word = FINISH_WORD;
    }

    /// Refreshes from an RGB source colour.
    pub fn refresh_rgb(&mut self, source: Rgba, swatch: Option<&mut SwatchControl>) -> Refresh {
        let bytes = rgb_bytes(source);
        self.ramps = channel_ramps(bytes);
        self.rgb_sliders = [source[0], source[1], source[2]];
        self.rgb_fields = self.rgb_sliders.map(|c| c * 255.0);
        self.preview = [bytes[0], bytes[1], bytes[2], 255];
        self.colour = source;
        if !self.syncing {
            self.syncing = true;
            let [h, s, v] = rgb_to_hsv([source[0], source[1], source[2]]);
            self.refresh_hsv([h, s, v, self.hsv[3]], None);
            self.syncing = false;
        }
        let mut refresh = Refresh {
            wrote_back: false,
            called_owner: false,
        };
        if let Some(swatch) = swatch
            && swatch.class == COLOUR_SWATCH_CLASS
        {
            swatch.colour = packed_bytes(self.colour);
            refresh.wrote_back = true;
            refresh.called_owner = swatch.callback;
        }
        self.word = REFRESH_WORD;
        refresh
    }

    /// Refreshes from an HSV source.
    pub fn refresh_hsv(&mut self, source: Hsv, swatch: Option<&mut SwatchControl>) -> Refresh {
        self.hsv = source;
        self.hsv_sliders = [source[0], source[1], source[2]];
        self.hsv_fields = self.hsv_sliders.map(|c| c * 255.0);
        if !self.syncing {
            let [r, g, b] = hsv(source[0], source[1], source[2]);
            self.colour = [r, g, b, self.colour[3]];
        }
        self.corners = field_corners(source);
        if self.syncing {
            return Refresh {
                wrote_back: false,
                called_owner: false,
            };
        }
        self.syncing = true;
        let refresh = self.refresh_rgb(self.colour, swatch);
        self.syncing = false;
        refresh
    }
}

/// Why a picker was opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opened {
    /// The swatch's picker was already open and was raised.
    Raised(usize),
    /// A new picker was made.
    Created(usize),
}

/// A control event the picker's controls dispatch to it, one per control
/// callback: the RGB and HSV sliders (a 0–255 slider divided by 255, or a
/// value already 0–1), the colour field's point (hue and saturation), a
/// colour control's bytes, the finish button and the revert button.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PickerEvent {
    /// An RGB slider on `channel`, its value 0–255.
    RgbSlider { channel: usize, value: f32 },
    /// An RGB control whose value is already 0–1.
    RgbRaw { channel: usize, value: f32 },
    /// An HSV slider on `channel`, its value 0–255.
    HsvSlider { channel: usize, value: f32 },
    /// An HSV control whose value is already 0–1.
    HsvRaw { channel: usize, value: f32 },
    /// The colour field's point: its two coordinates as hue and saturation.
    Field { hue: f32, saturation: f32 },
    /// A colour control's packed bytes.
    Bytes([u8; 4]),
    /// The finish button.
    Finish,
    /// The revert button: the colour the swatch had when the picker opened.
    Revert,
}

impl ColourPicker {
    /// Dispatches one control event to the picker's own rule: every event
    /// but the finish refreshes and answers its [`Refresh`]; the finish
    /// sets its word and answers nothing.
    pub fn dispatch(
        &mut self,
        event: PickerEvent,
        swatch: Option<&mut SwatchControl>,
    ) -> Option<Refresh> {
        match event {
            PickerEvent::RgbSlider { channel, value } => {
                Some(self.set_rgb_slider(channel, value, swatch))
            }
            PickerEvent::RgbRaw { channel, value } => {
                Some(self.set_rgb_slider_raw(channel, value, swatch))
            }
            PickerEvent::HsvSlider { channel, value } => {
                Some(self.set_hsv_slider(channel, value, swatch))
            }
            PickerEvent::HsvRaw { channel, value } => {
                Some(self.set_hsv_slider_raw(channel, value, swatch))
            }
            PickerEvent::Field { hue, saturation } => {
                Some(self.set_hue_saturation(hue, saturation, swatch))
            }
            PickerEvent::Bytes(bytes) => Some(self.set_bytes(bytes, swatch)),
            PickerEvent::Finish => {
                self.finish();
                None
            }
            PickerEvent::Revert => Some(self.revert(swatch)),
        }
    }
}

/// The unit's list of pickers, one per swatch control.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PickerSet {
    pub pickers: Vec<ColourPicker>,
}

impl PickerSet {
    /// Opens the picker of a swatch control: an open picker for that swatch
    /// is raised, else one is made and appended to the list.
    pub fn open(&mut self, swatch_id: usize, swatch: &mut SwatchControl) -> Opened {
        if let Some(index) = self
            .pickers
            .iter()
            .position(|picker| picker.swatch_id == swatch_id)
        {
            return Opened::Raised(index);
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

    fn swatch() -> SwatchControl {
        SwatchControl::new([51, 102, 153, 255])
    }

    #[test]
    fn the_events_reach_the_pickers_own_rules() {
        let mut swatch = swatch();
        let mut picker = ColourPicker::new(0, &mut swatch);
        // An HSV slider writes the HSV side over 255 and refreshes from it.
        assert!(
            picker
                .dispatch(
                    PickerEvent::HsvSlider {
                        channel: 2,
                        value: 255.0
                    },
                    Some(&mut swatch)
                )
                .is_some()
        );
        assert_eq!(picker.hsv[2], 1.0);
        // The field's point is the hue and the saturation, not RGB.
        picker.dispatch(
            PickerEvent::Field {
                hue: 0.25,
                saturation: 0.75,
            },
            None,
        );
        assert_eq!((picker.hsv[0], picker.hsv[1]), (0.25, 0.75));
        // The revert takes the opening colour back.
        picker.dispatch(
            PickerEvent::RgbSlider {
                channel: 0,
                value: 0.0,
            },
            None,
        );
        assert!(picker.dispatch(PickerEvent::Revert, None).is_some());
        assert_eq!(picker.colour, picker.original);
        // The finish sets the word and answers nothing.
        assert_eq!(picker.dispatch(PickerEvent::Finish, None), None);
        assert_eq!(picker.word, FINISH_WORD);
    }

    #[test]
    fn a_colour_becomes_bytes_with_round_to_even_and_the_low_byte() {
        assert_eq!(to_byte(0.0), 0);
        assert_eq!(to_byte(1.0), 255);
        assert_eq!(to_byte(0.5), 128);
        // 0.3 × 255 = 76.5 exactly ties to 76 (even).
        assert_eq!(0.3f32 * 255.0, 76.5);
        assert_eq!(to_byte(0.3), 76);
        // The refresh keeps the low byte of the rounded value...
        assert_eq!(to_byte(2.0), 254);
        assert_eq!(to_byte(f32::NAN), 0);
        // ...while the write-back saturates.
        assert_eq!(packed_bytes([2.0, -1.0, 0.3, 1.0]), [255, 0, 76, 255]);
        assert_eq!(unpacked([255, 0, 0, 51])[0], 1.0);
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
    fn opening_a_colour_swatch_refreshes_both_forms_and_writes_back() {
        let mut control = swatch();
        let picker = ColourPicker::new(0, &mut control);
        assert_eq!(picker.original, unpacked([51, 102, 153, 255]));
        assert_eq!(picker.ramps[0], [[0, 102, 153, 255], [255, 102, 153, 255]]);
        assert_eq!(picker.preview, [51, 102, 153, 255]);
        assert_eq!(picker.rgb_fields[0], picker.colour[0] * 255.0);
        // The HSV refresh ran under the guard: the corners exist.
        let [h, s, v] = rgb_to_hsv([picker.colour[0], picker.colour[1], picker.colour[2]]);
        assert_eq!(&picker.hsv[..3], &[h, s, v]);
        assert_eq!(picker.corners, field_corners(picker.hsv));
        assert_eq!(picker.word, REFRESH_WORD);
        assert!(!picker.syncing);
        assert_eq!(control.colour, [51, 102, 153, 255]);
        // Another control class leaves the picker blank and unrefreshed.
        let mut other = swatch();
        other.class = 1;
        let blank = ColourPicker::new(1, &mut other);
        assert_eq!((blank.colour, blank.word), ([0.0; 4], 0));
    }

    #[test]
    fn sliders_refresh_the_other_form_once() {
        let mut control = swatch();
        let mut picker = ColourPicker::new(0, &mut control);
        let refresh = picker.set_rgb_slider(0, 255.0, Some(&mut control));
        assert_eq!(
            refresh,
            Refresh {
                wrote_back: true,
                called_owner: true
            }
        );
        assert_eq!(picker.colour[0], 1.0);
        assert_eq!(control.colour[0], 255);
        // A hue slider converts to RGB and refreshes the rest.
        let before = picker.hsv;
        picker.set_hsv_slider(0, 127.5, Some(&mut control));
        assert_eq!(picker.hsv[1..], before[1..]);
        let rgb = hsv(0.5, before[1], before[2]);
        assert_eq!(&picker.colour[..3], &rgb);
        assert_eq!(picker.corners, field_corners(picker.hsv));
        assert!(!picker.syncing);
        // The field point sets hue and saturation.
        picker.set_hue_saturation(0.0, 1.0, None);
        assert_eq!((picker.hsv[0], picker.hsv[1]), (0.0, 1.0));
        // Revert goes back to the opening colour.
        picker.revert(Some(&mut control));
        assert_eq!(picker.colour, picker.original);
        assert_eq!(control.colour, [51, 102, 153, 255]);
        picker.finish();
        assert_eq!(picker.word, FINISH_WORD);
    }

    #[test]
    fn a_second_open_raises_the_same_swatch() {
        let mut control = swatch();
        let mut set = PickerSet::default();
        assert_eq!(set.open(7, &mut control), Opened::Created(0));
        assert_eq!(set.open(8, &mut control), Opened::Created(1));
        assert_eq!(set.open(7, &mut control), Opened::Raised(0));
        assert_eq!(set.pickers.len(), 2);
        assert_eq!(set.picker_for(7).unwrap().swatch_id, 7);
        assert_eq!(set.picker_for(9), None);
        set.finalize();
        assert!(set.pickers.is_empty());
    }
}
