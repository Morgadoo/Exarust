# RF_SceneNodes (unit routines): scenenodes3 report

Lane: `cl/scenenodes3`. Pack: `context/re/packs/scene_nodes/` (51 functions,
24,469 bytes). Code: `crates/exa_world/src/scene_nodes.rs`, the `exa_world`
README, specs `scene-nodes` and `scene-bookkeeping` (change
`reproduce-the-scene-removal-frame-steps`, archived).

This pass checked the frame (largest function after the flood), the removal
queue writer it drains (a gap function next to the unit, not in the pack's
list) and the scene-bounds pass against the listing, and fixed what the model
had wrong:

- **Removal queue.** The original queue is a 256-entry ring that callers
  write `{node, owner, kind}` into. The frame **drains it first** into a
  pending array and hangs every drained node from `_World`. At the frame
  end it works through the pending array with **swap removal** (a finished
  slot takes the last entry, which is looked at next). Kind 0 (hide) only
  gives the owner back. Kind 1 (delete) waits while the subtree is owned,
  otherwise gives the owner back, logs "World sector object deleted" when
  that owner is `_World`, then destroys or defers while `refs >= 1`. Kind 2
  (free, the node destructor's own entry) frees or defers without the wait.
  The model had one kind, kept order, never drained, removed hidden nodes
  outright and never handed owners back.
- **Light activation** is called for every sector by the frame (the model
  gated it on the scene bit).
- **Scene bounds** turn each room's first and second box into world-axis
  extents by the room's matrix, `(|r0|hx + |r1|hy) + |r2|hz` per axis (the
  first box capped at its radius, the second not, about its unmoved centre).
  The active-sector test meets the room's box along the same matrix. Sectors
  now carry a `rotation` (identity by default; Exarust's rooms are
  unrotated, so nothing in the running game changes).

## (a) Function map

The order and sizes follow `functions.md`. Items marked "this pass" changed
in this lane; the others were already in Exarust (see the scenenodes2
report, `context/re/packs/r22/scenenodes2.report.md`).

| # | size | what the original does | Rust |
|---|---|---|---|
| 1 | 56 | clears and frees a node list | `Vec` drop (already) |
| 2 | 12 | reads a list entry | indexing (already) |
| 3 | 62 | appends, growing by 0x40 | `Vec::push` (already) |
| 4 | 28 | climbs to the node under a sector | `Scene::sector_child` (already) |
| 5 | 78 | recursive sort key −100000 | `Scene::reset_sort_keys` (already) |
| 6 | 120 | node transform from the parent | `Scene::refresh` (already) |
| 7 | 143 | removes a light alias, clears its owner | `Scene::remove_alias` (already) |
| 8 | 309 | world light clears fixture part flags | `Scene::clear_world_light` (already) |
| 9 | 56 | renderer remove call | Bevy render phases (already) |
| 10 | 4 | identity | no-op (already) |
| 11 | 373 | 6 m cell arrays, grid mode | `Scene::set_grid`, `CellLights::new` (already) |
| 12 | 471 | per-alias bucket list | `Scene::rebuild_buckets` (already) |
| 13 | 35 | string release | `Drop` (already) |
| 14 | 300 | null children, stale transforms | `Scene::validate_children` (already) |
| 15 | 1203 | light/auxiliary activation | `Scene::activate_lights`; **this pass**: called for every sector |
| 16 | 35 | string release | `Drop` (already) |
| 17 | 1294 | registration by kind | `Scene::register` (already) |
| 18 | 118 | batch registration wrapper | `Scene::register_batch` (already) |
| 19 | 35 | string release | `Drop` (already) |
| 20 | 290 | leaves the per-material lists | `Scene::remove_from_materials` (already) |
| 21 | 49 | RTTI finalisation | `Drop` (already) |
| 22 | 2288 | membership rebuild | `Scene::rebuild_membership` (already) |
| 23 | 128 | new meshes' buckets | `Scene::finish_new_meshes` (already) |
| 24 | 481 | node cell and spill | `cell_spill`, `Scene::update_cell` (already) |
| 25 | 255 | directional fill | `CellLights::fill_directional` (already, no running caller) |
| 26 | 41 | string release | `Drop` (already) |
| 27 | 3352 | omni flood through the cells | `light_flood::CellLights::flood` (already) |
| 28 | 449 | cell insert, window union | `CellLights::insert`/`remove` (already) |
| 29 | 1089 | sector bounds | `sector_bounds` (already) |
| 30 | 849 | scene bounds | `scene_bounds`; **this pass**: room rotation (`Sector::rotation`, `world_extent`) |
| 31 | 97 | moved node's face masks into lights | `Scene::link_bucket_faces` (already) |
| 32 | 514 | doorway exemption | `doorway_portal`, `doorway` (already) |
| 33 | 1895 | visible node pass | `Scene::visible_pass`, `fade_node` (already) |
| 34 | 838 | hidden node pass | `Scene::hidden_pass` (already) |
| 35 | 93 | scene plugin | `Scene::add_plugin` (already) |
| 36 | 852 | batch add | `Scene::add_batch` (already) |
| 37 | 37 | material manager unlock | queue model (already) |
| 38 | 703 | material add/remove flush | `Scene::flush_materials` (already) |
| 39 | 37 | material manager unlock | queue model (already) |
| 40 | 281 | frame-end material removes | `Scene::flush_material_removes` (already) |
| 41 | 119 | creates a scene | `SceneSet::create` (already) |
| 42 | 230 | frees a scene | `SceneSet::free` (already) |
| 43 | 235 | cleanup hook | `SceneSet::cleanup` (already) |
| 44 | 130 | frees or marks owned | `SceneSet::release` (already) |
| 45 | 446 | portal scissor | `scissor_rect` (already) |
| 46 | 95 | subtree owned? | `Scene::owns_subtree` (already) |
| 47 | 35 | string release | `Drop` (already) |
| 48 | 3247 | per-frame scene update | `Scene::frame_with`; **this pass**: `drain_removals` first, `process_removals` with kinds, swap order, owner hand-back, `FrameEvents::world_deleted`; active-sector test along `Sector::rotation` in `classify` |
| 49 | 385 | subtree move keeping the world frame | `Scene::move_to_sector` (already) |
| 50 | 112 | unit init, `_World` | `SceneSet::new`, `install_cleanup` (already) |
| 51 | 85 | unit finaliser | `SceneSet::finish` (already) |

The queue writer next to the unit (outside the pack's list) is
`Scene::queue_removal_as` (`RemovalKind`); `queue_removal(node, detach)`
stays as the shorthand for hide or delete.

## (b) Ambiguities and guesses

- **Destructor modes.** The frame calls the node unit's destructor with mode
  2 (destroy) or 4 (deferred). That routine is in another unit. The model
  takes the subtree out for mode 2 and only sets `DELETING` for mode 4
  before listing the node on the free list (labelled in the code).
- **`_World` after the drain.** Once the ring is drained, the frame makes one
  more call on a scene field with `_World`. It is not identified and not
  modelled.
- **Log line.** The "World sector object deleted" warning is reported as
  `FrameEvents::world_deleted`. The model has no node names to print.
- **Sector rotation.** The original reads the room's matrix from the sector
  node. Exarust's sectors keep it in `Sector::rotation`, which nothing sets
  yet: the levels' rooms are unrotated.
- **Swap order and kept entries.** Reproduced exactly, so an entry that
  waits can change places with newer ones (pinned in the tests).

## (c) Commits

- `c4bde20a` Reproduce the scene frame's removal steps and rotated sector bounds
- (report and archived change) Archive the scene removal change and add the scenenodes3 report

Checks run before each commit, on the remote worker:

- `cargo fmt --all -- --check`: clean
- `cargo clippy -p exa_world -p exa_render -p exa_assets --all-targets -- -D warnings`: finished, no warnings
- `cargo test -p exa_world -p exa_render -p exa_assets`: exa_world 547 passed,
  3 ignored; exa_render 365 passed; exa_assets 301 passed, 4 ignored, plus
  4 integration tests; no failures
- `cargo check -p exa_game --all-targets`: clean (exa_game uses `Scene`)
- `openspec validate --all --strict`: all passed

## (d) What is missing

- **The draw path.** Nothing in the running game builds a `Scene`.
  `room_view` still draws from the room states, and `Scene::frame` and its
  `FrameEvents` have no live consumer. Wiring it in needs doorway quads from
  the level data and the masks, scissor and partial draws in `exa_render`.
  That is a change of its own, so nothing was wired in this lane, and load
  and frame time are unchanged.
- **Render-side callbacks** the frame calls (alias shadow test, relink,
  flood or fill call, flicker, the scene's callback slots) are still passed
  in or reported, not run.
- **Smaller frame steps** are not modelled: the scene-wide lights' shadow
  counter reset, the unidentified `_World` call after the drain, and the
  per-view slot that keeps a stale copy.
- `fill_directional` still has no running caller, and nothing sets the sector
  rotations from level data yet.

## (e) Estimate

At the model level, about 22.0 KB of 24.5 KB, roughly **90%** (scenenodes2
had 88%). The gain comes from the frame's removal tail, the scene-bounds
rotation and the activation call. Counting what the running game actually
uses, the component stays about **80%** until the model is on the draw
path.
