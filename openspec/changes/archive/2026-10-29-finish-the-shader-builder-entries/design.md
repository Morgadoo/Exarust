# Design

The new items sit beside the modules that already own the rule: the
callback next to `interface_fragment` (kept separate, since the two differ
on slots after the first), the tap table next to `shadow_sample_upload`,
the wait as its own small module because it belongs to the material
loader rather than the drawing state.

`ViewInputs` names the two time inputs after what the original reads: the
super-sampling loop's sub-frame offset (`0.5 − (i + 0.5)/N`, reset to 0
after the loop) and the frame scale (1, or `16.666…/ms` when the last frame
took longer than 1/60 s, formed in double and stored single). The exposure
stays an ambient and emissive input only.

The taps' third lane has no established reader; it is kept as data. The
`supersample_factor.max(1)` guard predates this change; the original's
factor is never 0 (it is 1 outside the loop).
