## Why

Phase 4 (Combat and items) in docs/PLAN.md, TMotilePart, the motilepart5
gap lane (round 187). The round-177 rating left these reachable
TMotilePart items: the setup's foot-child words and box-derived sole, the
NPC's combat target read a tick late by the attack starters, the link
iteration remainder, and the collapse and death partials.

## Evidence

REA (Ghidra pseudo-code and the listing of `Exanima.exe` 0.9.5):

- The attack starters read the character's combat target when they are
  called. The AI's combat state writes that target (its set-target, or an
  adopted foe that pressed harder) and requests the attack in the same
  tick, so a blow aims at the target of its own tick. Exarust's starters
  read `Attacker::foe_parry`, aimed in `FixedPreUpdate` from the target the
  character tick's attack stage copied a tick earlier.
- The death routine runs the parry end while the parrying flag is set,
  before it writes the dead condition: the parry flag and the countered
  motion are cleared, the shield's and the held items' parry links are
  disabled and both hands lose the parry's arm bit. Its stance restart
  (blend 40) is skipped only under condition `0x80`.
- Checked and already live (no change): the muscle setup's foot child (the
  first child of the limb class family) gets friction 0.5, half the shin's
  part factor, the support factor 0.25 and the kind word `0x13`, which
  only the pose sampler reads (Exarust keys the foot by its bone); the
  sole height is the foot box's half extent less its centre less 0.5 cm,
  read from the box words, as Exarust does. The collapse's held-item drop,
  torch release, leg layers, foot words, voice and the Gobbler's femur
  slack, and the death's leg-limit strength, link-length reset, layer
  writes and root link iterations of 10 are live.

## What Changes

- `exa_combat::parry`: `aim_foe_parries` split into `fighting_of` /
  `fighting_foes` and a per-fighter aim; `aim_at_new_target` takes a new
  combat target and aims again at once.
- `exa_ai` duellist tick: just before its attack starters it takes the
  combat target it holds now (the character's brain pick, else its foe in
  combat, as the attack stage resolves it) and aims again when it changed.
- `exa_combat::collapse::end_parry_at_death`: death's parry end, called by
  the dying tick.

Not in:

- The death's stance restart after the parry end: the dead body's clip is
  stripped.
- The death's player-side call (a check of the dying character's race list
  near the player), not traced.
- The link iteration remainder.
