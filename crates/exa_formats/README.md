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
| `rfc` | scene nodes and static meshes (all retail meshes decode) |
| `material` | material payloads: texture layers and slot roles |
| levels, `rsq`, terrain, factories, actors, powers | Phase 1 |
