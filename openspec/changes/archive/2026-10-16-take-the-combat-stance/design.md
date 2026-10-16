## Context

How the original keeps a fighter's stance. Each character has a stance
field, a motion of its moveset: Idle at creation, the looping clip it
returns to. Two parts of the engine use it:

1. **The end of a clip played once.** When a non-looping clip ends (with
   the muscles on), the body tick starts the stance motion, whatever it is
   at that moment, with a blend by the ending clip (twice the weapon's
   recovery for attacks; 0, 20 or 60 frames for others).
2. **The posture block of the body tick.** The body tick branches on the
   playing motion. With tone below 0.85 it takes the Defence stance. Else
   Kick, a parry, Cmbt - Idle, the powers, Walk and Run and Switch Weapons
   each have their own branch; every other motion reaches the general
   branch: with the muscles on, below balance 0.8 it takes the Defence
   stance (slow) or Rebalance (moving), else, when the playing clip loops,
   differs from the stance and no Rebalance hold runs, it starts the
   stance (blend 30). The Cmbt - Idle branch does its own work (readiness,
   the auto-parry check) and never reaches the block, so a fighter in
   Cmbt - Idle stays there whatever its stance becomes.

Who writes the stance:
- the player's behaviour tick: out of combat mode Idle through the
  set-stance helper (which starts the new stance when the old one's clip
  plays); in combat mode, written directly every tick: keep it while
  parrying; the Defence stance while yellow health is under 0.01;
  Cmbt - Idle while the energy is within 0.05 of its cap and either the
  target is nearer than 350 cm × its body scale or less than 3000 ms
  passed since the last attack time; else the Defence stance. The target
  is picked first among the nearby characters, not collapsing, visible:
  within 500 cm (|d|² ≤ 250 000), best (500 − d)(cos + 0.5) > 0 against
  the cursor direction. The distance is between the two chest points. The
  last attack time is written by the swing routine on every tick of an
  attack, so the 3 s run from the attack's last tick.
- the AI: entering its combat state, the Defence stance through the
  helper; in its stance branch the Defence stance when 4 × its reach is
  at most the distance, else Cmbt - Idle, then the stance starts (blend
  40, or twice the primary weapon's recovery) unless it parries, is in
  Break Tangle or Switch Weapons, or is busy. The clip start returns at
  once when the motion already plays. Reach: 60 cm × the scale plus the
  weapon's sweet spot (the parts before the last whole, then 60 % of the
  way along the last part's striking extent), or the race's own unarmed
  (50 cm for a human).
- the body tick's daze block (the Defence stance while dazed) and the
  balance cases, deferred (below).

Readiness g: +0.004 a tick, capped at 1, while the stance and the clip are
both Cmbt - Idle (or parrying, or an attack ready); else 0. The damage
routine scales penetration, impact and stab energy by 1 − 0.5g right after
the knockback step, which takes the unscaled impact.

Combat mode: the arena's match start and round spawns and the practice
start and next fight set it on; revive and the hub set it off.

The research notes, with addresses, are git-ignored:
`context/re/combat-stances.md` §2, §3, §5, §8 and §14,
`context/re/body-misc.md` §2.5–2.6, `context/re/combat-damage.md`,
`context/re/combat-look-plan-final.md` (L2, challenge notes).

The format pages this change extends are
[`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md) (Weapon stats:
reach; Damage: readiness), with the rules in
[`docs/PLAYER.md`](../../../docs/PLAYER.md) (The combat stance).

## Goals / Non-Goals

**Goals:**
- the fists-up guard at a fight start and the switch to Cmbt - Idle at
  3.5 m, with the original's quirk that Cmbt - Idle stays;
- the duellist's guard far and its combat idle near;
- every clip played once ending in the stance; readiness and its damage
  factor; the fight start in combat mode.

**Non-Goals:**
- the daze's stance and pose hold, every blend (C10); the balance and
  tone cases (C11); parries (L10); fists that strike (L3); AI footwork
  and its other combat branches (C16, L11).

## Decisions

- **The stance lives in `exa_combat`** (`stance::Stance`, on the player's
  body and the duellist; level characters have none, so the switch never
  replaces their clips). The body tick part (readiness, the follow-up of
  a clip played once, the posture switch) runs in the fixed step before
  the character ticks, like the falls. The player's rule runs in the
  fixed step after its steering; the duellist's in its frame system.
- **The follow-up.** Exarust's `Animator` holds a clip's follow-up when
  the clip starts; the body tick part rewrites it to the stance each tick
  when that follow-up is a stance clip, so attacks and get-ups end in the
  stance held at their end, as the original's, and Exarust's falls keep
  ending in their lying clip.
- **The posture switch's gates.** Balance ≥ 0.8 and tone ≥ 0.85 (the
  general branch's own gates); between 0.6 and 0.8 the original takes the
  Defence stance or Rebalance (C11), which Exarust leaves out, so the clip
  stays. Left alone: Cmbt - Idle, Walk, Run (the original's branches) and
  Exarust's Backpedal and Combat Walk (its interim footwork, which plays
  clips the original never plays in combat).
- **The interim combat walk** remembers the clip it replaced and gives it
  back: the original's footwork never changes the clip, so the guard must
  survive a step.
- **The fight start** is the end of the load (the first frame shown): the
  duellist waits for it, so the guard and the approach are seen from the
  start instead of happening behind the loading screen. The player's
  combat mode turns on then in practice and arena levels.
- **The player's target** is measured chest to chest (the balance's chest
  point, or 1.3 m over the root before it is measured), from the cursor's
  horizontal direction; the original measures the pick's vector from
  another point of the picker's body, which changes neither the ranking
  much nor the 3.5 m rule.
- **The AI's reach** uses the record's striking extent over the whole
  weapon length (exact for one-part weapons such as the arena sword).
- **Readiness** is a field of `Fighter`; `damage::resolve` takes it and
  returns the impact unscaled for the shove.

## Risks / Trade-offs

- Every stance change snaps until C10: the clip changes at once and only
  the muscles smooth it (the physical body takes about 0.2 s to raise the
  guard).
- The duellist holds its guard one frame at the fight start when it
  starts within four reaches; the live opponent held it 0.4 s, from parts
  of its combat state that are not traced.
- Readiness halves the wounds of a player who stood 3 s in Cmbt - Idle:
  the duellist's hits now take about half the health they did.
- The practice level (`play arena`) still puts the opponent 2.5 m behind
  the player (Exarust's stand-in), so the player keeps its guard there:
  the target behind it is no target.

## Open Questions

- Where the practice opponent starts and how far it walks in before the
  player drops its guard: retail request (an unarmed practice fight, shots
  at the start and at 3.5 m; `combat-look-plan-final.md` request 1).
- What keeps the live AI in its guard for 0.4 s at the fight start: REA on
  the AI combat state's other branches.
- The visibility test on the player's target and the point the pick
  measures from: REA on the scene's line-of-sight routine.
- Shot 2's raised fists: the fight-start guard, a parry, or dying (L10;
  the same retail request).
