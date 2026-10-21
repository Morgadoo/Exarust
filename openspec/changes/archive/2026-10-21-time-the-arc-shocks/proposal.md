# Proposal

## Why

The live effects of `run-the-thaum-effects` let a shocked fighter lose
yellow health at most once every 150 ms after its last shock, and let a
barricade lower a character's energy once a tick however many of its parts
the wall struck in front. The original does neither: the character's shock
routine lets a shock through once the clock is within 150 ms of the
character's next shock time and then sets that time 500 ms plus a draw
below 200 ms ahead, and the wall's body test runs for each body part, so
each part struck in front lowers the energy again. The knockback also went
out as a queued blow, which woke the body for 256 ticks and waited a tick,
where the original only sets the knockback step's target.

## What Changes

- A shock reaches a fighter when the game clock is within 150 ms of its
  next shock time (or it was never shocked); its next shock time becomes
  the clock plus 500 ms plus the engine generator's draw below 200 ms. A
  fighter that is collapsing or dead keeps its yellow health, the clock
  still moving.
- Each part of a character a barricade strikes in front lowers its energy
  while it is above 0.25; the knockback sets the step's target directly.

## Capabilities

### Modified Capabilities

- `thaumaturgy`: the live effects' shock timing and barricade knocks.

## Impact

- `exa_game::thaum::live` (`ShockReady`, `shock_reaction`,
  `energy_after_knocks`); `docs/THAUMATURGY.md`.
- Evidence: REA/Ghidra decompiles of the character's shock routine (its
  clock gate, spacing and generator draw), the barricade's body test (run
  per body part) and the knockback routine.
- Out of scope: the rest of the shock reaction (the condition change, the
  shock motion, the stagger and the voice), its two other branches for
  characters of other kinds, the barricade's sounds.

## Phase

Phase 5, visual and audio fidelity: thaumaturgy effects; the shock serves
Phase 4's combat.
