## Why

A duel screenshot shows the player's arm seemingly folded back over its
head, asking whether bones need a weight that stops such a bend. The pose
is the "Head Pain" clip (hands on the head, elbows bent forward to the top
of their 120° range) seen from 48° above. Exarust starts it on every wound
to bone 9 or above whose penetration plus impact exceeds 0.2, and
"Rebalance" below. The original starts no clip on the struck character:
Head Pain is a story clip, staggers come from balance, and a character
struck mid-swing has its swing stalled back to its stance.

The bones already bend as the original's do: a joint's range is its links'
slack, with no angle limit and no bend weight, and Exarust's links match.

Phase 4 (Combat and items), milestone M4 "first duel". It also pins, for
Phase 3 (Bodies, M3), how the links bound the joints.

## What Changes

- A blow that does not kill starts no clip on the struck fighter: no Head
  Pain, no Rebalance. Health, the daze, the shove (Exarust's stand-in for
  the knockback step) and going limp on death are unchanged.
- A fighter struck during an attack stops that attack and returns at once
  to the stance its attack would have returned to, so its blade deals no
  further hit. A killing blow also ends the attack. This stands in for the
  original's swing stall until Exarust has a swing strength.
- A test of the installed `humanoid.rsq` pins the hinge ranges its links
  give: each elbow from straight to 120°, with 10° past straight stretching
  its long link 1.8 cm beyond its slack, and each knee from 10° to 130°.
- Docs: what a wound does to the struck character (OTHER.md, Damage), the
  joint ranges and the absence of angle limits (RSQ.md, Physical body),
  Head Pain as a story clip (RSQ.md, Motion set), Phase 4 in PLAN.md.

Evidence:
- Reverse engineering of `Exanima.exe` 0.9.5 (REA, Ghidra, and the
  disassembly): every clip start in the damage routine goes through the
  striker's recoil; the daze and the swing stall it calls for a character
  that is swinging (the constants 4, 0.005, 0.001 and 80 ms read from the
  image); the stall block of the swing routine (the stance restarted with
  a blend from the weapon record); the only writer of the Head Pain action,
  a game-script handler that also shows `XU_Memory`; the Defence-stance
  reaction, called only by electricity and energy effects; both link
  iterations, which test only a link's length against its slack
  (`context/re/joint-limits.md` §0–§6 and S.1–S.5, `combat-damage.md`).
- Corpus: the links of `humanoid.rsq` (`exa-tool rsq`; the new test).
- Exarust in game through `gpu_run.sh`, with a temporary probe of each
  elbow's flexion and every arm link's stretch: the duel before and after
  the change, the player attacking the duellist, `--shove 60 play arena`,
  attack clips in the character view and `--limp character`.

Out of scope:
- the swing stall itself (the struck attacker's swing slowed by its
  draining swing strength, back to the stance within the swing's time) and
  the striker's recoil and arm states (C21);
- the knockback step that replaces the shove (C16);
- Rebalance and the Defence stance from balance (C11);
- a lethal wound's Fall Backward and collapse instead of going limp at
  once (C23);
- blending clip starts (C10), repairing mirrored parts, contacts between
  bodies and Break Tangle (C17).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `combat`: a blow that does not kill starts no reaction clip on the struck
  fighter; a fighter struck during an attack stops it and returns to its
  stance.
- `physical-body`: a joint's range comes from its links alone; the shipped
  elbow and knee ranges.

## Impact

- `exa_combat` (`lib.rs`: `react_to_hits`, `stance_of`, the reaction
  tests; `damage.rs`: the sword record as a test helper)
- `exa_game` (`limp.rs`: the shipped hinge range test)
- `docs/formats/OTHER.md` (Damage), `docs/formats/RSQ.md` (Physical body,
  Motion set), `docs/PLAN.md` (Phase 4)
