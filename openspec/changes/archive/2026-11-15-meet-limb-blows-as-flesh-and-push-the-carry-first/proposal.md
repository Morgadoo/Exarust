## Why

Roadmap task B4 (lane `motiledyn`), the open TMotileDynamics rows of the
function ledger. Two differences against the original's two largest
routines of the class, found by comparing the Rust with the assembly:

- The damage routine branches on the kind of contact the striker
  classified (flesh, a held weapon, a shield) and reads the struck body's
  parry flag only in the weapon branch. Exarust met a blow on a limb with
  the fighter's own guard, so a parrying fighter struck on the arm or the
  body was parried (no wound, a parry lock and sag) instead of wounded.
- The body tick adds the locomotion drive to the carry while the body
  moves, before the feet routine's lift servo and whatever the servo
  finds. Exarust added it after the lift and only with support, so a
  moving body whose pelvis rode over its target lost its push.

## Evidence

The disassembly of Exanima.exe 0.9.5:

- Damage routine: the weapon branch is taken on the striker's pending
  contact kind 2; inside it the struck body's parry byte picks the parry
  branch. Kind 1 (a living limb) runs the flesh path, kind 3 the block.
- Body tick: while the moving byte is set, carry += drive × (((S × S) ×
  push gain) × lag), in that order, with no support test, before the
  balance and feet calls (the feet routine runs the lift servo).

## What Changes

- `exa_combat::react_to_hits`: `Struck::Limb` meets `Guard::Open`.
- `exa_physics::Ragdoll::aim`: the push is added before the lift servo,
  with no support gate (S² is 0 without support).

## Impact

Traces change wherever a parrying fighter is struck on a limb (arena
duels with parries) and wherever a moving body's servo gives no lift
(stairs, steps down, a pelvis riding high). Owed once the GPU is back:
the determinism grid (`arena --opponent`, `exanima01` with W/S holds) and
the grid scenes `play arena --opponent --press LMB@…` and `play exanima01
--at 7.5,-0.95,64.4 --press W@300-420,S@600-700`.
