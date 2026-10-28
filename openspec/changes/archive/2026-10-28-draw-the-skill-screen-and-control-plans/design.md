# Design

The draw is a pure function from the screen, the fades and caller-supplied
skin values (its grey, its spoke ink, the atlases' handles and sizes, a
text measure) to a list of batches, glow boxes and text anchors, so a
future Bevy form can submit it through the existing GUI path without the
module knowing the renderer. Each batch names its program (skin solid,
skin textured, the ring program or the bar program), the texture it
samples and the `Colour`/`Prog` uniforms it sets; vertices keep the
original's submission order with each corner's texture coordinate.

The opening scale gates everything after the page-2 background with a
double compare at 0.001 (a NaN passes), as the original does; the spokes
switch to the skin's textured program, so the colour the original builds
for them and never uploads is not modelled. Glows take `scale × (alpha ×
0.6)` in double; text and icons take the plain ink's 120 times their alpha
in single, as the tooltip bodies do.

The extended controls' draw plans name the skin calls' arguments
(rectangles, ink slots, float alphas); the skin primitives that consume
them already live in `exa_ui::skin`.
