# Design

## Decision

Sprinting is the flag the overlay already stores: the sprint latch, a stance level above 0.9 and a speed above 3.5 cm a tick. While that flag is set and the clip is a forward gait, the clock's run row is the race's sprint rate blended toward its jog rate by the feral level, `sprint·(1 − f) + jog·f`, with f clamped to 0..=1. The walk rate still blends toward that row by the square root of the run blend. The result is then multiplied by the sprint clock factor, 1 − 0.1 times the encumbrance, and by the existing stride term (0.8·speed + 1).

A feral level of 0 uses the sprint rate. A feral level of 1 uses the jog rate, then the encumbrance factor. Without the flag the row stays the jog and the factor is not applied. The backpedal row is unchanged.

The capsule's commanded speed is not read or written here. The clock only decides how fast the walk clip plays.

## Open questions

The movement notes also multiply the clip rate by the encumbrance factor whenever the latch is kept, which could include a latch that has not passed the 3.5 cm gate. The animation clock note applies that factor when sprinting. This slice follows the animation note, so a kept latch below the gate stays on the jog clock. A retail read of that clock would settle the split; it does not block the sprint case.

The height multiplier and the crouch multiplier on the same rate, and the linear blend used in run mode 1, stay out. So does the alignment term that scales a supported stride.

## Risks

A fully feral sprinter's clock matches the jog at no encumbrance, so the pose will not look faster than a jog until the sprint row's thrust exists. The test pins the rates so that wait is visible.
