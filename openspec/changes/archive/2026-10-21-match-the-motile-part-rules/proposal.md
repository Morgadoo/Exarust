# Proposal

## Why

The character's body part (the original's TMotilePart) carries the state,
impact and damage bookkeeping, the voice request and its spacing, the sound
cooldown stamp, the knock-out and wake rules, the grip links that hold a
weapon to a hand, and the swing set cache. A DeepSeek lane ported these from
pseudo-code. The review checked each against the instructions: float order
(the decompiler misorders sums and mistakes MINSS/MAXSS for ordered
compares), the random draw order, and NaN handling.

## What Changes

- `exa_physics::part`: the part state word, the sound cooldown stamp, the
  impact-sound material tree, the hard-impact reaction and the wound tally.
  The strike mark the original's chain walk sets beside the strike window is
  now modelled (`PartState::STRIKE_MARK`).
- `exa_physics::grip`: the grip link group (builder, modes, flip, shield).
- `exa_combat`: the voice request and its spacing (`voice`), the part-material
  hit sounds and foley (`sounds`), wake and status effects (`condition`),
  the parry clash branches (`parry`) and the shield arm's stamina (`regen`).
  Review corrections: the voice request's two volume gains are applied in
  order and its scream wait is an unwrapped 64-bit sum; the wake tone and
  readiness are summed in double with unwrapped 64-bit grace sums; a NaN
  coverage no longer rejects an armour layer in the part's hit sound.
- `exa_formats::character_stats::attack_reach`: computed in double with the
  original's association (size × ((1 − 0.3·age) × base)), the negative-age
  branch, and a NaN age passing through.
- Retail offsets and addresses added by the lane are rewritten as plain
  descriptions in the code, crate READMEs and format docs.

## Capabilities

### New Capabilities

- `motile-part`: the body part's state, impact, voice and wake rules.

### Modified Capabilities

## Impact

- `exa_physics` (`part`, `grip`), `exa_combat` (`voice`, `sounds`,
  `condition`, `parry`, `regen`, `swing`), `exa_audio` (`loops`, `sounds`,
  `hits`), `exa_formats::character_stats`; `docs/formats/OTHER.md`,
  `docs/formats/RSQ.md` and the crate READMEs.
- Evidence: REA (Ghidra) instructions of the part's impact, damage, stamp,
  voice, wake, status, hard-impact and attack-reach routines, and the foley
  tables read from the executable's data. Unit tests with hand-worked
  numbers.
- Out of scope: the equipment runtime that would call the grip builder, the
  swing whoosh trigger (inferred, not traced), the status-effect pass's
  callers, the held-item refresh and drop masks, the cross-body solver stage,
  and a contact-impact scalar on the game's `Hit` (so the impact sound's
  conditional random draw is not wired; the per-blow draw stays).

## Phase

Phase 4, combat and items: the character body part's damage and impact rules.
