# Design

Extends [docs/formats/RFI.md](../../../docs/formats/RFI.md) (Generated mip
levels), [docs/formats/RFC.md](../../../docs/formats/RFC.md) (normals) and
[docs/RENDERING.md](../../../docs/RENDERING.md) (Surfaces).

## Behaviour

**Banks.** `MipKernel::for_format`: the uniform bank when
`word & 0x80010000 == 0x10000`, else Kaiser. `ChainOptions::for_image`
reads the format word (space, bank) and the header flags (wrap per axis,
renormalisation) and `build_mip_chain_rgba8_with` runs the chain. The
texture loader passes the header flags, which the original's image record
holds at load (the loader merges them into its word from bit 19 up, but
the chain reads the image's own).

**Float order.** Kaiser: half and tap position in f32, `t`, the window and
the sinc in f64, `u·π` then `·c`, each weight stored as f32, the sum of the
stored weights in f64 and each weight divided by it. Window kernel: the
assembly multiplies the polynomial by `((1 − t)(1 − t))(1 − t)`; the radius
divided by the scale goes through the engine's ceiling. Axis passes:
`dst + src·w` per channel in tap order, starting from zero.

**Renormalisation.** The engine's normaliser sums `(x² + z²) + y²`,
multiplies by the reciprocal square root estimate and masks a zero sum to
a zero factor; Exarust uses the same estimate (x86_64 `rsqrtss`).

**Normals.** Face normals by the estimate; the per-key sum normalised with
the exact root (the estimate scales the length only). A key with no usable
face keeps +Y where the original leaves zero (labelled deviation).

**Tangents.** The handedness test uses the accumulated tangent before the
projection; `< 0` gives 1, anything else (a NaN too) −1.

**Bounds.** `minps`/`maxps`, half extents `high − centre` floored at
1e-4 cm by the box helper and at 0.001 cm by the refit, `maxss` over the
squared distances (summed x, y, then z), square-rooted above `0x33FFFFD7`
cm², kept squared otherwise.

## Open questions and how they were answered

- Which bank does the chain use for data images? The chain driver's
  format-word test and the unit initialiser (REA assembly).
- Does the chain wrap by the image or the texture word? The loader passes
  the image record; the passes test its flag word (REA on the loader).
- Does a zero vector become NaN when renormalised? No: the normaliser masks
  a zero sum (disassembly of the helper).
- Is the corner weight a sine? No: `sqrtss(1 − cos)` (assembly).
- How many shipped images and faces do the corrected rules touch? Corpus
  counts in the real-data tests.

## Load time and picture

The corpus conversion of all shipped images (release, worker) took 8.7 s
and 10.0 s with this change against 13.4 s and 20.0 s before it (noisy, I/O
bound; the uniform bank is a quarter of the Kaiser taps). The picture
changes where the original differs: 33 data images get box-filtered mips,
58 images clamp an axis, two-channel images are no longer renormalised,
model normals and tangents follow the original's weights and sign.
