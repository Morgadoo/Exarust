# Design

Format page: [docs/formats/FLB.md](../../../docs/formats/FLB.md); rendering
notes: `docs/RENDERING.md` (Shader builder).

## Approach

The builder's state becomes a plain `TemplateContext` (four flag words,
the light count, shadow and displacement taps, the super-sampling setting,
the light slots); expansion is a pure function of the library and the
context, so tests use synthetic libraries and the installed one only when
`EXANIMA_DIR` is set. The factory's rules are a pure function of the
parsed material record and the shading quality. Nothing is wired into the
Bevy pipeline: Exarust's WESL stays the drawn program, corrected where the
generated text shows a difference.

## Decisions

- Faithful quirks are kept where they change text: the `&` chain keeps
  only its last term, `Mat.`/`Obj.` terms have no comparison, a `Shd.`/`Env.`
  term takes the first `=` up to the marker's end, `Shd.QuadRender` never
  matches, negative integers print `'0'` plus their value, a subtracted
  loop count shifts the loop variable.
- Library entries keep embedded NULs; the finished program ends at its
  first NUL, as GL reads it.
- Inside the GI volume the cube reflection follows `Mat.TexRef`: the
  model loader clears the reflection flag of records whose factory state
  drops the reflection layer; the cube stays bound for the ambient.

## Open questions and how they were answered

| question | answer |
|---|---|
| Do `Mat.` terms compare? | No: only `Shd.` and `Env.` have the `=` path (assembly of the condition routine). |
| What does a bare `SdwSmps` test? | Its truth callback: more than one tap or super-sampling on (callback disassembled). |
| How is a fraction printed? | The engine's three-decimal printer (decompiled and its instructions checked: single precision, final digit rounded to nearest). |
| `DspSdws`? | 4 over the super-sampling factor, at least 1, 0 without `DispSdw` (UBO writer and the factor table). |
| Which lights are `Direct`? | Slot word `0x200`, set for lights whose node type bits are clear (light-slot fill). |
| Parallax rounding? | `cvtss2si`/`cvtsd2si` under the default mode: nearest, ties to even (no rounding-mode change on the path). |
| Does `LocalRef` apply? | With a reflection layer, extra flag `0x800` and quality above 4 (factory). |
| The loader's factory method on the feature bits? | Not resolved (a virtual call); taken as the identity [guess]. |
| The factory's quality and extra shader bits? | Taken as the shading-quality setting and zero [guess]. |
