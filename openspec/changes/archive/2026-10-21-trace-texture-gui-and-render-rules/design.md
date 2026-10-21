# Design

## Decisions

- Rules are pure functions and small state structs in the crates that
  already own the concern (`exa_formats` for formats, `exa_assets` for the
  bind cache, `exa_render` for list, queue, capture, geometry and fades,
  `exa_ui` for blend, skin and table). No new crate and no new dependency.
- The original's state is kept where it changes results: the blend switch
  keeps its cached word (including the table lookup that runs past the
  recorded entries), the skin globals keep the unclamped alpha and the
  per-skin prepared flag, the draw queue keeps its bucket order.
- Where the first pass guessed, the code now states what is known: the
  shadow below the table header takes the client's height less six units
  unclamped, the choice list's own height is not modelled, and the meaning
  of the table's style and control words is not established.
- Compared in double where the original does (fades, light weight
  threshold, alpha byte rounding to even in 64 bits).
- Existing behaviour is untouched: nothing in the running renderer or UI
  calls the new rules except the image decoders.

## Risks

- The table's centre reads the form's client rectangle; the field order of
  that rectangle in the original was inferred from the arithmetic, so the
  centre is only as certain as that mapping.
- Unwired modules can drift from the code that will call them; their tests
  pin the recorded arithmetic.
