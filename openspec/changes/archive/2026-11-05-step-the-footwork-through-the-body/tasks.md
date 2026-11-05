## 1. Audit

- [x] 1.1 Read the player's step dispatcher, goal routine and queued-step taker, and the ground placement's tail, in the assembly
- [x] 1.2 Map the body's step fields to Exarust's (stepping, balance step, target, kind, queue, start clock)

## 2. Port

- [x] 2.1 Keep the balance-step flag and the queue word on the body's step; take a queued step only while the queue waits
- [x] 2.2 Give the player's footwork the body's step and write it back; start, re-lay and queue on the body
- [x] 2.3 Drop the capsule step velocity, its landing, `Locomotion::stepping` and the combat walk clip
- [x] 2.4 Read the body's episode in the movement gates, the muscles, the lift, the frame's follow and the look-ahead; clear the long kind without a step
- [x] 2.4b Trace the body's step (`--trace` columns)
- [x] 2.5 Restart the chest history, the feet centre and the foot states in the placement on the ground
- [x] 2.6 Unit tests

## 3. Check

- [x] 3.1 No corpus format is read (no `cargo exa` check applies); GPU grid run in exanima01 and the arena
- [x] 3.2 Update docs/PLAYER.md, docs/re/motiledyn-map.md and the exa_physics and exa_game READMEs (PLAN.md is the integrator's)
- [x] 3.3 fmt, clippy -D warnings and tests for exa_physics, exa_game and exa_ai on the worker; play build
