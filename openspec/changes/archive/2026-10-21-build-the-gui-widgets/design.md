# Design

## Shape

Each widget is plain state with methods that take the original's inputs
(client coordinates, pointer and key words, the clipboard) and return
its result codes and events; draws return positions and quads, not GL
calls. Text is bytes, positions inside a text 1-based, a caret the
count of bytes before it, as in the original's strings. Float-to-integer
roundings go through one helper that rounds to nearest-even through a
64-bit integer and keeps its low 32 bits (non-numbers give 0).

## Fonts and levels

The six fonts are tables with their recorded quad size, centring
height, slant, gap and line height, and the advances after the
start-up pass that adds the gap to every stored width. A level clamps to
0–5 and gives the unit (3, 4, 5, 6, 8, 12), the main and small fonts and
the two atlases' cells (16, 20, 28, 32, 42, 64 and 12, 16, 20, 24, 32,
48).

## Quirks kept

- A tag closed by the text's last byte is cut a byte short, and the `]`
  draws.
- Right from before `\r\n` stops between the two bytes.
- Enter over a selection grows the text by the length before the cut;
  the bytes that come back are the old storage (a guess that the
  allocator resizes in place; fresh bytes are zero).
- The pointer's caret uses the font's line height, not the box's pitch.
- A check-mode press on the selected item view entry sets every flag bit
  but the inverted mark.
- An item view column hit measures a name as bytes times the gap.
- A check box smaller than its 4-unit tick places it far off (unsigned
  halving).

## Deviations

- A tag opened in a text's last two bytes ends at the text's end (the
  original reads on past it).
- A picture the lookup does not know has no size (the original would
  load a texture).
- The colour stack holds eight entries and clamps its index.
- Backspace over a lone line feed at the start does not write before the
  text.
- The icon atlas takes icons at its cell size; resizing is the image
  library's.

## Open questions

- What the 0x1000 pointer bit means besides leaving an edit (named
  `STRICT`). Answer: the GUI's pointer dispatch (`RF_GUI_Main`), by REA.
- Which blend modes 0x201 and 0x203 are. Answer: the renderer's blend
  switch, by REA, when the draw bodies are rebuilt.

## Docs

`docs/UI.md`: Fonts (the recorded advances), Text boxes, Icons, Tables,
Item views, Controls. The `exa_ui` README lists the modules.
