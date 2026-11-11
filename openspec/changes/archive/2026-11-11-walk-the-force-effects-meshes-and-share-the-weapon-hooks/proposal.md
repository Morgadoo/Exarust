## Why

Phase 7 (Thaumaturgy) in docs/PLAN.md, the force2 gap lane (round 169):
TForceBlastEffect, TForceTorrentEffect, SG_Thaum_Force's and
SG_Thaum_DIsp's unit routines and TThaumTree's running-power limit. The
ratings left open the blast's level contact (a rapier swept ball stood in
for the mesh and sampled walks), the force wave's old limb-sphere strike,
the blast's sounds, the torrent's mesh contact, the shared Shell/Strikes
weapon hook slots, the barricade's floor-only probe in the cast path and
the focus item's caller of the running-power limit.

## Evidence

REA (Ghidra on `Exanima.exe` 0.9.5, checked against the assembly):

- The blast's static slot sets the reach scale 2 and the rebound callback,
  then runs the mesh walk (non-piercing) or the sampled walk (piercing).
  The mesh walk tests each live chunk's reach `reach·scale + margin + 10`
  against the mesh's bounding sphere and box and asks the mesh's
  two-sided sphere query with the push-out callback. The sampled walk's
  reach has no extra 10; a chunk that moves at least its reach is swept
  along its move from where it is (the swept query), else the front
  sphere query asks where it is; the sampled callback adds each push to
  the chunk and refreshes the query; a met chunk loses its weight and is
  worn 0.015. The sweep's record never has its end written (the object is
  zero-filled), so the leaf test reads the mesh frame's origin.
- The kd-tree installer sets seven query slots on a mesh and its
  instances; the two-sided sphere test (|s| ≤ r, closest point within
  r² − ε), the front sphere test (0 ≤ s < r, reach r − 1e-6, push or
  normal·r) and the swept test were read leaf by leaf.
- The force wave's class shares the blast's static, part and
  simulated-object slots (the two classes' tables point at the same
  routines).
- The blast's start plays the `thmfrc_cu01` buffer looping at volume 0,
  pitch 1, `length·0.8·r` in; the frame levels it and frees it past a
  charge of 0.9 once thrown; the throw plays the release set on its
  handle at `0.9 + 0.2·r`; the destructor frees both voices.
- The torrent's static and part slots walk each stream's points from the
  head back with the two-sided query and a push-out callback that gathers
  the surface points and pushes (reset per stream); the part slot's limb
  callback divides the gathered surface by the count in place, pushes the
  part at its weights and gathers the impulse; a point met eases toward
  its pushed place and raises the stream loop's volume and pitch.
- Force Strikes' activation sets the body's item-change and strike hook
  slots; Shock Weapons' sets the item-change slot; either's end clears
  both.
- The focus-item check sets the body's slot word from either hand's
  weapon section naming ability type 4 and, when the bit goes, calls the
  routine that tells the running powers past the slots to end.

## What Changes

- The blast's and wave's level contact walks the static grid's level
  meshes; the part contact uses the two-sided query.
- The wave takes the blast's contact slots (level, cloths, parts with the
  character knock), replacing the limb-sphere strike.
- The blast's and bolt's charge loop and release play.
- The torrent's level and part contacts walk the meshes.
- Strikes and Shock Weapons share the body's weapon hook slots.
- The cast barricade's floor probe meets floor nodes only.
- The focus-item check runs live and calls the running-power limit.
