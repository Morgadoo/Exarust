## Why

The user's screenshots of the original's arena show the unarmed player
in a fists-up guard, hunched, the fists before the chest and chin (shots 1
and 2), and an armed player in a low one-handed stance (shot 6). Exarust's
player always plays Cmbt - Idle in combat mode, so the guard never shows;
the arena needs Tab before a fight; and the duellist never guards.

The original keeps a stance on every character: the looping clip it
returns to. In combat mode the player's is the Defence stance (the guard)
until its target comes within 350 cm or it attacks, then Cmbt - Idle; the
arena and practice switch combat mode on at every fight start; an AI
opponent enters combat in its guard and guards beyond four reaches.

Phase 4 (Combat and items), milestone M4 "first duel": the second of the
combat-look changes (`context/re/combat-look-plan-final.md`, L2, as
corrected by the plan's challenge).

## What Changes

- Every fighter keeps a stance (Idle, Defence stance or Cmbt - Idle). A
  clip it plays once that ends in a stance (an attack, a get-up) ends in
  the one it holds when the clip ends. In the body tick, a balanced
  fighter (balance 0.8 or more, tone 0.85 or more) standing in another
  looping clip switches to its stance, except from Cmbt - Idle, the walk
  and the run, which the original's body tick handles in their own
  branches (and Exarust's interim backpedal and combat walk).
- The player's stance, each behaviour tick: Idle out of combat mode (the
  clip follows when it was the old stance's); in combat mode the Defence
  stance while dying (yellow under 0.01), Cmbt - Idle while its target is
  nearer than 3.5 m × the target's body scale (chest to chest) or within
  3 s of its last attack's last tick, else the Defence stance. The target
  is the best other living fighter within 5 m by (5 − d)·(cos + 0.5) from
  the cursor's direction. So the player keeps Cmbt - Idle once there,
  when the target leaves or dies, as the original does.
- Combat mode turns on at the start of a practice or arena fight (with
  `--opponent`, once the level shows; not in the arena hub). Tab still
  toggles it and no longer changes the clip itself. The interim combat
  walk gives back the clip it replaced, and a walk or run still playing
  when combat mode turns on ends in the stance.
- The duellist waits for the fight start, enters combat in the Defence
  stance, then holds the Defence stance at four reaches or more from its
  target and Cmbt - Idle within, starting it whenever it is free. Its
  reach is 60 cm plus its weapon's sweet spot (50 cm unarmed).
- Readiness: 0.004 a tick while a fighter's stance and clip are both
  Cmbt - Idle, else 0; a blow scales the victim's penetration, impact and
  stab energy by 1 − 0.5 × readiness, after the shove is taken.
- Debug helper: `--still-opponent`, a duellist that faces the player in
  its stance but never walks or strikes, to see the stances at a chosen
  distance (`--at`). The log names every stance change.
- Docs: PLAYER.md (The combat stance; Combat mode), OTHER.md (Damage:
  readiness; Weapon stats: reach), DEBUGGING.md, PLAN.md (Phase 4), crate
  READMEs.

Evidence:
- REA on `Exanima.exe` 0.9.5 and its disassembly: the player's stance
  rule (350.0, 3000 ms, yellow under 0.01, the energy clause) and its
  target pick (250 000 cm², 500, 0.5), the stance write each behaviour
  tick; the body tick's motion branches (tone ≥ 0.85, then the Kick,
  parry, Cmbt - Idle, power, walk/run and Switch Weapons branches before
  the posture block; balance ≥ 0.8; looping, not the stance, the
  Rebalance hold) and its clip-end start of the stance; the swing tick
  writing the last-attack time every tick; the AI's stance branch (4 ×
  reach, the start unless parrying, tangled, switching or busy) and its
  entry into combat (the set-stance helper); the clip start returning at
  once for the playing motion; readiness and its damage factor after the
  knockback step; the reach (60·scale + the sweet spot, 50 cm unarmed)
  (`context/re/combat-stances.md` §2, §3, §5, §8, §14;
  `combat-damage.md`; `combat-look-plan-final.md`).
- Live retail (read-only probe, earlier session): combat mode 0 → 1 at
  every fight start with the Defence stance at once; the opponent at
  383–388 cm still Defence, Cmbt - Idle from 350–356 cm; Cmbt - Idle with
  the stance Defence for seconds, also after the opponent died;
  readiness 0.004 a tick in Cmbt - Idle; the AI Idle → Defence → Cmbt -
  Idle at the fight start.
- Corpus: the "Default" and "One Handed" Defence stance and Cmbt - Idle
  clips of `humanoid.rsq` (`exa-tool rsq`; forward kinematics in the
  notes).
- Exarust in game through `gpu_run.sh`: the arena duel from the fight
  start, a still duellist at 8, 3.7 and 3 m, the sword stances.

Out of scope:
- the daze: the dazed fighter's Defence stance (the energy clause), the
  body's daze block and the sampler's pose hold, which land together in
  C10 (alone, the guard would rise at every dazing blow);
- the blends of every stance change (C10);
- the Defence stance and Rebalance from balance below 0.8 or tone below
  0.85 (C11);
- parries and their stances, readiness while parrying or with an attack
  ready (L10, C21); fists that strike, cooldowns, the energy gate (L3);
- the AI's footwork in its stance, its parries and attack scorer (C16,
  L11); the line-of-sight test of the player's target; the practice
  opponent's start (Exarust keeps its 2.5 m stand-in).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `combat`: the combat stance, readiness, the duellist's stance and fight
  start, the unarmed player scenario.
- `player-movement`: combat mode on at the fight start; the combat walk
  gives back its clip.
- `world-viewer`: `--still-opponent`; the scripted punch without Tab.

## Impact

- `exa_combat` (new `stance` module: `Stance`, the rules, the body tick
  system; `Fighter::readiness`; `damage::resolve` takes the victim's
  readiness; `WeaponRecord::reach`)
- `exa_game` (`player`: the stance rule, the fight-start setter, the
  combat walk's restore, attacks ending in the stance; `balance`: the
  get-up ends in the stance)
- `exa_ai` (the duellist's fight start, guard and stance by reach; hold
  ground)
- `exarust` (the fight start, `--still-opponent`, the duellist idle until
  then)
- `docs/PLAYER.md`, `docs/formats/OTHER.md`, `docs/DEBUGGING.md`,
  `docs/PLAN.md`, `crates/exa_combat/README.md`, `crates/exa_ai/README.md`,
  `crates/exa_game/README.md`, `crates/exarust/README.md`
