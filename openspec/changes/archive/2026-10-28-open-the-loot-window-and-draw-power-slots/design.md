# Design

## Corrections to the draft

- **Selector rows.** The original writes the technique byte − 1 into the
  rows without a bound, so a byte of 9 marks the ninth (forget) row; the
  draft had dropped it. A byte past 9 would write past the form's records
  and is not reproduced.
- **Power slot draw.** Kind 2 takes the same bright ink as the selected
  bit; a hovered filled slot sets no ink and goes through the full-scale
  draw; an empty slot draws its kind icon for kind 4, or for a kind past 2
  only under the pointer. The draft's 16-pixel empty icon had no source;
  the shadow's radius is two GUI units with alpha 0x40.
- **Preview build.** The callback always answers done; when an open
  inventory shows the view and the view holds a character, the inventory's
  camera becomes ten times the character's height. The draft answered
  whether a model existed.
- **Preview bars.** Each frame and its shadow span 2 pixels past the fill
  on both sides and 4 units − 2 down; the draft gave a 4 units − 4 height
  with no source.
- **Control message.** The control's handler takes three values after the
  record (the code and two words), not four.
- **Deferred close order.** The original walks the GUI's forms, so the
  windows close in window order, not the bodies' order.

## Kept as drafted (checked)

The loot window's constructor (title "Body", 244 × 440, x 400, y unset,
pack 228 × 180 at the client's bottom, equipment panel ending four units
above it), its show rule (refused for a flag word whose second byte is 3,
matched by tag and body), the open-container id with 0x20000000, the
right-release close (click release on the right button), the HUD reset
(hold count only from a fallen curtain, heartbeat fade when running), the
status hide clock (delay plus clock), the glow radius 8, share 0.6 (a
double multiply) and tint (140, 160, 255), the preview's 0.15 and 0.05,
the 0x1000 and 0x42 words, the six-word appearance copy and the 100-tick
build delay, and the form tree's per-axis place rules.
