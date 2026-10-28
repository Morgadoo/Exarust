# Design

Rules: [`crates/exa_character/README.md`](../../../crates/exa_character/README.md)
(`pose`, `runtime`) and [`crates/exa_physics/README.md`](../../../crates/exa_physics/README.md)
(sleep).

## Decisions

- The restores stay faithful (world frame from the record, local rows only);
  `pose::place` then derives every scene node's Bevy local from the restored
  world frames, since Bevy places nodes by their locals.
- Records are matched among the model's part nodes; a plain node between two
  parts is folded into the child's rest frame (a guess; the human and
  skeleton bodies have none).
- The character frame keeps its built orientation; only its translation is
  taken from the restored root, as the original sets only the translation.
- A placed corpse has no `Animator`, which is what "never sampled" means
  here; `exa_anim::SkinnedRoot` lets its skin bind without one.
- `register_placed` leaves the body awake with countdown 0; the resting band
  takes it to the floor of 1 and the post-step puts it to sleep, as the
  original's first tick does.

## Not wired

Listed in the proposal's out-of-scope paragraph.
