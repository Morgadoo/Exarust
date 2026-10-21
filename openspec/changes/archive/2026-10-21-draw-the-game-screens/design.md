# Design

Rules: `docs/UI.md` (The in-game GUI unit, Skin).

## Submission

A `GuiFrame` holds layers in submission order. The rasteriser walks layers
and the primitives inside them; each primitive carries the blend word the
batch held when it was begun. Word 0 (never set) draws as 0x203, the GUI's
steady state, because a vanishing primitive is never what the original
shows. Blending is off when the word under the mask 0x3f0f equals 0x100.
Quads split on a diagonal; a pixel on the shared edge belongs to one
triangle (top-left rule) so a blended quad has no doubled seam.

Mode 0x10 asks the skin's subclass. The line skin answers the flat program;
the HUD's stone skin answers mode 4, sampled in screen space from the
window's bottom. A `subclass` field on the batch carries the answer.

The window image is one CPU canvas shown through an `ImageNode`. A frame
equal to the last keeps the previous image so a still screen costs no raster
or upload.

## Screens

The forms' words come from the constructors: attributes 0x30022 (journal
0x38022, pack 0x130032), frame style 0x3000 (pack 0x80), spacing 8 for the
inventory and 2 for the journal. No shipped constructor sets the title bit,
so no title bar is drawn. Control records are stored as the constructors
write them (class, design-unit place, style, text) and laid out by
`layout_records` inside the client; a negative y or x is an anchor. A
caption is placed by `draw_control_label`, from the style word.

## Not guessed

Where the original's behaviour was not located (the second inventory
record, callbacks, the tab strip, the entry list) nothing is drawn and the
docs say so.
