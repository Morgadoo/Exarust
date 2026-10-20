# Design

## Behaviour

The skin measures are `base = 6u−border`, `close = 4u+1`, `rail = 4u−1` and `tab = u/2+close`; the DPI border is `max(2, trunc(dpi/135))`. The constructor stores 302×402, border 2, content 300×800, a 0.025 scroll step, a 76×90 page minimum and the unset position sentinel.

An unset anchor resolves to 4 px on X and 36 px on Y; centred anchors use half the parent span; far anchors use `parent − size − 4`. The client rectangle starts at the border, takes the parent top (plus the style's extra and the DPI border for the caption styles), subtracts the parent base on the right and the rail plus two borders on the bottom.

The scroll step is `10/overflow` where the overflow is `content_w − width`. The thumb is `trunc(cw/content_w·track)`, at least 9 and at most the track; its x is `border + rail + trunc((track−thumb)·f)`. Setting the fraction clamps to 0..1, snaps to whole steps when the style asks, drops to 0 without overflow, and recomputes the thumb x.

## Model

`exa_ui::form` exposes the types and pure functions; the menu row spacing is deliberately not modelled until a retail screenshot resolves the scale discrepancy.

## Open questions

Row spacing, text advances, key traversal and the settings/manual rows.

## Docs

`TGUIForm` is **12%**. The headline rises slightly.
