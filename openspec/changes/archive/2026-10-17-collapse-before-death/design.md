## Context

The original's damage routine ends a lethal wound in its collapse start;
from then on the body tick runs the collapsing branch instead of the
posture block: the frame re-anchors on the pelvis with no reach, the arms
go to 0.2, the dying tick runs, T is the tone times the stance factor, and
the rest of the tick (clip clock, balance, feet, lift) runs on with the
lift multiplied by the tone squared. Death at a tone of 0.25 makes it a
plain ragdoll. The research notes, with addresses, are git-ignored:
`context/re/corpses-and-markers.md` §1 (the collapse, its timeline, the
clips' poses, why corpses lie as they do), `combat-look-plan-final.md` L5
(the live collapses), `master-body.md` §5.3 (pins' gain) and the new §21
(this change's reading and corrections), `joint-limits.md` §S.5 (b).

The format page this change extends is
[`docs/formats/RSQ.md`](../../../docs/formats/RSQ.md) (Physical body:
Condition, a new The collapse part).

## Goals / Non-Goals

**Goals:**
- the collapse start and the dying tick as the original's, each rule a
  pure function pinned to its number (blend, tone, pins, Rebalance,
  heading);
- the body's collapsing rules (frame, lift, arms, feet) behind one bearing
  flag, so lane A's files change by a test each;
- death in the tick the tone reaches 0.25, the ragdoll keeping its motion.

**Non-Goals:**
- stagger steps (no step machinery yet), item drops (L13), voices, the
  collision layers, status effects.

## Decisions

- **Where it runs.** `dying_tick` sits in the posture stage
  (`PostureTick`, after `track_condition`, which leaves the collapsing
  alone), so it moves with that stage onto M0's. It needs a living body
  (a ragdoll with muscles); a fighter without one dies at once.
- **The start happens at the blow.** `react_to_hits` starts the collapse
  where it learns of the kill (the original's post-step), with the energy
  the blow's daze will leave (`daze_energy`, as the queued shove applies
  it at the next tick), so the blend is the original's. Exarust's `Fallen`
  is removed (a fighter killed while down or getting up collapses, as the
  original's posture block is skipped from then on), and so is the attack.
- **Off the controller.** The body falls off its controller (`fall`): no
  carry, no stance hold, so the blow's momentum and gravity play out, as
  the original's body has no controller. Its balance and feet are still
  measured standing (the probe a collapsing body takes is the standing
  one: the original's down probes are for bit 0x200, which 0x800 lacks).
  The controller only follows the frame, as in a fall.
- **Clip clock.** The original keeps the clip rate the posture block set
  last (most often the standing 1.0 × base, so 0.6 frames a tick);
  Exarust's character tick gives a collapsing body the standing rate.
- **Heading.** The original's dying tick drifts the heading and the stance
  facing, which its footwork reads; Exarust has no footwork yet, so the
  `Collapse` component keeps the drifted heading (from where the body faced
  at the first dying tick), with two draws a tick of the original's
  generator, for the footwork to read. The turn is 0 while helpless (M38).
- **Death.** At a tone of 0.25 the dying tick writes dead, takes the
  muscles in the same tick (`Ragdoll::go_limp`) and asks the frame to go
  limp with no velocity, which strips the clip and the controller.
  `GoLimp::velocity` becomes an option; the `--limp` view keeps its
  velocity, corpses and deaths have none.
- **A pause after the collapse.** Unattended shots of a collapse need a
  pause that follows the kill, whose tick shifts with the load and the
  fight: `--pause-after-collapse <n>` arms the pause n ticks after the
  frame that first finds a body collapsing (the kill is dealt in the
  frame's combat systems), as `--pause-on-hit` does for the player's blow.
- **Pins' gain.** No support pins read it yet (C14), but its rules belong
  to the same tick: it starts at the float 0.08, which is under the double
  0.08 the recovery compares with, so a living body's first tick lifts it
  to 0.081 (a correction to the notes' 0.08); it fades 0.0003 a tick while
  collapsing and is set to 0.01 only once it goes under 0 (not a floor).

## Risks / Trade-offs

- Without the stagger steps the body sinks where it stands instead of
  stumbling along its chest velocity; the blow's momentum still carries
  it. The live Rebalance came 0.58–1.44 s after the blow; Exarust's comes
  when its balance (measured as the original's) falls under 0.8: 1.6–1.9 s
  in the in-game runs at E 0.61–0.63, where the live ones at like energies
  took 0.88 s and 1.42 s; at 1 s the body still squats in Fall Backward's
  lead-in.
- A blow on a collapsing body is resolved as any blow (the original's
  damage routine runs; its collapse start returns at once), so it shoves
  and bleeds.
- Other lanes that insert `GoLimp { velocity: … }` need `Some(…)`.

## Open Questions

- What the stagger step's long steps do to the fall's direction (C15
  lands them; the target is chest + unit(fast chest velocity) × (tone +
  0.5)² × reach × max(balance, 0.4) × √scale): compared with the live
  collapses then (RR1).
- Whether the curled face-down corpse (screenshot 5) is a body killed
  while down: retail request R3 (`corpses-and-markers.md` §5).
