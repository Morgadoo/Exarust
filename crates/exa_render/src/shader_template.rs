//! The original's shader-template language: the `{?…?}`, `{%…%}` and
//! `{$…}` markers that turn a `.flb` entry into GLSL.
//!
//! The engine's builder walks a template once per variant and writes a plain
//! string: conditionals on named flags and integer variables, unrolled
//! loops, integer expressions, library includes, and the generated
//! `{$Interface}` block. The rules here mirror that walker, including its
//! whitespace handling (the blanks after a marker are dropped), its
//! left-to-right single-precision expression arithmetic, its number
//! printing, and its quirks (the `&` chain keeps only the last term; `Mat.`
//! and `Obj.` terms know no comparison; a `Shd.`/`Env.` term looks for its
//! `=` up to the marker's `}`). The flag words and the program choice live
//! in [`crate::shader_program`]; the parsed library is
//! [`exa_formats::flb::ShaderLibrary`]. Rules: `docs/formats/FLB.md`.
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
/// - **material**: the material state's shading and texture-slot bits;
/// - **shader**: the program class in the low nibble plus options such as
///   `DispSdw` and `FastLgt`, and the shading-quality lift in bits 20–21;
/// - **object**: the geometry's streams (`VrtCol`, `VrtDlt`), the
///   supersampling/`PassIdx` mode and the shadow-tap count;
/// - **environment**: GI, fog mode, weather, outdoor, sharpen, and two bits
///   per light slot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Flags {
    pub material: u32,
    pub shader: u32,
    pub object: u32,
    pub environment: u32,
}

/// The integer state behind the template's variables: `Lights`, `SdwSmps`
/// and `DspSdws`, and the super-sampling setting `SdwSmps` answers a bare
/// condition with. (`ShadeQual` is read from the shader word.)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Numbers {
    /// Lights on this object, 0 through 6.
    pub lights: u32,
    /// Shadow taps this sub-frame takes (`SdwSmps`), 1 through 32.
    pub shadow_samples: u32,
    /// Displacement-shadow taps per sub-frame (`DspSdws`): 0 without
    /// `Shd.DispSdw`, otherwise 4, 2 or 1 by the super-sampling factor.
    pub displacement_samples: u32,
    /// The super-sampling setting (0 off). A bare `{?SdwSmps?}` holds when
    /// there is more than one tap or this is on.
    pub supersample: u32,
}

/// The kind of one of an object's light slots, as the template's
/// `Light[i].Direct` reads it. The builder stores `0x200` for a light whose
/// node type bits (`0x300`) are clear, the directional sun with its 2D
/// shadow map, and `0x100` for the rest, the cube-shadowed point lights.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LightKind {
    #[default]
    Point,
    Directional,
}

impl LightKind {
    /// The slot word the builder keeps for the light (and shifts into the
    /// environment word, two bits per slot).
    pub fn slot(self) -> u32 {
        match self {
            Self::Point => 0x100,
            Self::Directional => 0x200,
        }
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
    /// The geometry stage: the builder generates no interface for it.
    Geometry,
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
    /// The kind of each light slot, indexed by `Light[i]`. Missing entries
    /// count as a point light.
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
    /// The value that ends the loop.
    count: i32,
    /// The current value (starts at the loop's initial digit).
    current: i32,
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

const fn flag(name: &'static str, mask: u32) -> FlagDef {
    FlagDef {
        name,
        mask,
        shift: 0,
    }
}

/// The `Mat.` keywords, in the order the builder registers them.
const MATERIAL_FLAGS: &[FlagDef] = &[
    flag("MatDif", 0x2),
    flag("MatSpc", 0x4),
    flag("MatEmi", 0x8),
    flag("TexDif", 0x20000),
    flag("TexSpc", 0x40000),
    flag("TexNrm", 0x200000),
    flag("TexEmi", 0x80000),
    flag("Textures", 0xFFF0000),
    flag("TexOpc", 0x400000),
    flag("MatAmbDif", 0x3),
    flag("TexGls", 0x1000000),
    flag("TexRef", 0x100000),
    flag("TexDsp", 0x800000),
    flag("OpctFall", 0x800),
    flag("Refract", 0x200),
    flag("Twosided", 0x10),
];

/// The `Shd.` keywords. The builder also writes a `QuadRender` name into
/// this table but leaves its length at zero, so `Shd.QuadRender` never
/// matches; it is left out.
const SHADER_FLAGS: &[FlagDef] = &[
    flag("UseNrm", 0x10000),
    flag("LocalRef", 0x400000),
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
    flag("AlphaTst", 0x8000),
    flag("NegNdL", 0x800),
    flag("TngtNrm", 0x20000),
    flag("DispSdw", 0x100),
    flag("FastLgt", 0x200),
];

/// The `Obj.` keywords.
const OBJECT_FLAGS: &[FlagDef] = &[
    flag("VrtDlt", 0x100000),
    flag("VrtCol", 0x200000),
    flag("QuadRender", 0x1000),
];

/// The `Env.` keywords.
const ENVIRONMENT_FLAGS: &[FlagDef] = &[
    flag("VxlGI", 0x10),
    flag("FogMode", 0x3),
    flag("WetFloor", 0x20),
    flag("ToneMap", 0x4),
    flag("Outdoor", 0x40),
    flag("VXGISpec", 0x80),
    flag("Sharpen", 0x8),
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

/// The four integer variables the builder registers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Variable {
    Lights,
    ShadowSamples,
    ShadeQuality,
    DisplacementSamples,
}

impl TemplateContext {
    pub fn new(flags: Flags, numbers: Numbers) -> Self {
        Self {
            flags,
            numbers,
            ..Self::default()
        }
    }

    /// The integer variable named exactly `name`.
    fn variable(name: &[u8]) -> Option<Variable> {
        Some(match name {
            b"Lights" => Variable::Lights,
            b"SdwSmps" => Variable::ShadowSamples,
            b"ShadeQual" => Variable::ShadeQuality,
            b"DspSdws" => Variable::DisplacementSamples,
            _ => return None,
        })
    }

    /// A variable's value as the builder's getter returns it.
    fn value(&self, variable: Variable) -> i32 {
        match variable {
            Variable::Lights => self.numbers.lights as i32,
            Variable::ShadowSamples => self.numbers.shadow_samples as i32,
            // The shading-quality lift in bits 20–21 of the shader word.
            Variable::ShadeQuality => ((self.flags.shader & 0x30_0000) >> 20) as i32,
            Variable::DisplacementSamples => self.numbers.displacement_samples as i32,
        }
    }

    /// A bare `{?Name?}` on a variable: the getter's truth callback, which
    /// `Lights` and `ShadeQual` do not have (always false).
    fn truth(&self, variable: Variable) -> bool {
        match variable {
            Variable::Lights | Variable::ShadeQuality => false,
            Variable::ShadowSamples => {
                (self.numbers.shadow_samples as i32) > 1 || (self.numbers.supersample as i32) > 0
            }
            Variable::DisplacementSamples => (self.numbers.displacement_samples as i32) > 0,
        }
    }

    /// The value of the integer variable named `name`, or `None`.
    fn number(&self, name: &[u8]) -> Option<i32> {
        Self::variable(name).map(|v| self.value(v))
    }

    /// A loop variable's current value, when the name is exactly one
    /// character (the builder searches the loop stack only then).
    fn loop_value(&self, variable: u8) -> Option<i32> {
        self.loops
            .iter()
            .rev()
            .find(|f| f.variable == variable)
            .map(|f| f.current)
    }

    fn kind_of(&self, index: usize) -> LightKind {
        self.lights.get(index).copied().unwrap_or_default()
    }

    /// `Light[i].Direct` and `Light[i].Sky`: both need `i` below `Lights`;
    /// `Direct` then tests the slot word, `Sky` only that `i` is 0.
    fn light_member(&self, member: &[u8], index: i32) -> bool {
        if index < 0 || index >= self.numbers.lights as i32 {
            return false;
        }
        match member {
            b"Direct" => self.kind_of(index as usize).slot() & 0x300 == 0x200,
            b"Sky" => index == 0,
            _ => false,
        }
    }
}

/// Expand one library entry into GLSL.
///
/// `#version 150` is prepended when the entry does not start with `#`,
/// exactly as the builder does, and the text ends at its first NUL (the
/// engine hands GL a NUL-terminated string).
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
    if let Some(nul) = out.find('\0') {
        out.truncate(nul);
    }
    Ok(out)
}

/// Expand a surface program: the shared vertex program and the entry
/// `<set>_f` (the original builds every surface set that way).
pub fn expand_surface(
    library: &impl Includes,
    set: &str,
    context: &TemplateContext,
) -> Result<ProgramText, TemplateError> {
    let entry = crate::shader_program::surface_entry(set)
        .ok_or_else(|| TemplateError::MissingEntry(format!("{set}_f")))?;
    let mut vertex_context = context.clone();
    vertex_context.interface = InterfaceMode::Vertex;
    let mut fragment_context = context.clone();
    fragment_context.interface = InterfaceMode::Fragment;
    let vertex = expand_with(library, "shading_v", &mut vertex_context)?;
    let fragment = expand_with(library, &entry, &mut fragment_context)?;
    Ok(ProgramText { vertex, fragment })
}

/// The stage a program entry is compiled for.
///
/// The builder's stage also decides `{$Interface}`: the vertex and fragment
/// stages expand their generated interface, the geometry stage expands none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShaderStage {
    Vertex,
    Geometry,
    Fragment,
}

impl ShaderStage {
    /// The GL shader kind the engine creates for this stage.
    pub fn kind(self) -> u32 {
        match self {
            Self::Vertex => 0x8B31,
            Self::Geometry => 0x8DD9,
            Self::Fragment => 0x8B30,
        }
    }

    /// The interface this stage's `{$Interface}` expands.
    pub fn interface(self) -> InterfaceMode {
        match self {
            Self::Vertex => InterfaceMode::Vertex,
            Self::Geometry => InterfaceMode::Geometry,
            Self::Fragment => InterfaceMode::Fragment,
        }
    }
}

/// A stage's expanded source and the GL kind it is compiled as.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledStage {
    pub stage: ShaderStage,
    pub kind: u32,
    pub source: String,
}

/// Compile one library entry for a stage, as the builder's per-stage compile
/// wrappers do: set the stage, expand the entry with that stage's interface
/// and return the source with the GL kind.
pub fn expand_stage(
    library: &impl Includes,
    entry: &str,
    context: &TemplateContext,
    stage: ShaderStage,
) -> Result<CompiledStage, TemplateError> {
    let mut context = context.clone();
    context.interface = stage.interface();
    let source = expand_with(library, entry, &mut context)?;
    Ok(CompiledStage {
        stage,
        kind: stage.kind(),
        source,
    })
}

/// Build an effect program: a named vertex entry and a named fragment entry,
/// each expanded with its own stage's interface.
pub fn expand_pair(
    library: &impl Includes,
    vertex_entry: &str,
    fragment_entry: &str,
    context: &TemplateContext,
) -> Result<ProgramText, TemplateError> {
    let vertex = expand_stage(library, vertex_entry, context, ShaderStage::Vertex)?.source;
    let fragment = expand_stage(library, fragment_entry, context, ShaderStage::Fragment)?.source;
    Ok(ProgramText { vertex, fragment })
}

/// Text bytes as the builder copies them: one character per byte.
fn push_bytes(out: &mut String, bytes: &[u8]) {
    out.extend(bytes.iter().map(|&b| b as char));
}

fn expand_into(
    library: &impl Includes,
    source: &[u8],
    context: &mut TemplateContext,
    out: &mut String,
) {
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
        push_bytes(out, &source[copy_from..i]);
        let next = match marker {
            b'%' => command(source, i + 2, context, out),
            b'?' => conditional(source, i + 2, context),
            _ => include(library, source, i + 2, context, out),
        };
        i = skip_blanks(source, next.min(source.len()));
        copy_from = i;
    }
    push_bytes(out, &source[copy_from..]);
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

/// `{$name}`: an include, `{$Interface}` (any name starting with it: the
/// builder compares nine characters), or nothing when the library has no
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
    let name = &source[start.min(end)..end];
    if name.starts_with(b"Interface") {
        match context.interface {
            InterfaceMode::None => {}
            InterfaceMode::Vertex => out.push_str(&interface_vertex(&context.flags)),
            InterfaceMode::Fragment => out.push_str(&interface_fragment(context)),
            // The geometry stage expands no generated interface: the
            // builder's `{$Interface}` branch only serves the vertex and
            // fragment stages.
            InterfaceMode::Geometry => {}
        }
    } else {
        let name: String = name.iter().map(|&b| b as char).collect();
        if let Some(entry) = library.source(&name) {
            expand_into(library, entry.as_bytes(), context, out);
        } else {
            context.unknown_includes.push(name);
        }
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
            if frame.current == frame.count {
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
        out.push('.');
        if let Some(&channel) = usize::try_from(index).ok().and_then(|i| b"xyzw".get(i)) {
            out.push(channel as char);
        }
        return cursor;
    }

    match expression(source, start, percent + 1, context) {
        Some(Value::Integer(value)) => push_integer(out, value),
        Some(Value::Fraction(value)) => out.push_str(&format_fraction(value)),
        None => {}
    }
    cursor
}

/// Parse the `LOOP` marker: its count, optional variable and optional
/// initial value. Returns `Some(position)` when the loop must be skipped
/// (fewer than one pass) and `None` after pushing its frame.
fn loop_start(
    source: &[u8],
    start: usize,
    percent: usize,
    cursor: usize,
    context: &mut TemplateContext,
) -> Option<usize> {
    let byte = |at: usize| source.get(at).copied().unwrap_or(0);
    let mut p = start + 5;
    // The name or number runs to a blank, an operator or the marker's end.
    while p < percent && !matches!(byte(p), b' ' | b'*' | b'+' | b'-' | b'[') {
        p += 1;
    }
    let mut count;
    if byte(start + 5).is_ascii_digit() {
        count = i32::from(byte(start + 5) - b'0');
        p = start + 6;
        while byte(p).is_ascii_digit() {
            count = count
                .wrapping_mul(10)
                .wrapping_add(i32::from(byte(p) - b'0'));
            p += 1;
        }
    } else {
        count = context
            .number(source.get(start + 5..p).unwrap_or(&[]))
            .unwrap_or(0);
        if byte(p) == b'-' {
            // The builder leaves its cursor on the digit it subtracts.
            p += 1;
            count = count.wrapping_sub(i32::from(byte(p)).wrapping_sub(i32::from(b'0')));
        }
    }
    let mut variable = 0u8;
    let mut initial = 0i32;
    let mut q = p + 1;
    if q < percent {
        variable = byte(q);
        q = p + 3;
    }
    if q < percent && byte(q).is_ascii_digit() {
        initial = i32::from(byte(q) - b'0');
    }
    if i64::from(count) - i64::from(initial) < 1 {
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
                        return (i + 7).min(source.len());
                    }
                    level -= 1;
                }
                _ => {}
            }
        }
        i += 1;
    }
    start
}

/// Evaluate one condition term and return `(value, position after it)`.
///
/// The returned position is the builder's cursor: one past the term's `?`
/// (or blank), past a comparison's digit, or past a `Shd.`/`Env.`
/// comparison found further on.
fn term(source: &[u8], pos: usize, context: &TemplateContext) -> (bool, usize) {
    let len = source.len();
    let byte = |at: usize| source.get(at).copied().unwrap_or(0);
    let mut i = pos;
    let mut name_len = 0usize;
    let mut member = 0usize;
    let mut bracket = 0i32;
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
                bracket = parse_number(source, i + 1, 1, context);
                i += 2;
            }
            b'.' => {
                member = i + 1;
                if name_len == 0 {
                    name_len = i - pos;
                }
            }
            _ => {}
        }
        i += 1;
    }
    let after = i + 1;
    if pos >= len {
        return (false, after);
    }
    let name = &source[pos..(pos + name_len).min(len)];
    // A digit the comparisons read, as the byte minus '0'.
    let digit = |at: usize| i32::from(byte(at)) - i32::from(b'0');
    if member == 0 {
        let Some(variable) = TemplateContext::variable(name) else {
            return (false, after);
        };
        let value = context.value(variable);
        return match byte(after) {
            b'>' => (value > digit(after + 2), after + 4),
            b'=' => (value == digit(after + 2), after + 4),
            _ => (context.truth(variable), after),
        };
    }
    let member_name = &source[member.min(len)..i.min(len)];
    let prefix = source.get(pos..pos + 4).unwrap_or(&[]);
    let (table, word, compares) = match prefix {
        b"Mat." => (MATERIAL_FLAGS, context.flags.material, false),
        b"Shd." => (SHADER_FLAGS, context.flags.shader, true),
        b"Obj." => (OBJECT_FLAGS, context.flags.object, false),
        b"Env." => (ENVIRONMENT_FLAGS, context.flags.environment, true),
        _ => {
            let value = name == b"Light" && context.light_member(member_name, bracket);
            return (value, after);
        }
    };
    let Some(def) = table.iter().find(|d| d.name.as_bytes() == member_name) else {
        return (false, after);
    };
    let field = (word & def.mask) >> def.shift;
    if compares && i < len {
        // The builder looks for `=` from the term's end up to the marker's
        // `}` and compares the field with the digit two places on.
        let mut r = i;
        loop {
            match byte(r) {
                b'=' => return (field == digit(r + 2) as u32, r + 4),
                b'}' => break,
                _ => {}
            }
            if r + 1 >= len {
                break;
            }
            r += 1;
        }
    }
    // A plain term: the field, as a signed integer, is at least one.
    ((field as i32) > 0, after)
}

/// The builder's condition chain: one term (negated by a leading `!`), then
/// `&` joins. Each `&` term is read without its `!`; a false one ends the
/// chain false, a true one is read again (with its `!`) as the chain's
/// value, so only the last term counts.
fn chain(source: &[u8], pos: usize, context: &TemplateContext) -> (bool, usize) {
    let byte = |at: usize| source.get(at).copied().unwrap_or(0);
    let mut at = pos;
    loop {
        let (mut value, mut cursor) = if byte(at) == b'!' {
            let (v, c) = term(source, at + 1, context);
            (!v, c)
        } else {
            term(source, at, context)
        };
        let mut q = cursor;
        while byte(q) == b' ' {
            q += 1;
        }
        if byte(q) == b'}' {
            return (value, q + 1);
        }
        if byte(q) == b'&' {
            q += 2;
            (value, cursor) = term(source, q, context);
        }
        if !value || q >= source.len() {
            return (false, cursor);
        }
        at = q;
    }
}

/// Handle `{?…` at `pos`, returning the source position to continue from.
fn conditional(source: &[u8], pos: usize, context: &mut TemplateContext) -> usize {
    let head = source.get(pos..pos + 4);
    if head.is_some_and(|h| h == b"END?") {
        return pos + 5;
    }
    if head.is_some_and(|h| h == b"ELSE") {
        // Inside the taken branch: jump over the alternate branch to its
        // matching end.
        return skip_to_end(source, pos + 5);
    }
    let (value, after) = chain(source, pos, context);
    if value {
        return after;
    }
    find_else_or_end(source, after, context)
}

/// Skip a conditional's alternate branch up to and including its matching
/// `{?END?}`. Nested conditionals are counted; every `ELSE` is ignored.
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
    start
}

/// Find the alternate branch of a false conditional, or its end: a
/// `{?ELSE?}` or an `{?ELSE term?}` whose single term holds (no `!`, no `&`)
/// at the conditional's own level.
fn find_else_or_end(source: &[u8], start: usize, context: &TemplateContext) -> usize {
    let mut level = 0u32;
    let mut i = start;
    while i + 1 < source.len() {
        if source[i] == b'{' && source[i + 1] == b'?' {
            match source.get(i + 2..i + 6) {
                Some(head) if head == b"ELSE" => {
                    if level == 0 {
                        if source.get(i + 6) == Some(&b'?') {
                            return (i + 8).min(source.len());
                        }
                        let (value, mut after) = term(source, i + 7, context);
                        if source.get(after) == Some(&b'}') {
                            after += 1;
                        }
                        if value {
                            return after.min(source.len());
                        }
                    }
                }
                Some(head) if head == b"END?" => {
                    if level == 0 {
                        return (i + 7).min(source.len());
                    }
                    level -= 1;
                }
                _ => level += 1,
            }
        }
        i += 1;
    }
    start
}

/// Parse one integer token of `length` characters. A leading digit makes it
/// a decimal number whose characters are all taken as digits (the quotient
/// operator asks for a zero-length token and the builder still reads its
/// first digit); otherwise a one-character name is a loop variable first,
/// and any name an integer variable. Unknown names are 0.
fn parse_number(source: &[u8], start: usize, length: usize, context: &TemplateContext) -> i32 {
    let Some(&first) = source.get(start) else {
        return 0;
    };
    if first.is_ascii_digit() {
        let mut value = i32::from(first - b'0');
        for k in 1..length {
            let c = source.get(start + k).copied().unwrap_or(0);
            value = i32::from(c)
                .wrapping_add(value.wrapping_mul(10))
                .wrapping_sub(i32::from(b'0'));
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

/// An expression's result: an integer when within 0.0001 of its truncation,
/// otherwise the single-precision value.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Value {
    Integer(i32),
    Fraction(f32),
}

/// x86 `cvttss2si` into 64 bits: truncation, with the "integer indefinite"
/// value for NaN and out-of-range inputs.
fn truncate64(value: f32) -> i64 {
    if value.is_nan() || !(-9.223_372e18..9.223_372e18).contains(&value) {
        i64::MIN
    } else {
        value as i64
    }
}

/// x86 `cvttss2si` into 32 bits.
fn truncate32(value: f32) -> i32 {
    if value.is_nan() || !(-2.147_483_6e9..2.147_483_6e9).contains(&value) {
        i32::MIN
    } else {
        value as i32
    }
}

/// x86 `cvtss2si` into 64 bits under the default rounding (to nearest,
/// ties to even).
fn round64(value: f32) -> i64 {
    if value.is_nan() || !(-9.223_372e18..9.223_372e18).contains(&value) {
        i64::MIN
    } else {
        value.round_ties_even() as i64
    }
}

/// The builder's expression evaluator: tokens separated by `* + - /`,
/// applied strictly left to right in single precision (no precedence),
/// ending at a blank or the `%`. `None` when the text has neither (the
/// builder then prints whatever its result register held).
fn expression(source: &[u8], start: usize, end: usize, context: &TemplateContext) -> Option<Value> {
    let mut accumulator = 0.0f32;
    let mut operation = 0u8;
    let mut token = start;
    let mut i = start;
    while i <= end {
        let c = source.get(i).copied().unwrap_or(0);
        let token_end = match c {
            b' ' | b'%' | b'*' | b'+' | b'-' => i,
            // The quotient's left token is read one character short.
            b'/' => i.saturating_sub(1),
            _ => {
                i += 1;
                continue;
            }
        };
        let value = parse_number(source, token, token_end.saturating_sub(token), context) as f32;
        accumulator = match operation {
            0 => value,
            b'*' => accumulator * value,
            b'+' => accumulator + value,
            b'-' => accumulator - value,
            _ => accumulator / value,
        };
        if c == b' ' || c == b'%' {
            let whole = truncate64(accumulator) as i32;
            return Some(if (accumulator - whole as f32).abs() <= 0.0001 {
                Value::Integer(whole)
            } else {
                Value::Fraction(accumulator)
            });
        }
        operation = c;
        token = i + 1;
        i += 1;
    }
    None
}

/// The builder's integer printing: one digit below 10, two below 100, three
/// otherwise, each as `'0'` plus the digit's value (so a negative value or
/// one of 1000 and more prints the characters that arithmetic gives).
fn push_integer(out: &mut String, value: i32) {
    let digit = |d: i32| char::from((i32::from(b'0') + d) as u8);
    if value < 10 {
        out.push(digit(value));
    } else if value < 100 {
        out.push(digit(value / 10));
        out.push(digit(value % 10));
    } else {
        out.push(digit(value / 100));
        out.push(digit(value % 100 / 10));
        out.push(digit(value % 10));
    }
}

/// The engine's fixed three-decimal float printing with trailing zeros
/// trimmed (one kept after the point): the decimals are cut from the
/// fraction digit by digit in single precision, the last one rounded, a
/// carry rippling back into the integer part. `1/4` prints `0.25`.
fn format_fraction(value: f32) -> String {
    const DECIMALS: usize = 3;
    let negative = value.is_sign_negative();
    let magnitude = value.abs();
    let mut digits = vec![b'.'];
    let mut f = (magnitude - truncate32(magnitude) as f32) * 10.0;
    for _ in 1..DECIMALS {
        let d = truncate64(f) as i32;
        digits.push((i32::from(b'0') + d) as u8);
        f = (f - d as f32) * 10.0;
    }
    let last = round64(f) as i32;
    let mut whole = truncate64(value) as i32;
    if last < 10 {
        digits.push((i32::from(b'0') + last) as u8);
    } else {
        let mut n = DECIMALS;
        while digits[n - 1] == b'9' {
            digits[n - 1] = b'0';
            n -= 1;
        }
        if n < 2 {
            whole = if negative {
                whole.wrapping_sub(1)
            } else {
                whole.wrapping_add(1)
            };
        } else {
            digits[n - 1] += 1;
        }
        digits.push(b'0');
    }
    let mut keep = digits.len();
    while keep > 0 && digits[keep - 1] == b'0' {
        keep -= 1;
    }
    if keep > 0 && digits[keep - 1] == b'.' {
        keep += 1;
    }
    digits.truncate(keep);
    let mut text = if whole == 0 {
        if negative { "-0" } else { "0" }.to_string()
    } else {
        whole.to_string()
    };
    text.extend(digits.iter().map(|&b| b as char));
    text
}

/// The formatted text of one `{%…%}` expression, without its marker, for
/// tests and diagnostics.
pub fn expression_text(source: &str, context: &TemplateContext) -> String {
    let text = format!("{source}%");
    let mut out = String::new();
    match expression(text.as_bytes(), 0, source.len(), context) {
        Some(Value::Integer(value)) => push_integer(&mut out, value),
        Some(Value::Fraction(value)) => out.push_str(&format_fraction(value)),
        None => {}
    }
    out
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

/// One generated declaration: the text, then the index when the builder
/// passes one (one or two digits), then `;` and CR LF.
fn declare_indexed(out: &mut String, text: &str, index: Option<u32>) {
    out.push_str(text);
    match index {
        Some(i) if i < 10 => out.push(char::from(b'0' + i as u8)),
        Some(i) => {
            out.push(char::from(b'0'.wrapping_add((i / 10) as u8)));
            out.push(char::from(b'0' + (i % 10) as u8));
        }
        None => {}
    }
    out.push_str(";\r\n");
}

fn declare(out: &mut String, text: &str) {
    declare_indexed(out, text, None);
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
    if numbers.lights as i32 > 0 {
        // Only the first slot can hold the sun's 2D shadow map.
        if context.kind_of(0).slot() & 0x300 == 0x200 {
            declare(&mut out, "in vec3 vrSdw");
            declare_indexed(&mut out, "uniform sampler2DShadow Shadow", Some(0));
        } else {
            declare_indexed(&mut out, "uniform samplerCubeShadow Shadow", Some(0));
        }
        for i in 1..numbers.lights {
            declare_indexed(&mut out, "uniform samplerCubeShadow Shadow", Some(i));
        }
    }
    if flags.material & 4 != 0 {
        declare(&mut out, "uniform sampler2D FrmBuf");
    }
    let taps = if flags.object & 0x1000 != 0 {
        declare(&mut out, "uniform int PassIdx");
        (numbers.shadow_samples as i32).wrapping_shl(2)
    } else {
        numbers.shadow_samples as i32
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
                // ShadeQual 2 (quality 5).
                shader: 0x20_0000,
                object: 0,
                environment: 0x10,
            },
            Numbers {
                lights: 2,
                shadow_samples: 2,
                displacement_samples: 4,
                supersample: 0,
            },
        )
    }

    fn eval(text: &str, context: &TemplateContext) -> bool {
        let source = format!("{text}?}}");
        chain(source.as_bytes(), 0, context).0
    }

    fn with_loop(mut c: TemplateContext, variable: u8, current: i32) -> TemplateContext {
        c.loops.push(LoopFrame {
            body: 0,
            count: current + 1,
            current,
            variable,
        });
        c
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
        assert_eq!(MATERIAL_FLAGS.len(), 16);
        assert_eq!(SHADER_FLAGS.len(), 9);
        assert_eq!(OBJECT_FLAGS.len(), 3);
        assert_eq!(ENVIRONMENT_FLAGS.len(), 7);
    }

    #[test]
    fn condition_terms_read_flags_and_fields() {
        let mut c = context();
        c.flags.shader |= 0x4000 | 0x40 | 0x2;
        assert!(eval("Mat.MatAmbDif", &c));
        assert!(eval("Mat.TexDif", &c));
        assert!(!eval("Mat.TexEmi", &c));
        assert!(eval("Shd.DirtMode = 4", &c));
        assert!(!eval("Shd.DirtMode = 2", &c));
        assert!(eval("Shd.BlendMode = 4", &c));
        assert!(eval("Lights > 1", &c));
        assert!(!eval("Lights > 2", &c));
        assert!(eval("Lights = 2", &c));
        assert!(eval("ShadeQual > 1", &c));
        assert!(!eval("ShadeQual > 2", &c));
        assert!(eval("Env.VxlGI", &c));
        assert!(!eval("Obj.VrtCol", &c));
        assert!(!eval("Unknown.Thing", &c));
        // Shd.QuadRender's name is registered with no length: never true.
        c.flags.shader |= 0x1000;
        assert!(!eval("Shd.QuadRender", &c));
    }

    #[test]
    fn material_and_object_terms_know_no_comparison() {
        let mut c = context();
        c.flags.object = 0x200000;
        // The `= n` after a material or object term is not read by the
        // term; the chain reads it as a further term, which fails.
        assert!(!eval("Obj.VrtCol = 0", &c));
        assert!(!eval("Obj.VrtCol = 1", &c));
        assert!(!eval("Mat.MatAmbDif = 3", &c));
        // The environment and shader words compare.
        c.flags.environment = 0x2;
        assert!(eval("Env.FogMode = 2", &c));
        assert!(!eval("Env.FogMode = 1", &c));
    }

    #[test]
    fn a_shader_term_finds_its_comparison_further_on() {
        // The builder looks for `=` up to the marker's `}`: the first term
        // of an `&` chain takes the second term's comparison.
        let mut c = context();
        c.flags.shader = 0x20 | 0x100;
        let text = "Shd.DispSdw & Shd.DirtMode = 2?}";
        let (value, cursor) = term(text.as_bytes(), 0, &c);
        // DispSdw (0x100 >> 0 = 256) is not 2.
        assert!(!value);
        assert_eq!(&text.as_bytes()[cursor..], b"}");
    }

    #[test]
    fn bare_variables_use_their_truth_callbacks() {
        let mut c = context();
        // Lights and ShadeQual have none.
        assert!(!eval("Lights", &c));
        assert!(!eval("ShadeQual", &c));
        assert!(eval("DspSdws", &c));
        c.numbers.displacement_samples = 0;
        assert!(!eval("DspSdws", &c));
        // SdwSmps: more than one tap, or super-sampling on.
        assert!(eval("SdwSmps", &c));
        c.numbers.shadow_samples = 1;
        assert!(!eval("SdwSmps", &c));
        c.numbers.supersample = 3;
        assert!(eval("SdwSmps", &c));
    }

    #[test]
    fn light_members_need_a_light_in_the_slot() {
        let mut c = context();
        c.lights = vec![LightKind::Point, LightKind::Directional];
        assert!(!eval("Light[0].Direct", &c));
        assert!(eval("Light[1].Direct", &c));
        assert!(eval("Light[0].Sky", &c));
        assert!(!eval("Light[1].Sky", &c));
        assert!(!eval("Light[0].Other", &c));
        // A directional record past `Lights` is not seen.
        c.numbers.lights = 1;
        assert!(!eval("Light[1].Direct", &c));
        c.numbers.lights = 0;
        assert!(!eval("Light[0].Sky", &c));
        // A loop variable indexes the slot.
        c.numbers.lights = 2;
        assert!(eval("Light[i].Direct", &with_loop(c, b'i', 1)));
    }

    #[test]
    fn and_chain_keeps_only_the_last_term() {
        let mut c = context();
        c.numbers.lights = 1;
        c.lights = vec![LightKind::Directional];
        // The false first term is forgotten.
        assert!(eval("Lights > 2 & Light[0].Direct", &c));
        c.lights = vec![LightKind::Point];
        assert!(!eval("Lights > 0 & Light[0].Direct", &c));
        // A `!` before an `&` term is not honoured on its first reading.
        assert!(!eval("Mat.TexDif & !Mat.TexEmi", &c));
    }

    #[test]
    fn expression_arithmetic_is_left_to_right() {
        let c = context();
        assert_eq!(expression_text("Lights*3+2", &c), "8");
        assert_eq!(expression_text("Lights*2+1", &c), "5");
        assert_eq!(expression_text("2+Lights*3", &c), "12");
        assert_eq!(expression_text("1/DspSdws", &c), "0.25");
        assert_eq!(expression_text("SdwSmps+1", &c), "3");
        assert_eq!(expression_text("7", &c), "7");
        assert_eq!(expression_text("0", &c), "0");
        assert_eq!(expression_text("40*3+45", &c), "165");
    }

    #[test]
    fn numbers_print_as_the_builder_prints_them() {
        let c = context();
        // A negative result is '0' plus its value.
        assert_eq!(expression_text("Lights-3", &c), "/");
        // Three decimals, trailing zeros trimmed, the last one rounded to
        // even.
        assert_eq!(expression_text("1/3", &c), "0.333");
        assert_eq!(expression_text("2/3", &c), "0.667");
        assert_eq!(expression_text("1/8", &c), "0.125");
        assert_eq!(expression_text("1/16", &c), "0.062");
        assert_eq!(expression_text("5/2", &c), "2.5");
        assert_eq!(expression_text("0-1/2", &c), "-0.5");
        // A zero divisor prints what the conversions make of infinity.
        assert_eq!(expression_text("1/0", &c), "0.0");
        assert_eq!(format_fraction(0.9996), "1.0");
        assert_eq!(format_fraction(2.9999), "3.0");
    }

    #[test]
    fn loop_variables_hold_their_current_value() {
        let c = with_loop(context(), b'i', 3);
        assert_eq!(expression_text("i*3+1", &c), "10");
        assert_eq!(expression_text("i", &c), "3");
    }

    #[test]
    fn loops_repeat_and_end_after_the_count() {
        let library = lib(&[("p", "{%LOOP Lights i%}[{%i%}]{%END%}")]);
        let text = expand(&library, "p", &context()).unwrap();
        assert_eq!(text, "#version 150\r\n[0][1]");
    }

    #[test]
    fn loop_with_initial_and_variable() {
        let library = lib(&[("p", "{%LOOP 3 i 1%}{%i%},{%END%}")]);
        let text = expand(&library, "p", &context()).unwrap();
        assert_eq!(text, "#version 150\r\n1,2,");
    }

    #[test]
    fn zero_length_loop_skips_its_body() {
        let library = lib(&[("p", "a{%LOOP DspSdws-4 b%}x{%END%}z")]);
        let text = expand(&library, "p", &context()).unwrap();
        assert_eq!(text, "#version 150\r\naz");
    }

    #[test]
    fn a_subtracted_count_shifts_the_loop_variable() {
        // After `-1` the builder's cursor sits on the digit, so the variable
        // it takes is the blank that follows, and `i` stays unknown (0).
        let library = lib(&[("p", "{%LOOP Lights-1 i%}{%i%}{%END%}")]);
        let mut c = context();
        c.numbers.lights = 3;
        assert_eq!(expand(&library, "p", &c).unwrap(), "#version 150\r\n00");
    }

    #[test]
    fn conditionals_take_else_branches_and_nest() {
        let library = lib(&[(
            "p",
            "{%LOOP Lights i%}{?Light[i].Direct?}d{?ELSE?}p{?END?}{%END%}",
        )]);
        let mut c = context();
        c.lights = vec![LightKind::Point, LightKind::Directional];
        assert_eq!(expand(&library, "p", &c).unwrap(), "#version 150\r\npd");
    }

    #[test]
    fn else_after_taken_branch_is_skipped() {
        let library = lib(&[("p", "{?Mat.MatAmbDif?}yes{?ELSE?}no{?END?}")]);
        assert_eq!(
            expand(&library, "p", &context()).unwrap(),
            "#version 150\r\nyes"
        );
    }

    #[test]
    fn an_else_condition_is_a_single_term() {
        let library = lib(&[
            ("p", "{?Mat.TexEmi?}a{?ELSE !Mat.TexEmi?}b{?END?}c"),
            ("q", "{?Mat.TexEmi?}a{?ELSE Mat.TexDif?}b{?END?}c"),
        ]);
        let c = context();
        assert_eq!(expand(&library, "p", &c).unwrap(), "#version 150\r\nc");
        assert_eq!(expand(&library, "q", &c).unwrap(), "#version 150\r\nbc");
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
    fn the_program_ends_at_its_first_nul() {
        let library = lib(&[("p", "a{$inner}c"), ("inner", "b\0d")]);
        assert_eq!(
            expand(&library, "p", &context()).unwrap(),
            "#version 150\r\nab"
        );
    }

    #[test]
    fn stage_expansion_sets_the_interface_and_the_gl_kind() {
        let library = lib(&[
            ("v", "void main(){}{$Interface}"),
            ("f", "out vec4 C;{$Interface}"),
        ]);
        let c = context();
        let vertex = expand_stage(&library, "v", &c, ShaderStage::Vertex).unwrap();
        assert_eq!(vertex.kind, 0x8B31);
        assert!(vertex.source.contains("in vec3 vNrm") || vertex.source.contains("uniform Global"));
        let fragment = expand_stage(&library, "f", &c, ShaderStage::Fragment).unwrap();
        assert_eq!(fragment.kind, 0x8B30);
        assert!(fragment.source.contains("uniform Global"));
        let geometry = expand_stage(&library, "v", &c, ShaderStage::Geometry).unwrap();
        assert_eq!(geometry.kind, 0x8DD9);
        assert!(!geometry.source.contains("uniform Global"));
        // An effect program pairs a vertex entry with a fragment one.
        let pair = expand_pair(&library, "v", "f", &c).unwrap();
        assert!(pair.vertex.contains("uniform Global"));
        assert!(pair.fragment.contains("uniform Global"));
        // A missing stage entry is an error.
        assert!(expand_stage(&library, "nope", &c, ShaderStage::Geometry).is_err());
    }

    #[test]
    fn the_geometry_stage_generates_no_interface() {
        let library = lib(&[("geometry", "void main(){}{$Interface}{$Interface}")]);
        let mut c = context();
        c.interface = InterfaceMode::Geometry;
        assert_eq!(
            expand(&library, "geometry", &c).unwrap(),
            "#version 150\r\nvoid main(){}"
        );
        // The vertex and fragment stages still expand theirs.
        c.interface = InterfaceMode::Vertex;
        let text = expand(&library, "geometry", &c).unwrap();
        assert!(text.contains("uniform Global"));
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
    fn interface_fragment_uses_a_2d_shadow_for_a_directional_first_light() {
        let mut c = context();
        c.numbers.lights = 1;
        c.lights = vec![LightKind::Directional];
        let text = interface_fragment(&c);
        assert!(text.contains("in vec3 vrSdw;\r\n"));
        assert!(text.contains("uniform sampler2DShadow Shadow0;\r\n"));
        assert!(!text.contains("samplerCubeShadow"));
    }

    #[test]
    fn whitespace_right_after_a_marker_is_dropped() {
        let library = lib(&[("p", "a {?Mat.MatAmbDif?}\r\n  b {%i%} c")]);
        let c = with_loop(context(), b'i', 0);
        assert_eq!(expand(&library, "p", &c).unwrap(), "#version 150\r\na b 0c");
    }

    #[test]
    fn malformed_markers_do_not_panic() {
        let c = context();
        for source in [
            "{%",
            "{%LOOP",
            "{?Mat",
            "{?",
            "{$",
            "{%LOOP x",
            "{%1/",
            "{?ELSE",
            "{?Shd.X =",
            "{?A & ",
            "{?Light[",
            "{%LOOP 9 i 9",
            "{%.",
        ] {
            let library = lib(&[("p", source)]);
            let _ = expand(&library, "p", &c);
        }
    }

    #[test]
    fn version_is_only_prepended_without_a_hash() {
        let library = lib(&[("p", "#version 330\r\nx")]);
        assert_eq!(
            expand(&library, "p", &context()).unwrap(),
            "#version 330\r\nx"
        );
    }

    #[test]
    fn missing_entry_is_an_error() {
        let library = lib(&[]);
        assert_eq!(
            expand(&library, "nope", &context()),
            Err(TemplateError::MissingEntry("nope".into()))
        );
    }

    #[test]
    fn the_dot_command_swizzles_a_loop_variable() {
        let library = lib(&[("p", "{%LOOP 3 i%}{%.i%}{%END%}")]);
        assert_eq!(
            expand(&library, "p", &context()).unwrap(),
            "#version 150\r\n.x.y.z"
        );
    }

    #[test]
    fn surface_program_uses_shading_v_for_every_set() {
        let library = lib(&[("shading_v", "vertex"), ("shadeopt_f", "fragment")]);
        let text = expand_surface(&library, "shadeopt", &context()).unwrap();
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
    fn shadeopt_unrolls_its_lights() {
        let Some(library) = library() else { return };
        let mut c = context();
        c.interface = InterfaceMode::Fragment;
        let text = expand_with(&library, "shadeopt_f", &mut c).unwrap();
        // Three lights: the array holds three vectors per light and the
        // loop unrolls with the indices written out.
        assert!(text.contains("Light[9]"));
        assert!(text.contains("Light[6]"));
        assert!(text.contains("Light[8]"));
        assert!(!text.contains("Light[9]."));
        assert!(!text.contains("{%"));
    }
}
