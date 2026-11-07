## Why

Component-gap lane for the original's resampling unit and the scene
renderer. The resampling unit's last rating listed the icon manager that
calls the resample and sharpen as missing; the HUD status icon said so in
its own docs ("the atlas's sharpening of a reduced icon is not applied").
A routine-by-routine pass over the unit (REA, assembly) also found two
rules the port did not keep: the float filters refuse all but six pixel
layouts, and the image sharpen copies an axis one texel long instead of
filtering it.

## Evidence

REA and the assembly of the unit's routines: the icon manager's add clears
the image flags `0x70000`, resizes an icon that is not the cell's square
with the resample to an arbitrary size and, when the manager's sharpen
byte is set, sharpens it with amount 0.1 and radius 1, before the format
conversion and the cell placement. The resample's windowed path and the
sharpen's setup accept the word masked by `0x800CFFFF` only when it is
`0x2008`, `0x4200`, `0x6208`, `0x8400`, `0xC600` or `0xE608`, and return
false otherwise. The single-threaded convolution copies the source when
the width is below two and the horizontal result when the height is below
two. The sharpen calls the convolution driver with its threaded flag
cleared (`xor r9b, r9b`), so the 12-thread pool path and its two row and
column workers are unreachable in 0.9.5.

A new corpus test compares Exarust's generated levels with the 65 shipped
uncompressed images that carry their own chain: 15 match bit for bit and
29 within one step on every byte; several others match exactly on every
level but one or two, the levels an artist replaced.

## What Changes

- exa_render: `mipmap::filters_word`; `resample_rgba8_to_size` refuses
  other words on the windowed path; `sharpen_rgba8` leaves them unchanged
  and copies a one-texel axis.
- exa_ui: `IconAtlas::add_resized` (clear the flags, resample to the cell,
  sharpen in a sharpening atlas); the HUD status icon goes through it.
- exa_assets: the stored-chain comparison test.
