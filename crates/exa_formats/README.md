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
| `rfc` | scene nodes (meshes, lights, voxel lights) and static meshes (all retail meshes decode) |
| `material` | material payloads: texture layers and slot roles |
| levels, `rsq`, terrain, factories, actors, powers | Phase 1 |
