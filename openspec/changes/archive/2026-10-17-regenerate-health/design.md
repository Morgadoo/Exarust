## Context

The original's body tick keeps every character's health in two places.
Its prologue, before the dead test, regenerates red for the bodies whose
rule bits allow it; its health bookkeeping, after the posture block and
the attack and before the movement and clip clock, moves the heal pool to
red, runs the second wind, regenerates yellow and clamps. Damage resets
the regeneration rate in two places: the damage call (every blow that
deals health) and the weapon damage routine (every weapon contact on a
character, before it decides what the contact does). Death sets yellow to
0. The research notes, with addresses, are git-ignored:
`context/re/master-body.md` §4.2–4.5 (the rules, the retail fit), §7.4 (the
rule bits), §10.2 (the body tick's order), `m41-notes.md` (0x120 = 0x20)
and the new §22 (this change's reading).

The format page this change extends is
[`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md) (Damage: a new
Regeneration part); the HUD's rule is in [`docs/UI.md`](../../../docs/UI.md)
(Health bars).

## Goals / Non-Goals

**Goals:**
- the yellow regeneration, red regeneration, heal pool and clamps as the
  original's, each a pure method on `Health` pinned to its numbers (the
  ramp's 3,334 ticks, the retail fit, 0.005 a second, the red rule bits,
  the pool's 15,999 ticks);
- the gates read from the condition word (M38) and the clip playing;
- the rate reset wherever the original resets it in Exarust's damage path.

**Non-Goals:**
- second wind, bandages, potions (M46), burning (M45), saves (M47), the
  damage rules (M29), block stamina (M26).

## Decisions

- **Where it runs.** One system, `regenerate_health`, in a new set
  `HealthTick` after `PostureTick` and before `CharacterTicks`: after the
  condition is written for the tick and the dying tick has run (a body
  that dies this tick is dead and skips the yellow part, as the
  original's collapse branch jumps to the end), before the clips advance.
  M0's staged character tick moves it to the attack stage. Red
  regeneration runs first in the same system: it is in the prologue in
  the original, but nothing between the prologue and the bookkeeping
  reads red, and burning (the only writer between) is not ported.
- **The gate.** Held when the condition is attacking (2) or the clip
  playing is the combat idle (action 4), whatever the condition: the
  original tests the playing motion, so a weak fighter (0x100) still in
  Cmbt - Idle does not heal. The clip's action index is looked up in the
  installed motion set, as the posture decision does.
- **Precision.** As the original: the ramp in double precision cast to
  float, the cap a float (0.01/83 in float is the image's value), the
  yellow gain √red × rate in float, red regeneration and the pool's
  transfer 0.25 × 5e-5 in double cast to float; the clamps in order (yellow
  within red first, then red within the maximum, so a red brought down to
  the maximum takes yellow with it only on the next tick). Pinned tests
  compare to a float32 simulation of the same arithmetic.
- **The rate on `Health`.** The original keeps the rate and the pool on
  the character beside yellow and red; `Health` gains `rate` and
  `heal_pool` so `take` can reset the rate and the HUD's copy stays one
  value. The system writes the component only when it changed (a fighter
  at full health at the cap leaves it untouched).
- **Resets.** `Health::take` (the damage call) and `react_to_hits` for
  every hit on a fighter before it is resolved (the damage routine's own
  reset, which covers contacts that deal nothing). Death in the dying
  tick sets yellow to 0: the collapse regenerates yellow (it is not dead),
  about 0.0006 over its 250 ticks.
- **Dead bodies.** Red regeneration runs for the dead too (the
  prologue's test comes before the dead test): a dead ogre's red bar
  refills. Nothing reads a corpse's red yet (the undead re-rise, C23).

## Open questions (answered)

- *Is the cap written anywhere?* No: the `.data` float has three readers
  and no writer in `exe.asm`; the 0.25 of the red rate likewise.
- *Does the damage routine reset the victim's rate or the attacker's?*
  The victim's: the routine runs in the attacker's post-step and resets
  the rate of the struck body's owner, after checking it is a character
  and before choosing what the contact does.
- *Does a collapsing body regenerate?* Yes: the collapse branch continues
  into the bookkeeping unless the dying tick killed it.

## Risks / Trade-offs

- Exarust draws the tick's yellow, not the original's value interpolated
  between ticks; at frame rates above 83 Hz some frames repeat the value,
  which the bars' 200 ms hold covers.
- Only the player's yellow is visible (the HUD); an NPC's regeneration is
  seen only in its logs.
