# Design

The GUI draw path rasterises on the CPU, so each shader runs per fragment
into an owned image (`GuiTexture`) that a textured quad layer shows. The
shader bodies keep the GLSL operation order; images sample with `v` running
up as the original's texture coordinates do.

The divination frame walks `draw_order`'s steps: a `Turn` both sets the
card's matrix and draws its shadow, as the paint calls the shadow mesh with
the turned matrix; `Back` and `Face` pass the light's turned z as
`par0.z` (the matrix's last diagonal term, summed `(0·m6 + 0·m7) + 1·m8`
as the engine's vector-matrix product does). The turn matrix reuses the
shared engine routine rather than a second copy.

The destination screen's two symbol colours come from the skinned form in
the original; Exarust passes the line skin's text and highlight colours
(labelled guess) through `SelectionPalette { symbol_base, symbol_add }`.

Screens push their frames into `GuiFrames`; the renderer takes and clears
the list each frame, so no screen clears it.
