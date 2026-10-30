# Design

## Light activation

The activation clamps an intensity below the engine's epsilon to 0 (an
ordered compare, so a NaN stays), then on the modern-lighting path takes
the linear colour times the intensity as the direct colour, its Rec. 601
luma (the sum the engine's luma helper makes, `exa_formats::colour::luma`),
the GI colour as that times the multiplier unless the node word has
`0x10000`, a second colour slot zeroed by the node word's `0x20000`, and
the light node's own colour as the linear colour times
`(√I · M) · 3`. The range test is the node word's `0x40000` with a radius
above zero, which `exa_formats::rfc::ranged` already gives as
`Light::ranged`; the falloff is `0.8 · (1/R)` with `1/R` in single
precision and the product in double. Exarust's radius is in metres, so the
coefficient is per metre.

## Alias copy

The sync copies the node word's bits 16–23 into each alias's word (its
other bits kept), the derived direct colour into two slots, the GI and
second colours, the luminance, the falloff, the softness, the node word's
`0x100000` and the reach, plus the world matrix and a frame stamp Exarust's
alias nodes take from their own transform.

## Geometry node

Activation order and gates are the assembly's: the bound reset to zero
and raised per axis with ordered compares, the option word's `0x10` set
then cleared by any part material with a class, the cast bit gated by the
external-draw bit, the `0x1000000` drop under a non-geometry parent, the
threshold, and the squared-bound test summed `(x² + y²) + z²`. The
separate hand-off overwrites the new master's slot with the list's last
entry and drops the last slot.

## Kd builder

The reset's flag is a discard flag: a push clears it, a build finished
under it is not handed back. There is no refusal and no resume.

## Smoke

Each pass draws the mesh once per sub-view the pass draws, the level's
first-pass count (`SubViewPass::subviews`).
