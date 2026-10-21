# Design

## Behaviour

The skill screen reads the record's skills through their byte layout (two
point bytes then six technique bytes a skill), because its scans index by
the funded level count and read on into the next skill when that passes
six. Its requirement mask uses its own node table's present flags, whose
prerequisite pairs equal the record's table; the record's lookup is reused.
The screen's unlearning differs from the record's: it also moves the
committed points and walks dependents from the last funded level down.
The experience pass rewrites the record's points from the committed ones
and a budget that is 32-bit and compared unsigned. The press state and the
ready flag gate the learning row and the forget dialog.

The extended controls are modelled as each class's message rules; the
text edit unit they call (typing, deleting, selection moves) is outside
the evidence and is not modelled.

## Model

`SkillScreen` holds the screen's fields (ring, page, state bits, popup,
hover, pending learn or forget code, requirement mask, committed points,
experience, leftover, ready, locked) with `press_node`, `commit`, `refresh`
and `fade`; the geometry is pure functions. The experience system's
conversions are passed in (`ExperienceRates`), since they live in the game
crate. The controls are plain data and functions in `exa_ui::controls`.

## Open questions

- The technique descriptions and the screen's icon ids are filled at run
  time or read from `.rdata`; the model exposes the slot and leaves the
  text to the caller.
- The number formatter behind the spinner, progress and scroll captions is
  outside the group; Rust's fixed-point formatting stands in.
- The global inset-layout flag's meaning is not traced; it is named by what
  it changes (the 58-unit radius and the page's one-unit shift).
