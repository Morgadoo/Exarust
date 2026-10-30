## Why

Two routines of the GUI form base class had no exact counterpart: the
client-area refresh a shown form runs over its tree, and the sweep a freed
form runs over the GUI's single-form slots. A third routine the lane read
as a "scroll timer" turned out to be the wheel handler already reproduced.

Phase: 7 (interface) in docs/PLAN.md, the GUI form base.

## What Changes

- `exa_ui::form::refresh_client_areas`: the form and every descendant,
  depth first with children in order, recompute their client rectangle.
- `exa_ui::form::sweep_freed_form` with `FormSlots` and `SweepFollowUp`:
  a freed form's tree is walked parent first; each slot that held a form
  is cleared in the original's order, and the follow-ups (focus fallback,
  parent focus or cleared focus, tooltip reset, drop callback reset) are
  returned in order.
- The lane's duplicate `scroll_by_elapsed` is removed; `wheel_scroll`
  already covers that routine.

## Evidence

REA (Ghidra) on Exanima.exe and the objdump listing: the refresh calls the
client-area virtual and an empty base hook, then recurses over the child
array; the sweep tests nine slots in order, the focused case reading the
parent's visibility byte (the byte the focus fallback tests); the wheel
routine's two double constants are ±1/120 with `CVTSD2SI`.

## Out of scope

- A caller: the running game frees and shows forms through `GuiTask`,
  which keeps its own bookkeeping; neither rule is wired.
- The roles of two slots (named "pressed" and "dragging" by guess).
