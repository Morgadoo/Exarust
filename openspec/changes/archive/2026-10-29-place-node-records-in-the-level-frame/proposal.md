## Why

Round 28 found that a placement's node transform records (`0x3DE0ECAC`) are in the level's frame: the original's placement loader copies each record over its node's world matrix and derives the local rotation from the parent's. Exarust applied them as node-local, so all 1433 such nodes in exanima01 (sacks, buckets, carts, wall rings, chains, mattresses, door leaves) stood 45 to 90 m from where they belong; every one matches the placement composed with the model's own local transform within 1 cm. Applying them in the level's frame was held back because it changed the navigation grid: Exarust stamped every mesh into the grid as solid, while the original treats meshes with rigid bodies differently.

## What Changes

- `exa_world`: a node transform record becomes its node's local transform seen from the parent's frame at the placement, so the node's frame in the level is the record's (`apply_transform_attachments`). A placement tree's entries (`0x3DE0EC00`, none shipped) stay local.
- `exa_world::bodies`: each spawned model's meshes whose node carries a rigid body are marked for the grid (`MovingMesh`), stamped too where the placement makes the node level geometry (every node of a prop placed static, the root of a placed object with the static bits).
- `exa_world::nav`: the build stamps no point of a mesh with a body unless it is level geometry, and every mesh with a body raises the floors under it once the layers are written and before the cost passes (`raise_from_movers`), by the refresh's rule. The floors as built stay.
- The shipped-level corpus numbers change. A closed door's leaf raises its doorway's floor to its top, so exanima01's start rooms are closed by their doors (46 113 cells reached); on the floors as built (every body away) the walk reaches the stair room's foot (195 049 cells).

Phase 3 (bodies) in docs/PLAN.md: the placement of the bodies, and the navigation build's treatment of moving props.

## Evidence

REA assembly of `Exanima.exe`:

- The placement loader's transform record handler: the record is copied over the node's world matrix and the local rotation is the record's rotation times the transposed parent rotation; the local translation is left.
- The grid build's mesh pass walks every sector's registered meshes (registration walks whole subtrees) and skips a node whose second flag word has `0x10` unless its first has `0x80000`.
- The body attach (reached both from the body chunk reader and from an object instance's copy) stores the body on the node, sets the node's body bits and the second flag word's `0x10`.
- After the layers are written the build passes each registered mesh with points, a body of class `0x10000000` and no character mark to the floor raise, over cells 1 to `W − 3` and voxels 0 to 254, before the cost passes.

Corpus: `exa_world::nav::tests::shipped_levels` and the placement records of exanima01.

## Out of scope

- Queuing moving bodies into the grid's refresh as they move (an opened door's doorway stays raised in Exarust until the level loads again).
- The 30 Resource meshes whose file sets the second flag word's `0x10` (stamped as before).
- The GI's centred splat for meshes whose body sets that bit at run time.
