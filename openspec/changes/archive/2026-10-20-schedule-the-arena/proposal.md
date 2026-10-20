# Proposal

## Why

The arena's templates are modelled, but the rules that turn a day slot into a rolled match, filter the generation pool, draw a tournament prize and settle a result are not. Those rules carry the arena's actual scheduling behaviour.

## What Changes

- A template's rank rolls to `slot` when flag 2 is set, to −1 when unranked, or to `trunc(slot·(1−r⁴))`; Challenger opens the 0–5 window.
- The secondary value scales by the slot in base-12 money.
- Generation rejects a full per-pass cap, a slot before the minimum, and a reject-mask roll.
- Template and prize draws use the engine's `0x8088405` generator and retry around caps and history.
- A match settles to a win (pays, logs 0x20000, counts a Beast), a practice (logs 0x40000) or another result (0x30000); the played counter always rises.
- The season armour set is awarded only in the last week with all four weekly wins.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the arena's generation and settlement rules.

## Impact

- `exa_game::arena`, from `preset_rank`, `roll_rank`, `rank_window`, `scaled_secondary`, `eligible`, `pick_template`, `pick_prize`, `Outcome`, `Settlement`, `settle` and `season_award`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `XA_Arena` becomes **18%** in the source rating. The headline rises slightly.
- Evidence: `0x100183f80` (the roll and window), `0x10018cf70`/`0x10018cc90` (the cap/slot/reject filter), `0x100184250` (the generator draw), `0x10018c2d0` (settlement tags and the Beast counter), `0x100192b90` (the season award) and `0x1002bcba0` (the prize pool). No new corpus files.
- Out of scope: the live board, opponent generation, the shop, tournament resolution and the UI.

## Phase

Phase 7, arena. The board's generation.
