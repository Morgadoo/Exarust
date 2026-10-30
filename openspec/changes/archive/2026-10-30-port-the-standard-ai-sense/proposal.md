# Proposal

## Why

Phase 6 (AI) of docs/PLAN.md. The motile AI's attach installs one of two
per-contact senses from the character record's race word: the standard one
for race class `0x12xx` (the Golem Suit) and the ally-aware one for every
other race carrying the AI bit `0x1000` (the golems). Exarust had the ally
sense but not the standard one, and no placed brain read the race word, so
every brain kept the default and the ally sense never ran; its fellow test
also took any brain for a fellow AI.

Evidence: REA/objdump listing of the motile AI's attach (the race word at
the record's profession word, the `0x1000` and `0xFF00 == 0x1200` tests, the
statue bit `0x08000000`), the standard per-contact sense (the kind gate, the
gone bit, the 150 cm flat band, the 1200/1800 cm strict squared range, the
double 0.99 clamp, the dead/asleep, knocked-out and live noticeability, the
view, the player test, the threat multiplier and weight, the softening
`min((concern + hostility)·0.15, 1)` through the engine's MINSS helper, the
selected entry read from the observer's own list), the ally sense's entry
(hostility and threat cleared), the contact integration's 0.999 hostility
decay (identifying the cleared word as hostility) and the golem wake that
clears the statue bit.

## What Changes

- `exa_ai::combatant::standard_sense`: the standard per-contact sense, with
  tests pinned to hand-worked numbers.
- `exa_ai::combatant::ally_sense` clears hostility (not the unused alarm
  word) at entry.
- `exa_ai::brain::Npc::of_record` takes the sense, statue mark and AI bit
  from the record's profession word; the perception pass's fellow test is
  that AI bit (`Npc::race_ai`).

## Out of scope

- Scheduling `standard_sense`: the original runs it instead of the
  behaviour base's perception, and Exarust's role-driven hostility still
  comes from `perceive`; the Golem Suit keeps `perceive`.
- The golem alarm-zone wake that clears the statue mark (`golem::wake` has
  no live caller): a placed statue stays blind.
- The attach's effect level, guard motion and sentry state install.
