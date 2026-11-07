# Design

Extends [docs/formats/RFI.md](../../../../docs/formats/RFI.md) (Generated
mip levels) and [docs/UI.md](../../../../docs/UI.md) (Icons).

## Behaviour

**Accepted layouts.** The resample's windowed path and the sharpen take
the six layouts the unit converts to floats (one channel `0x2008`,
`0x4200`; two `0x6208`, `0x8400`; three `0xC600`; four `0xE608`, after the
mask `0x800CFFFF`). The chain test for a power-of-two ratio runs before
the format test, so such a resample still goes through the chain.
Exarust filters decoded RGBA8 in every case; the channels are filtered
independently, so the layout changes which images are accepted, not the
filtered values.

**One-texel axes.** The convolution copies the image when it is one texel
wide (no row pass) and copies the row pass's result when it is one texel
high, so the kernel's f32 sum never touches such an axis.

**Icon add.** An icon of the cell's square goes in untouched. Any other
size loses the flags `0x70000` (both wraps and bit 18), so the resample and
the sharpen clamp; it is resized to the cell, sharpened when the atlas
sharpens, and then placed. Exarust resizes four-channel icons only.

## Unreachable in 0.9.5

The threaded convolution (the pool job and its row and column workers):
its only caller, the image sharpen, passes the threaded flag as false.

## Open questions and how they were answered

- Is the sharpen threaded? No: the call clears the flag register.
- Which words does the sharpen accept? Its setup routine's word switch.
- Do the shipped stored chains come from the same filter? Mostly: the
  corpus test (`generated_levels_against_the_stored_chains`).
