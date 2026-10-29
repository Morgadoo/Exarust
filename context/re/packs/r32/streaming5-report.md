# Streaming and textures, fifth pass

LANE = `cl/streaming5` (`streaming5`). Components: **FLX_Content_Stream (unit
routines)** (71 functions, 12 464 bytes) and **RF_Textures (unit routines)**
(51 functions, 11 317 bytes), Exanima 0.9.5 decompilation pack
`context/re/packs/streaming/`.

This pass builds on `context/re/packs/r23/streaming4.report.md` (which maps
every function; rows not repeated below are unchanged from it). It wires the
two rules the fourth pass left without a caller in the running game: the
terrain material path's stream-first texture load and the content stream's
level pass over a level's placed objects. It also traces the texture
manager's fast-load flag (read once, never written), adds the material
path's status-3 rule, the level wait's report and grid-environment handoff,
and the manager's named-image and named-file inserts.

Every number was checked against `context/re/exe.asm`: the terrain routine
sets the record status to 1 unconditionally and returns no record when no
provider holds the name; the material routine stores 3 when the fast-load
byte is non-zero and 1 otherwise, and that byte has exactly one reference in
the whole image (the compare), no RIP-relative write and no stored pointer;
the level wait tests the scene's grid-environment pointer, then
`room word & 3` (`and eax,3; jbe`), and the scene constructor stores `0x73`
in that word; the wait's message is the decimal count followed by
" missing database objects." at level 5 under "Content management".

No retail address or decompiler name is in code, comments, tests or crate
docs.

## (a) Function map (changed rows)

| function (pack) | bytes | Exarust item | status |
|---|---:|---|---|
| terrain texture path (live record, else queue behind the placeholder with status 1) | 564 | `ModelLoader::texture_stream_first`, now called by the terrain build (`exa_world::terrain::TerrainBuild::request_textures` / `textures_arrived`), with `ModelLoader::texture_waiting` and `texture_finish_now` | partial → **done (wired)** |
| material streaming load (fast-load status 3 vs 1) | 626 | `TextureCache::{fast_load,set_fast_load}`; `texture_with_flags` queues the request promoted under the flag | partial → **done** (flag traced: never set in the shipped game, so off) |
| insert a named image now | 104 | `ModelLoader::add_texture_image(.., ImageInsert::Now)` | partial → **done** (no game caller) |
| insert a named image queued ("Added texture already exists.") | 326 | `add_texture_image(.., ImageInsert::Queued)` + `texture_cache::LOG_SOURCE` (was `TextureCache::insert_new` only) | done (now with the image) |
| read a file, placeholder on failure, queued insert | 139 | `ModelLoader::add_texture_file(.., Queued, true)` | partial → **done** (no game caller) |
| read a file, insert now or nothing | 146 | `add_texture_file(.., Now, false)` | partial → **done** (no game caller) |
| read a file, queued insert or nothing | 136 | `add_texture_file(.., Queued, false)` | partial → **done** (no game caller) |
| level wait: spin, report missing, clear, hand off to the grid environment or append | 469 | `ContentStream::wait_for_level` + `content_stream::{missing_message, LOG_SOURCE, LevelHandoff, ROOM_MODE_BITS, SCENE_ROOM_WORD}`; called by `exa_world::LevelLoad` | partial → **done (wired)** |
| placed-object builder (one entry per pass) | 1082 | `exa_world::PlacedObjectBuilder` (a `ContentBuilders` over `spawn_placed_object`) reports entity, operative kind, position (cm) and marker words; `LevelLoad` steps the stream; post-build hooks still in `exa_world` | partial (now driven in the game) |
| begin a level (arrival word, previous level) | 477 | `ContentStream::begin_level`, called by `LevelLoad` with `LevelLoad::{arrival, previous_level}` | done (wired; the game does not pass the session words yet) |
| thread loop | 1092 | `ContentStream::step`, driven by `LevelLoad` for the level pass | done (level pass wired) |

Unchanged partial rows (see the streaming4 report): the provider-image
create variants that take a provider image object (391, 411, 248 bytes), the
create-with-target word variant (356), the format registry (119, 130), the
character builders (324, 776), the streamed-character chunk handoff (1074),
the database object builder (561), thread start/entry and global init.
Platform glue (critical sections, `CreateThread`, FPC string release, RTTI
finalize, empty hooks) stays n/a.

## (b) Ambiguities and guesses

1. **Fast-load flag.** One read in the whole image, no write, no pointer to
   it in the file: treated as always zero, so material loads queue with
   status 1. The flag exists (`TextureCache::set_fast_load`) and is tested.
2. **Terrain waits rather than binds the placeholder.** The original binds
   whatever record it got; Exarust spawns a terrain tile only once its
   textures arrived, because its splat bind group and composite bake read the
   images once. The loads start as soon as a built tile names a material, so
   the wait overlaps the surface packing; the main thread never decodes.
3. **One-call loads.** `spawn_level` steps with an unbounded budget, so no
   frame runs the loader; the terrain then finishes a waiting record on the
   spot (`texture_finish_now`, the priority wait, no new reference).
4. **Streaming switch as "loader runs".** The original's terrain path queues
   whatever the manager's switch says; Exarust queues only while its loader
   runs (the switch the game turns on with the loader), and decodes
   synchronously in tools and tests that have no loader.
5. **Placed objects without an entity.** Exarust spawns nothing for records
   without a model (triggers, markers) or of another slot; the original
   builds a node for any record its database finds. Those count as built
   (`Entity::PLACEHOLDER` as the node); a record of another slot counting as
   built is a labelled guess.
6. **Operative kind.** The original tests the built node's class word
   (operative class) and reads the kind from it; Exarust takes the record's
   operative section's kind.
7. **Handoff.** The grid environment's room sorting in Exarust is the
   existing per-object `PendingRoom`; `LevelHandoff::of` is the tested rule,
   not a list handoff.
8. **Named-image word.** The queued insert takes the image's own high 16
   bits into the record word; Bevy's `Image` has no such word, so the
   caller's word is kept whole (labelled deviation). The queued insert is
   stored at once (the original uploads on the same frame's pass).
9. **Stream in a resource.** `LevelLoad` holds the stream behind a mutex
   only to stay `Sync` (the job ring holds non-`Sync` callbacks); it is
   reached with `get_mut`, never locked.

## (c) Commits

Each preceded by `cargo fmt --all`, `cargo clippy -p exa_assets -p exa_rpk
-p exa_render -p exa_world --all-targets -- -D warnings` and the tests
(remote worker, `EXANIMA_DIR=/root/exanima`); `cargo check --workspace
--all-targets` before the content-stream commit.

1. `6119b06f` Stream terrain textures through the stream-first path
2. `a1c4b8a5` Build placed objects through the content stream's level pass
3. `5ddb1e7f` Add and archive the terrain-texture and placed-object stream change
4. `1d7df110` Add the texture manager's named image and file inserts
5. (this report)

## (d) What is still missing

- The request rings and streamed characters still have no driver: the
  character spawner and database object requests build directly, so
  `request_character`/`fetch_character`/`request_object`/`fetch_object` and
  the nearest-first streamed pass run only in tests.
- The game does not pass the session's arrival word and previous level to
  `LevelLoad` yet (they default to "none"), so `player_focus` is `None` in
  play until `exa_game` sets them.
- Placed-object post-build hooks (door registration with the grid
  environment, dynamic bodies, the `0x300000` branches) stay in `exa_world`.
- The provider-image create variants that take a provider image object, the
  create-with-target variant, the runtime format registry, and the
  character builders' object creation.
- No in-game smoke run this pass (building the play binary locally was out
  of budget); the terrain path was checked by the real-data level tests
  (one-call load with streaming on; stepped equal to one-call).

## (e) Coverage estimate

Weights as before (done = 1, partial = ½, the content stream's builder rows
¾ once driven, n/a excluded).

| component | weighted share before | after | conservative before | after |
|---|---:|---:|---:|---:|
| RF_Textures (unit routines) | 83.3 % | **91.4 %** | ≈77 % | **≈83 %** |
| FLX_Content_Stream (unit routines) | 85.3 % | **89.5 %** | ≈60 % (published) | **≈65 %** |

The texture side gained the wired terrain path (564 B), the fast-load rule
(626 B) and four named-insert routines (525 B); the content side the wired
level wait (469 B) and the level pass driving the placed-object builder in
the game (1 082 B, still partial). The conservative column keeps discounting
the content stream's unwired request rings and builders.
