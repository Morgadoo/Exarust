## Why

Round 90's check held free mode: a dead character stood upright in free mode (lying in lockstep). The frame's
go-limp removed the body's clip player (and its ticked-clips marker); step packets in flight still carried the clip
player and put it back on the frame, where the frame's clip player then ran it and sent it back to the simulation,
whose muscles-off sampler stood the corpse up. The end-of-session menu also held the simulation, freezing the world
behind it. docs/PLAN.md Phase 3, the simulation thread.

Evidence: the round 90 grid (arena1 duel, the player killed); `EXA_BRIDGE_WRITES=Animator` logs of the frame's writes;
REA: the pause-counter raises attributed to the end-session menu are the arena's next-fight setup (it rebuilds the
opponent, music and scene) and its leave routine, not the menu's display.

## What Changes

- Free mode: each side numbers when it removed a component (frame packet number, simulation step); a value the
  other side sent before it had seen that removal does not put the component back.
- Free mode: a frame value still equal to what the simulation last sent (only touched) is not sent back.
- The end-of-session menu no longer holds the simulation.
- `EXA_BRIDGE_WRITES=<type>`: logs the first frame writes of a type that met a newer simulation value.

Rule change: none in lockstep (traces identical to main).

## Out of scope

- Field-level merging of values both sides write.
