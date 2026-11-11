## 1. Trace

- [x] 1.1 Read add-to-world, the re-scale routine and the ghouls' init against the assembly
- [x] 1.2 Read the muscle setup's smoothing gate and contact move
- [x] 1.3 Read the living step's plain, dead and frozen branches and the rigid step
- [x] 1.4 Trace the player's behaviour tick: auto-target before the attack executor

## 2. Port

- [x] 2.1 Pins times the character size as a body joins the world
- [x] 2.2 Smoothing gated on the body bit
- [x] 2.3 Plain parts of a living body take the rigid step and no pose drive
- [x] 2.4 The player's combat target picked before the starters aim
- [x] 2.5 Unit tests

## 3. Check

- [x] 3.1 fmt, clippy -D warnings and tests on the worker; play build
- [x] 3.2 GPU grid: exanima01, plain, sword and unarmed duels, two runs each, against r176
- [x] 3.3 Update the exa_physics and exa_game READMEs
