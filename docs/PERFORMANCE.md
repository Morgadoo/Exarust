# Performance

How Exarust keeps its frame rate without changing what the original shows.
The rule: an optimisation may skip work, reuse work or reorder work, but the
picture must stay the original's. Changing LOD, draw distance, resolution or
temporal reconstruction would change it, and the original uses none of them.

Measurements: the exanima01 log room, 2464 × 1600 window, 4× supersampled,
RTX 3090, optimised build (`--profile play`), `--no-vsync`. The original
shows about 31 FPS at the same spot on the same machine.

## Where the frame goes

| configuration | FPS |
|---|---|
| everything, before shadow links (decayed after ~30 s, ~12 GB RAM) | ~1.3 |
| shadow links (per light, render layers) | ~6 under load |
| + lights stop at walls | ~12–13 under the same load |
| shadows off | ~245 |
| shadows off, 4× MSAA on top of supersampling (now removed) | ~195 |

Point-light shadows dominate. Everything else together costs a few
milliseconds.

## Techniques against Exarust

Status: **done**, **next**, **later**, or **no** (would change the picture,
or the original does not do it).

### Visibility

| technique | status | notes |
|---|---|---|
| Frustum culling | done | Bevy culls every mesh by its box per view, and point lights by their sphere. |
| Back-face culling | done | Per material; two-sided materials draw both faces, as in the original. |
| Portal / cell culling | later | The original draws the camera's room, plus the next one when within ~2 m of an open door, and rooms one door further only through the door's clipped frustum with a scissor. Nothing further is drawn. It saves draw calls behind walls that frustum culling keeps. |
| Occlusion culling | later | Bevy's GPU two-phase occlusion culling (Hi-Z from the previous frame) needs a depth prepass. It skips nothing visible, so the picture stays the same. The original has none; this is a pure gain. |
| Distance / detail culling | no | The original has no draw distance and no mesh LOD indoors. |
| GPU-driven culling | done | Bevy's GPU preprocessing culls and builds indirect draws per view. |

### Geometry

| technique | status | notes |
|---|---|---|
| LOD, impostors | no | The original has none; swapping meshes would change silhouettes. |
| Instancing / batching | done | Bevy batches meshes that share mesh and material (grid pieces, repeated props) into multi-draw indirect calls. |
| Spatial structures | done | The light flood uses the 6 m wall grid. Lights link to meshes by sphere first. |

### Shading and lighting

| technique | status | notes |
|---|---|---|
| Clustered lighting | done | Bevy's clusters, read by Exarust's own shading. |
| Lights per object | done | At most the six strongest links per mesh, as in the original. |
| Lights stop at walls | done | The original's flood through the wall grid bounds each light's casters to its rooms. |
| Shadow caching | next | The original renders a cube face only when it holds a visible receiver and its light or a caster in it moved; flicker never dirties it. A static dungeon renders almost no cube faces per frame. Bevy re-renders every face every frame, so this needs Exarust's own cube pass for static casters. Characters stay on Bevy's shadows. |
| Shadow tap split | done | Under supersampling each sub-frame takes 2 of the 8 shadow and GI taps, as in the original: a quarter of the filter work. |
| Depth prepass / early Z | later | The surface shader is heavy: parallax, GI taps, six lights with 2 shadow taps each. A depth prepass would shade each pixel once in cluttered rooms. It is also what occlusion culling needs. |
| Shader specialisation | later | Material features are runtime flags today. Shader variants per flag set would drop dead branches; the image is the same. |
| Baked lighting | no | The original computes its voxel GI live (one propagation step per frame). |
| SSAO / SSR, VRS | no | Not in the original; VRS would lower shading detail. |

### Resolution and temporal reuse

| technique | status | notes |
|---|---|---|
| Supersampling 4× | kept | The original's own setting. MSAA on top of it was removed (`HWMultisample = 0` in the original). |
| Upscaling (DLSS, FSR), TAA, frame generation, dynamic resolution | no | They change the image and the original's look. They could be offered later as options, off by default. |

### Memory and simulation

| technique | status | notes |
|---|---|---|
| Texture compression (BC) | done | BC textures upload as BC. |
| Mipmaps | done | Every stored level is uploaded. Terrain reads its base level on purpose, as the original's mip-less composites show. |
| Texture streaming | later | The original decodes textures on a loader thread with a placeholder until ready. That shortens loads, not frames. |
| Fixed-step simulation | done | Physics, AI and particles tick at 83 Hz, as in the original. |
| Physics sleeping, AI throttling | later | The original picks targets for one AI per tick. |

## Order of work

1. Shadow caching: Exarust's own cube pass for static casters, rendering only
   needed and dirty faces.
2. Depth prepass, then Bevy's GPU occlusion culling.
3. Room and portal culling from the grid's rooms and doors.
4. Shader specialisation of the surface materials.

Each step is checked against the original at the log room camera. The
picture must not change, and the frame rate is measured with `--no-vsync`.
