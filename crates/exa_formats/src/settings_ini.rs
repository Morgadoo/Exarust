//! The text settings file (`<user folder><project>.ini`): `Name = value`
//! lines read through a registry of typed settings, the two ways the
//! original rewrites one line in place, the value formats, the graphics and
//! window settings with their rules and defaults, and the `KeyDef` hotkey
//! macros.
//!
//! Rules: `docs/formats/SETTINGS.md` (Settings file). Everything here is the
//! original's behaviour, quirks included: a line is split at its first `=`
//! (or space), names match ignoring ASCII case, the value loses the byte
//! before its line feed, and the game's own one-setting save starts its
//! replacement one byte before the name.

use std::path::{Path, PathBuf};

use crate::file_ref::{find_byte, read_whole_file, write_whole_file};

/// How a setting's value is converted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Registered but never converted.
    None,
    /// The raw text.
    Text,
    /// A decimal integer.
    Int,
    /// `1`, `true` or `yes`.
    Bool,
    /// A decimal fraction.
    Float,
    /// A hexadecimal colour; six digits or fewer are RGB, shifted to RGBA.
    Colour,
}

/// Which part of the original registers a setting (and owns its rules).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owner {
    /// The graphics and key settings, whose rules are [`Settings`]'s.
    Graphics,
    /// The launcher's window settings.
    Launcher,
    /// The hardware configuration and the settings screen.
    Hardware,
    /// The game's own settings.
    Game,
    /// The audio volume.
    Audio,
}

/// One registered setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Registration {
    pub name: &'static str,
    pub kind: Kind,
    pub owner: Owner,
}

const fn reg(name: &'static str, kind: Kind, owner: Owner) -> Registration {
    Registration { name, kind, owner }
}

/// Every setting the original registers, grouped by the part that does.
pub const REGISTRY: [Registration; 41] = [
    reg("GUISize", Kind::Int, Owner::Graphics),
    reg("HWMultisample", Kind::Int, Owner::Graphics),
    reg("ShadingQuality", Kind::Int, Owner::Graphics),
    reg("ShadowQuality", Kind::Int, Owner::Graphics),
    reg("ShadowDetail", Kind::Int, Owner::Graphics),
    reg("TextureQuality", Kind::Int, Owner::Graphics),
    reg("TextureDetail", Kind::Int, Owner::Graphics),
    reg("VerticalSync", Kind::Bool, Owner::Graphics),
    reg("Fullscreen", Kind::Bool, Owner::Graphics),
    reg("NoTopMost", Kind::Bool, Owner::Graphics),
    reg("Supersample", Kind::Int, Owner::Graphics),
    reg("FPSLimit", Kind::Int, Owner::Graphics),
    reg("KeyDef", Kind::Text, Owner::Graphics),
    reg("WindowWidth", Kind::Int, Owner::Launcher),
    reg("WindowHeight", Kind::Int, Owner::Launcher),
    reg("Touchscreen", Kind::Bool, Owner::Launcher),
    reg("StereoMode", Kind::Int, Owner::Launcher),
    reg("PerfIndex", Kind::Float, Owner::Hardware),
    reg("ShowFPS", Kind::Bool, Owner::Hardware),
    reg("Music", Kind::Bool, Owner::Hardware),
    reg("CamFollow", Kind::Bool, Owner::Hardware),
    reg("LinearRGB", Kind::Bool, Owner::Hardware),
    reg("SoftwareAudio", Kind::Bool, Owner::Hardware),
    reg("NoProcMips", Kind::Bool, Owner::Hardware),
    reg("NoDynamicMips", Kind::Bool, Owner::Hardware),
    reg("ShadowFmt", Kind::Int, Owner::Hardware),
    reg("DynamicVBO", Kind::Bool, Owner::Hardware),
    reg("ScreenWidth", Kind::Int, Owner::Hardware),
    reg("ScreenHeight", Kind::Int, Owner::Hardware),
    reg("SwapCopy", Kind::Bool, Owner::Hardware),
    reg("ForceVBO", Kind::Bool, Owner::Hardware),
    reg("GILowPrecision", Kind::Bool, Owner::Hardware),
    reg("InputMode", Kind::Int, Owner::Hardware),
    reg("AimSpeed", Kind::Int, Owner::Hardware),
    reg("CursorSize", Kind::Float, Owner::Game),
    reg("CursorInt", Kind::Colour, Owner::Game),
    reg("CursorCom", Kind::Colour, Owner::Game),
    reg("FirstRun", Kind::Bool, Owner::Game),
    reg("ShowGameTips", Kind::Bool, Owner::Game),
    reg("CheckPoints", Kind::Bool, Owner::Game),
    reg("Volume", Kind::Float, Owner::Audio),
];

/// A converted value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Text(Vec<u8>),
    Int(i32),
    Bool(bool),
    Float(f32),
    Colour(i32),
}

/// A line that named a registered setting and carried a value.
#[derive(Debug, Clone, PartialEq)]
pub struct Assignment {
    /// The index of the setting in the registry the file was read with.
    pub index: usize,
    pub value: Value,
}

/// The index of the first setting whose name matches, ignoring ASCII case.
pub fn lookup(registry: &[Registration], name: &[u8]) -> Option<usize> {
    registry
        .iter()
        .position(|r| r.name.as_bytes().eq_ignore_ascii_case(name))
}

/// Reads a settings file's lines into assignments, in file order. For each
/// line (to a line feed): leading bytes up to space are skipped; the name
/// ends at the first `=`, or at the first space when there is none (a line
/// with neither is skipped); trailing spaces leave the name, and a control
/// byte or space inside it restarts it after that byte. The value starts
/// after the separator and its leading spaces, and ends one byte before the
/// line feed (the CR of a CR LF), less trailing spaces; on the last line,
/// without a line feed, it runs to the end. An empty value or an unknown
/// name sets nothing.
pub fn read_assignments(bytes: &[u8], registry: &[Registration]) -> Vec<Assignment> {
    let len = bytes.len();
    let at = |i: i64| -> u8 {
        if i >= 0 && (i as usize) < len {
            bytes[i as usize]
        } else {
            0
        }
    };
    let window = |p: i64, n: i64| -> &[u8] {
        let start = (p.max(0) as usize).min(len);
        let end = ((p + n.max(0)).max(0) as usize).min(len);
        &bytes[start..end.max(start)]
    };
    let mut out = Vec::new();
    let mut p: i64 = 0;
    let mut n: i64 = len as i64;
    if n <= 0 {
        return out;
    }
    loop {
        while n > 0 && at(p) < 0x21 {
            p += 1;
            n -= 1;
        }
        let mut eol = find_byte(b'\n', window(p, n)) as i64;
        if eol < 0 {
            eol = n + 1;
        }
        let mut eq = find_byte(b'=', window(p, eol)) as i64;
        if eq < 0 {
            eq = find_byte(b' ', window(p, eol)) as i64;
        }
        if eq < 0 {
            p += eol;
            n -= eol;
        } else {
            let mut value_start = eq + 1;
            while eq > 0 && at(p + eq - 1) == b' ' {
                eq -= 1;
            }
            let mut i = 0;
            while i < eq {
                if at(p + i) < 0x21 {
                    let k = i + 1;
                    eq -= k;
                    value_start -= k;
                    p += k;
                    n -= k;
                    i = 0;
                } else {
                    i += 1;
                }
            }
            let index = lookup(registry, window(p, eq));
            p += value_start;
            n -= value_start;
            while at(p) == b' ' && n > 0 {
                p += 1;
                n -= 1;
            }
            let mut eol = find_byte(b'\n', window(p, n)) as i64;
            if eol < 0 {
                eol = n + 1;
            }
            let advance = eol + 1;
            if let Some(index) = index {
                let mut vlen = eol;
                loop {
                    vlen -= 1;
                    if vlen < 1 || at(p + vlen - 1) != b' ' {
                        break;
                    }
                }
                if vlen > 0 {
                    let text = window(p, vlen);
                    if let Some(value) = convert(registry[index].kind, text) {
                        out.push(Assignment { index, value });
                    }
                }
            }
            p += advance;
            n -= advance;
        }
        if n <= 0 {
            break;
        }
    }
    out
}

/// Converts a value's text by kind.
pub fn convert(kind: Kind, text: &[u8]) -> Option<Value> {
    Some(match kind {
        Kind::None => return None,
        Kind::Text => Value::Text(text.to_vec()),
        Kind::Int => Value::Int(parse_int(text)),
        Kind::Bool => Value::Bool(parse_bool(text)),
        Kind::Float => Value::Float(parse_float(text)),
        Kind::Colour => {
            let mut colour = parse_hex(text);
            if text.len() < 7 {
                colour = colour.wrapping_shl(8);
            }
            Value::Colour(colour)
        }
    })
}

/// A decimal integer: an optional leading `-`, then digits up to the first
/// other byte (none gives 0), wrapping as the original's 64-bit sum cut to
/// 32 bits.
pub fn parse_int(text: &[u8]) -> i32 {
    let negative = text.first() == Some(&b'-');
    let start = usize::from(negative);
    let mut v: i64 = 0;
    for &c in text.iter().skip(start) {
        if !c.is_ascii_digit() {
            break;
        }
        v = v.wrapping_mul(10).wrapping_add(i64::from(c) - 0x30);
    }
    if negative {
        v = v.wrapping_neg();
    }
    v as i32
}

/// `true` when the text starts with `1` or is `true` or `yes` in any ASCII
/// case.
pub fn parse_bool(text: &[u8]) -> bool {
    text.first() == Some(&b'1')
        || text.eq_ignore_ascii_case(b"true")
        || text.eq_ignore_ascii_case(b"yes")
}

/// A decimal fraction: an optional `-`, digits summed in single precision
/// (`(c + v·10) − 48`), then, when a `.` follows that is not the last byte,
/// each digit d after it adds d / 10ᵏ in double precision, rounded back to
/// single after each.
pub fn parse_float(text: &[u8]) -> f32 {
    let len = text.len() as i32;
    let byte = |i: i32| text[(i - 1) as usize];
    let mut v = 0.0f32;
    let mut i = if text.first() == Some(&b'-') { 2 } else { 1 };
    while i <= len && byte(i).is_ascii_digit() {
        v = (f32::from(byte(i)) + v * 10.0) - 48.0;
        i += 1;
    }
    if i < len && byte(i) == b'.' {
        let mut div: i32 = 10;
        i += 1;
        while i <= len && byte(i).is_ascii_digit() {
            let d = f64::from(byte(i) - 0x30);
            v = (d / f64::from(div) + f64::from(v)) as f32;
            div = div.wrapping_mul(10);
            i += 1;
        }
    }
    if text.first() == Some(&b'-') {
        v = -v;
    }
    v
}

/// Hexadecimal digits (either case) up to the first other byte.
pub fn parse_hex(text: &[u8]) -> i32 {
    let mut v: i32 = 0;
    for &c in text {
        let d = match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'f' => c - 0x57,
            b'A'..=b'F' => c - 0x37,
            _ => return v,
        };
        v = v.wrapping_mul(16).wrapping_add(i32::from(d));
    }
    v
}

/// An integer as the original writes it: `0`, or its digits with a leading
/// `-`. The smallest integer, whose negation overflows, writes as `-`.
pub fn format_int(v: i32) -> String {
    if v == 0 {
        return "0".into();
    }
    let negative = v < 0;
    let mut n = if negative { v.wrapping_neg() } else { v };
    let mut digits = String::new();
    while n > 0 {
        digits.insert(0, char::from(b'0' + (n % 10) as u8));
        n /= 10;
    }
    if negative {
        digits.insert(0, '-');
    }
    digits
}

/// A boolean as the original writes it.
pub fn format_bool(v: bool) -> &'static str {
    if v { "1" } else { "0" }
}

/// A fraction with `digits` decimals as the original writes it: with no
/// decimals the value rounded to even; otherwise `digits − 1` truncated
/// digits of the absolute fraction and a last one rounded to even, carrying
/// into the earlier digits (and the whole part, by its sign, when they are
/// all nines). Trailing zeros go unless `keep_zeros`, leaving one after the
/// point. A zero whole part writes as `0`, or `-0` for a negative value.
pub fn format_float(v: f32, digits: i32, keep_zeros: bool) -> String {
    if digits < 1 {
        return format_int(round_even(v));
    }
    let mut text: Vec<u8> = vec![b'.'];
    let mut f = frac(v.abs()) * 10.0;
    for _ in 1..digits {
        let d = f as i64 as i32;
        text.push((d + 0x30) as u8);
        f = (f - d as f32) * 10.0;
    }
    let last = round_even(f);
    let whole;
    if last < 10 {
        text.push((last + 0x30) as u8);
        whole = v as i64 as i32;
    } else {
        let mut k = digits as usize;
        while text.get(k - 1) == Some(&b'9') {
            text[k - 1] = b'0';
            k -= 1;
        }
        if k < 2 {
            let t = v as i64;
            whole = if v.is_sign_negative() { t - 1 } else { t + 1 } as i32;
        } else {
            text[k - 1] += 1;
            whole = v as i64 as i32;
        }
        text.push(b'0');
    }
    if !keep_zeros {
        let mut end = text.len();
        while end > 0 && text[end - 1] == b'0' {
            end -= 1;
        }
        if end > 0 && text[end - 1] == b'.' {
            end += 1;
        }
        text.truncate(end);
    }
    let mut out = if whole == 0 {
        if v.is_sign_negative() { "-0" } else { "0" }.to_string()
    } else {
        format_int(whole)
    };
    out.push_str(&String::from_utf8_lossy(&text));
    out
}

/// The fraction of `v` as the original takes it: `v − trunc(v)` with a
/// 32-bit truncation.
fn frac(v: f32) -> f32 {
    v - (v as i32) as f32
}

/// Single-precision rounding to the nearest integer, ties to even (the
/// processor's default mode).
fn round_even(v: f32) -> i32 {
    v.round_ties_even() as i32
}

/// The 0-based offset of the first occurrence of `needle` in `hay`,
/// scanning from offset `start − 1` (so a start of 0 or 1 scans the whole
/// text), or −1. Nothing matches when the needle is longer than the text
/// from `start` allows, or either is empty.
pub fn find_text(needle: &[u8], hay: &[u8], start: i32) -> i32 {
    if needle.is_empty() || hay.is_empty() || start < 0 {
        return -1;
    }
    let n = needle.len() as i64;
    if i64::from(start) + n - 1 > hay.len() as i64 || hay.len() < needle.len() {
        return -1;
    }
    let from = (start - 1).max(0) as usize;
    (from..=hay.len() - needle.len())
        .find(|&i| &hay[i..i + needle.len()] == needle)
        .map_or(-1, |i| i as i32)
}

/// Replaces `old` bytes at `at` with `text`.
fn splice(file: &mut Vec<u8>, at: usize, old: usize, text: &[u8]) {
    let at = at.min(file.len());
    let end = at.saturating_add(old).min(file.len());
    file.splice(at..end, text.iter().copied());
}

/// The line a setting is written as.
fn line(name: &[u8], value: &[u8]) -> Vec<u8> {
    [name, b" = ", value].concat()
}

/// Writes one setting into a file's text as the settings screen does: the
/// first occurrence of the name (anywhere, matching case) is replaced up to
/// the next CR (or to the end) by `name = value`; without one, a CR LF and
/// the line are appended.
pub fn put_setting(file: &mut Vec<u8>, name: &[u8], value: &[u8]) {
    let text = line(name, value);
    let pos = find_text(name, file, 0);
    if pos + 1 < 1 {
        file.extend_from_slice(b"\r\n");
        file.extend_from_slice(&text);
        return;
    }
    let after = pos as i64 + 1;
    let len = file.len() as i64;
    let mut old = cr_after(file, after, len - after - 1) + 1;
    if old < 1 {
        old = len - after + 1;
    }
    splice(file, pos as usize, old.max(0) as usize, &text);
}

/// Writes one setting into a file's text as the game's one-setting save
/// does: as [`put_setting`], but a name found at the very start counts as
/// missing (so the line is appended), and the replacement starts one byte
/// before the name, so the line feed before it goes.
pub fn save_one_setting(file: &mut Vec<u8>, name: &[u8], value: &[u8]) {
    let text = line(name, value);
    let pos = find_text(name, file, 0);
    if pos < 1 {
        file.extend_from_slice(b"\r\n");
        file.extend_from_slice(&text);
        return;
    }
    let pos = pos as i64;
    let len = file.len() as i64;
    let mut old = cr_after(file, pos, len - pos - 1) + 1;
    if old < 1 {
        old = len - pos + 1;
    }
    splice(file, (pos - 1) as usize, old.max(0) as usize, &text);
}

/// The offset of the first CR in `count` bytes from `from`, or −1.
fn cr_after(file: &[u8], from: i64, count: i64) -> i64 {
    if count <= 0 || from < 0 {
        return -1;
    }
    let start = (from as usize).min(file.len());
    let end = (from + count).clamp(0, file.len() as i64) as usize;
    find_byte(b'\r', &file[start..end.max(start)]) as i64
}

/// The folder the original keeps its user files in: the installation's
/// `Userdata` folder when it exists, else the user's application-data
/// folder and the project's name.
pub fn user_folder(
    game_dir: &Path,
    userdata_exists: bool,
    app_data: &Path,
    project: &str,
) -> PathBuf {
    if userdata_exists {
        game_dir.join("Userdata")
    } else {
        app_data.join(project)
    }
}

/// The settings file: the project's name and `.ini` in the user folder.
pub fn settings_file(folder: &Path, project: &str) -> PathBuf {
    folder.join(format!("{project}.ini"))
}

/// The settings store's file: the project's name and `.set`.
pub fn store_file(folder: &Path, project: &str) -> PathBuf {
    folder.join(format!("{project}.set"))
}

/// Reads the settings file, creating it empty when it cannot be read.
pub fn open_settings_file(path: &Path) -> Vec<u8> {
    match read_whole_file(path) {
        Some(bytes) => bytes,
        None => {
            write_whole_file(path, &[]);
            Vec::new()
        }
    }
}

/// Saves one setting the way the game does: reads the file, rewrites the
/// line ([`save_one_setting`]) and writes it back. Nothing is written when
/// the file cannot be read. Whether it was written.
pub fn save_one_setting_to_file(path: &Path, name: &str, value: &str) -> bool {
    let Some(mut file) = read_whole_file(path) else {
        return false;
    };
    save_one_setting(&mut file, name.as_bytes(), value.as_bytes());
    write_whole_file(path, &file)
}

/// A modifier of a `KeyDef` hotkey, as the bit the original passes.
pub const SHIFT: u8 = 0x20;
pub const CTRL: u8 = 0x40;
pub const ALT: u8 = 0x80;
/// How many `KeyDef` hotkeys get a key handler.
pub const BOUND_HOTKEYS: usize = 6;

/// A `KeyDef` hotkey: a modifier bit, a key byte and its commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hotkey {
    pub modifier: u8,
    pub key: u8,
    pub commands: Vec<Vec<u8>>,
    /// One of the first six, which the original binds to a handler.
    pub bound: bool,
}

/// Parses a `KeyDef` value, `<shift|ctrl|alt>+<key>: cmd; cmd; …`, adding
/// a hotkey to `hotkeys`. Nothing is added without a colon, with another
/// modifier, without a colon after the key, or when at most one byte
/// follows that colon and its spaces. The key byte is the first after the
/// `+` and its spaces, with `[` and above moved down by 0x20 (lower case to
/// upper). Commands are split at `;`, lose their leading and trailing
/// spaces, and a trailing single byte is dropped.
pub fn parse_key_def(text: &[u8], hotkeys: &mut Vec<Hotkey>) {
    let len = text.len() as i64;
    let at = |i: i64| -> u8 {
        if i >= 0 && i < len {
            text[i as usize]
        } else {
            0
        }
    };
    let window = |p: i64, n: i64| -> &[u8] {
        let start = p.clamp(0, len) as usize;
        let end = (p + n.max(0)).clamp(0, len) as usize;
        &text[start..end.max(start)]
    };
    if find_byte(b':', text) < 0 {
        return;
    }
    let plus = find_byte(b'+', text) as i64;
    let mut m = plus;
    while m > 0 && at(m - 1) == b' ' {
        m -= 1;
    }
    let name = window(0, m);
    let modifier = if name.eq_ignore_ascii_case(b"shift") {
        SHIFT
    } else if name.eq_ignore_ascii_case(b"ctrl") {
        CTRL
    } else if name.eq_ignore_ascii_case(b"alt") {
        ALT
    } else {
        return;
    };
    let rest = len - plus;
    let mut i = 1;
    while i < rest && at(plus + i) == b' ' {
        i += 1;
    }
    let mut b = at(plus + i);
    if b < 0x5b {
        b = b.wrapping_add(0x20);
    }
    let key = b.wrapping_sub(0x20);
    let base = plus + i;
    let mut rem = rest - i;
    let mut c = find_byte(b':', window(base, rem)) as i64 + 1;
    if c <= 0 {
        return;
    }
    while c < rem && at(base + c) == b' ' {
        c += 1;
    }
    let mut cmd = base + c;
    rem -= c;
    if rem <= 1 {
        return;
    }
    let bound = hotkeys.len() < BOUND_HOTKEYS;
    let mut commands = Vec::new();
    while rem > 1 {
        while rem > 0 && at(cmd) == b' ' {
            cmd += 1;
            rem -= 1;
        }
        let mut e = find_byte(b';', window(cmd, rem)) as i64 + 1;
        let mut advance = e;
        if e < 1 {
            e = rem + 1;
            advance = e;
        }
        loop {
            e -= 1;
            if e < 1 || at(cmd + e - 1) != b' ' {
                break;
            }
        }
        commands.push(window(cmd, e.max(0)).to_vec());
        cmd += advance;
        rem -= advance;
    }
    hotkeys.push(Hotkey {
        modifier,
        key,
        commands,
        bound,
    });
}

/// The settings a fresh installation starts with, with the original's values
/// before the file is read.
///
/// The graphics-and-keys, launcher and hardware-configuration settings that
/// Exarust reads are here. The hardware ones are kept as read; the launcher
/// applies the ones it can and documents the rest.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    /// The GUI size offset (see [`Settings::apply`] for its rule).
    pub gui_size: i32,
    /// Set when a positive GUI size was read on a display 800 pixels tall.
    pub compact_display: bool,
    pub hw_multisample: i32,
    /// The shading level, at least 3.
    pub shading_quality: i32,
    /// A render word the shading level picks: bit 0x80 is set from level 4
    /// (`0x2f3`, else `0x273`). Its readers are not traced.
    pub shading_flags: u32,
    /// Set from shading level 6. Its readers are not traced.
    pub shading_extra: bool,
    pub shadow_quality: i32,
    /// At least 3.
    pub shadow_detail: i32,
    /// At least 2.
    pub texture_quality: i32,
    /// At least 2.
    pub texture_detail: i32,
    pub vertical_sync: bool,
    pub fullscreen: bool,
    pub no_top_most: bool,
    /// At most 4.
    pub supersample: i32,
    /// The frame cap the `FPSLimit` setting gives: 0 below 10 (no cap),
    /// otherwise 1000 / trunc(1000 / limit); −1 until set.
    pub fps_cap: i32,
    /// The `KeyDef` hotkeys, in file order.
    pub hotkeys: Vec<Hotkey>,
    pub window_width: i32,
    pub window_height: i32,
    pub touchscreen: bool,
    pub stereo_mode: i32,
    /// The last GPU benchmark's rate over 60.
    pub perf_index: f32,
    pub show_fps: bool,
    pub music: bool,
    /// Whether the camera follows the player (read only; not traced).
    pub cam_follow: bool,
    /// Whether the renderer works in linear colour (read only; not traced).
    pub linear_rgb: bool,
    pub software_audio: bool,
    pub no_proc_mips: bool,
    pub no_dynamic_mips: bool,
    /// The shadow-map format word (read only; not traced).
    pub shadow_format: i32,
    pub dynamic_vbo: bool,
    /// The full-screen display mode's width and height (kept; Bevy's
    /// borderless full screen uses the monitor's).
    pub screen_width: i32,
    pub screen_height: i32,
    pub swap_copy: bool,
    pub force_vbo: bool,
    /// Whether the voxel GI uses its low-precision format. Exarust's GI is
    /// already 8-bit.
    pub gi_low_precision: bool,
    /// The input mode word: 0 for the keyboard, 51 for a controller.
    pub input_mode: i32,
    /// The aim speed as a fraction, from the file's 1-based step.
    pub aim_speed: f32,
    /// The master volume, 0 to 1.5.
    pub volume: f32,
    /// Whether gameplay tips are shown.
    pub show_game_tips: bool,
    /// Whether the game makes checkpoints.
    pub checkpoints: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            gui_size: 0,
            compact_display: false,
            hw_multisample: 0,
            shading_quality: 4,
            shading_flags: 0x2f3,
            shading_extra: false,
            shadow_quality: 4,
            shadow_detail: 4,
            texture_quality: 4,
            texture_detail: 3,
            vertical_sync: true,
            fullscreen: false,
            no_top_most: false,
            supersample: 0,
            fps_cap: -1,
            hotkeys: Vec::new(),
            window_width: 1600,
            window_height: 1080,
            touchscreen: false,
            stereo_mode: 0,
            perf_index: 0.0,
            show_fps: false,
            music: true,
            // The three defaults below are guesses: the handlers were not
            // traced, and only their variables' kinds are known.
            cam_follow: true,
            linear_rgb: false,
            software_audio: false,
            no_proc_mips: false,
            no_dynamic_mips: false,
            shadow_format: 0,
            dynamic_vbo: false,
            screen_width: 0,
            screen_height: 0,
            swap_copy: false,
            force_vbo: false,
            gi_low_precision: false,
            input_mode: 0,
            aim_speed: 0.29,
            volume: 1.0,
            show_game_tips: true,
            checkpoints: true,
        }
    }
}

impl Settings {
    /// Applies one assignment read with [`REGISTRY`]; settings owned by
    /// other parts are left to them. `display_height` is the display's
    /// height in pixels, which the GUI size rule reads.
    pub fn apply(&mut self, assignment: &Assignment, display_height: u32) {
        let Some(reg) = REGISTRY.get(assignment.index) else {
            return;
        };
        let int = match assignment.value {
            Value::Int(v) => v,
            _ => 0,
        };
        let flag = matches!(assignment.value, Value::Bool(true));
        match reg.name {
            // A positive size is 1 on a display 800 pixels tall (which is
            // also flagged as compact) and 0 on any other.
            "GUISize" => {
                self.gui_size = int;
                if int > 0 {
                    if display_height == 800 {
                        self.gui_size = 1;
                        self.compact_display = true;
                    } else {
                        self.gui_size = 0;
                    }
                }
            }
            "HWMultisample" => self.hw_multisample = int,
            "ShadingQuality" => {
                self.shading_quality = int.max(3);
                self.shading_flags = if self.shading_quality >= 4 {
                    0x2f3
                } else {
                    0x273
                };
                self.shading_extra = self.shading_quality >= 6;
            }
            "ShadowQuality" => self.shadow_quality = int,
            "ShadowDetail" => self.shadow_detail = int.max(3),
            "TextureQuality" => self.texture_quality = int.max(2),
            "TextureDetail" => self.texture_detail = int.max(2),
            "VerticalSync" => self.vertical_sync = flag,
            "Fullscreen" => self.fullscreen = flag,
            "NoTopMost" => self.no_top_most = flag,
            "Supersample" => self.supersample = int.min(4),
            "FPSLimit" => self.fps_cap = fps_cap(int),
            "KeyDef" => {
                if let Value::Text(text) = &assignment.value {
                    parse_key_def(text, &mut self.hotkeys);
                }
            }
            "WindowWidth" => self.window_width = int,
            "WindowHeight" => self.window_height = int,
            // Only a true value does anything.
            "Touchscreen" => self.touchscreen |= flag,
            "StereoMode" => self.stereo_mode = int,
            // The hardware configuration's settings (the settings screen's
            // benchmark and startup keys).
            _ => crate::hw_config::apply_setting(self, reg.name, &assignment.value),
        }
    }

    /// The settings a file's text gives over the defaults, and every
    /// assignment it made, in order.
    pub fn read(bytes: &[u8], display_height: u32) -> (Self, Vec<Assignment>) {
        let assignments = read_assignments(bytes, &REGISTRY);
        let mut settings = Self::default();
        for a in &assignments {
            settings.apply(a, display_height);
        }
        (settings, assignments)
    }
}

/// The frame cap an `FPSLimit` gives: 0 below 10; otherwise the
/// millisecond frame time 1000 / limit, truncated, back to frames. A limit
/// above 1000 makes the original divide by zero; here it gives no cap.
pub fn fps_cap(limit: i32) -> i32 {
    if limit < 10 {
        return 0;
    }
    let frame_ms = (1000.0 / f64::from(limit)) as i64;
    1000i64.checked_div(frame_ms).map_or(0, |v| v as i32)
}

/// Whether an assignment set the named setting.
pub fn sets(assignments: &[Assignment], name: &str) -> bool {
    assignments
        .iter()
        .any(|a| REGISTRY.get(a.index).is_some_and(|r| r.name == name))
}

#[cfg(test)]
#[path = "settings_ini_test.rs"]
mod tests;
