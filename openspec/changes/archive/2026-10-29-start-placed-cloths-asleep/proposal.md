## Why

The round 28 GPU run with face-flag pins still showed five of exanima01's fourteen banners awake: countdown 32, a velocity sum² of 0.9 to 17, and pushes from sleeping wall rings ('MetalRing', countdown 0). One banner was pushed only by a mattress part ('B007'). Two departures from the original put them there.

- **Saved positions are in the level's frame.** A placement's mesh-position record (`0x3DE0ECDB`, the banner's saved cloth) and its node transform records (`0x3DE0ECAC`) hold coordinates in the level's frame. Every shipped position list's box sits about its placement. Every exanima01 transform record (69 wall rings, 156 sacks, 55 buckets, 53 chains, 18 carts, the mattresses) matches the placement composed with the model's own local transform to within 1 cm. Exarust read both as node-local. So each overridden node landed 45 to 90 m away. The banners turned half round, with the rings and mattresses turned the same way, all landed in one heap near the world origin, where each banner met eleven rings.
- **A placed cloth starts asleep.** The original's placement loader copies the position record's word into the cloth controller's countdown, and marks the cloth asleep when the word is under 1. Every shipped record stores 0. A sleeping cloth meets only awake bodies, so the sleeping props a banner hangs among never push it. Exarust started every cloth awake at 32.

## What Changes

- `exa_world`: a position record is taken from the level's frame into its node's own frame at the placement (`level_to_node`, `node_world`).
- `exa_world`: a position record's word starts the cloth's countdown (`ClothStartSleep`, `start_countdown`: asleep under 1).
- The transform records stay local, as a labelled known divergence. Reading them in the level's frame drops the navigation grid's reach from exanima01's start from 191 328 to 2713 cells, and the walk no longer reaches the stair room. That needs its own change, with the navigation build's treatment of moving props.

The contacts stay opt-in until the GPU check passes.

Phase 3 (bodies) in docs/PLAN.md.

## Evidence

REA assembly of the placement loader's record handlers:

- The transform record is copied into the node's world matrix, and the local matrix is derived from the parent's.
- The position record replaces the mesh's position stream when the counts match. Whatever the count, when the node's cloth controller carries the cloth class bits, the record's word goes into the controller's countdown, with the asleep bit set under 1.
- The cloth constructor starts an unplaced cloth at 32.

The data: every placement record in exanima01 and arena, compared with the models' node transforms.
