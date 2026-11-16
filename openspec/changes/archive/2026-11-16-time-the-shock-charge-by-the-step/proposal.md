## Why

Lane `bridgeaudit` (finding F549). With the casters' walk running again
(F547) the player's Energy Shock threw its burst about ten frames after the
cast. The charge's rate came from the cast clip's length over 1200 (0.80 a
tick for the player's clip), so the charge passed one on the second
charging tick. The original's Shock creator reads the word 0x58 of the hand
body's scene object, the simulation's step milliseconds (12, the step the
game clock advances by each tick), not a clip: the rate is 12/1200 = 0.01,
and the burst fires on the 101st charging tick, about 1.2 s after the
cast. There is no hold until a release: the charge fires by itself unless a
call clears it.

## What changes

- The Shock's charge reads the simulation's step milliseconds (12); the
  cast clip's length is no longer looked up.

## Evidence

The creator and the class's control routine read from `Exanima.exe`; the
step word is the scene's +0x58 the step clock notes record (12 ms). Worker
GPU runs before and after (`context/artifacts/2026-11-16-bridgeaudit/`).
