//! The engine's text and number helpers: decimal, hexadecimal and float
//! formatting, parsers for the same, the 16-byte asset-name helpers, hex
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

/// An integer as two decimal digits (the original's clock-style formatter):
/// zero is `"00"`, a one-digit value takes a leading zero, and any value
/// that produces no digits (a negative one) is `"00"`.
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
    // The length is compared as a 32-bit word with `width - 1`.
    let len = digits.len() as i32;
    let pad = if width != i32::MIN && len <= width - 1 {
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

/// An integer from decimal text: an optional leading minus, then digits up
/// to the first non-digit. A string with no digits is 0.
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

/// An integer from at most `len` bytes of decimal text ([`int_from_text`]).
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
/// byte that is not a hex digit.
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

/// An integer from at most `len` bytes of hexadecimal text.
pub fn hex_from_bytes(text: &[u8], len: usize) -> i32 {
    hex_from_text(text.get(..len).unwrap_or(text))
}

/// A colour from six hexadecimal digits: three bytes, pairs from the most
/// significant. Fewer than six digits leave the higher bytes zero; a digit
/// count that does not pair up leaves the last partial byte zero. The
/// fourth byte (alpha) is left zero.
pub fn colour_from_hex6(text: &[u8]) -> [u8; 4] {
    colour_from_hex(text, 5)
}

/// A colour from eight hexadecimal digits: four bytes. The original reads
/// at most seven digits' worth of pairs, so an eighth digit is ignored.
pub fn colour_from_hex8(text: &[u8]) -> [u8; 4] {
    colour_from_hex(text, 7)
}

/// The original's hex-colour loop: at most `last` digit indices, a byte is
/// written after every second digit.
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
/// check: `(b0 − '0')·100 + (b1 − '0')·10 + b2 − '0'`, wrapping.
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
/// (lower-case a, e, i, o or u) and so takes "an".
pub fn takes_an(word: &str) -> bool {
    matches!(
        word.as_bytes().first(),
        Some(b'a' | b'e' | b'i' | b'o' | b'u')
    )
}

/// The indefinite article for `word`: `"an"` before a vowel the original
/// counts, else `"a"`.
pub fn article(word: &str) -> &'static str {
    if takes_an(word) { "an" } else { "a" }
}

/// Title-cases ASCII text the original's way: every lower-case letter at
/// the start or after a space or underscore becomes upper case; every other
/// byte is kept. A zero byte ends the text there.
pub fn title_case(text: &[u8]) -> String {
    let mut out = Vec::with_capacity(text.len());
    let mut start = true;
    for &byte in text {
        if byte == 0 {
            break;
        }
        if start && byte.is_ascii_lowercase() {
            out.push(byte.to_ascii_uppercase());
            start = false;
        } else {
            out.push(byte);
            start = matches!(byte, b' ' | b'_');
        }
    }
    String::from_utf8_lossy(&out).into_owned()
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
        // The original's loop stops after six digit indices.
        assert_eq!(colour_from_hex6(b"11223344"), [0x11, 0x22, 0x33, 0]);
        assert_eq!(colour_from_hex8(b"11223344"), [0x11, 0x22, 0x33, 0x44]);
        // Seven digits leave the third pair as the lone digit.
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
        assert_eq!(title_case(b"hello world_again"), "Hello World_Again");
        assert_eq!(title_case(b"a  b"), "A  B");
        assert_eq!(title_case(b"abc\0def"), "Abc");
        assert_eq!(title_case(b"1st place"), "1st Place");
    }
}
