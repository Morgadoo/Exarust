# Proposal

## Why

docs/PLAN.md Phase 2 (rendering), the RFE_GridEnvs gap lane. The last
rating's gap: the per-cell merged floor and ceiling meshes and the 20 cm
subdivision, the room manager hook, the room reset after build. Exarust
spawned every floor piece of a cell as its own entities, with the floor
UVs re-mapped per piece; the original merges them into one `+Floor` node
per cell, and its build ends by resetting the room states.

Evidence (REA, Ghidra listing and assembly of `Exanima.exe`): the cell
builder (the floor and ceiling entry lists, the `+Floor` node's name,
words and the merge, the `+` surface swap, the planar UV pass with its
1/600 multiply and per-position UV index, the GI point generator at 20),
the ceiling cap, the room-mode build and its tail (shell strips, the
room manager's reset), the room manager hook and its no-op callback.

## What Changes

- A cell's floor pieces are merged into one `+Floor` node at the cell
  centre with one mesh per material run (`FloorMerge`), replacing the
  per-piece floor entities and the per-placement UV cache.
- The grid build ends with the room manager's reset (`hide_all`).

## Out of scope / findings

- The "20 cm subdivision" is the GI point generator called with 20 on the
  merged floor (`RF_MeshVoxels`), already live per cell in `gi_volume`.
- The `+Ceiling` node is marked externally drawn and never drawn; the
  ceilings show as the room lids (unchanged).
- The room manager hook's callback is `mov rax, rcx; ret` (no effect);
  the light-list propagation it runs is `RoomLights::propagate`.
- The non-room build is not reached: every grid level sets room mode.

## Impact

`exa_world` (grid build); docs/formats/RFC.md, exa_world README.
