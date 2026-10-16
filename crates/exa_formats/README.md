# `exa_formats`

Engine-independent parsers for the formats inside the packs. Each takes
bytes and returns plain data or an error. No Bevy dependency.

**Phase:** 0–1. Spec and status per format: [`docs/formats/`](../../docs/formats/README.md).

| module | status |
|---|---|
| `kind` | format detection by magic for every shipped entry |
| `chunk` | `.rfc` chunk framing and list chunks |
| `rfi` | images: header, mip chain, BC1/3/4/5 + uncompressed decoding, RLE unpacking |
| `rdb` | database index |
| `races` | races: names, flag words (bleeding, red regeneration, claws, hovering, quadruped), body model, movesets, health, natural armour, the body scalars (clip-rate base, size, scale, mass divisor) and the body controller's factors, the gait table with its brake distances and the feral sprint blend, and the stats (unarmed reach, weapon swing-turn and recovery factors, AI attack bonus, perception, held offset, footsteps, voice) |
| `characters` | a level's placed characters (position, facing, condition, pose) and their records read whole, versions 5–7 (level databases and `actors.rpk`): looks, held and worn items, feral level, saved pools, combat skill, traits, experience, the six skills with their techniques, inventory, roles, relations, saved state, powers; the perks the learnt techniques give, the Manoeuvring tier and Deflection |
| `rfc` | scene nodes (meshes, lights, voxel lights; box, rigid body, cloth marker) and static meshes (all retail meshes decode) |
| `rigid_body` | models' rigid bodies: class, mobility, rest tetrahedron, point masses, collision spheres, links (pins, ropes, sliders with their travel), scrape and roll sound slots (every body of every pack decodes) |
| `material` | material payloads: texture layers and slot roles |
| `objdb` | object database items: names, models, occupancy, appearance, protection, weapon, shield, door and operative sections, and the mechanism a record makes of its model |
| `weapon_stats` | weapon stat bytes and how a factory recipe and the item's condition produce them |
| `weapon_record` | the record a held weapon fights with (all 28 elements, the holding pass and the equip refresh), its length along the model's rigid parts, the built-in fists and claws by race, the mass table, the shield record and the step reach |
| levels, `rsq`, terrain, factories, actors, powers | Phase 1 |
