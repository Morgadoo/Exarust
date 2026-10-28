## Context

`exa_world::cloth::step_cloths` gathers each cloth's contacts through the sector's object stages, then steps it.

## Decisions

- Fix the bound radius (units) rather than tune the pre-test.
- Gate the live gathering behind `ClothContacts` (default from `EXA_CLOTH_CONTACTS`), keeping the code path and its tests; the test that lands a cloth on the level turns it on.
- Skip gathering for a sleeping cloth: its step is skipped, so the contacts were dead work.

## Risks

Cloths no longer collide with bodies or the level by default, as before round 24.
