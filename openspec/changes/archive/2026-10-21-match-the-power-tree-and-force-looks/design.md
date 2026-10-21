# Design

Documentation: [`docs/THAUMATURGY.md`](../../../docs/THAUMATURGY.md) and
[`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md).

## Float order and rounding

- The effect shader's two texture-matrix callbacks add `layer + frame` to
  0.5 as a single, then multiply. The centred one takes the product, the
  global divisor (a global holding 1, not the layer count), the base and
  the 0.001 in double precision and narrows once; the other works in
  single. The earlier code divided by the layer count.
- The hum start and the phase-streak pitch multiply in double precision,
  then narrow, as the original's converts show.
- The tree compares through the assembly's ordered compares where the
  decompiler wrote min/max, and rounds the node place to the nearest even
  integer.

## Text

A tree's text table holds `last index + 1` strings, one offset each. The
original reads an offset that lies outside the text as an empty string, and
the retail Force tree has such offsets, so the reader leaves the string
empty rather than refusing the file. The nodes name their text by index.

## Drawing

The tree canvas is drawn by a software rasteriser that evaluates the same
six fragment programs the original compiles (their GLSL sources are kept in
`exa_ui::tree_draw`). Each domain draws its icons from its own sheet
(`Texture::Domain`). The frame program reads a mip level 4 of the frames
sheet, built once per draw. The canvas is redrawn at most every 33 ms and
only while the screen is open, so the frame and load time of the game are
untouched while it is closed.

## Wiring

Wired: the P and C keys, the pointer through the GUI task, the canvas, the
bar's slots, the use-key cast (the power's creator is called against a
stand-in power body, and the effect nodes are marker entities).

Not wired, and why: the grant routine and the key handler (nothing in
Exarust calls them yet), the whole veil (it needs the displaced body's live
scene nodes), the Force creators' geometry (the effect passes do not exist
yet). Known differences kept visible in the module docs: the record is
filtered on a copy where the original filters in place, and
`tree::KnownPower` duplicates `thaum::power::KnownPower` (the tree's pairs
are `{power, flags}`, the power classes' `{id, word}`); merging them
touches the whole power lane and waits for the dispatcher.
