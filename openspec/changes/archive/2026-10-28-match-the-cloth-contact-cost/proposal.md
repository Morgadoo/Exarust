## Why

Round 25 made the level's cloth contacts opt-in: on exanima01 the 14 cloths never slept and their contact sweeps cost about 55 ms a tick. The original keeps the same contacts cheap. Its assembly shows why Exarust's did not:

- The cloth's static stage skips the static mesh of the cloth's own scene node. Exarust handed each banner its own mesh, so every particle sat on a level triangle.
- A level push never wakes a cloth, and a pushed particle slides: its travel loses 0.1 cm, or all of it when shorter. Exarust woke the cloth on every level hit.
- The countdown reads the squared length of the particles' summed velocity. Exarust read the mean squared step, which carries gravity, so a hanging cloth could never count down.
- A sleeping cloth is offered awake bodies only, and each body sphere, padded by 2 cm, must reach the cloth's box. The cloth sweeps a sphere that moved more than 0.01 cm² (in double precision), not 0.1 cm².

## What Changes

- New `exa_physics::cloth_level`: the cloth's level contact, run mesh by mesh after the step. It covers the mesh's sphere and sweep queries, the per-mesh box pre-test, the slide, and no wake.
- `cloth::step`: the sleep, the rustle and the mean drag read the velocity sum. Level planes and level meshes no longer wake the cloth.
- `sector::ObjectContacts`: the cloth's sphere cull and sweep gate. Level triangles come grouped by mesh (`LevelPiece`). `sector::object_takes_body` gives the sleep rule.
- `exa_world::cloth`: skips the cloth's own node's meshes. A sleeping cloth takes awake bodies only, and skips the stage while its frame is still and no body reaches it. Contacts are on by default (`EXA_CLOTH_CONTACTS=0` turns them off).

Phase 3 (bodies) in docs/PLAN.md.

## Evidence

REA assembly of the cloth's static-collider, body-contact, step, sleep and post-step slots and its constructor (padding 2 cm, slide 0.1 cm, flags). Also the sector's object stages (the sleeping object's body mask) and the mesh's sphere and sweep queries.

## Out of scope

The cloth's per-triangle candidate list against a body (the original culls its own triangles by the body's box first). The countdown equalised with the touching body's (Exarust wakes to 1).
