## Why

Exarust takes a blow's damage by a simplified rule: yellow and red always
drop, in single precision, and yellow under 0.01 always kills. The
original's damage call has more to it, and its variants already matter:
every living human of the story levels is feral, and blows take nothing
from a feral body's red health (they go to yellow instead), as for
skeletons and ancients. A potion halves the yellow damage; only a vital
blow kills (impacts are not vital until they accumulate); in the story a
killing blow also costs red health; and a blow that leaves yellow under
0.05 without killing runs the original's low-health check, which gives a
second wind or, for the player, the heartbeat. The HUD guesses that check
from the yellow it sees fall.

Phase 4 (Combat and items), milestone M4 "first duel", and the master
plan's MS3. This is change M29 of the master plan
(`context/re/master-plan.md`, lane F; body note B6), now that the
condition word and state flags (M38), the rule bits (M41) and the
regeneration rate (M43) exist.

## What Changes

- `Health::take` becomes the original's damage call: it takes a `Damage`
  (yellow, red, vital), the victim's rule bits and state flags and
  whether the game plays the story, and returns what it left (`Hurt`,
  `Weakened`, `Killed`). In order: the regeneration rate to 0; yellow
  −0.1 × Y (0.05 × Y with the potion flag); rule bit 0x8 (no lasting
  wounds): yellow −0.2 × R, red untouched; else red −0.2 × R and yellow
  scaled by red's share left (0 when red is gone); then under 0.05 the
  low-health check, unless the damage is vital and leaves yellow under
  0.01 (in double precision): yellow 0, in the story red −0.1 × the
  maximum, killed. Non-vital damage under 0.01 leaves yellow 0 and the
  fighter alive. Subtractions in double precision, as the original.
- The hit reaction passes each victim's rule bits, state flags and the
  game's story mode (a `StoryMode` resource the viewer sets for story
  levels), and writes a `Weakened` message for the low-health check.
- The HUD starts the heartbeat from `Weakened` on the player's body
  instead of watching its yellow fall between 0.01 and 0.05.
- `failing(yellow)`: the original's "under 0.01" test in double
  precision, used by the player's combat stance (Defence while failing).
- `exarust --feral f` (debug): the player's record has feral level f, to
  see the no-lasting-wounds rule and the feral stats in game.

Evidence:
- `exe.asm`, REA cross-references and constants read from `Exanima.exe`
  0.9.5: the damage call (its order, the double-precision weights 0.1,
  0.05, 0.2, the float 0.05 and double 0.01 thresholds, the never-written
  "no death" byte, the never-set player hook, the story byte written only
  by the story mode's setup), its two callers (the weapon routine, always
  vital; the impact routine, vital past 0.6), the low-health check, and
  the three readers of yellow under 0.01 (`context/re/master-body.md`
  §4.2, §4.7, §4.10 and the new §23).
- The installed corpus: which characters carry rule bit 0x8 (feral
  humans, skeletons, ancients; M41's corpus tests).

Out of scope:
- the second wind itself, bandages and potions' expiry (M46: nothing
  sets the potion flag or the second-wind item yet);
- impact damage and its accumulator (C22, the first non-vital caller),
  the block path's yellow cut (M26), burning (M45);
- the stance by energy and the parry's clearing under 0.01 (their
  callers, the swing stall, the AI's other branch and parries, are M22,
  M26, L10/M27 and lane D);
- the daze's potion and steadfast factors (M30).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `character-stats`: damage to health (new requirement).
- `user-interface`: the low-health heartbeat follows the damage call's
  low-health check (modified requirement).
- `world-viewer`: the `--feral` debug flag (new requirement).

## Impact

- `exa_combat` (`fighter.rs` with a new `fighter_test.rs`, `lib.rs`,
  `stance.rs`, `damage.rs`, `regen_test.rs`)
- `exa_game` (`hud.rs`)
- `exarust` (`main.rs`, `viewer.rs`)
- `docs/formats/OTHER.md`, `docs/UI.md`, `docs/PLAYER.md`,
  `docs/DEBUGGING.md`, `docs/PLAN.md`, `crates/exa_combat/README.md`,
  `crates/exa_game/README.md`
