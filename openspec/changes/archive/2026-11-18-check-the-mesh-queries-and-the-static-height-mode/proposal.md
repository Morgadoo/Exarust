## Why

Lane `collision2` (roadmap B4, the rest of port lane L02 collision &
queries): the mesh's per-face query slots were ported and live but checked
only by unit tests; the cloth's level sweep still built its plane with
glam and recomputed its segment; the placement's Height Query had one mode
where the original has two; the simulation's grid shift and rebuild rows
were open.

## What Changes

- Oracle specs for the mesh's query slots over six face records: the
  two-sided, front, swept and swept nearest queries and the cloth's sweep,
  each in its brute-force and kd-tree leaf form (ten functions), identical
  over 5000 cases each.
- The cloth's level contact takes each triangle's mesh record (normal,
  plane, closest point) and the sweep record's direction and length, as the
  mesh's cloth sweep query reads them.
- The placement on the ground gets the original's static mode: the Height
  Query over the sectors' static meshes only, no body meshes and no rigid
  body stand-ins.
- A test that the level teardown (the scene clear, the only writer of the
  simulation's rebuild flag) puts the scene grid back.
- Findings: the simulation's grid shift is never asked for in 0.9.5 (no
  instruction writes its shift words but the update's own clear).

## Impact

- `exa_physics::cloth_level` (`face_of`, `sphere_push`, `sweep_push`,
  `swept`), `exa_physics::sector_lists::StaticLevel`,
  `exa_game::limp::GroundPlacement::statics`, `exa_oracle` batch g.
- Cloth scenes: the level planes' last bits follow the record builder's
  normal (was glam's normalise), so draped cloth over level geometry may
  differ in the last bits.
