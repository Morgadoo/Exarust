# Design

## Context

Forward locomotion plays Walk. The sampler slerps each bone toward Run by a weight the living sampler passes. That weight is 0.7 times the run blend. The original uses the run blend itself while the sprint row is active: the latch kept, stance level above 0.9 and speed above 3.5 cm a tick. Keyboard Shift spends the latch on a jog before locomotion reads it. The right button beyond 3 m leaves the latch in place.

## Goals / Non-Goals

**Goals:**

- Shift with the cursor further than 3 m keeps the latch. Shift with the keys does not.
- The overlay weight is the run blend while sprinting, and 0.7 times it otherwise.
- A level of 0.9 or a speed of 3.5 cm a tick stays on the jog weight. The comparisons are strict.

**Non-Goals:**

- The sprint gait row, its clock factor `(1 − 0.1·encumbrance)`, and blending the clock linearly in the run blend.
- The run blend's cap of `1 − 0.4·encumbrance` while the latch is held.
- A gamepad sprint, and any change to how fast the capsule moves.
- Playing Walk backwards, gait plants, and the locomoting leg bend.

## Decisions

- The latch is a player flag, set each steering tick. Combat, a fall and a helpless condition clear it.
- The sampler reads the latch, the stance level and the horizontal distance walked last tick (cm). Sprinting is those three gates. The level comparison is in double precision, as the original's is.
- The sampler still receives one weight. The sprint choice does not change the slerp itself.
- A missing Run clip, or a weight of 0, leaves the walk sample unchanged.

## Risks / Trade-offs

- The body still moves at the jog speed while the pose reaches the run clip. The sprint row's thrust is a later slice.
- The walked distance is the controller's horizontal step, not only the part along the heading. A strafe above 3.5 cm a tick can open the speed gate.

## Open Questions

None that change this slice. The sprint row, its clock and the tighter run cap are named non-goals above.
