//! The engine's text and number helpers: decimal, hexadecimal and float
//! formatting, parsers for the same, the fixed 16-byte name type and its
//! helpers, the byte and string compares, UTF-8/Latin-1 conversion, hex
//! colours, the a/an vowel test and title-casing.
//!
//! The formatting rules are the original's, not Rust's: integers are built
//! from the least significant digit and prepended; the two-digit formatter
//! writes zero as `"00"`, pads a lone digit with a zero and writes a
//! negative value as `"0"`; the float formatter takes each fractional digit
//! with a truncating conversion and the last with a rounding one (ties to
//! even), carries a last digit of ten through trailing nines into the
//! integer part, and drops trailing zeros unless the caller keeps them. A
//! float with fewer than one decimal place is the rounding conversion to an
//! integer. Rounding assumes the processor's default mode (to nearest).
//!
//! Indexes follow the original: the search takes a 1-based start and
//! returns a 0-based position, and the last-byte search behind the name
//! helpers reports position 0, not "absent", for an empty text.
//!
//! Evidence: the text-formatting part of the core unit in
//! `docs/COMPONENT-MAP.md`; the assembly pins the conversion instructions,
//! the precision of each operation and the branch order.

/// The digit a hexadecimal value uses, upper case.
const HEX_DIGITS: &[u8; 16] = b"0123456789ABCDEF";

/// The bit population count (`POPCNT`).
pub fn popcount(value: u32) -> i32 {
    value.count_ones() as i32
}

/// The floor of the base-two logarithm, by halving until one or less. The
/// original compares the value as a signed word, so a value with the top
/// bit set counts no times and gives 0.
pub fn log2_floor(value: u32) -> i32 {
    let mut value = value as i32;
    let mut count = 0;
    while value > 1 {
        count += 1;
        value = ((value as u32) >> 1) as i32;
    }
    count
}

/// The 0-based position of the first occurrence of `needle` in `text`
/// whose window starts at or after the 1-based position `from`; `-1` when
/// there is none. `needle` is the original's first argument, `text` its
/// second. The search is refused (`-1`) when either is empty, `from` is
/// negative, or the first window (its last byte at 1-based `from +
/// needle.len() - 1`) runs past the text.
///
/// `from` 0 makes the first window start one byte before the text, where
/// the original reads the top byte of the string's length word (zero); a
/// match there returns its position, `-1`. The original's unrolled loop
/// tests each window's last byte first, then the rest; the first full
/// match in text order is the same.
pub fn find_from(needle: &[u8], text: &[u8], from: i32) -> i32 {
    if needle.is_empty() || text.is_empty() || from < 0 {
        return -1;
    }
    let needle_len = needle.len() as i64;
    let text_len = text.len() as i64;
    let first = i64::from(from) - 1;
    if first + needle_len > text_len {
        return -1;
    }
    let byte = |at: i64| if at < 0 { 0 } else { text[at as usize] };
    (first..=text_len - needle_len)
        .find(|&start| {
            needle
                .iter()
                .enumerate()
                .all(|(k, &want)| byte(start + k as i64) == want)
        })
        .map_or(-1, |start| start as i32)
}

/// `CVTSS2SI`: round half to even; a NaN or a value out of range is
/// `i32::MIN`, the processor's integer indefinite.
pub fn round_to_i32(value: f32) -> i32 {
    let rounded = value.round_ties_even();
    if rounded.is_nan() || rounded >= 2_147_483_648.0 || rounded < -2_147_483_648.0 {
        i32::MIN
    } else {
        rounded as i32
    }
}

/// `CVTTSS2SI`: truncate toward zero; a NaN or a value out of range is
/// `i32::MIN`.
pub fn trunc_to_i32(value: f32) -> i32 {
    if value.is_nan() || value >= 2_147_483_648.0 || value < -2_147_483_648.0 {
        i32::MIN
    } else {
        value as i32
    }
}

/// The fractional part of a float: `x` less its truncation to a 32-bit
/// integer. Out-of-range values are changed by the wrapping conversion,
/// as the original's are.
fn fraction(value: f32) -> f32 {
    value - trunc_to_i32(value) as f32
}

/// An integer as decimal text (the original's integer formatter). Zero is
/// `"0"`; a negative value is its magnitude behind a minus; the most
/// negative word has no representable magnitude and comes out as `"-"`.
pub fn format_int(value: i32) -> String {
    if value == 0 {
        return "0".to_string();
    }
    let negative = value < 0;
    let mut value = if negative {
        value.wrapping_neg()
    } else {
        value
    };
    let mut digits = Vec::new();
    while value > 0 {
        digits.push(b'0' + (value % 10) as u8);
        value /= 10;
    }
    digits.reverse();
    let mut out = String::with_capacity(digits.len() + usize::from(negative));
    if negative {
        out.push('-');
    }
    out.push_str(&String::from_utf8_lossy(&digits));
    out
}

/// An integer as at least two decimal digits (the original's clock-style
/// formatter): zero is `"00"` and a one-digit value takes a leading zero.
/// A negative value writes no digits and gets the one zero: `"0"`.
pub fn format_int_two(value: i32) -> String {
    if value == 0 {
        return "00".to_string();
    }
    let mut value = value;
    let mut digits = Vec::new();
    while value > 0 {
        digits.push(b'0' + (value % 10) as u8);
        value /= 10;
    }
    digits.reverse();
    let mut out = String::with_capacity(2);
    if digits.len() < 2 {
        out.push('0');
    }
    out.push_str(&String::from_utf8_lossy(&digits));
    out
}

/// An integer as upper-case hexadecimal, zero-padded to `width` digits
/// (the original's hex formatter). Zero is `"0"` without padding. A
/// negative value is its magnitude's digits, padded, behind a minus; the
/// most negative value has no magnitude, so it is `width` zeros behind the
/// minus. A `width` of one or less pads nothing; `i32::MIN`, which the
/// original turns into a count of 2^31 zeros, pads nothing here either.
pub fn format_hex(value: i64, width: i32) -> String {
    if value == 0 {
        return "0".to_string();
    }
    let negative = value < 0;
    // `i64::MIN` negates to itself and writes no digits.
    let mut magnitude = if negative {
        value.wrapping_neg()
    } else {
        value
    };
    let mut digits = Vec::new();
    while magnitude > 0 {
        digits.push(HEX_DIGITS[(magnitude % 16) as usize]);
        magnitude >>= 4;
    }
    // The original pads when the 32-bit length is not above `width - 1`,
    // which for every width but `i32::MIN` is `len < width`.
    let len = digits.len() as i32;
    let pad = if width != i32::MIN && len < width {
        (width - len) as usize
    } else {
        0
    };
    let mut out = String::with_capacity(pad + digits.len() + 1);
    if negative {
        out.push('-');
    }
    out.extend(std::iter::repeat_n('0', pad));
    out.extend(digits.iter().rev().map(|&digit| char::from(digit)));
    out
}

/// `CVTTSS2SI` into a 64-bit register: truncate toward zero; a NaN or a
/// value out of range is `i64::MIN`.
fn trunc_to_i64(value: f32) -> i64 {
    if value.is_nan()
        || value >= 9_223_372_036_854_775_808.0
        || value < -9_223_372_036_854_775_808.0
    {
        i64::MIN
    } else {
        value as i64
    }
}

/// `CVTSS2SI` into a 64-bit register: round half to even; a NaN or a value
/// out of range is `i64::MIN`.
fn round_to_i64(value: f32) -> i64 {
    trunc_to_i64(value.round_ties_even())
}

/// A float with `decimals` decimal places (the original's fixed-point
/// formatter). `keep_zeros` is the original's last argument: when it is
/// false, trailing zeros are dropped and a bare trailing point keeps one.
///
/// Below one decimal place the value goes through the 64-bit rounding
/// conversion and its low word is written by [`format_int`]. Otherwise:
/// the fraction of the magnitude (less its 32-bit truncation) is multiplied
/// by ten in single precision; each digit but the last is its 64-bit
/// truncation (low word), the remainder times ten carrying on; the last
/// digit is the 64-bit rounding conversion (ties to even). A last digit
/// above nine turns the trailing nines into zeros and adds one to the digit
/// before them, or, when the nines reach the point, moves the integer part
/// one away from zero (by the sign bit); a `0` is then appended. The
/// integer part is the 64-bit truncation's low word, written as `0` or `-0`
/// (sign bit set) when it is zero. A digit byte is `(digit + '0') & 0xff`;
/// bytes are mapped one to one to the characters U+0000 to U+00FF, so the
/// out-of-range bytes that a value of 2^31 or more produces stay distinct.
pub fn format_float(value: f32, decimals: i32, keep_zeros: bool) -> String {
    if decimals < 1 {
        return format_int(round_to_i64(value) as i32);
    }
    let mut out = vec![b'.'];
    let mut fraction = fraction(value.abs()) * 10.0;
    let mut written = 0;
    while written < decimals - 1 {
        written += 1;
        let digit = trunc_to_i64(fraction) as i32;
        out.push(digit.wrapping_add(0x30) as u8);
        fraction = (fraction - digit as f32) * 10.0;
    }
    let last = round_to_i64(fraction) as i32;
    let negative = value.to_bits() & 0x8000_0000 != 0;
    let integer = if last <= 9 {
        out.push(last.wrapping_add(0x30) as u8);
        trunc_to_i64(value) as i32
    } else {
        // `at` is the original's 1-based index; the point at index 1 stops
        // the walk.
        let mut at = decimals as usize;
        while out[at - 1] == b'9' {
            out[at - 1] = b'0';
            at -= 1;
        }
        let integer = if at < 2 {
            let step = if negative { -1 } else { 1 };
            trunc_to_i64(value).wrapping_add(step) as i32
        } else {
            out[at - 1] = out[at - 1].wrapping_add(1);
            trunc_to_i64(value) as i32
        };
        out.push(b'0');
        integer
    };
    if !keep_zeros {
        let mut len = out.len();
        while out[len - 1] == b'0' {
            len -= 1;
        }
        if out[len - 1] == b'.' {
            len += 1;
        }
        out.truncate(len);
    }
    let prefix = match integer {
        0 if negative => "-0".to_string(),
        0 => "0".to_string(),
        _ => format_int(integer),
    };
    prefix + &out.iter().map(|&byte| char::from(byte)).collect::<String>()
}

/// An integer from decimal text (the original's own loop, not an RTL
/// conversion): an optional leading minus, then digits up to the first
/// non-digit, accumulated as a wrapping 64-bit `value·10 + byte − 48`. No
/// digits (including a bare minus) is 0; trailing garbage is ignored.
pub fn int_from_text(text: &[u8]) -> i64 {
    let start = usize::from(text.first() == Some(&b'-'));
    let mut value: i64 = 0;
    for &byte in &text[start..] {
        if !byte.is_ascii_digit() {
            break;
        }
        value = value.wrapping_mul(10).wrapping_add(i64::from(byte) - 0x30);
    }
    if text.first() == Some(&b'-') {
        value.wrapping_neg()
    } else {
        value
    }
}

/// An integer from the first `len` bytes of decimal text, the same loop as
/// [`int_from_text`] over a pointer and a length. The original reads `len`
/// bytes whatever the text holds; a slice shorter than `len` is read whole.
pub fn int_from_bytes(text: &[u8], len: usize) -> i64 {
    int_from_text(text.get(..len).unwrap_or(text))
}

/// A float from decimal text: an optional leading minus, digits, an
/// optional point and digits; no exponent and nothing after the fractional
/// run. The integer digits accumulate in single precision as
/// `(byte + value·10) − 48`; each fractional digit is divided by its power
/// of ten (a 32-bit integer, wrapping) and added to the value in double
/// precision, then rounded back to single. A minus flips the sign bit at
/// the end, so `"-0"` is −0.0.
pub fn float_from_text(text: &[u8]) -> f32 {
    let mut value = 0.0f32;
    let mut at = usize::from(text.first() == Some(&b'-'));
    while at < text.len() && text[at].is_ascii_digit() {
        value = f32::from(text[at]) + value * 10.0 - 48.0;
        at += 1;
    }
    if at < text.len() && text[at] == b'.' {
        at += 1;
        let mut divisor = 10i32;
        while at < text.len() && text[at].is_ascii_digit() {
            value = (f64::from(text[at] - 0x30) / f64::from(divisor) + f64::from(value)) as f32;
            divisor = divisor.wrapping_mul(10);
            at += 1;
        }
    }
    if text.first() == Some(&b'-') {
        f32::from_bits(value.to_bits() ^ 0x8000_0000)
    } else {
        value
    }
}

/// An integer from hexadecimal text, upper or lower case, up to the first
/// byte that is not a hex digit, as a wrapping 32-bit `value·16 + digit`.
/// No sign is read; no digits is 0.
pub fn hex_from_text(text: &[u8]) -> i32 {
    let mut value: i32 = 0;
    for &byte in text {
        let digit = match byte {
            b'0'..=b'9' => i32::from(byte) - 0x30,
            b'a'..=b'f' => i32::from(byte) - 0x57,
            b'A'..=b'F' => i32::from(byte) - 0x37,
            _ => break,
        };
        value = value.wrapping_mul(16).wrapping_add(digit);
    }
    value
}

/// An integer from the first `len` bytes of hexadecimal text
/// ([`hex_from_text`] over a pointer and a length; a shorter slice is read
/// whole).
pub fn hex_from_bytes(text: &[u8], len: usize) -> i32 {
    hex_from_text(text.get(..len).unwrap_or(text))
}

/// A colour from six hexadecimal digits: three bytes in text order, each
/// written after its second digit. The four output bytes start at zero; the
/// first byte that is not a hex digit ends the read, so the bytes not
/// reached stay zero and a lone last digit is dropped. The fourth byte
/// (alpha) is never written and stays zero.
pub fn colour_from_hex6(text: &[u8]) -> [u8; 4] {
    colour_from_hex(text, 5)
}

/// A colour from eight hexadecimal digits: four bytes in text order, the
/// fourth from the seventh and eighth digits ([`colour_from_hex6`]'s rules
/// otherwise).
pub fn colour_from_hex8(text: &[u8]) -> [u8; 4] {
    colour_from_hex(text, 7)
}

/// The original's hex-colour loop: digit indices 0 to `last`, a byte
/// written after each odd index.
fn colour_from_hex(text: &[u8], last: usize) -> [u8; 4] {
    let mut out = [0u8; 4];
    let mut byte = 0u8;
    let mut written = 0usize;
    for index in 0..=last {
        let Some(&raw) = text.get(index) else {
            break;
        };
        let digit = match raw {
            b'0'..=b'9' => raw.wrapping_sub(0x30),
            b'a'..=b'f' => raw.wrapping_sub(0x57),
            b'A'..=b'F' => raw.wrapping_sub(0x37),
            _ => break,
        };
        byte = byte.wrapping_mul(16).wrapping_add(digit);
        if written * 2 < index {
            if written < out.len() {
                out[written] = byte;
            }
            byte = 0;
            written += 1;
        }
    }
    out
}

/// The original's backward byte search: the 0-based index of the last
/// `byte` in `text`, or -1. An empty text gives 0, not -1: the scan runs no
/// step and the zero flag left by the routine's own setup reads as a find.
fn last_index(byte: u8, text: &[u8]) -> i32 {
    if text.is_empty() {
        return 0;
    }
    text.iter()
        .rposition(|&at| at == byte)
        .map_or(-1, |at| at as i32)
}

/// The 16-byte extension of a length-bounded name (the original's
/// extension helper): the bytes after the last dot, zero-padded. `None`
/// when there is no dot or the extension is longer than sixteen bytes; a
/// dot as the first byte is fine. An empty name gives an all-zero
/// extension (see [`last_index`]). The original zeroes its output before
/// it looks, so a refused name also leaves zeros there.
pub fn extension_name(text: &[u8]) -> Option<[u8; 16]> {
    let mut out = [0u8; 16];
    let at = last_index(b'.', text) + 1;
    if at <= 0 {
        return None;
    }
    let len = text.len() as i64 - i64::from(at);
    if len > 16 {
        return None;
    }
    if len > 0 {
        out[..len as usize].copy_from_slice(&text[at as usize..]);
    }
    Some(out)
}

/// The 16-byte extension of a string ([`extension_name`], with the length
/// taken from the string, and an empty string also giving zeros). The
/// original copies the whole extension without a bound, past its sixteen
/// bytes; Exarust cuts it to sixteen.
pub fn extension_of_str(text: &str) -> Option<[u8; 16]> {
    let bytes = text.as_bytes();
    let at = last_index(b'.', bytes) + 1;
    if at <= 0 {
        return None;
    }
    let mut out = [0u8; 16];
    let tail = bytes.get(at as usize..).unwrap_or_default();
    let len = tail.len().min(16);
    out[..len].copy_from_slice(&tail[..len]);
    Some(out)
}

/// A name with a new extension: the original copies the name, finds the
/// last dot among its sixteen bytes (else its length up to a zero, at most
/// sixteen) and, when that is below sixteen, fills every byte from there
/// to the end from the extension, itself a zero-padded 16-byte name; the
/// dot is overwritten. `extension` is that name's leading bytes; the bytes
/// past it are written as the zeros of its padding.
pub fn replace_extension(name: &[u8; 16], extension: &[u8]) -> [u8; 16] {
    let mut out = *name;
    let found = name.iter().rposition(|&byte| byte == b'.');
    let at = found.unwrap_or_else(|| {
        name.iter()
            .position(|&byte| byte == 0)
            .unwrap_or(name.len())
    });
    for (k, slot) in out.iter_mut().enumerate().skip(at) {
        *slot = extension.get(k - at).copied().unwrap_or(0);
    }
    out
}

/// A name without its extension, followed by `suffix`: the bytes before the
/// last dot when there is one, else the whole name.
pub fn strip_extension_append(name: &str, suffix: &str) -> String {
    let bytes = name.as_bytes();
    match last_index(b'.', bytes) {
        -1 => format!("{name}{suffix}"),
        at => format!("{}{suffix}", &name[..at as usize]),
    }
}

/// The part of a path after its last backslash; the whole path when there
/// is none.
pub fn file_name_of(path: &str) -> &str {
    match path.rfind('\\') {
        Some(at) => &path[at + 1..],
        None => path,
    }
}

/// The part of a path up to and including its last backslash; empty when
/// the first byte is a backslash or there is none.
pub fn directory_of(path: &str) -> &str {
    match path.rfind('\\') {
        Some(at) if at > 0 => &path[..=at],
        _ => "",
    }
}

/// Three bytes read as a decimal number the original's way, with no digit
/// check: `(b0 − '0')·100 + (b1 − '0')·10 + b2 − '0'`, wrapping. The
/// original reads three bytes unconditionally; a missing byte counts as
/// zero here.
pub fn three_digits(bytes: &[u8]) -> i32 {
    let byte = |at: usize| i32::from(bytes.get(at).copied().unwrap_or(0));
    byte(0)
        .wrapping_sub(0x30)
        .wrapping_mul(100)
        .wrapping_add(byte(1).wrapping_sub(0x30).wrapping_mul(10))
        .wrapping_add(byte(2))
        .wrapping_sub(0x30)
}

/// Whether `word` starts with a letter the original counts as a vowel
/// (lower-case a, e, i, o or u, nothing else) and so takes "an". The
/// original tests one byte; this tests the first byte of `word`, and an
/// empty word is false.
pub fn takes_an(word: &str) -> bool {
    matches!(
        word.as_bytes().first(),
        Some(b'a' | b'e' | b'i' | b'o' | b'u')
    )
}

/// The indefinite article for `word`: `"an"` when [`takes_an`], else
/// `"a"`. Exarust's convenience over the original's test.
pub fn article(word: &str) -> &'static str {
    if takes_an(word) { "an" } else { "a" }
}

/// Title-cases text the original's way: a lower-case ASCII letter at the
/// start or right after a space or underscore becomes upper case (byte
/// minus 32); every other byte is copied, and any byte but a space or
/// underscore ends the word. A zero byte cuts the result there. Bytes, not
/// UTF-8: the game's text is single-byte.
pub fn title_case(text: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len());
    let mut start = true;
    for &byte in text {
        if start && byte.is_ascii_lowercase() {
            out.push(byte - 0x20);
            start = false;
            continue;
        }
        out.push(byte);
        match byte {
            b' ' | b'_' => start = true,
            0 => {
                out.pop();
                break;
            }
            _ => start = false,
        }
    }
    out
}

/// The byte an ANSI-string-to-name conversion writes over the sixteenth
/// byte when the text is longer than a name: the Latin-1 ellipsis.
pub const NAME_ELLIPSIS: u8 = 0x85;
/// Bytes in an engine name.
pub const NAME_LEN: usize = 16;

/// The engine's fixed sixteen-byte name: zero-padded, zero-terminated when
/// shorter than sixteen, ASCII letters lower-cased on the ways in.
///
/// The original keeps these as bare `array[0..15] of Byte` and its helpers
/// touch all sixteen bytes, so the type keeps the padding and compares it
/// like the original does: a short name is compared with the zero padding
/// still in place.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Name16([u8; NAME_LEN]);

impl std::fmt::Debug for Name16 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Name16")
            .field(&String::from_utf8_lossy(self.text()))
            .finish()
    }
}

impl Name16 {
    /// A name from the sixteen bytes a file stored, unchanged.
    pub const fn from_raw(bytes: [u8; NAME_LEN]) -> Self {
        Self(bytes)
    }

    /// All-zero name.
    pub const fn empty() -> Self {
        Self([0; NAME_LEN])
    }

    /// The length up to the first zero byte, at most sixteen. The original
    /// scans seventeen bytes; a name's sixteenth byte is its own terminator
    /// at the latest, so both agree.
    pub fn len(&self) -> usize {
        self.0
            .iter()
            .position(|&byte| byte == 0)
            .unwrap_or(NAME_LEN)
    }

    /// Whether the name has no text.
    pub fn is_empty(&self) -> bool {
        self.0[0] == 0
    }

    /// The sixteen stored bytes, padding included.
    pub fn as_bytes(&self) -> &[u8; NAME_LEN] {
        &self.0
    }

    /// The text up to the first zero byte.
    pub fn text(&self) -> &[u8] {
        &self.0[..self.len()]
    }

    /// A name from a zero-terminated byte string, ASCII letters lowered and
    /// the rest zero: at most sixteen source bytes are read and the first
    /// zero ends the name.
    pub fn from_cstr(text: &[u8]) -> Self {
        let mut name = [0u8; NAME_LEN];
        for (slot, &byte) in name.iter_mut().zip(text.iter()) {
            if byte == 0 {
                break;
            }
            *slot = lower_ascii(byte);
        }
        Self(name)
    }

    /// A name from an ANSI string, the way the original stores one: a text
    /// of at most sixteen bytes is copied and the tail zeroed; longer text
    /// keeps its first fifteen bytes and the sixteenth becomes
    /// [`NAME_ELLIPSIS`]. The bytes themselves are not lowered.
    pub fn from_ansi(text: &[u8]) -> Self {
        let mut name = [0u8; NAME_LEN];
        if text.len() <= NAME_LEN {
            name[..text.len()].copy_from_slice(text);
        } else {
            name[..NAME_LEN - 1].copy_from_slice(&text[..NAME_LEN - 1]);
            name[NAME_LEN - 1] = NAME_ELLIPSIS;
        }
        Self(name)
    }

    /// Sixteen stored bytes copied with their ASCII letters lowered.
    pub fn lowered(bytes: &[u8; NAME_LEN]) -> Self {
        let mut name = *bytes;
        name.iter_mut().for_each(|byte| *byte = lower_ascii(*byte));
        Self(name)
    }

    /// Lowers the name's ASCII letters in place.
    pub fn lowercase(&mut self) {
        self.0
            .iter_mut()
            .for_each(|byte| *byte = lower_ascii(*byte));
    }

    /// Writes `text` at `offset` and zeroes the rest, unless the text does
    /// not fit before the sixteenth byte: then only the bytes up to the
    /// name's end are copied, and an offset at or past sixteen writes
    /// nothing. The original's offset is a signed 32-bit word; a negative
    /// offset is outside this API.
    pub fn write_at(&mut self, text: &[u8], offset: usize) {
        if offset >= NAME_LEN {
            return;
        }
        let end = offset.saturating_add(text.len());
        if end < NAME_LEN {
            self.0[offset..end].copy_from_slice(text);
            self.0[end..].fill(0);
        } else {
            let copied = NAME_LEN - offset;
            self.0[offset..].copy_from_slice(&text[..copied]);
        }
    }

    /// Appends `text` at the name's length, as the original does: the whole
    /// text when it fits in the remaining bytes (a name may end up full,
    /// with no terminator), else as much as fits. Returns whether the whole
    /// text fitted.
    pub fn append(&mut self, text: &[u8]) -> bool {
        let at = self.len();
        let fitted = at + text.len() <= NAME_LEN;
        let copied = text.len().min(NAME_LEN - at);
        self.0[at..at + copied].copy_from_slice(&text[..copied]);
        fitted
    }

    /// The name with its extension replaced by `extension`'s bytes from the
    /// last dot (or from the name's end when it has none), zero-padded
    /// ([`replace_extension`]).
    pub fn with_extension(&self, extension: &[u8]) -> Self {
        Self(replace_extension(&self.0, extension))
    }

    /// The text as a zero-terminated heap string, the original's name to
    /// `PChar`: empty for an empty name.
    pub fn to_cstr(&self) -> Vec<u8> {
        let mut out = self.text().to_vec();
        if !out.is_empty() {
            out.push(0);
        }
        out
    }

    /// All sixteen bytes compared in order, returning the unsigned
    /// difference of the first pair that differs (0 when equal).
    pub fn compare(&self, other: &Self) -> i32 {
        compare_bytes(&self.0, &other.0)
    }

    /// The names compared as four 32-bit words by wrapping subtraction,
    /// returning the first non-zero difference.
    pub fn compare_words(&self, other: &Self) -> i32 {
        for at in (0..NAME_LEN).step_by(4) {
            let a = i32::from_le_bytes(self.0[at..at + 4].try_into().unwrap());
            let b = i32::from_le_bytes(other.0[at..at + 4].try_into().unwrap());
            let difference = a.wrapping_sub(b);
            if difference != 0 {
                return difference;
            }
        }
        0
    }

    /// Whether the sixteen bytes are equal as two 64-bit words.
    pub fn equals(&self, other: &Self) -> bool {
        self.0[..8] == other.0[..8] && self.0[8..] == other.0[8..]
    }

    /// The names compared over sixteen bytes with ASCII letters raised, so
    /// case does not matter; the difference of the raised bytes.
    pub fn compare_case_insensitive(&self, other: &Self) -> i32 {
        for (a, b) in self.0.iter().zip(other.0.iter()) {
            let a = upper_ascii(*a);
            let b = upper_ascii(*b);
            if a != b {
                return i32::from(a) - i32::from(b);
            }
        }
        0
    }
}

/// Lowers the ASCII letters of a byte string in place; every other byte is
/// left alone. The original's copy-on-write ANSI-string lowering writes
/// the same bytes back.
pub fn lowercase_bytes(text: &mut [u8]) {
    text.iter_mut().for_each(|byte| *byte = lower_ascii(*byte));
}

/// `A`..`Z` moved to lower case; every other byte unchanged.
fn lower_ascii(byte: u8) -> u8 {
    if byte.is_ascii_uppercase() {
        byte + 0x20
    } else {
        byte
    }
}

/// `a`..`z` moved to upper case; every other byte unchanged.
fn upper_ascii(byte: u8) -> u8 {
    if byte.is_ascii_lowercase() {
        byte - 0x20
    } else {
        byte
    }
}

/// Up to the shorter length, the unsigned difference of the first pair of
/// bytes that differs; zero when the common prefix is equal.
pub fn compare_bytes(a: &[u8], b: &[u8]) -> i32 {
    for (x, y) in a.iter().zip(b.iter()) {
        if x != y {
            return i32::from(*x) - i32::from(*y);
        }
    }
    0
}

/// Whether the two byte strings are equal over their common length only
/// (the original's bounded equality): a prefix counts as equal.
pub fn equals_prefix(a: &[u8], b: &[u8]) -> bool {
    a.iter().zip(b.iter()).all(|(x, y)| x == y)
}

/// [`compare_bytes`] with ASCII letters raised on both sides.
pub fn compare_bytes_case_insensitive(a: &[u8], b: &[u8]) -> i32 {
    for (x, y) in a.iter().zip(b.iter()) {
        let x = upper_ascii(*x);
        let y = upper_ascii(*y);
        if x != y {
            return i32::from(x) - i32::from(y);
        }
    }
    0
}

/// [`compare_bytes`] with ASCII letters lowered on the second argument only
/// (the original's mixed compare).
pub fn compare_bytes_lower_second(a: &[u8], b: &[u8]) -> i32 {
    for (x, y) in a.iter().zip(b.iter()) {
        let y = lower_ascii(*y);
        if *x != y {
            return i32::from(*x) - i32::from(y);
        }
    }
    0
}

/// Two ANSI strings compared the original's way: the common length's bytes
/// first, and when they are equal the difference of the two lengths (the
/// unsigned lengths as a signed 64-bit difference).
pub fn compare_ansi(a: &[u8], b: &[u8]) -> i64 {
    let common = a.len().min(b.len());
    let difference = compare_bytes(&a[..common], &b[..common]);
    if difference != 0 {
        i64::from(difference)
    } else {
        a.len() as i64 - b.len() as i64
    }
}

/// [`compare_ansi`] with ASCII letters raised on both sides.
pub fn compare_ansi_case_insensitive(a: &[u8], b: &[u8]) -> i32 {
    let common = a.len().min(b.len());
    compare_bytes_case_insensitive(&a[..common], &b[..common])
}

/// The original's zero-string compare of two byte strings: when their
/// lengths differ it answers the last byte of the longer string; when they
/// are equal but empty, zero; otherwise the bounded compare's first
/// difference. Reproduced as the instructions have it, although the result
/// is not an ordering.
pub fn compare_by_last_byte(a: &[u8], b: &[u8]) -> i32 {
    match a.len().cmp(&b.len()) {
        std::cmp::Ordering::Greater => i32::from(a[a.len() - 1]),
        std::cmp::Ordering::Less => i32::from(b[b.len() - 1]),
        std::cmp::Ordering::Equal if a.is_empty() => 0,
        std::cmp::Ordering::Equal => compare_bytes(a, b),
    }
}

/// The first index holding `byte`, or -1. The original's forward byte
/// search, unrelated to [`find_from`]'s substring search; its length
/// argument is the slice's length here.
pub fn find_byte(byte: u8, text: &[u8]) -> i32 {
    text.iter()
        .position(|&at| at == byte)
        .map_or(-1, |at| at as i32)
}

/// The last index holding `byte`, or -1; an empty text answers 0, not -1,
/// because the original's scan tests its zero flag after no step.
/// ([`last_index`] made public for the name and path helpers.)
pub fn last_byte(byte: u8, text: &[u8]) -> i32 {
    last_index(byte, text)
}

/// The original's splice: `buffer[..from]`, then `insert`, then the rest of
/// the buffer after the `remove` bytes at `from`. The tail is copied only
/// when `from + remove` is below the buffer's length, as the original skips
/// it past the end; the result's length is `insert.len() + buffer.len() -
/// remove`, so the two lengths must leave the removal inside the buffer.
pub fn splice(buffer: &[u8], from: usize, remove: usize, insert: &[u8]) -> Vec<u8> {
    let from = from.min(buffer.len());
    let remove = remove.min(buffer.len() - from);
    let mut out = Vec::with_capacity(insert.len() + buffer.len() - remove);
    out.extend_from_slice(&buffer[..from]);
    out.extend_from_slice(insert);
    out.extend_from_slice(&buffer[from + remove..]);
    out
}

/// The seed the original makes from a performance-counter reading: its low
/// 32 bits, byte-reversed.
pub fn reversed_counter_seed(counter: u64) -> u32 {
    (counter as u32).swap_bytes()
}

/// The bytes a UTF-8 string needs in Latin-1 when only ASCII and the
/// two-byte `C2`/`C3` sequences convert: every ASCII byte counts one, a
/// leading `C2` or `C3` counts one for the pair, and three- and four-byte
/// sequences (and other two-byte leads) count nothing.
pub fn utf8_to_latin1_len(bytes: &[u8]) -> usize {
    let mut count = 0;
    let mut at = 0;
    while at < bytes.len() {
        let lead = bytes[at];
        if lead < 0x80 {
            count += 1;
            at += 1;
        } else if lead < 0xE0 {
            if lead == 0xC2 || lead == 0xC3 {
                count += 1;
            }
            at += 2;
        } else if lead < 0xF0 {
            at += 3;
        } else {
            at += 4;
        }
    }
    count
}

/// Writes the Latin-1 form of a UTF-8 string into `out` and returns how many
/// bytes it wrote: ASCII copied, `C2 xx` copied as `xx`, `C3 xx` as
/// `xx + 0x40`, every other sequence dropped. A sequence cut off by the
/// end reads its missing continuation as zero, which is the string's
/// terminator in the original.
pub fn utf8_to_latin1(bytes: &[u8], out: &mut [u8]) -> usize {
    let mut written = 0;
    let mut at = 0;
    while at < bytes.len() {
        let lead = bytes[at];
        if lead < 0x80 {
            if written < out.len() {
                out[written] = lead;
            }
            written += 1;
            at += 1;
        } else if lead < 0xE0 {
            let second = bytes.get(at + 1).copied().unwrap_or(0);
            if lead == 0xC3 {
                if written < out.len() {
                    out[written] = second.wrapping_add(0x40);
                }
                written += 1;
            } else if lead == 0xC2 {
                if written < out.len() {
                    out[written] = second;
                }
                written += 1;
            }
            at += 2;
        } else if lead < 0xF0 {
            at += 3;
        } else {
            at += 4;
        }
    }
    written
}

/// The bytes a Latin-1 string needs in UTF-8: one for ASCII, two otherwise.
pub fn latin1_to_utf8_len(bytes: &[u8]) -> usize {
    bytes
        .iter()
        .map(|&byte| if byte < 0x80 { 1 } else { 2 })
        .sum()
}

/// Writes the UTF-8 form of a Latin-1 string into `out` and returns how many
/// bytes it wrote: ASCII copied, `80`..`BF` as `C2 xx`, everything else as
/// `C3 (xx − 0x40)`.
pub fn latin1_to_utf8(bytes: &[u8], out: &mut [u8]) -> usize {
    let mut written = 0;
    for &byte in bytes {
        if byte < 0x80 {
            if written < out.len() {
                out[written] = byte;
            }
            written += 1;
        } else {
            let (lead, second) = if byte < 0xC0 {
                (0xC2, byte)
            } else {
                (0xC3, byte.wrapping_sub(0x40))
            };
            if written < out.len() {
                out[written] = lead;
            }
            if written + 1 < out.len() {
                out[written + 1] = second;
            }
            written += 2;
        }
    }
    written
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn popcount_and_log2() {
        assert_eq!(popcount(0), 0);
        assert_eq!(popcount(0xF0F0_F0F0), 16);
        assert_eq!(popcount(u32::MAX), 32);
        assert_eq!(log2_floor(0), 0);
        assert_eq!(log2_floor(1), 0);
        assert_eq!(log2_floor(2), 1);
        assert_eq!(log2_floor(7), 2);
        assert_eq!(log2_floor(8), 3);
        assert_eq!(log2_floor(1000), 9);
        // The compare is signed: the top bit set counts no times.
        assert_eq!(log2_floor(0x8000_0000), 0);
    }

    #[test]
    fn a_forward_search_from_an_offset() {
        // `from` is 1-based, the result 0-based.
        assert_eq!(find_from(b"bc", b"abcdbc", 1), 1);
        assert_eq!(find_from(b"bc", b"abcdbc", 2), 1);
        assert_eq!(find_from(b"bc", b"abcdbc", 3), 4);
        assert_eq!(find_from(b"bc", b"abcdbc", 5), 4);
        assert_eq!(find_from(b"bc", b"abcdbc", 6), -1);
        // From 0 starts one byte early, before the text.
        assert_eq!(find_from(b"bc", b"abcdbc", 0), 1);
        assert_eq!(find_from(b"abc", b"ab", 0), -1);
        assert_eq!(find_from(b"\0a", b"ab", 0), -1);
        assert_eq!(find_from(b"c", b"abc", 3), 2);
        assert_eq!(find_from(b"c", b"abc", 4), -1);
        assert_eq!(find_from(b"", b"abc", 1), -1);
        assert_eq!(find_from(b"a", b"", 1), -1);
        assert_eq!(find_from(b"a", b"abc", -1), -1);
        assert_eq!(find_from(b"abcd", b"abc", 1), -1);
    }

    #[test]
    fn integers_are_built_and_prepended() {
        assert_eq!(format_int(0), "0");
        assert_eq!(format_int(1), "1");
        assert_eq!(format_int(10), "10");
        assert_eq!(format_int(1234), "1234");
        assert_eq!(format_int(-7), "-7");
        assert_eq!(format_int(-1234), "-1234");
        assert_eq!(format_int(i32::MIN), "-");
        assert_eq!(format_int_two(0), "00");
        assert_eq!(format_int_two(1), "01");
        assert_eq!(format_int_two(9), "09");
        assert_eq!(format_int_two(12), "12");
        assert_eq!(format_int_two(123), "123");
        assert_eq!(format_int_two(-7), "0");
        assert_eq!(format_hex(0, 4), "0");
        assert_eq!(format_hex(0x2A, 4), "002A");
        assert_eq!(format_hex(0xBEEF, 4), "BEEF");
        assert_eq!(format_hex(0x12345, 4), "12345");
        assert_eq!(format_hex(-0x2A, 2), "-2A");
        assert_eq!(format_hex(-0x2A, 4), "-002A");
        assert_eq!(format_hex(0x2A, 0), "2A");
        assert_eq!(format_hex(0x2A, -5), "2A");
        assert_eq!(format_hex(0x2A, i32::MIN), "2A");
        assert_eq!(format_hex(i64::MIN, 4), "-0000");
        assert_eq!(format_hex(i64::MIN, 0), "-");
        assert_eq!(format_hex(-1, 1), "-1");
    }

    #[test]
    fn a_float_takes_its_decimals_and_trims() {
        assert_eq!(format_float(1.25, 2, true), "1.25");
        assert_eq!(format_float(1.25, 2, false), "1.25");
        assert_eq!(format_float(1.2, 3, true), "1.200");
        assert_eq!(format_float(1.2, 3, false), "1.2");
        assert_eq!(format_float(1.0, 2, true), "1.00");
        assert_eq!(format_float(1.0, 2, false), "1.0");
        assert_eq!(format_float(0.5, 2, true), "0.50");
        assert_eq!(format_float(0.5, 2, false), "0.5");
        assert_eq!(format_float(-0.5, 2, false), "-0.5");
        assert_eq!(format_float(0.0, 2, false), "0.0");
        assert_eq!(format_float(f32::from_bits(0x8000_0000), 2, false), "-0.0");
        // No decimals: the rounding conversion, ties to even.
        assert_eq!(format_float(2.5, 0, false), "2");
        assert_eq!(format_float(3.5, 0, false), "4");
        assert_eq!(format_float(-2.5, 0, false), "-2");
        // The conversion is 64-bit and its low word is written.
        assert_eq!(format_float(3.0e9, 0, false), "-1294967296");
        // Nines carry into the integer part.
        assert_eq!(format_float(0.999, 2, true), "1.00");
        assert_eq!(format_float(9.9995, 3, true), "10.000");
        assert_eq!(format_float(1.999, 2, false), "2.0");
        assert_eq!(format_float(-0.999, 2, true), "-1.00");
        assert_eq!(format_float(-0.01, 1, true), "-0.0");
        // A NaN or an infinity converts to the indefinite value, whose low
        // word is zero.
        assert_eq!(format_float(f32::NAN, 2, true), "0.00");
        assert_eq!(format_float(f32::INFINITY, 1, true), "0.0");
    }

    #[test]
    fn text_parsers_stop_at_the_first_odd_byte() {
        assert_eq!(int_from_text(b"0"), 0);
        assert_eq!(int_from_text(b"1234"), 1234);
        assert_eq!(int_from_text(b"-42"), -42);
        assert_eq!(int_from_text(b"12abc"), 12);
        assert_eq!(int_from_text(b"-"), 0);
        assert_eq!(int_from_text(b"abc"), 0);
        assert_eq!(int_from_bytes(b"99x", 2), 99);
        assert_eq!(float_from_text(b"0"), 0.0);
        assert_eq!(float_from_text(b"1.5"), 1.5);
        assert_eq!(float_from_text(b"-1.25"), -1.25);
        assert_eq!(float_from_text(b"1."), 1.0);
        assert_eq!(float_from_text(b".5"), 0.5);
        assert_eq!(float_from_text(b"1.5x"), 1.5);
        assert_eq!(float_from_text(b"x"), 0.0);
        assert_eq!(float_from_text(b"-0").to_bits(), 0x8000_0000);
        // The fractional sum is double precision: a single-precision sum
        // would give 0x3E94_7AE2 and 0x3EC2_8F5C.
        assert_eq!(float_from_text(b"0.29").to_bits(), 0x3E94_7AE1);
        assert_eq!(float_from_text(b"0.38").to_bits(), 0x3EC2_8F5D);
        assert_eq!(hex_from_text(b"FF"), 255);
        assert_eq!(hex_from_text(b"ff"), 255);
        assert_eq!(hex_from_text(b"10zz"), 16);
        assert_eq!(hex_from_bytes(b"AB CD", 2), 0xAB);
        assert_eq!(three_digits(b"007"), 7);
        assert_eq!(three_digits(b"999"), 999);
        // No digit check: ':' is '0' plus ten.
        assert_eq!(three_digits(b"1:2"), 202);
    }

    #[test]
    fn hex_colours_pack_pairs() {
        assert_eq!(colour_from_hex6(b"FF8000"), [0xFF, 0x80, 0x00, 0]);
        assert_eq!(colour_from_hex6(b"f0a"), [0xF0, 0, 0, 0]);
        // The six-digit loop reads indices 0 to 5 only.
        assert_eq!(colour_from_hex6(b"11223344"), [0x11, 0x22, 0x33, 0]);
        assert_eq!(colour_from_hex8(b"11223344"), [0x11, 0x22, 0x33, 0x44]);
        // Five digits leave the fifth as a lone digit, dropped.
        assert_eq!(colour_from_hex6(b"FF800"), [0xFF, 0x80, 0, 0]);
    }

    #[test]
    fn names_extensions_and_directories() {
        assert_eq!(
            extension_name(b"level.rfc"),
            Some(*b"rfc\0\0\0\0\0\0\0\0\0\0\0\0\0")
        );
        assert_eq!(extension_name(b"level"), None);
        assert_eq!(
            extension_of_str(".hidden"),
            Some(*b"hidden\0\0\0\0\0\0\0\0\0\0")
        );
        assert_eq!(
            extension_name(b"a.0123456789abcdef"),
            Some(*b"0123456789abcdef")
        );
        assert_eq!(extension_name(b"a.0123456789abcdefg"), None);
        assert_eq!(
            extension_of_str("x.RFI"),
            Some(*b"RFI\0\0\0\0\0\0\0\0\0\0\0\0\0")
        );
        let mut name = [0u8; 16];
        name[..14].copy_from_slice(b"Exanima001.rsg");
        name = replace_extension(&name, b".rcp");
        assert_eq!(&name[..14], b"Exanima001.rcp");
        assert_eq!(&name[14..], &[0, 0]);
        let spaced = *b"noext\0\0\0\0\0\0\0\0\0\0\0";
        assert_eq!(&replace_extension(&spaced, b"txt")[..8], b"noexttxt");
        // Every byte after the dot comes from the zero-padded extension.
        let long = *b"ab.cdefgh\0\0\0\0\0\0\0";
        assert_eq!(
            replace_extension(&long, b".x"),
            *b"ab.x\0\0\0\0\0\0\0\0\0\0\0\0"
        );
        // A full name without a dot is left alone.
        assert_eq!(
            replace_extension(b"0123456789abcdef", b".x"),
            *b"0123456789abcdef"
        );
        // An empty name counts as a dot at position 0.
        assert_eq!(extension_name(b""), Some([0; 16]));
        assert_eq!(extension_of_str(""), Some([0; 16]));
        assert_eq!(strip_extension_append("", ".rcp"), ".rcp");
        assert_eq!(strip_extension_append("save.rsg", ".rcp"), "save.rcp");
        assert_eq!(strip_extension_append("save", ".rcp"), "save.rcp");
        assert_eq!(file_name_of(r"a\b\c.rfc"), "c.rfc");
        assert_eq!(file_name_of("c.rfc"), "c.rfc");
        assert_eq!(directory_of(r"a\b\c.rfc"), r"a\b\");
        assert_eq!(directory_of(r"\c.rfc"), "");
        assert_eq!(directory_of("c.rfc"), "");
    }

    #[test]
    fn articles_and_title_case() {
        assert!(takes_an("apple"));
        assert!(takes_an("ugly"));
        assert!(!takes_an("Boot"));
        assert!(!takes_an(""));
        assert_eq!(article("apple"), "an");
        assert_eq!(article("boot"), "a");
        assert_eq!(title_case(b"hello world_again"), b"Hello World_Again");
        assert_eq!(title_case(b"a  b"), b"A  B");
        assert_eq!(title_case(b"abc\0def"), b"Abc");
        assert_eq!(title_case(b"1st place"), b"1st Place");
        assert_eq!(title_case(b"\xE9t\xE9 ok"), b"\xE9t\xE9 Ok");
        assert!(!takes_an("Apple"));
    }

    #[test]
    fn names_carry_sixteen_padded_bytes() {
        let name = Name16::from_cstr(b"Hello.RFC\0junk");
        assert_eq!(name.text(), b"hello.rfc");
        assert_eq!(name.len(), 9);
        assert_eq!(name.as_bytes()[9..], [0; 7]);
        assert!(!name.is_empty());
        assert_eq!(Name16::empty().len(), 0);
        assert!(Name16::empty().is_empty());
        // Sixteen source bytes all lower-cased, no terminator.
        let full = Name16::from_cstr(b"ABCDEFGHIJKLMNOPQ");
        assert_eq!(full.text(), b"abcdefghijklmnop");
        assert_eq!(full.len(), 16);
        // Non-letters and high bytes are copied.
        let high = Name16::from_cstr(b"a1_\xFF");
        assert_eq!(high.text(), b"a1_\xFF");
    }

    #[test]
    fn ansi_text_becomes_a_name_or_an_ellipsis() {
        assert_eq!(Name16::from_ansi(b"Hello.RFC").text(), b"Hello.RFC");
        // Exactly sixteen bytes are copied whole.
        let sixteen = Name16::from_ansi(b"0123456789abcdef");
        assert_eq!(sixteen.text(), b"0123456789abcdef");
        // Longer text keeps fifteen bytes and the Latin-1 ellipsis.
        let long = Name16::from_ansi(b"0123456789abcdefgh");
        assert_eq!(&long.as_bytes()[..15], b"0123456789abcde");
        assert_eq!(long.as_bytes()[15], NAME_ELLIPSIS);
        assert_eq!(long.len(), 16);
        assert_eq!(Name16::from_ansi(b""), Name16::empty());
    }

    #[test]
    fn lowercasing_touches_every_byte() {
        let raw = *b"ABC\0DEFghijklmno";
        let lowered = Name16::lowered(&raw);
        assert_eq!(lowered.as_bytes(), b"abc\0defghijklmno");
        let mut name = Name16::from_raw(*b"AbCdEfGhIjKlMnOp");
        name.lowercase();
        assert_eq!(name.as_bytes(), b"abcdefghijklmnop");
        // The ANSI-string path lowers in place and leaves high bytes.
        let mut text = *b"AbC\xE9 Z";
        lowercase_bytes(&mut text);
        assert_eq!(text, *b"abc\xE9 z");
    }

    #[test]
    fn writing_at_an_offset_truncates_or_zero_fills() {
        let mut name = Name16::from_cstr(b"abc");
        name.write_at(b"de", 1);
        assert_eq!(name.text(), b"ade");
        assert_eq!(&name.as_bytes()[3..], [0; 13]);
        // Too long: only the bytes before the end are copied, no padding.
        let mut full = Name16::from_cstr(b"abcdefgh");
        full.write_at(b"0123456789abcdef", 8);
        assert_eq!(full.as_bytes(), b"abcdefgh01234567");
        // A later, shorter write zeroes the tail after it.
        let mut tail = Name16::from_cstr(b"abcdefgh");
        tail.write_at(b"x", 2);
        assert_eq!(tail.text(), b"abx");
        // At or past sixteen nothing is written.
        let mut untouched = Name16::from_cstr(b"abc");
        untouched.write_at(b"x", 16);
        assert_eq!(untouched.text(), b"abc");
        untouched.write_at(b"x", 40);
        assert_eq!(untouched.text(), b"abc");
    }

    #[test]
    fn appending_stops_at_the_sixteenth_byte() {
        let mut name = Name16::empty();
        assert!(name.append(b"abc"));
        assert_eq!(name.text(), b"abc");
        // 3 + 13 = 16 exactly: the whole text fits, with no terminator.
        assert!(name.append(b"0123456789abc"));
        assert_eq!(name.text(), b"abc0123456789abc");
        // A full name takes nothing.
        let mut full = Name16::from_cstr(b"0123456789abcdef");
        assert!(!full.append(b"x"));
        assert_eq!(full.text(), b"0123456789abcdef");
        // 15 + 2 does not fit; one byte is copied.
        let mut almost = Name16::from_cstr(b"0123456789abcde");
        assert!(!almost.append(b"XY"));
        assert_eq!(almost.text(), b"0123456789abcdeX");
        assert_eq!(almost.len(), 16);
    }

    #[test]
    fn names_replace_their_extension_and_export_as_c_strings() {
        let name = Name16::from_cstr(b"save.rsg");
        assert_eq!(name.with_extension(b".rcp").text(), b"save.rcp");
        assert_eq!(name.with_extension(b"x").text(), b"savex");
        assert_eq!(
            Name16::from_cstr(b"none").with_extension(b".x").text(),
            b"none.x"
        );
        assert_eq!(name.to_cstr(), b"save.rsg\0");
        assert!(Name16::empty().to_cstr().is_empty());
    }

    #[test]
    fn name_compares_follow_the_stored_bytes() {
        let abc = Name16::from_raw(*b"abc\0\0\0\0\0\0\0\0\0\0\0\0\0");
        let abd = Name16::from_raw(*b"abd\0\0\0\0\0\0\0\0\0\0\0\0\0");
        assert_eq!(abc.compare(&abd), -1);
        assert_eq!(abd.compare(&abc), 1);
        assert_eq!(abc.compare(&abc), 0);
        // The padding counts: "ab" is below "abc" because its third byte is
        // zero.
        let ab = Name16::from_raw(*b"ab\0\0\0\0\0\0\0\0\0\0\0\0\0\0");
        assert_eq!(ab.compare(&abc), -99);
        // Word-wise order reports the first word's difference.
        let mut one = [0u8; 16];
        let mut two = [0u8; 16];
        for (at, (a, b)) in [1i32, 2, 3, 4].into_iter().zip([1i32, 2, 7, 4]).enumerate() {
            one[at * 4..at * 4 + 4].copy_from_slice(&a.to_le_bytes());
            two[at * 4..at * 4 + 4].copy_from_slice(&b.to_le_bytes());
        }
        assert_eq!(
            Name16::from_raw(one).compare_words(&Name16::from_raw(two)),
            -4
        );
        assert_eq!(
            Name16::from_raw(one).compare_words(&Name16::from_raw(one)),
            0
        );
        assert!(Name16::from_raw(one).equals(&Name16::from_raw(one)));
        let mut other = one;
        other[9] = 1;
        assert!(!Name16::from_raw(one).equals(&Name16::from_raw(other)));
        // Case-insensitive compare raises both sides.
        assert_eq!(
            Name16::from_raw(*b"AbC\0\0\0\0\0\0\0\0\0\0\0\0\0")
                .compare_case_insensitive(&Name16::from_raw(*b"aBc\0\0\0\0\0\0\0\0\0\0\0\0\0")),
            0
        );
        assert_eq!(
            Name16::from_raw(*b"ABC\0\0\0\0\0\0\0\0\0\0\0\0\0")
                .compare_case_insensitive(&Name16::from_raw(*b"ABD\0\0\0\0\0\0\0\0\0\0\0\0\0")),
            -1
        );
    }

    #[test]
    fn byte_compares_take_the_common_length() {
        assert_eq!(compare_bytes(b"\x01\x02\x03", b"\x01\x09"), -7);
        assert_eq!(compare_bytes(b"abc", b"abc"), 0);
        assert_eq!(compare_bytes(b"", b"a"), 0);
        assert!(equals_prefix(b"abc", b"ab"));
        assert!(equals_prefix(b"", b"abc"));
        assert!(!equals_prefix(b"abc", b"abd"));
        assert_eq!(compare_bytes_case_insensitive(b"aX", b"Ay"), -1);
        assert_eq!(compare_bytes_case_insensitive(b"aX", b"Ax"), 0);
        assert_eq!(compare_bytes_lower_second(b"B", b"a"), -31);
        assert_eq!(compare_bytes_lower_second(b"a", b"a"), 0);
        assert_eq!(compare_bytes_lower_second(b"a", b"A"), 0);
        assert_eq!(compare_ansi(b"abc", b"abd"), -1);
        assert_eq!(compare_ansi(b"abc", b"ab"), 1);
        assert_eq!(compare_ansi(b"ab", b"abc"), -1);
        assert_eq!(compare_ansi(b"abc", b"abc"), 0);
        assert_eq!(compare_ansi_case_insensitive(b"AbC", b"aBc"), 0);
        assert_eq!(compare_ansi_case_insensitive(b"abc", b"abd"), -1);
    }

    #[test]
    fn the_zero_string_compare_answers_the_longer_last_byte() {
        assert_eq!(compare_by_last_byte(b"abc", b"de"), i32::from(b'c'));
        assert_eq!(compare_by_last_byte(b"ab", b"xyz"), i32::from(b'z'));
        assert_eq!(compare_by_last_byte(b"abc", b"abd"), -1);
        assert_eq!(compare_by_last_byte(b"", b""), 0);
    }

    #[test]
    fn byte_search_and_splice() {
        assert_eq!(find_byte(b'x', b"abxc"), 2);
        assert_eq!(find_byte(b'x', b"abc"), -1);
        assert_eq!(last_byte(b'x', b"abxc"), 2);
        assert_eq!(last_byte(b'x', b"abc"), -1);
        assert_eq!(last_byte(b'x', b""), 0);
        assert_eq!(splice(b"abcdef", 2, 2, b"XY"), b"abXYef");
        assert_eq!(splice(b"abcdef", 0, 6, b""), b"");
        assert_eq!(splice(b"abcdef", 5, 1, b"Z"), b"abcdeZ");
        assert_eq!(splice(b"abcdef", 0, 0, b"1"), b"1abcdef");
    }

    #[test]
    fn the_counter_seed_is_byte_reversed() {
        assert_eq!(reversed_counter_seed(0xAABB_CCDD), 0xDDCC_BBAA);
        assert_eq!(reversed_counter_seed(0x0000_0001), 0x0100_0000);
        assert_eq!(reversed_counter_seed(0x1_2345_6789), 0x8967_4523);
    }

    #[test]
    fn utf8_latin1_conversions_keep_ascii_and_c2_c3() {
        assert_eq!(utf8_to_latin1_len(b"abc"), 3);
        assert_eq!(utf8_to_latin1_len(b"caf\xC3\xA9"), 4);
        assert_eq!(utf8_to_latin1_len(b"\xC2\xA9"), 1);
        // A three-byte and a four-byte sequence, and a two-byte lead that
        // is not C2/C3, convert to nothing.
        assert_eq!(utf8_to_latin1_len(b"\xE2\x82\xAC"), 0);
        assert_eq!(utf8_to_latin1_len(b"\xF0\x9F\x98\x80"), 0);
        assert_eq!(utf8_to_latin1_len(b"\xC4\x80"), 0);

        let mut out = [0u8; 8];
        assert_eq!(utf8_to_latin1(b"caf\xC3\xA9", &mut out), 4);
        assert_eq!(&out[..4], b"caf\xE9");
        assert_eq!(utf8_to_latin1(b"\xC3\xA9", &mut out), 1);
        assert_eq!(out[0], 0xE9);
        assert_eq!(utf8_to_latin1(b"\xC2\xA9", &mut out), 1);
        assert_eq!(out[0], 0xA9);
        assert_eq!(utf8_to_latin1(b"a\xE2\x82\xACb", &mut out), 2);
        assert_eq!(&out[..2], b"ab");
        // A lead cut off by the end reads the terminator.
        assert_eq!(utf8_to_latin1(b"\xC3", &mut out), 1);
        assert_eq!(out[0], 0x40);

        // "caf" is three bytes and the Latin-1 é needs two.
        assert_eq!(latin1_to_utf8_len(b"caf\xE9"), 5);
        assert_eq!(latin1_to_utf8(b"a\xE9\xA9", &mut out), 5);
        assert_eq!(&out[..5], b"a\xC3\xA9\xC2\xA9");
        assert_eq!(latin1_to_utf8_len(b"\x7F"), 1);
        assert_eq!(latin1_to_utf8(b"\xFF", &mut out), 2);
        assert_eq!(&out[..2], b"\xC3\xBF");

        // Round trip.
        let latin1 = b"caf\xE9 \xA9";
        let mut utf8 = [0u8; 16];
        let n = latin1_to_utf8(latin1, &mut utf8);
        let mut back = [0u8; 16];
        assert_eq!(utf8_to_latin1(&utf8[..n], &mut back), latin1.len());
        assert_eq!(&back[..latin1.len()], latin1);
    }
}
