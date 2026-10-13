# Chunk trees (`.rfc`): meshes, materials, levels

Status: **framing** — the chunk container is understood and walked by
`exa_formats::chunk`; payload layouts below are first observations from
hex dumps and must be confirmed in Phase 1.

## Container

```
u32 magic   0x3D23AFCF   (CF AF 23 3D); 0x3D21AFCF on 3 meshes in Characters.rpk
chunk*      until end of file

chunk:
  u32 tag
  u32 size
  u8  payload[size]
```

List chunks start their payload with `u32 count` followed by exactly `count`
child chunks. Other chunks mix fixed fields with nested chunks.

Tag families (low/high 16 bits) seen so far:

| tag | role | parent |
|---|---|---|
| `0x0000BA00` | material list | root |
| `0x0000BA01` | material | `BA00` |
| `0x3D000000` | mesh list | root |
| `0x00003D03` | mesh (23 127 instances) | `3D000000` |
| `0x00003D01`, `0x00003D06`, `0x00003D0C` | mesh variants (77 / 22 / 13) | `3D000000` |
| `0x00003DC0`..`0x00003DC3` | polygon topology inside a mesh | mesh |
| `0x00003DED` | level grid (dungeon tile map, 600 units per tile) | root (level files) |
| `0x3DE10000` | props | root (level files) |
| `0x0000DBC0` | local items, strings, object placement | root (level files) |
| `0xCAC00001` | characters placed in the level | root (level files) |
| `0x3DD0B001` | collision body | mesh |
| `0xEC00DC00` | ? | root (level files) |

The first word is a `u16 0xAFCF` sanity value followed by `u16` format
version `0x3D23` (`0x3D21`/`0x3D22` are older versions). The `0x3D000000`
list is a scene graph whose nodes include meshes, lights, cameras and voxel
lights. Tag roles beyond the mesh and material lists come from the community
spec (Structs.txt, see `../research/online-research.md`) and are still to be
confirmed against this build.

`cargo exa chunks Resource <file>.rfc` prints the tree.

## Material (`BA01`) — observed

`name[16]`, then RGBA float colours (diffuse, specular, …), scalar
parameters, and texture slot records each carrying a `name[16]` of an image
(`GrassPalA01D`). Material libraries (`.rml`, see OTHER.md) hold the same
records keyed by material name; meshes reference materials by name.

## Mesh (`3D03`) — observed

```
char  name[16]               "Grass01 B"
u32   flags                  e.g. 0x89
u32   ?                      e.g. 0x00031000
f32   transform[...]         identity-like rotation, then position (units ≈ cm)
f32   bounds[...]
u32   ?, ?                   1, 1
stream*                      vertex streams
chunk 3DC0..3DC3             polygon groups (material name, index lists, per-face data)

stream:
  u16 semantic               1 = position, 0x10 = uv?
  u16 type                   0xF3 = float3, 0xF2 = float2
  u32 count
  data[count]
```

Polygon lists appear to be n-gons (counts per face) rather than triangle
lists; triangulation happens at load time.

Character meshes additionally carry skin weights against the humanoid
skeleton (`humanoid.rsq`) and morph targets (`*morph.rfc`).

## Level (`3DED`) — observed

Tile-grid dungeons are assembled from modular sets in `Resource.rpk`:
`gews_*` (wall sets, `TGEWallSet`) and `gefs_*` (floor sets,
`TGEFloorSet`).

```
u32  n_wall_sets;  char name[16] × n     "GEWS_UWA01"
u32  n_floor_sets; char name[16] × n     "GEFS_WodPlA1"
u32  grid_w, grid_h                      32, 32
u32  ?                                   0
u32  cell_size?                          600
u8   cells[grid_w * grid_h][12]          per-cell wall/floor/flags
```

Large outdoor-style levels (`arenao*`, `exanima05..08`) have a companion
`.rft` terrain with the same base name.

## Level and set naming

- Story levels: `exanima01..08`, `exanima67` (+ `_c1`, `_sw` variants),
  `exanimac1`; arenas: `arena`, `arena1..7`, `arenae`, `arenahub`,
  `arenao1..5`; `menuscene` is the title scene.
- Base bodies: `humanbase`, `ogrebase`, `golembase`, `demona01base`,
  `skelbase`, `wraithhabase`, `gobblerbase`, … with `*morph.rfc` morph sets.
