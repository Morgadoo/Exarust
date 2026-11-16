## Why

Roadmap task B4 (lane `motiledyn5`, docs/roadmap/phase-b-close.md; Phase B
close of docs/PLAN.md): the fifth pass over TMotileDynamics. The body
tick's step trigger block set the step radius base by its own rules (65
for an unsupported body, a decay while locomoting, 30 at spawn); a walk's
stop left the jolt's decaying term unseeded; the armed parry took the
attacker's blade tip instead of its point at the parry height; a shield's
parry link had no plane law; the footsteps were one-shots, not the body's
two alternating step voices.

## Evidence

REA (Exanima.exe 0.9.5), checked against the assembly, not only the
pseudo-code:

- The body tick's step-trigger block: the radius base's three writes (30
  with the moving byte, the 0.5 decay over 30 that is not floored, 65 with
  the off-feet bit and no support test), the jolt's 0.85 decay and its
  subtraction (SUBPS), the stop branch's seed (slow velocity × −0.6, MULPS)
  under condition 1 after the jolt block; the character init's and the
  placements' 60.
- The armed grip: the parry height's compare (a NaN gap takes the computing
  branch, a NaN height is kept), the barycentric weights of (0, h, 0) on the
  weapon's rest tetrahedron applied to its particles now and before; the
  shield branch's link flags 0x800, its axis (the frame forward) and its
  strength.
- The link iteration's plain law: with flags 0x800 the offset loses
  (d·u)·0.8 (in double) along the axis before the slack test.
- The balance routine's release footstep and the living sampler's walking
  plant: both flip the body's step-voice index and restart that voice on a
  set variant; the character init allocates the two voices.
- Write-only fields closed as vestigial: the second clock stamp (three
  stores, no load), the get-up's fourth zeroed field (stores only; the one
  other reference is the hardware-config form's own field). The pathfinder
  slot 31 the step trigger calls is `xor al,al; ret`.

## What Changes

- `steps::next_base` takes the off-feet bit and the moving byte; a new
  body's radius base is 60.
- `Ragdoll::walk_stopped` seeds the jolt's term after that tick's jolt;
  the movement stage calls it when a body in the walk's condition stops.
  The movement stage's dead trailing-foot call is removed.
- The armed parry's and the counter hold's point is the attacker's blade
  point at the parry height (`held_blade_point`).
- `GripLinks::aim_parry_plane` and the grip solver's plane law (`PLANE`,
  `GripLink::normal`); `hold_parry_links` lays a shield's link.
- `exa_audio` `StepVoices`: the gait plants and the release steps restart
  the body's two step voices in turn.

## Out of scope

The ground placement's sampler (F545, waits for a GPU check of placed
heights); the weapon-in-hand gate (its points through the hand and frame
matrices are not mapped); condition 0x4000 (no runtime writer; only the
content stream's saved condition word could carry it); the behaviour's
heading snap (slot 23, the AI's and the player controls' components); other
components' rows.

## Impact

Simulation: the step trigger's radius changes for a body that has just
spawned, stood up or walked (more recovery steps are refused while it is
60 cm after a spawn); a stopped walker's next jolt reads the seeded term;
parries aim their weapon at the blade's parry height, and shield parries
pull the shield in the plane across the frame forward. Audio: at most two
footsteps of a body ring at once. Determinism and grid scenes owed (GPU on
hold).
