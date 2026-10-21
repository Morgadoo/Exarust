//! The original's shader-template language: the `{?…?}`, `{%…%}` and
//! `{$…}` markers that turn a `.flb` entry into GLSL.
//!
//! The engine's builder walks a template once per variant and writes a plain
//! string: conditionals on named flags and integer variables, unrolled
//! loops, integer expressions, library includes, and the generated
//! `{$Interface}` block. The rules here mirror that walker, including its
//! whitespace handling (the newline after a marker is dropped), its
//! left-to-right expression arithmetic, and its 0/1 truth values. The flag
//! words and the program choice live in [`crate::shader_program`]; the parsed
//! library is [`exa_formats::flb::ShaderLibrary`].
//!
//! ```text
//! {?Mat.TexDif?} … {?ELSE Shd.AlphaTst?} … {?END?}
//! {?Lights > 0 & Light[0].Direct?}
//! {%LOOP Lights i%} Light[{%i*3%}] {%END%}
//! {$lights}   {$Interface}
//! ```

use std::collections::HashMap;

/// One word of the state a template sees: the material, shader, object and
/// environment flags.
///
/// The original keeps four words:
/// - **material**: the record's shading and texture-slot bits;
/// - **shader**: the program class in the low nibble plus options such as
///   `DispSdw` and `FastLgt`;
/// - **object**: the geometry's streams (`VrtCol`, `VrtDlt`) and the
///   supersampling/`PassIdx` mode;
/// - **environment**: GI, fog mode, weather, outdoor, sharpen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Flags {
    pub material: u32,
    pub shader: u32,
    pub object: u32,
    pub environment: u32,
}

/// The integer variables of a template, the values of `Lights`,
/// `SdwSmps`, `ShadeQual` and `DspSdws`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Numbers {
    /// Lights on this object, 0 through 6.
    pub lights: u32,
    /// Shadow taps this sub-frame takes (`SdwSmps`), 1 through 32.
    pub shadow_samples: u32,
    /// The quality lift used by the GI and parallax conditions, 0 through 3.
    pub shade_quality: u32,
    /// Height-map displacement taps, 0 or 4.
    pub displacement_samples: u32,
}

/// What `Light[i].Direct` and `Light[i].Sky` answer for one of an object's
/// lights. `Direct` is the sun's directional record; `Sky` a light without
/// a position (the engine's sky light).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LightKind {
    #[default]
    Point,
    Spot,
    Directional,
    Sky,
}

impl LightKind {
    fn is_direct(self) -> bool {
        self == Self::Directional
    }

    fn is_sky(self) -> bool {
        self == Self::Sky
    }
}

/// Where in an interface `{$Interface}` expands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InterfaceMode {
    /// No generated interface (the library's own text only).
    #[default]
    None,
    /// The vertex stage: inputs, varyings, the `Global` block.
    Vertex,
    /// The fragment stage: varyings, samplers, the `Global` block, shadow
    /// and `SdwSmp` declarations.
    Fragment,
}

/// A parsed library the expander can include from.
pub trait Includes {
    /// The source of the entry named `name`, or `None`.
    fn source(&self, name: &str) -> Option<&str>;
}

impl Includes for exa_formats::flb::ShaderLibrary {
    fn source(&self, name: &str) -> Option<&str> {
        self.get(name)
    }
}

impl Includes for HashMap<String, String> {
    fn source(&self, name: &str) -> Option<&str> {
        self.get(name).map(String::as_str)
    }
}

/// The state one template expansion runs with: flags, numbers, light kinds
/// and the interface mode.
#[derive(Clone, Debug, Default)]
pub struct TemplateContext {
    pub flags: Flags,
    pub numbers: Numbers,
    /// The kind of each light in the object's list, indexed by `Light[i]`.
    /// Missing entries count as a point light.
    pub lights: Vec<LightKind>,
    pub interface: InterfaceMode,
    /// `{$…}` includes the library did not have, in the order met. The
    /// engine silently drops them; this list exists for diagnostics.
    pub unknown_includes: Vec<String>,
    loops: Vec<LoopFrame>,
}

#[derive(Clone, Debug)]
struct LoopFrame {
    /// Source index of the loop body, after the `{%LOOP …%}` marker.
    body: usize,
    /// Number of values the loop runs through.
    count: i64,
    /// The current value (starts at the loop's initial digit).
    current: i64,
    /// The single-character loop variable, or 0.
    variable: u8,
}

/// A flag table entry: a name, the bit field it reads and the shift that
/// brings the field down to a small integer.
#[derive(Debug)]
struct FlagDef {
    name: &'static str,
    mask: u32,
    shift: u32,
}

/// The `Mat.` keywords, in the order the builder registers them.
const MATERIAL_FLAGS: &[FlagDef] = &[
    FlagDef {
        name: "MatDif",
        mask: 0x2,
        shift: 0,
    },
    FlagDef {
        name: "MatSpc",
        mask: 0x4,
        shift: 0,
    },
    FlagDef {
        name: "MatEmi",
        mask: 0x8,
        shift: 0,
    },
    FlagDef {
        name: "TexDif",
        mask: 0x20000,
        shift: 0,
    },
    FlagDef {
        name: "TexSpc",
        mask: 0x40000,
        shift: 0,
    },
    FlagDef {
        name: "TexNrm",
        mask: 0x200000,
        shift: 0,
    },
    FlagDef {
        name: "TexEmi",
        mask: 0x80000,
        shift: 0,
    },
    FlagDef {
        name: "Textures",
        mask: 0xFFF0000,
        shift: 0,
    },
    FlagDef {
        name: "TexOpc",
        mask: 0x400000,
        shift: 0,
    },
    FlagDef {
        name: "MatAmbDif",
        mask: 0x3,
        shift: 0,
    },
    FlagDef {
        name: "TexGls",
        mask: 0x1000000,
        shift: 0,
    },
    FlagDef {
        name: "TexRef",
        mask: 0x100000,
        shift: 0,
    },
    FlagDef {
        name: "TexDsp",
        mask: 0x800000,
        shift: 0,
    },
    FlagDef {
        name: "OpctFall",
        mask: 0x800,
        shift: 0,
    },
    FlagDef {
        name: "Refract",
        mask: 0x200,
        shift: 0,
    },
    FlagDef {
        name: "Twosided",
        mask: 0x10,
        shift: 0,
    },
];

/// The `Shd.` keywords.
const SHADER_FLAGS: &[FlagDef] = &[
    FlagDef {
        name: "UseNrm",
        mask: 0x10000,
        shift: 0,
    },
    FlagDef {
        name: "QuadRender",
        mask: 0x1000,
        shift: 0,
    },
    FlagDef {
        name: "LocalRef",
        mask: 0x400000,
        shift: 0,
    },
    FlagDef {
        name: "DirtMode",
        mask: 0x70,
        shift: 4,
    },
    FlagDef {
        name: "BlendMode",
        mask: 0x7000,
        shift: 12,
    },
    FlagDef {
        name: "AlphaTst",
        mask: 0x8000,
        shift: 0,
    },
    FlagDef {
        name: "NegNdL",
        mask: 0x800,
        shift: 0,
    },
    FlagDef {
        name: "TngtNrm",
        mask: 0x20000,
        shift: 0,
    },
    FlagDef {
        name: "DispSdw",
        mask: 0x100,
        shift: 0,
    },
    FlagDef {
        name: "FastLgt",
        mask: 0x200,
        shift: 0,
    },
];

/// The `Obj.` keywords.
const OBJECT_FLAGS: &[FlagDef] = &[
    FlagDef {
        name: "VrtDlt",
        mask: 0x100000,
        shift: 0,
    },
    FlagDef {
        name: "VrtCol",
        mask: 0x200000,
        shift: 0,
    },
    FlagDef {
        name: "QuadRender",
        mask: 0x1000,
        shift: 0,
    },
];

/// The `Env.` keywords.
const ENVIRONMENT_FLAGS: &[FlagDef] = &[
    FlagDef {
        name: "VxlGI",
        mask: 0x10,
        shift: 0,
    },
    FlagDef {
        name: "FogMode",
        mask: 0x3,
        shift: 0,
    },
    FlagDef {
        name: "WetFloor",
        mask: 0x20,
        shift: 0,
    },
    FlagDef {
        name: "ToneMap",
        mask: 0x4,
        shift: 0,
    },
    FlagDef {
        name: "Outdoor",
        mask: 0x40,
        shift: 0,
    },
    FlagDef {
        name: "VXGISpec",
        mask: 0x80,
        shift: 0,
    },
    FlagDef {
        name: "Sharpen",
        mask: 0x8,
        shift: 0,
    },
];

/// Why a program could not be produced.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TemplateError {
    #[error("the shader library has no entry {0:?}")]
    MissingEntry(String),
}

/// A complete original program: the vertex and fragment sources.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgramText {
    pub vertex: String,
    pub fragment: String,
}

impl TemplateContext {
    pub fn new(flags: Flags, numbers: Numbers) -> Self {
        Self {
            flags,
            numbers,
            ..Self::default()
        }
    }

    /// The value of the integer variable named `name`, as the builder's
    /// table answers it. Unknown names are `None`.
    fn number(&self, name: &[u8]) -> Option<i64> {
        let value = match name {
            b"Lights" => self.numbers.lights,
            b"SdwSmps" => self.numbers.shadow_samples,
            b"ShadeQual" => self.numbers.shade_quality,
            b"DspSdws" => self.numbers.displacement_samples,
            _ => return None,
        };
        Some(i64::from(value))
    }

    /// A loop variable's current value, when the name is exactly one
    /// character (the builder searches the loop stack only then).
    fn loop_value(&self, variable: u8) -> Option<i64> {
        self.loops
            .iter()
            .rev()
            .find(|f| f.variable == variable)
            .map(|f| f.current)
    }

    fn kind_of(&self, index: usize) -> LightKind {
        self.lights.get(index).copied().unwrap_or_default()
    }
}

/// Expand one library entry into GLSL.
///
/// `#version 150` is prepended when the entry does not start with `#`,
/// exactly as the builder does.
pub fn expand(
    library: &impl Includes,
    entry: &str,
    context: &TemplateContext,
) -> Result<String, TemplateError> {
    let mut context = context.clone();
    expand_with(library, entry, &mut context)
}

/// Expand one entry into `context`, keeping its diagnostics
/// ([`TemplateContext::unknown_includes`]) and the loop stack.
pub fn expand_with(
    library: &impl Includes,
    entry: &str,
    context: &mut TemplateContext,
) -> Result<String, TemplateError> {
    let source = library
        .source(entry)
        .ok_or_else(|| TemplateError::MissingEntry(entry.to_string()))?;
    let mut out = String::with_capacity(source.len() + 512);
    if !source.starts_with('#') {
        out.push_str("#version 150\r\n");
    }
    expand_into(library, source.as_bytes(), context, &mut out);
    Ok(out)
}

/// Expand a surface program: the shared vertex program and the entry
/// `<set>_f` (the original builds every surface set that way).
pub fn expand_surface(
    library: &impl Includes,
    set: &str,
    context: &TemplateContext,
) -> Result<ProgramText, TemplateError> {
    let mut vertex_context = context.clone();
    vertex_context.interface = InterfaceMode::Vertex;
    let mut fragment_context = context.clone();
    fragment_context.interface = InterfaceMode::Fragment;
    let vertex = expand_with(library, "shading_v", &mut vertex_context)?;
    let fragment = expand_with(library, &format!("{set}_f"), &mut fragment_context)?;
    Ok(ProgramText { vertex, fragment })
}

fn expand_into(
    library: &impl Includes,
    source: &[u8],
    context: &mut TemplateContext,
    out: &mut String,
) {
    let mut depth: u32 = 0;
    let mut i = 0usize;
    let mut copy_from = 0usize;
    while i < source.len() {
        if source[i] != b'{' {
            i += 1;
            continue;
        }
        let Some(&marker) = source.get(i + 1) else {
            i += 1;
            continue;
        };
        if marker != b'%' && marker != b'?' && marker != b'$' {
            i += 1;
            continue;
        }
        out.push_str(&String::from_utf8_lossy(&source[copy_from..i]));
        let next = match marker {
            b'%' => command(source, i + 2, context, out),
            b'?' => conditional(source, i + 2, context, &mut depth),
            _ => include(library, source, i + 2, context, out),
        };
        i = skip_blanks(source, next.min(source.len()));
        copy_from = i;
    }
    out.push_str(&String::from_utf8_lossy(&source[copy_from..]));
}

/// The builder drops every whitespace character right after a marker,
/// unless only whitespace remains; then the run is copied as usual.
fn skip_blanks(source: &[u8], start: usize) -> usize {
    let mut i = start;
    while i < source.len() {
        if source[i] > 0x20 {
            return i;
        }
        i += 1;
    }
    start
}

/// `{$name}`: an include, `{$Interface}`, or nothing when the library has no
/// such entry.
fn include(
    library: &impl Includes,
    source: &[u8],
    start: usize,
    context: &mut TemplateContext,
    out: &mut String,
) -> usize {
    let mut end = start;
    while end < source.len() && source[end] != b'}' {
        end += 1;
    }
    let name = String::from_utf8_lossy(&source[start..end]);
    if name == "Interface" {
        match context.interface {
            InterfaceMode::None => {}
            InterfaceMode::Vertex => out.push_str(&interface_vertex(&context.flags)),
            InterfaceMode::Fragment => out.push_str(&interface_fragment(context)),
        }
    } else if let Some(entry) = library.source(&name) {
        expand_into(library, entry.as_bytes(), context, out);
    } else {
        context.unknown_includes.push(name.into_owned());
    }
    (end + 1).min(source.len())
}

/// `{%…%}`: a loop marker, a loop end, a swizzle, or an integer expression.
fn command(source: &[u8], start: usize, context: &mut TemplateContext, out: &mut String) -> usize {
    let mut close = start;
    while close + 1 < source.len() {
        if source[close] == b'%' && source[close + 1] == b'}' {
            break;
        }
        close += 1;
    }
    let percent = close.min(source.len().saturating_sub(1));
    let after = (percent + 2).min(source.len());
    // The command skipper consumes CR/LF pairs of its own; the caller then
    // drops the remaining blanks.
    let mut cursor = after;
    while cursor < source.len() && source[cursor] == b'\r' {
        cursor += 1;
        if cursor < source.len() && source[cursor] == b'\n' {
            cursor += 1;
        }
    }

    if source.get(start..start + 4).is_some_and(|s| s == b"LOOP") {
        return match loop_start(source, start, percent, cursor, context) {
            Some(skip_to) => skip_to,
            None => cursor,
        };
    }
    if source.get(start..start + 4).is_some_and(|s| s == b"END%") {
        if let Some(frame) = context.loops.last_mut() {
            frame.current += 1;
            if frame.current >= frame.count {
                context.loops.pop();
            } else {
                return frame.body;
            }
        }
        return cursor;
    }
    if source.get(start) == Some(&b'.') {
        // The builder maps one character through "xyzw".
        let index = parse_number(source, start + 1, 1, context);
        if let Some(&channel) = b"xyzw".get(index.max(0) as usize) {
            out.push('.');
            out.push(channel as char);
        }
        return cursor;
    }

    let value = expression(source, start, percent + 1, context);
    out.push_str(&format_number(value));
    cursor
}

/// Parse the `LOOP` marker: its count, optional variable and optional
/// initial value. Returns `Some(position)` when the loop must be skipped
/// (a count below one) and `None` after pushing its frame.
fn loop_start(
    source: &[u8],
    start: usize,
    percent: usize,
    cursor: usize,
    context: &mut TemplateContext,
) -> Option<usize> {
    let mut p = start + 5;
    let count = if source.get(p).is_some_and(u8::is_ascii_digit) {
        let mut value = 0i64;
        while p < percent {
            let Some(&c) = source.get(p) else { break };
            if !c.is_ascii_digit() {
                break;
            }
            value = value * 10 + i64::from(c - b'0');
            p += 1;
        }
        value
    } else {
        let name_start = p;
        while p < percent
            && !matches!(
                source.get(p),
                Some(b' ') | Some(b'*') | Some(b'+') | Some(b'-') | Some(b'[') | None
            )
        {
            p += 1;
        }
        let mut value = context
            .number(source.get(name_start..p).unwrap_or(&[]))
            .unwrap_or(0);
        if source.get(p) == Some(&b'-') {
            p += 1;
            value -= i64::from(source.get(p).copied().unwrap_or(b'0').saturating_sub(b'0'));
            p += 1;
        }
        value
    };
    let mut variable = 0u8;
    let mut initial = 0i64;
    if p + 1 < percent {
        variable = source.get(p + 1).copied().unwrap_or(0);
        if p + 3 < percent
            && let Some(&digit) = source.get(p + 3)
            && digit.is_ascii_digit()
        {
            initial = i64::from(digit - b'0');
        }
    }
    if count - initial < 1 {
        return Some(skip_loop(source, cursor));
    }
    context.loops.push(LoopFrame {
        body: cursor,
        count,
        current: initial,
        variable,
    });
    None
}

/// Skip a loop body to just after its matching `{%END%}`.
fn skip_loop(source: &[u8], start: usize) -> usize {
    let mut level = 0usize;
    let mut i = start;
    while i + 1 < source.len() {
        if source[i] == b'{' && source[i + 1] == b'%' {
            match source.get(i + 2..i + 6) {
                Some(head) if head == b"LOOP" => level += 1,
                Some(head) if head == b"END%" => {
                    if level == 0 {
                        let mut j = i + 2;
                        while j + 1 < source.len() {
                            if source[j] == b'%' && source[j + 1] == b'}' {
                                return j + 2;
                            }
                            j += 1;
                        }
                        return source.len();
                    }
                    level -= 1;
                }
                _ => {}
            }
        }
        i += 1;
    }
    source.len()
}

/// Evaluate one condition term and return `(value, position after it)`.
///
/// The returned position is after the term's `?` (or space), where the
/// caller looks for `}` or `&`, and after the whole marker when the term
/// ended at its `}`.
fn term(source: &[u8], pos: usize, context: &TemplateContext) -> (bool, usize) {
    let len = source.len();
    let mut i = pos;
    let mut name_len = 0usize;
    let mut member = 0usize;
    let mut bracket: Option<i64> = None;
    while i < len {
        match source[i] {
            b' ' | b'?' => {
                if name_len == 0 {
                    name_len = i - pos;
                }
                break;
            }
            b'[' => {
                name_len = i - pos;
                bracket = Some(parse_number(source, i + 1, 1, context));
                i += 2;
            }
            b'.' => {
                if name_len == 0 {
                    name_len = i - pos;
                }
                member = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    let after = i.saturating_add(1);
    let name_end = (pos + name_len).min(len);
    let name = &source[pos.min(len)..name_end];
    let digit_at = |at: usize, default: i64| {
        source
            .get(at)
            .copied()
            .filter(u8::is_ascii_digit)
            .map_or(default, |c| i64::from(c - b'0'))
    };
    if member == 0 {
        let Some(value) = context.number(name) else {
            return (false, after);
        };
        if source.get(after) == Some(&b'>') {
            return (value > digit_at(after + 2, 0), (after + 4).min(len));
        }
        if source.get(after) == Some(&b'=') {
            return (value == digit_at(after + 2, 0), (after + 4).min(len));
        }
        return (value != 0, after);
    }
    let member_name = &source[member.min(len)..i.min(len)];
    let value = if name == b"Mat" {
        flag_value(MATERIAL_FLAGS, member_name, context.flags.material)
    } else if name == b"Shd" {
        flag_value(SHADER_FLAGS, member_name, context.flags.shader)
    } else if name == b"Obj" {
        flag_value(OBJECT_FLAGS, member_name, context.flags.object)
    } else if name == b"Env" {
        flag_value(ENVIRONMENT_FLAGS, member_name, context.flags.environment)
    } else if name == b"Light" {
        let index = bracket.unwrap_or(0).max(0) as usize;
        let kind = context.kind_of(index);
        match member_name {
            b"Direct" => Some(i64::from(kind.is_direct())),
            b"Sky" => Some(i64::from(kind.is_sky())),
            _ => None,
        }
    } else {
        None
    };
    let Some(value) = value else {
        return (false, after);
    };
    if source.get(after) == Some(&b'>') {
        return (value > digit_at(after + 2, 0), (after + 4).min(len));
    }
    if source.get(after) == Some(&b'=') {
        return (value == digit_at(after + 2, 0), (after + 4).min(len));
    }
    (value >= 1, after)
}

fn flag_value(table: &[FlagDef], member: &[u8], word: u32) -> Option<i64> {
    let def = table.iter().find(|def| def.name.as_bytes() == member)?;
    Some(i64::from((word & def.mask) >> def.shift))
}

/// The builder's condition chain: one term, then `&` joins. Its loop
/// re-evaluates the term after each `&` and takes the last term's value, so
/// a false term before an `&` is forgotten; that quirk is reproduced (only
/// `Lights > 0 & Light[0].Direct` relies on the chain).
fn chain(source: &[u8], pos: usize, context: &TemplateContext) -> (bool, usize) {
    let mut param = pos;
    loop {
        let negate = source.get(param) == Some(&b'!');
        let (mut value, after) = term(source, param + usize::from(negate), context);
        if negate {
            value = !value;
        }
        let mut q = after;
        while source.get(q) == Some(&b' ') {
            q += 1;
        }
        if source.get(q) == Some(&b'}') {
            return (value, (q + 1).min(source.len()));
        }
        if source.get(q) != Some(&b'&') {
            return (value, after);
        }
        param = q + 2;
        let (next, next_after) = term(source, param, context);
        let mut r = next_after;
        while source.get(r) == Some(&b' ') {
            r += 1;
        }
        if !next {
            // The builder clears its result and leaves the loop; the caller
            // looks for an alternate branch from the term's end.
            return (false, next_after);
        }
        if source.get(r) == Some(&b'}') {
            return (true, (r + 1).min(source.len()));
        }
        if source.get(r) != Some(&b'&') {
            return (true, r);
        }
        // Loop with the term after the `&`, as the builder does.
    }
}

/// Handle `{?…` at `pos`, returning the source position to continue from.
fn conditional(source: &[u8], pos: usize, context: &mut TemplateContext, depth: &mut u32) -> usize {
    let head = source.get(pos..pos + 4);
    if head.is_some_and(|h| h == b"END?") {
        *depth = depth.saturating_sub(1);
        return pos + 5;
    }
    if head.is_some_and(|h| h == b"ELSE") {
        // Inside the taken branch: jump over the alternate branch to its
        // matching end. The end closes the conditional we are in.
        let end = skip_to_end(source, pos + 5);
        *depth = depth.saturating_sub(1);
        return end;
    }
    let (value, after) = chain(source, pos, context);
    if value {
        *depth += 1;
        return after;
    }
    let (position, taken_else) = find_else_or_end(source, after, context);
    if taken_else {
        *depth += 1;
    }
    position
}

/// Skip a conditional's alternate branch up to and including its matching
/// `{?END?}`. Nested conditionals are counted; a nested `ELSE` does not open
/// a level.
fn skip_to_end(source: &[u8], start: usize) -> usize {
    let mut level = 0u32;
    let mut i = start;
    while i + 1 < source.len() {
        if source[i] == b'{' && source[i + 1] == b'?' {
            match source.get(i + 2..i + 6) {
                Some(head) if head == b"END?" => {
                    if level == 0 {
                        return (i + 7).min(source.len());
                    }
                    level -= 1;
                }
                Some(head) if head == b"ELSE" => {}
                _ => level += 1,
            }
        }
        i += 1;
    }
    source.len()
}

/// Find the alternate branch of a false conditional, or its end.
///
/// Returns the position to continue from and whether the alternate branch
/// was taken (`{?ELSE?}` or an `{?ELSE cond?}` whose condition holds).
fn find_else_or_end(source: &[u8], start: usize, context: &TemplateContext) -> (usize, bool) {
    let mut level = 0u32;
    let mut i = start;
    while i + 1 < source.len() {
        if source[i] == b'{' && source[i + 1] == b'?' {
            match source.get(i + 2..i + 6) {
                Some(head) if head == b"ELSE" => {
                    if level == 0 {
                        if source.get(i + 6) == Some(&b'?') {
                            return ((i + 8).min(source.len()), true);
                        }
                        let (value, after) = chain(source, i + 7, context);
                        let mut next = after;
                        if source.get(next) == Some(&b'}') {
                            next += 1;
                        }
                        if value {
                            return (next.min(source.len()), true);
                        }
                        i = next;
                        continue;
                    }
                }
                Some(head) if head == b"END?" => {
                    if level == 0 {
                        return ((i + 7).min(source.len()), false);
                    }
                    level -= 1;
                }
                _ => level += 1,
            }
        }
        i += 1;
    }
    (source.len(), false)
}

/// Parse one integer token. A leading digit is a decimal number, whatever
/// the requested length (the quotient operator asks for a zero-length token
/// and the builder still reads its first digit); otherwise one-character
/// names are loop variables and longer names are integer variables.
fn parse_number(source: &[u8], start: usize, length: usize, context: &TemplateContext) -> i64 {
    let Some(&first) = source.get(start) else {
        return 0;
    };
    if first.is_ascii_digit() {
        let mut value = i64::from(first - b'0');
        for k in 1..length.max(1) {
            let Some(&c) = source.get(start + k) else {
                break;
            };
            if !c.is_ascii_digit() {
                break;
            }
            value = value * 10 + i64::from(c - b'0');
        }
        return value;
    }
    if length == 1
        && let Some(value) = context.loop_value(first)
    {
        return value;
    }
    context
        .number(&source[start..(start + length).min(source.len())])
        .unwrap_or(0)
}

/// The builder's expression evaluator: tokens separated by `* + - /`,
/// applied strictly left to right (no precedence), ending at `%`.
fn expression(source: &[u8], start: usize, end: usize, context: &TemplateContext) -> f32 {
    let end = end.min(source.len());
    let mut accumulator = 0.0f32;
    let mut operation = 0u8;
    let mut token = start;
    let mut i = start;
    while i < end {
        let c = source[i];
        let token_end = match c {
            b' ' | b'%' => i,
            b'*' | b'+' | b'-' => i,
            b'/' => i.saturating_sub(1),
            _ => {
                i += 1;
                continue;
            }
        };
        let value = parse_number(source, token, token_end.saturating_sub(token), context) as f32;
        accumulator = apply(accumulator, operation, value);
        if c == b' ' || c == b'%' {
            return accumulator;
        }
        operation = c;
        token = i + 1;
        i += 1;
    }
    // A token with no terminator (the helper's own use).
    let value = parse_number(source, token, end.saturating_sub(token), context) as f32;
    apply(accumulator, operation, value)
}

fn apply(accumulator: f32, operation: u8, value: f32) -> f32 {
    match operation {
        0 => value,
        b'*' => accumulator * value,
        b'+' => accumulator + value,
        b'-' => accumulator - value,
        b'/' => {
            if value == 0.0 {
                // The original's float division by zero would produce a
                // non-integer and print it; no shipped expression divides by
                // zero, so 0 is a safe, documented fallback.
                0.0
            } else {
                accumulator / value
            }
        }
        _ => accumulator,
    }
}

/// The builder prints an integral expression as an integer and anything
/// else as a general-format number with three significant digits (the one
/// shipped fractional expression, `1/DspSdws`, prints `0.25`).
fn format_number(value: f32) -> String {
    if !value.is_finite() {
        return "0".to_string();
    }
    let truncated = value.trunc();
    if (value - truncated).abs() <= 0.0001 {
        return format!("{}", truncated as i64);
    }
    format_general(value)
}

fn format_general(value: f32) -> String {
    let exponent = value.abs().log10().floor() as i32;
    let decimals = (2 - exponent).max(0) as usize;
    let mut text = format!("{value:.decimals$}");
    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
    text
}

/// The formatted text of one `{%…%}` expression, without its marker, for
/// tests and diagnostics.
pub fn expression_text(source: &str, context: &TemplateContext) -> String {
    format_number(expression(source.as_bytes(), 0, source.len(), context))
}

/// The `Global` block's declaration: std140, 252 bytes.
pub const GLOBAL_BLOCK: &str = "layout (std140) uniform Global {\r\n\
vec4 ViewPos;\r\n\
vec2 ViewSiz;\r\n\
vec2 ViewPrj;\r\n\
mat4 CamMat;\r\n\
mat4 SdwMat;\r\n\
vec4 VXGIScl;\r\n\
vec4 Ambient;\r\n\
vec2 TimeDlt;\r\n\
vec2 ViewRcp;\r\n\
vec4 FogColr;\r\n\
vec4 FogPos;\r\n\
vec2 BufrPos;\r\n\
float Wetness;\r\n\
}";

fn declare(out: &mut String, text: &str) {
    out.push_str(text);
    out.push_str(";\r\n");
}

/// The vertex-stage `{$Interface}`.
pub fn interface_vertex(flags: &Flags) -> String {
    let mut out = String::new();
    declare(&mut out, GLOBAL_BLOCK);
    if flags.shader & 0x10000 != 0 {
        declare(&mut out, "in vec3 vNrm");
        declare(&mut out, "out vec3 vrNrm");
    }
    if flags.object & 0x100000 != 0 {
        declare(&mut out, "in vec3 vDlt");
    }
    if flags.object & 0x200000 != 0 {
        declare(&mut out, "in vec4 vCol");
        declare(&mut out, "out vec4 vrCol");
    }
    if flags.material & 0xFFF0000 != 0 {
        declare(&mut out, "in vec2 vTex");
        if flags.material & 0x800000 == 0 {
            declare(&mut out, "out vec2 vrTex");
        } else {
            declare(&mut out, "out vec2 vriTex");
        }
    }
    if flags.object & 0x800000 != 0 {
        declare(&mut out, "uniform vec4 VrtCldr");
    }
    if flags.object & 0x1000 != 0 && flags.object & 0x100000 != 0 {
        declare(&mut out, "uniform int PassIdx");
    }
    out
}

/// The fragment-stage `{$Interface}`.
pub fn interface_fragment(context: &TemplateContext) -> String {
    let flags = &context.flags;
    let numbers = &context.numbers;
    let mut out = String::new();
    if flags.shader & 0x10000 != 0 {
        declare(&mut out, "in vec3 vrNrm");
    }
    if flags.object & 0x200000 != 0 {
        declare(&mut out, "in vec4 vrCol");
    }
    if flags.material & 0xFFF0000 != 0 {
        if flags.material & 0x800000 == 0 {
            declare(&mut out, "in vec2 vrTex");
        } else {
            declare(&mut out, "in vec2 vriTex");
        }
        if flags.material & 0x200000 != 0 {
            declare(&mut out, "uniform sampler2D NrmTex");
            if flags.shader & 0x20000 != 0 {
                declare(&mut out, "in vec4 vrTng");
            }
        }
        if flags.material & 0x20000 != 0 {
            declare(&mut out, "uniform sampler2D DifTex");
        }
        if flags.material & 0x80000 != 0 {
            declare(&mut out, "uniform sampler2D EmiTex");
        }
        if flags.material & 0x40000 != 0 {
            declare(&mut out, "uniform sampler2D SpcTex");
        }
        if flags.material & 0x400000 != 0 {
            declare(&mut out, "uniform sampler2D OpcTex");
        }
        if flags.material & 0x1000000 != 0 {
            declare(&mut out, "uniform sampler2D GlsTex");
        }
        if flags.material & 0x800000 != 0 || flags.shader & 0x100 != 0 {
            declare(&mut out, "uniform sampler2D DspTex");
        }
        if flags.material & 0x8000000 != 0 {
            declare(&mut out, "uniform sampler2D DrtTex");
        }
        if flags.material & 0x800000 != 0 {
            declare(&mut out, "vec2 vrTex");
        }
    }
    if flags.shader & 0x80000000 == 0 {
        declare(&mut out, GLOBAL_BLOCK);
    }
    if flags.material & 7 != 0 {
        declare(&mut out, "uniform samplerCube EnvMap");
    }
    if flags.material & 0x300 == 0x200 {
        declare(&mut out, "uniform float LqdLvl");
    }
    if flags.environment & 0x10 != 0 {
        declare(&mut out, "uniform vec3 VXGIPos");
        declare(&mut out, "uniform sampler3D VoxelGI");
    }
    if numbers.lights > 0 {
        if matches!(context.kind_of(0), LightKind::Spot | LightKind::Directional) {
            declare(&mut out, "in vec3 vrSdw");
            declare(&mut out, "uniform sampler2DShadow Shadow0");
        } else {
            declare(&mut out, "uniform samplerCubeShadow Shadow0");
        }
        for i in 1..numbers.lights {
            declare(&mut out, &format!("uniform samplerCubeShadow Shadow{i}"));
        }
    }
    if flags.material & 4 != 0 {
        declare(&mut out, "uniform sampler2D FrmBuf");
    }
    let taps = if flags.object & 0x1000 != 0 {
        declare(&mut out, "uniform int PassIdx");
        numbers.shadow_samples.saturating_mul(4)
    } else {
        numbers.shadow_samples
    };
    if flags.material & 7 != 0 {
        declare(&mut out, &format!("uniform vec3 SdwSmp[{taps}]"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lib(entries: &[(&str, &str)]) -> HashMap<String, String> {
        entries
            .iter()
            .map(|(name, source)| (name.to_string(), source.to_string()))
            .collect()
    }

    fn context() -> TemplateContext {
        TemplateContext::new(
            Flags {
                material: 0x2 | 0x4 | 0x20000,
                shader: 0,
                object: 0,
                environment: 0x10,
            },
            Numbers {
                lights: 2,
                shadow_samples: 2,
                shade_quality: 2,
                displacement_samples: 4,
            },
        )
    }

    fn eval(text: &str, context: &TemplateContext) -> bool {
        let source = format!("{text}?}}");
        chain(source.as_bytes(), 0, context).0
    }

    #[test]
    fn flag_tables_hold_the_engine_masks() {
        let def = |table: &[FlagDef], name: &str| {
            let def = table.iter().find(|d| d.name == name).unwrap();
            (def.mask, def.shift)
        };
        assert_eq!(def(MATERIAL_FLAGS, "MatDif"), (0x2, 0));
        assert_eq!(def(MATERIAL_FLAGS, "MatSpc"), (0x4, 0));
        assert_eq!(def(MATERIAL_FLAGS, "MatEmi"), (0x8, 0));
        assert_eq!(def(MATERIAL_FLAGS, "MatAmbDif"), (0x3, 0));
        assert_eq!(def(MATERIAL_FLAGS, "Textures"), (0xFFF0000, 0));
        assert_eq!(def(MATERIAL_FLAGS, "TexOpc"), (0x400000, 0));
        assert_eq!(def(MATERIAL_FLAGS, "Refract"), (0x200, 0));
        assert_eq!(def(SHADER_FLAGS, "DirtMode"), (0x70, 4));
        assert_eq!(def(SHADER_FLAGS, "BlendMode"), (0x7000, 12));
        assert_eq!(def(SHADER_FLAGS, "DispSdw"), (0x100, 0));
        assert_eq!(def(SHADER_FLAGS, "FastLgt"), (0x200, 0));
        assert_eq!(def(OBJECT_FLAGS, "VrtCol"), (0x200000, 0));
        assert_eq!(def(ENVIRONMENT_FLAGS, "FogMode"), (0x3, 0));
        assert_eq!(def(ENVIRONMENT_FLAGS, "VXGISpec"), (0x80, 0));
    }

    #[test]
    fn condition_terms_read_flags_and_fields() {
        let mut c = context();
        c.flags.shader = 0x4000 | 0x40 | 0x2;
        assert!(eval("Mat.MatAmbDif", &c));
        assert!(eval("Mat.TexDif", &c));
        assert!(!eval("Mat.TexEmi", &c));
        assert!(eval("Shd.DirtMode = 4", &c));
        assert!(!eval("Shd.DirtMode = 2", &c));
        assert!(eval("Shd.BlendMode = 4", &c));
        assert!(!eval("Shd.DirtMode > 4", &c));
        assert!(eval("Lights > 1", &c));
        assert!(!eval("Lights > 2", &c));
        assert!(eval("Lights = 2", &c));
        assert!(eval("SdwSmps", &c));
        assert!(eval("Env.VxlGI", &c));
        assert!(!eval("Obj.VrtCol", &c));
        assert!(!eval("Unknown.Thing", &c));
    }

    #[test]
    fn direct_light_is_the_directional_kind() {
        let mut c = context();
        c.lights = vec![LightKind::Point, LightKind::Directional];
        assert!(!eval("Light[0].Direct", &c));
        assert!(eval("Light[1].Direct", &c));
        c.lights[0] = LightKind::Sky;
        assert!(eval("Light[0].Sky", &c));
        assert!(!eval("Light[0].Direct", &c));
    }

    #[test]
    fn and_chain_reproduces_the_builders_quirk() {
        let mut c = context();
        c.numbers.lights = 0;
        c.lights = vec![LightKind::Directional];
        // The builder forgets the false first term and takes the second.
        assert!(eval("Lights > 0 & Light[0].Direct", &c));
        c.lights = vec![LightKind::Point];
        assert!(!eval("Lights > 0 & Light[0].Direct", &c));
    }

    #[test]
    fn expression_arithmetic_is_left_to_right() {
        let c = context();
        assert_eq!(expression_text("Lights*3+2", &c), "8");
        assert_eq!(expression_text("Lights*2+1", &c), "5");
        assert_eq!(expression_text("1/DspSdws", &c), "0.25");
        assert_eq!(expression_text("SdwSmps+1", &c), "3");
        assert_eq!(expression_text("7", &c), "7");
        assert_eq!(expression_text("0", &c), "0");
    }

    #[test]
    fn loop_variables_hold_their_current_value() {
        let mut c = context();
        c.loops.push(LoopFrame {
            body: 0,
            count: 5,
            current: 3,
            variable: b'i',
        });
        assert_eq!(expression_text("i*3+1", &c), "10");
        assert_eq!(expression_text("i", &c), "3");
    }

    #[test]
    fn loops_repeat_and_end_after_the_count() {
        let library = lib(&[("p", "{%LOOP Lights i%}[{%i%}]{%END%}")]);
        let c = context();
        let text = expand(&library, "p", &c).unwrap();
        assert_eq!(text, "#version 150\r\n[0][1]");
    }

    #[test]
    fn loop_with_initial_and_variable() {
        let library = lib(&[("p", "{%LOOP 3 i 1%}{%i%},{%END%}")]);
        let c = context();
        let text = expand(&library, "p", &c).unwrap();
        assert_eq!(text, "#version 150\r\n1,2,");
    }

    #[test]
    fn zero_length_loop_skips_its_body() {
        let library = lib(&[("p", "a{%LOOP DspSdws-4 b%}x{%END%}z")]);
        let c = context();
        let text = expand(&library, "p", &c).unwrap();
        assert_eq!(text, "#version 150\r\naz");
    }

    #[test]
    fn conditionals_take_else_branches_and_nest() {
        let library = lib(&[(
            "p",
            "{%LOOP Lights i%}{?Light[i].Direct?}d{?ELSE?}p{?END?}{%END%}",
        )]);
        let mut c = context();
        c.lights = vec![LightKind::Point, LightKind::Directional];
        let text = expand(&library, "p", &c).unwrap();
        assert_eq!(text, "#version 150\r\npd");
    }

    #[test]
    fn else_after_taken_branch_is_skipped() {
        let library = lib(&[("p", "{?Mat.MatAmbDif?}yes{?ELSE?}no{?END?}")]);
        let c = context();
        assert_eq!(expand(&library, "p", &c).unwrap(), "#version 150\r\nyes");
    }

    #[test]
    fn negation_and_and_chain() {
        let library = lib(&[(
            "p",
            "{?Mat.TexDif & Shd.FastLgt?}both{?END?}{?!Mat.TexEmi?}no-emi{?END?}",
        )]);
        let mut c = context();
        c.flags.shader = 0x200;
        assert_eq!(
            expand(&library, "p", &c).unwrap(),
            "#version 150\r\nbothno-emi"
        );
    }

    #[test]
    fn includes_expand_in_place_and_unknown_ones_are_dropped() {
        let library = lib(&[
            ("p", "a{$inner}b{$missing}c"),
            ("inner", "[{$leaf}]"),
            ("leaf", "leaf"),
        ]);
        let mut c = context();
        let text = expand_with(&library, "p", &mut c).unwrap();
        assert_eq!(text, "#version 150\r\na[leaf]bc");
        assert_eq!(c.unknown_includes, vec!["missing".to_string()]);
    }

    #[test]
    fn interface_vertex_lists_the_streams() {
        let flags = Flags {
            material: 0x20000 | 0x800000,
            shader: 0x10000,
            object: 0x200000 | 0x100000 | 0x1000,
            environment: 0,
        };
        let text = interface_vertex(&flags);
        assert!(text.starts_with("layout (std140) uniform Global {"));
        assert!(text.contains("out vec3 vrNrm;\r\n"));
        assert!(text.contains("in vec3 vDlt;\r\n"));
        assert!(text.contains("in vec4 vCol;\r\n"));
        assert!(text.contains("out vec2 vriTex;\r\n"));
        assert!(text.contains("uniform int PassIdx;\r\n"));
        assert!(text.contains("float Wetness;\r\n};\r\n"));
    }

    #[test]
    fn interface_fragment_declares_samplers_shadows_and_taps() {
        let mut c = context();
        c.flags.material =
            0x2 | 0x4 | 0x20000 | 0x40000 | 0x80000 | 0x200000 | 0x400000 | 0x800000 | 0x1000000;
        c.flags.shader = 0x20000 | 0x100;
        c.flags.object = 0x1000;
        c.lights = vec![LightKind::Point, LightKind::Point];
        let text = interface_fragment(&c);
        assert!(text.contains("in vec2 vriTex;\r\n"));
        assert!(text.contains("uniform sampler2D NrmTex;\r\n"));
        assert!(text.contains("in vec4 vrTng;\r\n"));
        assert!(text.contains("uniform sampler2D DspTex;\r\n"));
        assert!(text.contains("vec2 vrTex;\r\n"));
        assert!(text.contains("uniform samplerCubeShadow Shadow0;\r\n"));
        assert!(text.contains("uniform samplerCubeShadow Shadow1;\r\n"));
        assert!(text.contains("uniform int PassIdx;\r\n"));
        assert!(text.contains("uniform vec3 SdwSmp[8];\r\n"));
        assert!(text.contains("uniform samplerCube EnvMap;\r\n"));
        assert!(text.contains("uniform sampler3D VoxelGI;\r\n"));
    }

    #[test]
    fn interface_fragment_uses_a_2d_shadow_for_a_spot_first_light() {
        let mut c = context();
        c.numbers.lights = 1;
        c.lights = vec![LightKind::Spot];
        let text = interface_fragment(&c);
        assert!(text.contains("in vec3 vrSdw;\r\n"));
        assert!(text.contains("uniform sampler2DShadow Shadow0;\r\n"));
        assert!(!text.contains("samplerCubeShadow"));
    }

    #[test]
    fn whitespace_right_after_a_marker_is_dropped() {
        let library = lib(&[("p", "a {?Mat.MatAmbDif?}\r\n  b {%i%} c")]);
        let mut c = context();
        c.loops.push(LoopFrame {
            body: 0,
            count: 1,
            current: 0,
            variable: b'i',
        });
        let text = expand(&library, "p", &c).unwrap();
        assert_eq!(text, "#version 150\r\na b 0c");
    }

    #[test]
    fn malformed_markers_do_not_panic() {
        let c = context();
        for source in [
            "{%", "{%LOOP", "{?Mat", "{?", "{$", "{%LOOP x", "{%1/", "{?ELSE",
        ] {
            let library = lib(&[("p", source)]);
            let _ = expand(&library, "p", &c);
        }
    }

    #[test]
    fn version_is_only_prepended_without_a_hash() {
        let library = lib(&[("p", "#version 330\r\nx")]);
        let c = context();
        assert_eq!(expand(&library, "p", &c).unwrap(), "#version 330\r\nx");
    }

    #[test]
    fn missing_entry_is_an_error() {
        let library = lib(&[]);
        let c = context();
        assert_eq!(
            expand(&library, "nope", &c),
            Err(TemplateError::MissingEntry("nope".into()))
        );
    }

    #[test]
    fn the_dot_command_swizzles_a_loop_variable() {
        let library = lib(&[("p", "{%LOOP 3 i%}{%.i%}{%END%}")]);
        let c = context();
        assert_eq!(expand(&library, "p", &c).unwrap(), "#version 150\r\n.x.y.z");
    }

    #[test]
    fn surface_program_uses_shading_v_for_every_set() {
        let library = lib(&[("shading_v", "vertex"), ("shadeopt_f", "fragment")]);
        let c = context();
        let text = expand_surface(&library, "shadeopt", &c).unwrap();
        assert_eq!(text.vertex, "#version 150\r\nvertex");
        assert_eq!(text.fragment, "#version 150\r\nfragment");
    }
}

#[cfg(test)]
mod installed {
    use super::*;
    use crate::shader_program::{GeometryOptions, SceneOptions, program_keys};
    use exa_formats::flb::ShaderLibrary;

    fn library() -> Option<ShaderLibrary> {
        let dir = exa_rpk::locate_install(None)?;
        let game = exa_rpk::GameData::open(&dir).ok()?;
        let resource = game.pack(exa_rpk::PackKind::Resource)?;
        ShaderLibrary::parse(resource.get("shading.flb")?).ok()
    }

    fn context() -> TemplateContext {
        let material = exa_formats::material::Material {
            diffuse: [0.5, 0.25, 0.125, 1.0],
            specular: [0.3, 0.3, 0.3, 1.0],
            opacity: 0.9,
            glossiness: 0.7,
            specular_level: 0.8,
            class: 0x100,
            flags: 0x3 | 0x4 | 0x8 | 0x10 | 0x20000 | 0x40000 | 0x80000 | 0x200000 | 0x1000000,
            kind: 1,
            ..Default::default()
        };
        let keys = program_keys(
            &material,
            GeometryOptions::default(),
            &SceneOptions::default(),
        );
        keys.into_iter()
            .find(|k| k.numbers.lights == 3)
            .map(|k| k.context())
            .unwrap()
    }

    #[test]
    fn every_shading_entry_expands_without_markers() {
        let Some(library) = library() else { return };
        let mut context = context();
        context.flags.object |= 0x100000 | 0x200000;
        for set in ["shading", "shadeopt", "shadehar", "shadesss", "shadetrl"] {
            let mut vertex_context = context.clone();
            vertex_context.interface = InterfaceMode::Vertex;
            let mut fragment_context = context.clone();
            fragment_context.interface = InterfaceMode::Fragment;
            let vertex = expand_with(&library, "shading_v", &mut vertex_context).unwrap();
            let fragment =
                expand_with(&library, &format!("{set}_f"), &mut fragment_context).unwrap();
            for text in [&vertex, &fragment] {
                assert!(!text.contains("{?"), "{set}: unexpanded condition");
                assert!(!text.contains("{%"), "{set}: unexpanded command");
                assert!(!text.contains("{$"), "{set}: unexpanded include");
            }
            assert!(vertex_context.unknown_includes.is_empty());
            assert!(fragment_context.unknown_includes.is_empty());
            assert!(vertex.starts_with("#version 150\r\n"));
            assert!(fragment.starts_with("#version 150\r\n"));
        }
    }

    #[test]
    fn every_library_entry_expands() {
        let Some(library) = library() else { return };
        let context = context();
        let mut unknown = Vec::new();
        for entry in library.entries() {
            let mut c = context.clone();
            let text = expand_with(&library, &entry.name, &mut c).unwrap();
            assert!(
                !text.contains("{?") && !text.contains("{%") && !text.contains("{$"),
                "{}: unexpanded marker",
                entry.name
            );
            unknown.extend(
                c.unknown_includes
                    .into_iter()
                    .map(|u| (entry.name.clone(), u)),
            );
        }
        assert!(unknown.is_empty(), "{unknown:?}");
    }

    #[test]
    fn shadeopt_has_the_pbr_lines() {
        let Some(library) = library() else { return };
        let mut c = context();
        c.interface = InterfaceMode::Fragment;
        let text = expand_with(&library, "shadeopt_f", &mut c).unwrap();
        assert!(text.contains("Gloss = 1.0 - MatSpc.w * SpcRGM.g;"));
        assert!(text.contains("difCol = MatDif.rgb * smp.rgb * (1.0-SpcRGM.b);"));
        assert!(text.contains("f0 = mix(vec3(0.04), smp.rgb * MatSpc.rgb, SpcRGM.b);"));
        assert!(text.contains("diffuse.rgb += MatEmi.rgb * texture(EmiTex, vrTex).rgb * a;"));
        // Three lights: the loop unrolls with the light indices written out.
        assert!(text.contains("uniform vec4 Light[9];"));
        assert!(text.contains("Light[6]"));
        assert!(!text.contains("{%"));
    }
}
