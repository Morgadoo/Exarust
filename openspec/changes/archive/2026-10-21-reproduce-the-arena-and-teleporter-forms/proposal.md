# Proposal

## Why

Six of the original's forms had no counterpart: the technique selector, the compact skill screen of character creation, the arena loadout editor, the divination card reading, the company emblem editor and the teleporter's destination screen (77 routines, 30 743 bytes). A first pass modelled them; a review against the instructions found the technique selector working on six experience words per skill instead of the record's one, the destination screen's journey read as a session mode with the bubble's timings mistaken for the teleporter's glow, the ring's sine and cosine swapped, the reading eased by a guessed sine instead of the engine's smooth step, the compact screen's hit word off by one column and the loadout form's fields swapped.

## What Changes

- The technique selector picks and forgets techniques on the record's six skills, replaces the technique in training, refunds into it, and refreshes the tree's slot states by the original's ranges.
- The compact skill screen's metrics, hit test, keys, wheel, pips, nodes and paint follow the original.
- The loadout form's layout, cost rule, equip rule, drag flags and close follow the original; a rank's kit is the record's own loadout.
- The divination reading deals, eases, turns and shadows its cards as the original does.
- The emblem editor starts on the original's palette and its randomiser keeps the colours' own bytes.
- The destination screen places, reveals, hit-tests and paints its rows as the original does; a click starts the journey: the bubble above the teleporter, the teleporter's glow, the small crystal's charge, the level change when the resting bubble shrinks, and for the adapter the single-row mark and the save.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the six forms.
- `level-devices`: the teleporter's journey.
- `company-emblem`: the editor's start and randomiser.

## Impact

- `exa_ui::{technique_selector, compact_skill, loadout, div_card, emblem_form, selection, engine_math}`, `exa_game::{techniques, loadout, emblem, destination}`, `exa_game::effects::bubble` (a journey steps its own bubble), `career::Member::loadouts`.
- `docs/UI.md` (Forms) and the two crate READMEs.
- Evidence: REA (Ghidra) assembly and pseudo-code of the 77 routines and of the helpers they call (the engine's sine and cosine, ease, lerp, normalisation and 3 × 3 rotate; the character data's technique in training, forget and prerequisite pairs; the glow envelope; the level change; the bubble's creation); the teleporter's screen opener in the devices unit. Every float-to-integer conversion in the group is an inline `CVTSS2SI`/`CVTSD2SI` (no engine round helper is called), so they round half to even.

## Out of scope

- Drawing any of the forms (the skin, font and quad renderer is not driven yet); the controller action layer and the player's controls.
- The loadout's drop handler as the original runs it (the three item containers reconciled against the arena stock) and its item records' rank and point fields; the arsenal icon tree and the preview's character.
- The level change and the save the journey asks for; the bubble's GI light.
- The emblem shader's execution and its pictures; the compact screen's name and icon tables.

## Phase

Phase 7, game modes and content: the arena's and the story's forms (the arena career's editors and the teleporter's screen beside the level devices of Phase 2).
