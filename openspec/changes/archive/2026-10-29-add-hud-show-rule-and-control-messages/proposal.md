## Why

The in-game GUI unit's remaining small routines had no tested counterpart: the HUD bars' and gauge's show tests as one rule, the counter's and the bar control's message handlers, the inventory fill's page-button states, the journal selection's control state, the technique selector's open plan and the deferred inventory fill's answer. The HUD draw also repeated the show test inline, without the power-bar input.

Phase 4 (combat and items) in docs/PLAN.md: the in-game GUI (SG_GameGUI unit routines).

## What Changes

- `exa_ui::game_gui::bars_shown` and `gauge_shown` hold the bars' and gauge's show tests; the HUD draw calls them every frame (no new work per frame: five compares).
- The counter's message handler (1 its segments, 2 the skin's control, 0x200 the value set, caption, callback and answer 3) and the bar control's (1 the frame, 2 the bar two pixels in, one down and four narrower).
- The inventory fill's page-button states and fill plan, the journal's list state, the technique selector's open plan and the deferred fill's answer on the GUI job queue.

## Evidence

Read from the instructions of the HUD's bars and gauge fades, the counter's and bar control's message handlers, the inventory fill, the journal's selection, the technique selector's open and the GUI job queue's runner (which keeps a job that answers true). The flag the bars test after the focus is set and cleared only by the thaumaturgy unit's power bar.

## Out of scope

The power bar (Exarust has none, so the input is false), the counter and bar controls on a live form, the item panels' load and the pack's bulk from the worn equipment, and the inventory view's thaumaturgy check.
