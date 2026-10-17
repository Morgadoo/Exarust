## Why

C10 (`blend-clip-starts`) made every clip start from a capture of where the
body is, with a blend and an ease its caller sets, and gave a dazed body
the original's pose hold. The stance code of L2 (`take-the-combat-stance`)
and the duellist were written before it and start their clips with
`play_loop`, which now means no blend and the linear ease: a stance change
blends only over its clip's first key, never with the original's 30 or 40
frames. Without the daze's own Defence start, a dazed standing body holds
its physical pose in full and, the tick its energy is back, its targets
jump to the playing clip (10–18° in one tick after each of a duellist's
blows). L2 left the daze's Defence stance and the stance rule's energy
clause for C10's pose hold, which is now merged. L3 (`strike-with-fists`)
added the other side's attacks after C10 was written; the original starts
them with their own blend.

Phase 4 (Combat), milestone M4 "first duel"; the follow-ups C10 flagged
once it met L2 and L3.

## What Changes

- The set-stance helper starts the new stance with 30 frames of blend,
  smooth; the posture switch with 30 frames, linear (the start's own
  ease); a duellist's stance with 40 frames without a primary striker,
  else twice its primary's stance blend (W19), smooth, its ease set smooth
  on every pass.
- The daze: while a fighter's energy is more than 0.05 under its most
  (compared in double precision, as the pose hold now is too), its body
  tick makes its stance the Defence stance and, unless it is down or
  attacking, starts the Defence stance when another clip plays, with
  round((1.05 − E) × 300) frames of blend, smooth. The fall is decided
  after the daze in the same tick, as the original's posture block
  follows its daze block.
- The player's stance rule gives Cmbt - Idle only when it is not dazed.
- Attacks: the other side starts with round((20 − W19) × 0.6) frames of
  blend (W19 the stance blend of the striker it makes primary), the main
  side with none, both smooth. The swing pace's requirement names the
  primary striker, whose record paces the swing since C10 met L3's two
  fist strikers (the rebase made `pace_swings` read it).

Evidence:
- `exe.asm` of the set-stance helper, the body tick's posture block and
  daze block, the AI's combat-state stance branch, the player's stance
  rule and the side-2 attack starter, with constants read from
  `Exanima.exe` (0.05, 1.05, 300, 0.6 doubles; blends 30, 40, 2·W19);
  `context/re/combat-stances.md` §2, §6.2, §7, §8, §14,
  `context/re/body-misc.md` §2.6, §2.9, `context/re/master-animation.md`
  §2.6, §5.2.
- Exarust runs through `gpu_run.sh` (light settings): `--opponent --trace
  play arenao1` against the C10-only build, `--shove 60 --trace play
  arena`.

Out of scope:
- the stab's and the overhead's start blends (the original's executor
  passes them in from callers not traced) and the Remise chain's
  round(W18 × 6) + 2 (Exarust does not chain attacks yet);
- the walk's end in the stance and every locomotion start (C18);
- the balance's Defence stance and Rebalance starts with the fast ease
  (C11); the parries' blends (L10);
- the AI's other stance writers (recoil, failed parry, clinch clear).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `combat`: the dazed stance, the stance blends and the attack starts are
  added; the swing pace names the primary striker.
- `physical-body`: the pose hold's daze is compared in double precision.

## Impact

`exa_combat::stance` (`set_stance`, `take_ai_stance`, `take_daze`,
`daze_blend`, `StanceTick`, the posture switch), `exa_combat::attack`
(`Attacker::primary_record`, `start_blend`), `exa_combat::start_attack`
(takes the blades), `exa_ai` (the
duellist's stance), `exa_game::player` (the stance rule's daze),
`exa_game::balance` (the fall after the daze), `exa_physics::body`
(`Drive::dazed` in double precision). Docs: `docs/PLAYER.md`,
`docs/formats/RSQ.md`, `docs/PLAN.md`, `crates/exa_combat/README.md`.
