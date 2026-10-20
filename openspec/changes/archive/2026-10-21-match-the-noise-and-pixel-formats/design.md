# Design

## Behaviour

**Noise** (`exa_render::image_noise`). The state steps
`s · 0x08088405 + 1` before each sample. A byte is the high word of
`s · 255`; a float is `s` as a signed integer times 2⁻³².

`value_noise_bytes(w, h, octaves, seed)` fills white bytes. For k in
1..=octaves it blends `add_octave_bytes` with scale 1/2ᵏ, reading the first
(w >> k)·(h >> k) white bytes as the source image. Per texel,
c = trunc((x + ½)·s) and f = x·s − c select the blend half. The value is
(old + sample)·½, clamped to 0 and 255, else rounded to even.
`ridge_bytes` folds v below 128 to round((v/127)²·255) and the rest via
255 − v, in doubles.

`ridged_noise_floats` fills white floats and blends the float octaves at
weight ½ with `ridge(v) = (1 − |1.8v|)⁴` (doubles), from the first
(w/2)·(h/2) samples.

`NoiseField` keeps four planes, white noise and an output. `new` builds
four ridged planes and the white noise from five seeds.
`configure(octaves, period_s, target)` rounds the period to milliseconds
and resets the offset so the phase at the last tick holds. `tick(now)` does
nothing below 16 ms. It recomputes the phase. A new whole part finishes the
building plane, draws fresh white noise, rotates the planes and shifts the
plane amplitudes. A tick that leaves the phase unchanged reports no change.
The amplitude eases (+0.05 or −0.1 per second, snapping within 0.11·dt,
dt ≤ 0.1). Rows up to round(h·f) − 1 are built with
`ridge · add + old · keep`. The output is p2·w2 + (p1·w1 + p0·w0) with
wᵢ = amplitude · ⅔ · sin²((f + 2 − i)·π/3). The optional wrap copies each
row's first value one past the row (stride w + 1) and the first row after
the last, as the original does over a densely written output.

**Pixel formats** (`exa_formats::pixel`). `EIGHT_BIT_FORMATS` and
`FLOAT_FORMATS` hold the 20 ids, fallbacks, names and converter kinds.
`bits_per_pixel` and `pixel_bytes` follow the id's bit fields.
`convert(source, target, src, pixels, dst)` dispatches on the source
format's converter. It returns `Ok(false)` when that converter declines the
target, and an error for an unknown source or a short buffer. Rounding goes
through `round_f32`/`round_f64`: ties to even, NaN or out of range to
`i64::MIN`, keeping the low byte.

**Mips**. `mip_conversion(word)` reproduces the builder's choice.
`build_mip_chain_rgba8(…, conversion)` converts the whole base to floats,
filters every level from them and converts each level back. The texture
loader passes the stored word's conversion, and linear for block formats
decoded on the CPU, which the original never mips itself.

## Open questions

- The sine: Exarust uses the standard library's. The original's own sine
  may differ by an ulp; to be answered by a capture of the field if
  it is ever compared pixel for pixel.
- The order in which the two registration units run is not settled; the
  tables keep each unit's own order.
- Whether the original runs a converter when source and target share a
  layout: the decoder copies RGBA images, which gives the same bytes.

## Docs

`docs/formats/RFI.md` (Pixel-format conversion, Generated mip levels),
`docs/formats/TERRAIN.md` (noise maps), `docs/RENDERING.md` (Procedural
noise).
