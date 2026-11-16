## Why

Roadmap tasks B1 and B4 (wiring lane `charview`, docs/roadmap/phase-b-close.md):
the inventory's character view and its equipment routines had about 45
open ledger rows (TCharacterView, its equipment routines, TCharViewBehaviour,
TArsenalView) and the findings F278, F281 and F282. The view's mind-cost
arms were never driven, a script's HasItem always answered no, the shield
drop used a reduced take, the swap gate read the hand's lift instead of the
GUI's drag, the inventory's page buttons never followed the shown body's
weapon set, and the open arsenal never received its keys or right release.

## Evidence

REA (Exanima.exe), checked against the assembly where constants or order
matter:

- The character view's drag start, drop and release: the mind window byte,
  the shown character, the stored cost, the order of the charge before the
  take, the window's close on a refusal.
- The mind unit's trust change on a bound body and the Loyalty marker's
  limit test (trust plus the change, against a tenth of the negated
  relation, ten more with the body's learnt bit `0x10000`); the possession
  marker runs the same test.
- The displaced value of a take-in and the held item's value (the weapon's
  desirability, else the shield routine, whose double answer the caller
  reads as a single).
- The HasItem routine (the open pack saved, the carried list, the nested
  container search through the container sections).
- The shield take's job (the off hand taken out, the record written, the
  weapon-part clear on a class word without weapon bits).
- The swap gate (the GUI's drag count, the body's open view) and the swap's
  page-button states and pack reload.
- The arsenal's key slot (Escape, the bound inventory key) and mouse slot
  (a right release closes).

## What changes

- `exa_game::view_trust` (`ViewTrust`, `ItemValues`, `held_value`) and
  `mind::casts::change_binding_trust` / `thaum::power::mind::trust_holds`;
  the drag start (`figure_pick`), the cancelled release and the drop
  (`doll_view::equip_dropped`, `wear_room_over`, `displaced_by`) weigh the
  change with the mind window set; the writes go through the fixed step's
  commands, as the pack drag's binding end now does too.
- `script_cast::mirror_items` / `has_item_list` fill the script cast's item
  lists.
- `doll_view::hold_shield` runs `exa_items::doll::take_shield`.
- `hotkeys` passes the GUI drag and the open inventory to the swap gate.
- `gui_screens::inventory_follows_the_set` sets the page buttons and
  reloads the pack after a swap.
- `arsenal_view::arsenal_input` and `ArsenalMouse`.
- `exa_ai::search::{Arm, Apparel}::of_payload`.

## Out of scope

- The world body's part attach and take-in physics (the equipment jobs and
  their finalisers), the deferred take-out task and the swap animation's
  state; the core crystal's drop; the arsenal's column fill from the arena
  store and its write-back on close; the view's background wash (only the
  creator's view draws it) and the behaviour's turn target (the camera's
  point, the camera lane's).

## Impact

- Lockstep: a drag on a bound body's inventory now changes its markers'
  trust at the next step and may end a binding; HasItem now answers yes for
  carried items, which changes any scene whose scripts test it.
- Grid scenes owed while the GPU is on hold (see tasks).
