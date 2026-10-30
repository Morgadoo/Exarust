## Context

The form base class's rules live in `exa_ui::form` as plain functions over
ids, children lists and words; `GuiTask` runs the live screens.

## Decisions

- The sweep owns the slot values (`FormSlots`) and mutates them, so the
  slots the original clears directly are cleared; the main and focused
  slots are left to the follow-ups that clear them in the original (the
  focus fallback and the focus call), which the caller runs.
- Parents and their visibility are passed as per-form slices; the
  parent's active-child slot as a per-form slice indexed by the parent.
- The refresh takes the client-area measure as a closure; the base class's
  second hook is empty and is not modelled.
- The wheel routine stays `wheel_scroll`; no timer exists in the original.
