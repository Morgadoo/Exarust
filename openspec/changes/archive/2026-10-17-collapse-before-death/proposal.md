## Why

In Exarust a killing blow sends the body limp at once, with an invented
velocity (half the blade's push and 0.3 m/s up): the corpse lies with its
arms along its sides. The original never kills at once. A lethal wound
starts a 3 s collapse (condition 0x800): the fighter plays Fall Backward
from where it is, slowly, its muscles weakening 0.003 a tick, its lift
fading, its arms loose, its heading wandering; once its balance goes it
throws its arms wide with Rebalance; at a tone of 0.25 it dies and its
ragdoll keeps the pose and the motion it has. Every one of six live retail
collapses ended on the back with the arms flung wide (the user's
screenshots 3 and 7); the world cursor stays the ring through it.

Phase 4 (Combat and items), milestone M4 "first duel", and the master
plan's MS1. This is change L5 of the master plan
(`context/re/master-plan.md`, lane F, F7; `combat-look-plan-final.md`
L5), now that its frame anchor (C9), clip starts (C10) and condition word
(M38) are on main.

## What Changes

- New `exa_combat::collapse`: `start_collapse` (condition collapsing, the
  waking flag cleared, Fall Backward as a looping held pose with
  round((1.2 − E) × 300) frames of blend, ties to even, eased smoothly, E
  the energy the killing blow's daze leaves; the body off its controller);
  the `Collapse` component; the dying tick (`dying_tick`, the collapsing
  branch of the posture stage): the heading's random walk by the
  original's generator, Rebalance once (20 frames, linear) under balance
  0.8 for bodies under size 1.2, tone −0.003 and pins' gain −0.0003 in
  double precision, death at a tone of 0.25 (250 ticks): dead, the muscles
  gone that tick, limp with the motion it has.
- `react_to_hits`: a killing blow on a living body starts the collapse
  (removing Exarust's fall and the attack); a body without muscles dies at
  once; no velocity is invented; a further blow on a collapsing body
  shoves and bleeds but leaves its collapse.
- `exa_physics`: `Bearing::collapsing` (the frame follows the pelvis
  exactly, the lift × tone², the arms 0.2, the feet probed standing, the
  pins' gain not recovering); `Drive::pin_gain` with the living recovery
  (the float 0.08 under the double 0.08: 0.081 after a tick), read by the
  support pins later (C14); `daze_energy`; `GoLimp::velocity` an option
  (none keeps the motion).
- `exa_game`: the bearing's collapsing flag from the condition, the falls
  and the get-up skip the dying, `go_limp` keeps a death's motion; the
  cursor already shows the ring while collapsing.
- `exarust --pause-after-collapse <n>` (debugging): pauses n ticks after
  the first killing blow starts a collapse, for unattended shots of it.

Evidence:
- REA and `exe.asm` on `Exanima.exe` 0.9.5: the collapse start (condition,
  stance 0x11, blend in double, ease 1, the held items, the waking flag),
  the dying tick (two draws, size 1.2 and balance 0.8 in double, Rebalance
  blend 20, tone −0.003 in double, death at 0.5 × 0.5f, pins' gain
  −0.0003 and its reset to 0.01 under 0), the collapse branch of the body
  tick (re-anchor with no reach, arms 0.2, T = tone × s, the lift's × t²
  for `& 0x800`), death (0xA00), the clip start's no-op on a playing clip,
  the pins' gain's start (0.08f) and recovery (under the double 0.08)
  (`context/re/corpses-and-markers.md` §1, `master-body.md` §20–§21).
- The installed `humanoid.rsq`: Fall Backward and Rebalance are looping
  held poses (mode 1); their keys (`corpses-and-markers.md` §1.4).
- Six live retail collapses (`combat-look-plan-final.md` L5): Fall
  Backward at the blow, Rebalance 0.58–1.44 s later, death at +3.00 s, on
  the back with the arms wide.

Out of scope:
- the stagger steps along the chest velocity (the step machinery is C15,
  footwork C16): the target is documented, not taken;
- dropping held items and releasing a grab (L13), the death voice (no
  character voices yet), the lower legs' collision layers (C17), ending
  status effects (no powers);
- the heading's readers (the footwork's stance frame, C12/C16): the
  heading drifts in the `Collapse` component;
- burning's and scripts' collapses (M45, story), knock-out then killed
  (C23), fall death (M44), the dead's fall-death check.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `combat`: a killing blow starts the collapse (modified Blow reactions).
- `character-stats`: the collapse before death (new requirement); a
  killing blow writes collapsing (modified Condition events and readers).
- `physical-body`: the collapsing body's frame, lift, feet and pins (new
  requirement); going limp keeps the motion (modified Going limp); the
  collapsing arms (modified Arm stiffness).
- `world-viewer`: the pause after a collapse (new requirement).

## Impact

- `exa_combat` (`collapse.rs` with `collapse_test.rs`, `lib.rs`)
- `exa_physics` (`body.rs`, `body_muscle.rs`, `body_frame.rs`,
  `body_lift.rs`, `body_balance.rs`: small edits in lane A's files, each
  behind `Bearing::collapsing` or the new fields)
- `exa_game` (`limp.rs`, `balance.rs`, `sim_control.rs`), `exarust`
  (`viewer.rs`: the `--limp` view's velocity; `main.rs`: the pause flag)
- `docs/formats/RSQ.md`, `docs/UI.md`, `docs/PLAN.md`,
  `docs/DEBUGGING.md`, crate READMEs
