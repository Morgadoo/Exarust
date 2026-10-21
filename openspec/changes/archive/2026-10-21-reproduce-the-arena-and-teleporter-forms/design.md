# Design

## Context

The six forms are pure rules in `exa_ui` (positions, timings, state words) with their Bevy bridges in `exa_game`. Formats touched: none (`docs/formats/` is unchanged; the record's skills and loadouts are those `exa_formats::characters` already reads).

## Decisions

- **One engine math module.** `exa_ui::engine_math` holds the engine's sine and cosine (the octant polynomials), `RSQRTSS`, and the processor's conversions with their NaN and range rules, because `exa_ui` cannot reach `exa_game`'s copies.
- **The journey outlives the screen.** A click closes the screen and puts a `TravelJourney` beside a `BubbleEffect` on the bubble's entity; the journey steps that bubble itself (as the original's holder does) and `tick_bubbles` skips it, so the journey sees the bubble's shrinking and freed states.
- **The selector duplicates the record's unlearning.** `exa_ui` does not depend on `exa_character`; a test in `exa_game` checks the selector's forget and technique in training against `exa_character::record` on every skill, slot and sample record.
- **The loadout keeps a list model.** The original's drop handler works on three item containers and the arena stock, none of which `exa_ui` has; the editor's weapon and armour lists stand for them (labelled a guess). The layout, cost, equip rule, drag flags and close are the original's.

## Open questions

- The player's controls the journey stops and hands back, and the controller action layer the screen pops: answered when the input layer exists (REA on the player-controls unit).
- The level change's arrival word and the save: answered by the session's level-change work (REA on the game-main level change).
- The loadout drop handler's container reconciliation: answered with the arena's item containers (REA on the drop handler and the arena stock routines).

## Verification

Hand-worked numbers in the tests come from the instructions (for example the emblem randomiser's sequence from seed 1: palette 8, 220, 66, 70, pictures 22 and 7). No test reads game data; the forms read no file.
