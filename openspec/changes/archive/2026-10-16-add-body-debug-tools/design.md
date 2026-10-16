## Context

The body solver ([`docs/formats/RSQ.md`](../../../docs/formats/RSQ.md),
Physical body) keeps each part as four particles, each joint as the
skeleton's links plus an origin pivot, and in a living body each slack
link's muscle anchor (weights on the parent's particles). Parts are drawn
between their last two ticks (`Part::drawn`). The fixed step runs the
character controllers, the character ticks and the solver at 83 Hz on
Bevy's virtual time; the existing `--trace` writes one row per body per
tick in the retail probe's columns.

This change adds no rule of the original: it measures and shows the
solver's own state, and lets the simulation be stopped. No reverse
engineering question is open; the one format page touched is the per-tick
trace table in `docs/formats/RSQ.md`.

## Goals / Non-Goals

**Goals:**
- see every body as the solver sees it, lined up with its mesh;
- measure each joint in terms that tell a bend from a fold: its turn
  from the rest split into swing and twist, its links against their
  slack, and how far it is from the clip;
- a per-tick, per-part file of the same, to find the tick a fold starts;
- stop, step and slow the simulation, by key and by flag, and take
  pictures of the stopped body.

**Non-Goals:**
- changing any body behaviour; replaying inputs; colliders and contacts
  on screen.

## Decisions

- **Measures live in `exa_physics::body::inspect`.** They read private
  link data (the points' weights, anchors), so they sit beside the solver
  as a child module, like the muscles; nothing calls them unless a tool
  is on.
- **The rest is the model's rest pose.** Each part's turn relative to its
  parent part in the body model is stored when the body is built
  (`Ragdoll::with_rest_turns`, identity when not given, so tests and
  hand-built bodies keep working). The joint's turn is rest⁻¹ · now, in
  the part's own space. The shipped skeleton's links are not centred on
  the rest (the elbow's flexion link is centred 60° from the straight
  arm, at the end of its slack there), so the range column, read from the
  links, carries the joint's limits and the angles carry its pose.
- **Swing and twist about the bone axis.** The axis runs from the part's
  origin to the mean of its children's origins, or its centre for a leaf
  (hand, foot, head). The twist is the turn's projection onto the axis;
  the swing is the rest, and its axis is reported so a bend and its
  reverse differ. Angles use atan2 of the vector part and the scalar part,
  exact near zero.
- **Stretch bands**: within up to 1 mm past the slack (a standing body's
  pivots stay 0–1 mm open from the solver's softness, a living body's
  feet up to 3 mm against the floor), yellow to 1 cm, red beyond, as the
  request asked.
- **Pause through virtual time.** Pausing `Time<Virtual>` stops the fixed
  loop without touching any system; a step adds one timestep to the fixed
  overstep before the loop (so exactly one tick runs and the drawn bodies
  keep their interpolation). A scripted pause pauses at the end of its
  tick and drops the whole timesteps left in the frame. The chase and fly
  cameras move on real time so they keep working while paused or slowed.
- **Gizmos and Bevy UI text.** Two gizmo groups (thick links, thin
  tetrahedra) drawn over the scene; the readout uses Bevy's default
  monospace font (the game's bitmap font has no fixed advance).
- **Joint trace rows** for every simulated body not in deep sleep, flushed
  each second like the body trace.

## Risks / Trade-offs

- Enabling the gizmo plugin adds its (empty) per-frame bookkeeping even
  with the overlay off; measured below the noise.
- The chase camera on real time eases over a long frame in full instead
  of the virtual clock's 48 ms cap; only visible after a hitch.

## Open Questions

(none)
