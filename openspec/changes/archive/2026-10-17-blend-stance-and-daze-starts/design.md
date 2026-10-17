## Context

C10 gave `Animator::start` a blend and an ease and a capture taken at the
body's next tick; `play_loop` and `play_once` start with blend 0 and the
linear ease. The original's start (`start(motion, blend, restart)`) does
nothing when the motion already plays, sets the ease to 0 itself, and its
callers write the ease (1 smooth, 2 fast) after it. Where the callers
stand:

| caller | blend | ease after |
|---|---|---|
| set-stance helper, only when the old stance's clip plays | 30 | 1 |
| posture block, a balanced body in another looping clip | 30 | none (0) |
| AI combat state, every pass unless parrying, in Break Tangle or Switch Weapons | 2·W19 of the primary, 40 without | 1, written even when the start did nothing |
| daze block, energy < most − 0.05 (doubles), not down, collapsing, dead, timed or attacking | round((1.05 − E)·300) (cvtsd2si) | 1, written even when the Defence stance plays already |
| side-1 starter | 0 | 1 |
| side-2 starter (fresh) | round((20 − W19)·0.6) of the new primary | 1 |
| stab, overhead starters | the caller's | 1 |

The daze block also writes the stance (3) every dazed tick, down or
attacking too, and runs first in the body tick, before the behaviour
(player input, AI) and the posture block (falls, staggers, the switch).
The player's stance rule gives Cmbt - Idle only when E ≥ most − 0.05 (in
doubles). Links: [formats/RSQ.md](../../../../docs/formats/RSQ.md) (Motion
playback, Stance starts), [PLAYER.md](../../../../docs/PLAYER.md) (The
combat stance, Combat mode).

## Goals / Non-Goals

**Goals:** every stance start and the side attacks with the original's
blend and ease; the daze's Defence stance and the stance rule's energy
clause, so a dazed body's targets blend back instead of jumping.

**Non-Goals:** the stab's and overhead's blends, Remise, locomotion
starts (C18), the balance's Defence stance and Rebalance (C11), parries
(L10).

## Decisions

- **Starts through `Animator::start`.** The helper, the switch, the AI and
  the daze call `start` with their blend and ease; `play_loop` stays the
  plain start for Exarust's interim footwork and gaits. A start's "does
  nothing when it plays" is each caller's check (the helper's old-stance
  test, `switches_posture`, the AI's and the daze's clip test).
- **The ease written without a start.** The AI and the daze set the ease
  smooth on every pass even when nothing starts, as the original writes
  it; Exarust's ease only acts while a clip still blends from its
  capture, which matches the original (its rebase sets it to 0).
- **Where the daze runs.** In the body tick's stance part
  (`take_stance`, before the clips advance), after readiness (the
  original's readiness comes earlier in its tick too). The fall is ordered
  after it (`StanceTick`), so a body dazed and falling in one tick plays
  its fall, as in the original; without the order the daze's start
  replaced the fall clip and the body lay in Idle. The player's behaviour
  runs before the body tick in Exarust (the original: after its daze
  block); out of combat mode the helper and the daze then restart Idle and
  the Defence stance in turn every dazed tick, in the original's order
  ending in Idle, in Exarust's in the Defence stance. Both freeze the
  pose at the capture while the pose hold keeps the body as it is; the
  first tick out of the daze starts Idle from where the body is in both.
- **Double precision.** `Drive::dazed` compares E with most − 0.05 in
  doubles (0.95f is dazed), as the daze block, the pose hold and the
  stance rule do.
- **The primary striker.** `Attacker::primary_record` finds the blade in
  the primary slot; the cooldown, the AI's stance blend and the side-2
  blend read it. The swing (`pace_swings`) reads the same slot since the
  rebase of C10 onto L3 (it took whichever blade came last, a Harvester's
  claw or fist at random); before any attack the first slot paces it.

## Open questions

- The stab's and the overhead's blends: the executor's callers of their
  starters pass them in; not traced (REA on the executor's callers).
- The original's player behaviour runs after the daze block; whether its
  exploration steering really restarts Idle every dazed tick out of
  combat (as read from code) was not seen at runtime (a retail capture of
  a dazed player out of combat mode).

## Risks / Trade-offs

- A dazed player stepping in combat mode has Exarust's interim `Combat
  Walk` and the daze's Defence stance restart each other every tick; the
  pose hold keeps the body as it is meanwhile, and the footwork gives the
  stance back once the keys are released.
- Readiness now drops to 0 while dazed (the stance is the Defence stance),
  as the original's: in the arenao1 duel the duellist killed the player in
  six blows (eight with C10 alone, at readiness 1 each).
