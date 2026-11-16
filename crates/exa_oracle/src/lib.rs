//! The function oracle's Rust side: recorded cases of the original's
//! functions replayed against the Rust ports, bit for bit.
//!
//! A case file holds the raw input bytes of many calls of one original
//! function and the bytes it produced (its out buffers and its return
//! value), recorded by running the original machine code in an emulator
//! under the engine's floating-point mode. The recorder and the map from a
//! function's name to its place in the original live outside the
//! repository; this crate only knows the names. [`ports`] maps each name to
//! a closure that decodes a case's input bytes, calls the Rust port and
//! encodes what it returns in the same layout. [`replay`] runs every case
//! of a file under the engine's floating-point mode
//! ([`exa_physics::float_mode::enter`]) and reports the first case whose
//! bytes differ, field by field.
//!
//! Case file layout (little-endian): the magic `EXAORC01`, a `u32` length
//! and that many bytes of UTF-8 metadata (`key=value` lines, then one
//! `in <kind> <field>` or `out <kind> <field>` line per field in byte
//! order), the input length, the output length and the case count (`u32`
//! each), then each case's input bytes followed by its output bytes.

mod ports;

pub use ports::{Case, Port, ports};

use std::fmt::Write as _;
use std::path::Path;

/// The magic at the start of a case file.
pub const MAGIC: &[u8; 8] = b"EXAORC01";

/// One field of a case's input or output: its kind (`f32`, `i32`, `u8`,
/// `pad4`, `ref:…`) and its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub kind: String,
    pub name: String,
}

impl Field {
    /// The field's size in bytes.
    pub fn size(&self) -> usize {
        let k = self.kind.as_str();
        if k.starts_with("ref") {
            return 8;
        }
        if let Some(n) = k.strip_prefix("pad") {
            return match n.strip_prefix("0x") {
                Some(hex) => usize::from_str_radix(hex, 16).unwrap_or(0),
                None => n.parse().unwrap_or(0),
            };
        }
        match k {
            "f64" | "i64" | "u64" => 8,
            "f32" | "i32" | "u32" => 4,
            "u16" | "i16" => 2,
            _ => 1,
        }
    }
}

/// A parsed case file.
#[derive(Debug, Clone)]
pub struct CaseFile {
    /// The function's registry name.
    pub name: String,
    /// The metadata's `key=value` pairs.
    pub meta: Vec<(String, String)>,
    pub inputs: Vec<Field>,
    pub outputs: Vec<Field>,
    pub in_len: usize,
    pub out_len: usize,
    /// Each case's input and output bytes.
    pub cases: Vec<(Vec<u8>, Vec<u8>)>,
}

impl CaseFile {
    /// Parses a case file's bytes.
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let mut at = 0usize;
        let mut take = |n: usize| -> Result<&[u8], String> {
            let slice = bytes
                .get(at..at + n)
                .ok_or_else(|| format!("truncated at byte {at}"))?;
            at += n;
            Ok(slice)
        };
        if take(8)? != MAGIC {
            return Err("not a case file".into());
        }
        let u32_of = |b: &[u8]| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize;
        let meta_len = u32_of(take(4)?);
        let text = std::str::from_utf8(take(meta_len)?).map_err(|e| e.to_string())?;
        let mut meta = Vec::new();
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        for line in text.lines() {
            let mut words = line.split_whitespace();
            match words.next() {
                Some(dir @ ("in" | "out")) => {
                    let kind = words.next().unwrap_or_default().to_string();
                    let name = words.next().unwrap_or_default().to_string();
                    let field = Field { kind, name };
                    if dir == "in" {
                        inputs.push(field);
                    } else {
                        outputs.push(field);
                    }
                }
                _ => {
                    if let Some((k, v)) = line.split_once('=') {
                        meta.push((k.to_string(), v.to_string()));
                    }
                }
            }
        }
        let in_len = u32_of(take(4)?);
        let out_len = u32_of(take(4)?);
        let count = u32_of(take(4)?);
        let mut cases = Vec::with_capacity(count);
        for _ in 0..count {
            let input = take(in_len)?.to_vec();
            let output = take(out_len)?.to_vec();
            cases.push((input, output));
        }
        let name = meta
            .iter()
            .find(|(k, _)| k == "name")
            .map(|(_, v)| v.clone())
            .ok_or("no name in the metadata")?;
        Ok(Self {
            name,
            meta,
            inputs,
            outputs,
            in_len,
            out_len,
            cases,
        })
    }

    /// Reads and parses a case file.
    pub fn read(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::parse(&bytes)
    }
}

/// What a replay of one case file found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Every case gave the original's bytes, but for `nan_payload` cases
    /// where some output was a NaN on both sides with other sign or payload
    /// bits (the processor answers the first NaN operand, and the compiler
    /// may swap a commutative operation's operands).
    Identical { cases: usize, nan_payload: usize },
    /// The first case whose bytes differ, described field by field.
    Mismatch {
        case: usize,
        cases: usize,
        differing: usize,
        detail: String,
    },
    /// No port is registered under the file's name.
    NoPort,
    /// The port returned a different number of bytes than the original.
    BadLength { expected: usize, got: usize },
}

/// Replays every case of `file` against its registered port under the
/// engine's floating-point mode.
pub fn replay(file: &CaseFile) -> Outcome {
    let Some(port) = ports().iter().find(|p| p.name == file.name) else {
        return Outcome::NoPort;
    };
    exa_physics::float_mode::enter();
    let mut first = None;
    let mut differing = 0;
    let mut nan_payload = 0;
    for (i, (input, expected)) in file.cases.iter().enumerate() {
        let case = Case {
            bytes: input,
            fields: &file.inputs,
        };
        let run = std::panic::catch_unwind(|| (port.run)(&case));
        let Ok(got) = run else {
            differing += 1;
            if first.is_none() {
                let mut detail = describe(file, input, expected, expected);
                detail.push_str("\n  the port panicked");
                first = Some((i, detail));
            }
            continue;
        };
        if got.len() != expected.len() {
            return Outcome::BadLength {
                expected: expected.len(),
                got: got.len(),
            };
        }
        if &got != expected {
            if only_nan_payload(file, expected, &got) {
                nan_payload += 1;
                continue;
            }
            differing += 1;
            if first.is_none() {
                first = Some((i, describe(file, input, expected, &got)));
            }
        }
    }
    match first {
        None => Outcome::Identical {
            cases: file.cases.len(),
            nan_payload,
        },
        Some((case, detail)) => Outcome::Mismatch {
            case,
            cases: file.cases.len(),
            differing,
            detail,
        },
    }
}

/// Whether every differing output field is a float that is NaN on both
/// sides.
fn only_nan_payload(file: &CaseFile, expected: &[u8], got: &[u8]) -> bool {
    let mut at = 0;
    for field in &file.outputs {
        let n = field.size();
        let (e, g) = (&expected[at..at + n], &got[at..at + n]);
        at += n;
        if e == g {
            continue;
        }
        let both_nan = match field.kind.as_str() {
            "f32" => {
                let bits = |b: &[u8]| f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
                bits(e).is_nan() && bits(g).is_nan()
            }
            "f64" => {
                let bits = |b: &[u8]| f64::from_le_bytes(b.try_into().unwrap_or([0; 8]));
                bits(e).is_nan() && bits(g).is_nan()
            }
            _ => false,
        };
        if !both_nan {
            return false;
        }
    }
    true
}

/// The case's inputs and every output field that differs, with its bits.
pub fn describe(file: &CaseFile, input: &[u8], expected: &[u8], got: &[u8]) -> String {
    let mut out = String::from("inputs:");
    let mut at = 0;
    for field in &file.inputs {
        let n = field.size();
        if !(field.kind.starts_with("pad") || field.kind.starts_with("ref")) {
            let _ = write!(
                out,
                " {}={}",
                field.name,
                show(&field.kind, &input[at..at + n])
            );
        }
        at += n;
    }
    out.push_str("\ndiffering outputs:");
    let mut at = 0;
    for field in &file.outputs {
        let n = field.size();
        let (e, g) = (&expected[at..at + n], &got[at..at + n]);
        if e != g {
            let _ = write!(
                out,
                "\n  {}: original {} port {}",
                field.name,
                show(&field.kind, e),
                show(&field.kind, g)
            );
        }
        at += n;
    }
    out
}

/// A field's bytes as `value (0xbits)`.
fn show(kind: &str, b: &[u8]) -> String {
    match kind {
        "f32" => {
            let bits = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
            format!("{:?} ({bits:#010x})", f32::from_bits(bits))
        }
        "f64" => {
            let bits = u64::from_le_bytes(b.try_into().unwrap_or([0; 8]));
            format!("{:?} ({bits:#018x})", f64::from_bits(bits))
        }
        "i32" => format!("{}", i32::from_le_bytes([b[0], b[1], b[2], b[3]])),
        "u32" => format!("{:#x}", u32::from_le_bytes([b[0], b[1], b[2], b[3]])),
        "u8" => format!("{}", b[0]),
        "i64" => format!("{}", i64::from_le_bytes(b.try_into().unwrap_or([0; 8]))),
        _ => format!("{b:02x?}"),
    }
}

/// Replays every `*.cases` file under `dir`, in name order: the file's
/// function name and what its replay found.
pub fn replay_dir(dir: &Path) -> Result<Vec<(String, Outcome)>, String> {
    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "cases"))
        .collect();
    paths.sort();
    let mut out = Vec::new();
    for path in paths {
        let file = CaseFile::read(&path)?;
        let outcome = replay(&file);
        out.push((file.name, outcome));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str, cases: &[(Vec<u8>, Vec<u8>)]) -> Vec<u8> {
        let meta = format!("name={name}\nin f32 x\nout f32 ret\n");
        let mut b = MAGIC.to_vec();
        b.extend((meta.len() as u32).to_le_bytes());
        b.extend(meta.as_bytes());
        b.extend(4u32.to_le_bytes());
        b.extend(4u32.to_le_bytes());
        b.extend((cases.len() as u32).to_le_bytes());
        for (i, o) in cases {
            b.extend(i);
            b.extend(o);
        }
        b
    }

    #[test]
    fn a_case_file_parses_into_its_fields_and_cases() {
        let x = 2.0f32.to_le_bytes().to_vec();
        let parsed = CaseFile::parse(&file("sign", &[(x.clone(), 1.0f32.to_le_bytes().to_vec())]))
            .expect("parses");
        assert_eq!(parsed.name, "sign");
        assert_eq!(parsed.inputs.len(), 1);
        assert_eq!(parsed.outputs[0].name, "ret");
        assert_eq!(parsed.cases.len(), 1);
        assert_eq!(parsed.cases[0].0, x);
    }

    #[test]
    fn a_replay_reports_the_first_differing_case_with_its_bits() {
        let good = (2.0f32.to_le_bytes().to_vec(), 1.0f32.to_le_bytes().to_vec());
        let bad = (
            (-3.0f32).to_le_bytes().to_vec(),
            1.0f32.to_le_bytes().to_vec(),
        );
        let parsed = CaseFile::parse(&file("sign", &[good.clone(), bad])).expect("parses");
        match replay(&parsed) {
            Outcome::Mismatch {
                case,
                differing,
                detail,
                ..
            } => {
                assert_eq!((case, differing), (1, 1));
                assert!(detail.contains("0xbf800000"), "{detail}");
            }
            other => panic!("{other:?}"),
        }
        let parsed = CaseFile::parse(&file("sign", &[good])).expect("parses");
        assert_eq!(
            replay(&parsed),
            Outcome::Identical {
                cases: 1,
                nan_payload: 0
            }
        );
    }

    #[test]
    fn every_port_name_is_registered_once() {
        let mut names: Vec<_> = ports().iter().map(|p| p.name).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len());
    }
}
