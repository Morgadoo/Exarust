# Design

## Context

`exa_ui::skin` already modelled most of the line skin beside the parent
draw skin, including the widget dispatcher (`dispatch_form`). A second,
ordered-batch model of the same dispatcher would have to be kept in step
with the first and disagreed with the assembly in a dozen places, so the
existing model stays and the missing pieces are added to it.

## Decisions

1. One model per original method. The lane's `draw_line_widget`, border,
   rectangle slot, marker, rule, label and detail strip duplicated
   `dispatch_form`, `draw_line_shade`, `draw_span_square`,
   `draw_capped_span`, `place_line_label` and `draw_span_bar`; they are
   removed.
2. The new quads use `SliderQuad` (the first four texture words and integer
   corners) like the dispatcher. Every term the original sums is a whole
   number, so its float sums are exact below 2^24.
3. Conversions follow the instruction: the scroll offset and the refresh's
   slide are 64-bit `CVTSS2SI` results stored as 32-bit words, so a NaN or
   out-of-range product reads as 0.
4. The progress strip takes the skin's first and third side lengths as
   arguments; both callers pass two and four units.

## Open questions

- What the icon manager's two draws do beyond the mirrored texture
  coordinates the evidence shows. Answer with REA on the icon manager.
- How the hit codes map to the window manager's actions. Answer from the
  form code that reads them.

No `docs/formats/` page changes: these are draw rules.
