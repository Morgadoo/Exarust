# Proposal

## Why

The character view's equipment routines (Phase 4 — Combat and items, see
docs/PLAN.md) still lacked the doll's hand-scene builder, the released
model's finaliser, the attach jobs' record flags, the weapon off-hand
request from Insight and the figure's drag-start charge and payload.
Review against the instructions also showed the re-dress needed a separate
grip recompute step, and that the detach step only runs for hands whose
body model is present.

Evidence: the character view's equipment cluster (hand-scene builder,
scene rebuild, take-out and its model finaliser, weapon and shield attach
jobs, take-in job, set swap) and the view's drop, drag-start and Insight
routines, read in the objdump listing and the pack's Ghidra listing.

## What Changes

- `exa_items::doll`: `scene_plan`/`scene_attach` (a model per changed hand,
  a target of 0 builds none, a modelled item with no section as a weapon,
  the off hand's lighting weapon as a light); `AttachPlan::rebuild_steps`
  (detach only present models, `RefreshGrip` always, the stance restart
  outside the swap state, the body refreshed when both targets are 0);
  `release_model` and `RESTART_MODE` (the take-out job's mode bit 1);
  `KEPT_HAND_FLAGS` (weapon and light take-ins write only the top flag bit
  into the record; a shield keeps all); `hand_node` settle words;
  `swap_timings` (the view's two `0x60`-frame counters).
- `exa_ui::character_view`: `insight_known`; the drop plan's `off_wanted`;
  `drag_start_plan` (stored and refunded mind cost, a refusal cancelling,
  the charge standing when the icon fails, the kind apart from the
  payload's icon, flags and half icon size).
- `exa_game`: a weapon dropped on the view's right half by a player who
  knows Insight's technique 8 goes into the off hand; take-ins write the
  masked flags.

## Out of scope

Creating the item models, growing their parts and inserting their scene
nodes (the running doll still draws only the main-hand model); the take-in
job's wait for loaded nodes; the drag-start plan and the swap counters are
not wired to the panels; the view constructor's unidentified fields.

## Impact

`exa_items`, `exa_ui`, `exa_game`; docs/UI.md, the exa_items README.
