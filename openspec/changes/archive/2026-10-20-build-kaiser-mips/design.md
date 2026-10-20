# Design

## Behaviour

The table of level L (1..13) has `n = 8 << (L−1)` taps, `h = n/2` and `c = 2^−L`. For tap i, `u = i − h + 0.5`, `t = u/h`; with `r = 1 − t²`, a tap with `r ≥ 0` gets `I0(4√r)/I0(4) · sinc(πcu)`, otherwise zero. The weights are divided by their sum, then narrowed to f32.

The axis pass uses `ratio = src/dst` (integer), `start = −((n − ratio) >> 1)`, and for output x the first tap is `start + x·ratio`. Each tap wraps modulo the source extent with `rem_euclid` or clamps to `[0, src−1]`; the four interleaved channels accumulate in f32.

## Model

`bessel_i0`, `sinc`, `kaiser_table(level)`, `mip_axis_start(taps, ratio)`, `resample_axis(src, src_extent, dst_extent, weights, wrap, out)`.

## Open questions

The format converters, the upload path and the NPOT fallback are separate.

## Docs

`RFP_Mipmaps_Kaiser` is **20%**. The headline rises slightly.
