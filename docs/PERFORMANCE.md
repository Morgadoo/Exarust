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
| + cached cube shadows for static casters | ~200–210 (quiet machine) |
| shadows off | ~245 |
| shadows off, 4× MSAA on top of supersampling (now removed) | ~195 |

Point-light shadows dominated until they were cached. With the cached
cubes the log room runs at about 200 FPS, six times the original's frame
rate at the same spot. Play mode, with 39 characters animated in the level,
is bound by the CPU (about 9 FPS) and is next.

## Frame profiler

Three tools, from a quick look to a full timeline:

- **`--profiler`** (F10 toggles its overlay) and **`--perf-log <seconds>`**
  (the same report in the log). Each frame's main thread is split by
  schedule: First, PreUpdate, the fixed 83 Hz ticks (with how many ran),
  Update, PostUpdate, Last, and the rest (extracting into the render
  world, presenting, waiting). The render thread's schedule time sits
  beside it, then Bevy's GPU timestamps per pass and Exarust's own GPU
  spans (`exa_cube_shadows`, `exa_gi`). The overlay graphs the last 120
  frames: green within 60 FPS, yellow within 30, red beyond. Limits of
  Bevy 0.20's GPU timings: a pass is keyed by name only, so the
  supersampled scene and the window's display share a line, and spans
  around whole views read as nothing. Overall GPU load is `nvidia-smi`'s
  (`nvidia-smi dmon -s u`).
- **Tracing builds**: `cargo build -p exarust --profile profiling --features
  trace-chrome` writes a `trace-*.json` of every system, schedule and render
  step into the working directory. Open it in <https://ui.perfetto.dev>,
  or summarise it with `context/tools/trace_summary.py trace.json
  --skip-seconds 40` (ms per frame for each span, per thread). With
  `--features trace-tracy` the same spans stream to the Tracy profiler
  instead. Tracing roughly doubles the frame time; read proportions, not
  absolute times. The `profiling` profile is the play build with line
  tables and symbols kept, also for `perf` or samply where the system
  allows them.

## Play mode, 2026-10-20

`play exanima01` at the start, 1600 × 900 window, 4× supersampling, RTX 3090,
`--profile profiling`, `--no-vsync`:

| configuration | FPS |
|---|---|
| everything | 26–32 |
| `--no-supersample` | 33 |
| `--no-shadows` | 55 |
| `--no-gi` | 27 |
| all three off | 60 |

The profiler's split of a 31.7 ms frame:

- Main thread, 29.2 ms of schedules: **the fixed ticks 19.5 ms** (2.6 ticks
  a frame, ~7.5 ms a tick) and **PostUpdate 8.5 ms**; the rest 2.4 ms.
- Render thread 28.9 ms.
- GPU busy 85 % on average (the desktop alone keeps it near 30 %).

The frame is bound by both CPU threads, with the GPU close behind. A
`trace-chrome` trace (proportions, tracing on) names the costs:

| thread | span | share |
|---|---|---|
| render | the camera driver runs the `Core3d` schedule 113 times a frame: two cameras and about 111 point-light shadow faces (Bevy's own maps for the lights that have a character linked, re-rendered every frame) | 60 % of the render thread |
| render | 743 command buffers submitted a frame | 27 % of the render thread |
| main | `exa_physics::body::step_bodies`, the 39 characters' bodies | 43 % of the main thread |
| main | `exa_game::stages::tick_characters` | 19 % |
| main | extracting into the render world (31 648 mesh entities, 418 visible) | 21 % |
| main | PostUpdate: transforms, skinned bounds, visibility, shadow links | 25 % |

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
| Shadow caching | done | Exarust's own cube pass for static casters renders a face only when it is needed and dirty, as the original does; characters stay on Bevy's shadows. |
| Skinned bounds | done | Character meshes carry per-joint boxes, so they are frustum-culled for the camera and per cube face instead of always drawn. |
| Shadow tap split | done | Under supersampling each sub-frame takes 2 of the 8 shadow and GI taps, as in the original: a quarter of the filter work. |
| Depth prepass / early Z | later | The surface shader is heavy: parallax, GI taps, six lights with 2 shadow taps each. A depth prepass would shade each pixel once in cluttered rooms. It is also what occlusion culling needs. |
| Shader specialisation | later | Material features are runtime flags today. Shader variants per flag set would drop dead branches; the image is the same. |
| Baked lighting | no | The original computes its voxel GI live (one propagation step per frame). |
| GI sector marks | done | As in the original, each step propagates only the sectors the camera marks (in view, near the focus) and each voxel loops over its sector group's lights, twelve at most; the rest is cleared. The CPU side is a few hundred box tests and the grouping of 64 sectors a frame, without per-sector allocations, and the pass parameters (about 42 KiB) are rewritten in one buffer kept across frames. |
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
| Fixed-step simulation | done | Physics, AI and particles tick at 83 Hz, as in the original. A slow frame catches up at most 4 steps, as the original drops its backlog; the fixed schedule no longer spirals (play mode 6 → 35 FPS). Characters are drawn between steps. |
| Settled characters | done | A grounded character not trying to move is not swept by the character controller. |
| Physics sleeping | done | A limp body sleeps by the original's countdown (128 ticks, 1.54 s, after its last fast motion once its parts move under 20 cm/s) and is not simulated until a blow wakes it; living bodies never sleep. |
| AI throttling | later | The original picks targets for one AI per tick. |

## Order of work

From the 2026-10-20 measurements, largest gain first:

1. Done (`cache-character-shadows`): characters in Exarust's own cached
   cube pass, a face re-rendered only when it is needed and a caster in it
   moved, as the original does. The render thread fell from 28.9 ms to
   7.6 ms a frame; the frame is now bound by the main thread.
2. Done (`step-bodies-in-parallel`): the body solver's per-body work runs
   in parallel, bodies no pair touches running their 20 iterations and 8
   friction passes at once, with identical results. The original's 200 m
   activation would not help here: it loads levels whole and simulates every
   character within 200 m, which in a dungeon is all of them. Play mode went
   from 34 to 51–54 FPS; the fixed step from 18.1 to 8.5 ms a frame.
3. Done (`relink-lights-by-cell`): a moved light (the carried torch, every
   frame) is relinked only against the casters filed in the cells it
   reaches, not all ~31 000. Play mode: 68 FPS; PostUpdate 8.5 → 6.5 ms.
   A later fix: a moved light also works out its casters' faces again (the
   carried torch's shadows were cut at cube-face edges), and the player's
   look-ahead walks the navigation grid as the original's does, so it stops
   in front of furniture instead of pushing into it (a stuck push cost the
   character controller several milliseconds a tick and slowed play mode to
   10–15 FPS after some walking).
   Left on the main thread: 8 578 skinned meshes (about 220 per character)
   move every frame and mark their faces one by one; the original merges a
   character's skinned meshes into one.
4. Fewer render-world entities: room meshes that never move need not be
   re-extracted or re-checked each frame (31 648 meshes, 418 visible).
5. Depth prepass, then Bevy's GPU occlusion culling.
6. Room and portal culling from the grid's rooms and doors.
7. Shader specialisation of the surface materials.

Each step is checked against the original at the log room camera. The
picture must not change, and the frame rate is measured with `--no-vsync`
and the profiler.

Each step is checked against the original at the log room camera. The
picture must not change, and the frame rate is measured with `--no-vsync`.
