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
| `characters` | a level's placed characters (position, facing, condition, pose) and their records read whole, versions 5–7 (level databases and `actors.rpk`): looks, held and worn items, feral level, saved pools, combat skill, traits, experience, the six skills with their techniques, inventory, roles, relations, saved state, powers; the perks the learnt techniques give, the Manoeuvring tier and Deflection; an actor's own item database |
| `character_stats` | the stats a record and its race spawn with, as the original's builder and init derive them: body scale and size by height, standing pelvis, leg reach, most energy, step reach, support pins' factor, the undead's head gain and factor and walking arms, sprint gait, perception, voice, maximum health with the race-byte rules, health from the saved pools, the body rule bits; Deflection and the encumbrance the armour pass sums |
| `encumbrance` | a character's encumbrance as the original's armour pass sums it (worn items' bytes, articulated pieces, Manoeuvring tiers, the floor, the held shield with Mobility, the golem suit, the cap) from a record and the databases its ids name, and the factors it gives the reach, run, sprint, turn, steps and get-up clock |
| `rfc` | scene nodes (meshes, lights, voxel lights; box, rigid body, cloth marker, the static node flag, keyframe tracks after any kind's body, a mesh's morph set) and static meshes with their position-stream count and smoothing keys (all retail meshes decode) |
| `morph` | morph sets (`*morph.rfc`: landmarks, base positions, targets; both shipped sets decode), the physique weights a record gives them, the transfer to body parts and garments (nearest surface within 10 cm, the nearest landmark's offset, neighbour fill) and morphing a scene's meshes, bounds and joints |
| `key_track` | node keyframe tracks (`0x3AC1`): play mode, duration, keys of time and position (the one shipped track decodes) |
| `rigid_body` | models' rigid bodies: class, mobility, rest tetrahedron, point masses, collision spheres, links (pins, ropes, sliders with their travel), scrape and roll sound slots (every body of every pack decodes) |
| `material` | material payloads: shading parameters, texture layers and slot roles, the physical divisor a rigid body's mobility is divided by and the impact sound prefix |
| `objdb` | object database items: names, models, occupancy, appearance, protection, what a worn item weighs (encumbrance, covered bones, articulated set, special kind, powered), weapon, shield, door and operative sections, and the mechanism a record makes of its model |
| `weapon_stats` | weapon stat bytes and how a factory recipe and the item's condition produce them |
| `weapon_record` | the record a held weapon fights with (all 28 elements, the holding pass and the equip refresh), its length along the model's rigid parts, the built-in fists and claws by race, the race's weapon fields from its decoded stats, the mass table, the shield record and the step reach |
| `rsq` | skeleton, bodies and motion set of `humanoid.rsq`; the moveset tree (parents are the nearest earlier moveset of their name) and a character's clip table from a base moveset and the held items' config word (`MotionSet::assemble`, `MovesetConfig`: the weapon moveset under each ancestor, then the Torch and Flaming overlays) |
| levels, terrain, factories, actors, powers | Phase 1 |
