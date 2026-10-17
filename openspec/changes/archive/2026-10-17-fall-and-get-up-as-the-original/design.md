## Context

The original's body tick runs a posture block after the behaviour update:
skipped for the collapsing (their dying tick runs instead, and a body it
kills leaves the tick) and the shocked; entered when the balance is under
0.6 or the condition is getting up. It sets the clip rate (1.25 × base, or
the get-up's (1.5 − 0.5·encumbrance) × base) and turns locomotion off; for
a getting-up body that is all. The rate is set on entry, so a get-up the
block itself starts (from the floor, or a stagger) runs its first tick at
1.25 × base (0.75 frames at full tone), as Exarust's clock does through
the posture stage's branch. Otherwise it decides down (under 0.3) and
then either the get-up start (lying still) or a response from the lean:
the Defence stance, Rebalance, a stagger or a fall. Every branch ends at a
common point that takes one off the posture-switch countdown. The
standing branches (balance 0.6 or more) hold the weak Defence stance and,
for a plain clip, the unsteady Defence stance or Rebalance and the posture
switch. The movement section later picks the get-up again while it is
under 40 % and turns the heading toward a fall under 0.8. The research
notes, with addresses, are git-ignored: `context/re/body-feet.md` §10 and
§16 (this change's reading), `body-misc.md` §2.4–2.6, `body-drive.md` §8
and §15, `master-body.md` §3, `master-animation.md` §3.

The format page this change extends is
[`docs/formats/RSQ.md`](../../../docs/formats/RSQ.md) (Physical body: a new
Falls and get-ups part before Condition; Motion playback).

## Goals / Non-Goals

**Goals:**
- every branch of the posture block that Exarust can reach, as the
  original's, pinned to its numbers and its float-in-double compares;
- the stagger's jump in, quirk included (its capture dropped at once, the
  get-up's first key held);
- the condition, the dying tick, the falls and get-ups, the fall check and
  the health bookkeeping in each character's own stages, in the original's
  order, a body killed in its posture stage leaving its tick.

**Non-Goals:**
- recovery steps, foot contact states, support pins (C15, C12, C14);
- knock-outs, dormancy, shocks (nothing sets their flags yet);
- the retail knockdown capture (RR7).

## Decisions

- **Where the rules live.** The pure rules and the posture actions in
  `exa_game::balance` (lane A), which knows the player's body (its yaw and
  heading turn with a get-up) and the controller glue; the condition and
  the dying tick stay in `exa_combat` (lane F), registered into
  `CharacterStage::Posture` themselves. Order inside the stage: condition,
  dying tick, posture actions, fall check, bearing.
- **Branches, not flags.** The posture actions read the branch
  `track_condition` already names (`PostureBranch`), so the original's
  tests are made once. The bearing's off balance is that branch (falling,
  lying still, getting up), its getting up the condition.
- **Starts that do nothing.** The original's clip start does nothing when
  the clip already plays; so do the posture starts, and the whole capture
  is asked only when a clip started (Exarust's capture would otherwise be
  used as the next sample's source). The fast ease is set every tick in
  the falls branch, as the original sets it, even with no start.
- **The jump in.** `Animator` gains the key its capture is meant for
  (`capture_key`, 0 after every start); `jump_in` sets it and the clock to
  the key before. The sampler takes the capture only while it aims at that
  key; otherwise it rebases as it already did. That is the original's test
  (target key index against the stored one), and it gives the stagger's
  quirk without special cases.
- **The daze's gate.** The daze's Defence start is gated by the condition
  alone (helpless or attacking), as the original's daze block is. A dazed
  body still falling (balance 0.3–0.6) therefore has its clip restarted
  every tick by the posture stage after the daze's start: kept, as read
  (seen in runs: the Rebalance clock stays at 0.6/0.75 until down).
- **The off-feet mark.** `Fallen` stays as Exarust's marker for its
  steering, attack requests, the duellist and the stance (a component
  inserted and removed by the posture stage with the branch), so lane D's
  `player.rs` and `exa_ai` need no edit. Its fields (time, getting up) go.
- **The controller.** Exarust's capsule still carries a standing body. A
  fall, or lying still, takes the body off it (no carry, no hold); a
  stagger, a get-up or a standing branch puts it back. A heading turn
  (get-up, toward a fall) drops the carry's reference root once
  (`Ragdoll::turned`), so the turn reaches the frame without rotating the
  body.
- **The Rebalance's wait** lives on the drive (`Drive::posture_wait`): the
  posture switch (`exa_combat::stance`, before the character ticks) reads
  last tick's value, the same value the original's standing branch reads
  before its decrement; no new required component on `Fighter` (lane C
  edits that list).
- **Health.** A ticked living character keeps its health in its upkeep
  (red regeneration, before the original's dead test) and attack stages;
  the fixed-step `regenerate_health` keeps doing everything for the
  fighters the driver does not tick (the dead, any without a living body),
  by the driver's own test (`ticked_by_its_character`).
- **A body killed mid-tick.** `ticked_lives` (muscles kept, condition not
  dead) is a run condition on the attack, movement, sampler and carry sets
  (configured by the driver's plugin) and on `keep_health`, so a body its
  dying tick kills runs nothing more, as the original's collapse branch
  jumps to the end; a body that falls to its death is dead too (the
  original runs its tick on, harmless).
- **Energy while getting up.** In the readiness stage by last tick's
  condition and progress, on top of the solver step's +0.005 and its cap
  (the original adds both in one double expression, then caps in the daze
  block: the same value to float rounding).

## Risks / Trade-offs

- Systems ordered `.after(take_posture)` across crates: the stage's order
  is explicit (condition → dying tick → posture → fall check → bearing).
- M22's `run_swings` (in flight) orders itself `.after(PostureTick)`; the
  set stays (empty), so it compiles, but it now runs before this tick's
  posture decision: it should move into `CharacterStage::Attack`.
- The character view's figure (no condition word) takes no posture now.

## Open questions

- RB1 (Q11): answered from the binary — the stagger plays Get Up 2 when
  the lean is not ahead (lean · forward ≤ 0, or not comparable), Get up
  when it is; `body-misc.md` §2.4 was right, `body-feet.md` §10 had them
  swapped. The routine the get-up start calls clears the feet's contact
  flags (the stagger does not call it).
- RR7 (retail knockdown and get-up timing) remains the live check.
