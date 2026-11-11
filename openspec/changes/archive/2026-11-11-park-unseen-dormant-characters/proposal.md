## Why

Phase 3 (the simulation thread) in docs/PLAN.md: the gap lane for RF_Dynamics (unit routines), Scene content helpers
and T3DNode. The dynamics unit's unmark-tree routine (86 B) was ported as `dynamics::World::clear_marks` with no
caller, and the previous lane found that the AI manager's frame calls it on a "parked" character whose mark Exarust's
body did not keep. This change settles what the mark is against the assembly and runs it live.

## What the original does

REA and `exe.asm` of the character body tick's dormant branch, the mark and unmark tree walks, the sector step's state
masks, the scene culling, the sun's shadow pass, the AI manager's frame and the golem wake:

- **The mark.** A body's state bit 4. The mark walk sets it on the body of a node and of every node under it; the
  unmark walk (the dynamics unit's) clears it the same way. The mark walk's only caller is the body tick; the unmark
  walk's are the AI manager's frame and the golem wake.
- **The body tick's dormant branch** (after the daze block): while the character's pose word has its dormant bits
  (`& 6`: the value 2 the AI's sleep, the statue sentry and the demon summon write), it sets the root's link
  iterations to 5 and reads its node's render-visibility word. With none of the low 24 bits set (no view drew the node
  in the last frame) it raises the pose word's bit 8 and, unless the root's own mark is set, marks the node tree. With
  any of them set it clears bit 8. Outside the branch bit 8 is cleared too.
- **The visibility word** is written each frame by the scene culling (`0x1000001` for a node in view, cleared when it
  is not) and by the sun's shadow pass (`0x80000000` for a node in the shadow volume).
- **The state masks.** The sector step integrates a body only when `state & 7` is under 3, so bit 4 keeps it out; such
  a body (not deep) rolls its particle sets (newest to previous, previous to oldest) and takes one off its countdown,
  or sets its deep bit once the countdown is under 1. The pair, link and contact passes test `0x804`, `0x805`, `0x105`
  and `0x15`: a marked body meets nothing. A wake clears the sleep bits, not bit 4.
- **The manager's frame** (the simulation's pre-step hook), for each character of the scene before its rise: when the
  root's own mark is set and its node's word has `0x81000000` (drawn, or in the sun's shadow), it unmarks the node tree.
- **The golem wake** (the alarm's callback) wakes the body to 256, then unmarks its node tree, then fades its glow.

So a dormant character nothing draws is taken out of the simulation where it lies, and comes back when it is seen.

## What Changes

- `exa_physics::body::Ragdoll` keeps the mark: `park`, `unpark`, `parked`, `park_unseen` (the dormant branch) and the
  parked tick in `step_bodies` (out of the solver: no integration, no stage; the roll and the countdown, then deep).
- `exa_physics::body::InView`: whether the renderer drew the body last frame, bridged to the simulation.
- `exa_game::park`: the frame's note (any mesh under the body among an active 3D camera's visible meshes, read in
  `First`, written only for dormant or parked bodies and only when it changes), the manager frame's wake in `SimPreStep` before the rise,
  `EXA_NO_PARK=1` to turn the note off.
- The posture stage calls `park_unseen` after the body's timers; the golem wake unparks after its wake.

## Out of scope

- The pose word's bit 8: its readers (the dormant lift lead, which compares the word with 2) only change a parked
  body's tick, which the solver leaves out.
- The culling's own test: Exarust asks Bevy's camera culling. The manager's sun-shadow bit: a body only the sun's
  shadow pass saw is unmarked by the frame and marked again by its body tick in the same step, before the solver, so
  it stays out; Exarust leaves it parked. A body never noted yet counts as drawn (the original's word starts at 0).
- Marks of other bodies under the character's node: Exarust's character is one body; held items are their own
  bodies, joined by grips. `dynamics::World::clear_marks` stays the unit model's.
- Scene content helpers: the reverb decay offset is applied (`exa_audio::reverb`, 1.49 s plus the offset), so that
  gap was stale; nothing else of the cluster or of T3DNode is ported here.
