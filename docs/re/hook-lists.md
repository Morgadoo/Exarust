# The engine's hook lists and deferred-call rings

The original keeps two kinds of callback list (`exa_game::hooks`): a method
list of `(code, data, priority)` entries and a procedure list of
`(code, priority)` entries. An insert goes before the first entry with a
strictly greater priority, so equal priorities keep their arrival order;
a dispatch calls every entry with one argument and drops the ones that
answer false. Exarust has no such lists: each registrant's work is a Bevy
system, and the list's order is the schedule's (`before`/`after`, `chain`,
system sets, and the main schedule's `RunFixedMainLoop`, `Update`,
`PostUpdate` sequence). `exa_physics::schedule_order` answers whether a
schedule forces one system before another, and whether two unordered
systems conflict in their data (if they do not, their order cannot change
what either computes).

Audited by the hookorder lane (2026-11-15): every insert, remove, clear
and dispatch call of the original, grouped by list. Priorities are the
original's; the lane's artifacts keep the retail addresses.

## The lists

| list (owner) | called | registrants, by priority | Exarust | order |
|---|---|---|---|---|
| scene frame list (scene) | at the start of the scene's frame, after the removed nodes are drained | GI manager's one-shot state flip (0), terrain stream's tile step (4), the simulation's node interpolation (6), the navigation camera's frame (7), the player light (7), the room tracker (10), the force-shell, shell and scan render steps (10) | GI rebuild on the next level; `drive_terrain_stream` (Update, reads the camera's propagated transform: the previous frame's, as the terrain hook, called before the camera's, sees it); the sim exchange and `show_characters` (RunFixedMainLoop), `draw_bodies` (PostUpdate, before propagation), the camera's own drawn-pose focus; `chase_camera` and `place_player_light` (Update, the light from the head drawn at the frame's fraction); `track_rooms` (PostUpdate, after propagation); `tick_shell_auras`, `step_vanishes`, `tick_scans` (Update, read no camera or room) | matches; `test:the_scene_frame_list_runs_in_its_priority_order` |
| node-removed list (scene) | for each removed node in the scene's frame, and for every node when a scene is cleared | the simulation's sector re-link mark (0), the skinning manager's controller drop (0), the controls' timed-list clear (4) | Bevy despawn; each registrant drops only its own reference | order unobservable |
| scene-cleared list (scene) | at the end of a scene's clear (level teardown) | environment's dirty mark (1), the player light's removal (1), terrain's clear slot (5), the room tracker's reset (10) | `level_lifecycle::teardown` and the drop of the level's resources | order unobservable: each resets its own state |
| camera-switch list (scene) | at a camera or scene mode switch | the room tracker's hide-all (10) | the hide-all runs at the end of the grid build only; the switch itself is not ported (`scene_editor::CameraSwitch` is test-only) | open on the switch's own row |
| shadow-caster list (scene) | per cube face and in the sun pass | the room lids (10), the plant chunks (10) | the lights' lid casters and the plant shadow flags (Bevy shadow phases) | order unobservable: depth-only draws |
| tile-inserted list (scene) | after the terrain stream inserts a tile | none | none | empty |
| pivot list (scene) | never: no routine dispatches it | the GL geometry's pivot refresh, the navigation camera's point shift | none | unreachable |
| game task list (GUI task) | once a frame, the game layer's frame step | the session's frame (0), the controls' frame or the held item (4, one replacing the other), the deferred scene start (5), the scene task's tick (5: the scene frame list above), the spirit screen (6) | `flow::tick`, `hand` (`HandSystems`), `player_controls`, the scene systems, `step_spirit_screen` (Update) | matches: the frame's phases (`exa_physics::frame_phase`: `Session`, `Camera`, `Hand`) run the session's frame, the camera, then the held item and the spirit screen; the held item, after the camera in Exarust, reads the camera's propagated transform (the previous frame's, as the original's controls, called before the camera's hook, see it); `test:the_game_task_list_keeps_its_order_by_schedule_or_by_data` |
| modal task list (GUI task) | once a frame, the modal layer's frame step | the shader warm-up screen (0), the key-read screen (0), the transition screen (100) | `drive_warm_screen`, `rebind_keys`, `run_transition` (Update) | matches: each pair is ordered as the list orders it or does not conflict; `test:the_modal_task_list_keeps_its_order_by_schedule_or_by_data` |
| creator task list (GUI task) | the creator's frame step | the deferred preview rebuild (0) | the creator screen | single member |
| renderer lists (renderer) | at fixed points of the scene pass | the lit list (weapon blood 0, terrain 10), plants (20), before-scene (vanish 100), the rooms' fade overlay (100), after-scene (vanish 100); the wireframe list is never dispatched | the render phases (`exa_render::passes::PASS_ORDER`) | belongs to the scene pass's own row |
| after-draw list (view) | after a view draws | the mind sight's overlay (0) and glow (0) | `run_mind_sight`, `draw_mind_sight` | the dispatch point is not ported (the view's after-draw routine is missing) |
| overlay procedure list (viewport) | in the viewport's draw | the HUD (game view, 100), an item icon or the preview bars (item views, 0) | the HUD systems, `preview_bars_layer` | one member per viewport |
| skin rebuilt list (body) | after a re-dress or a creator preview rebuilds the skin | the inventory preview and the shell, force-shell and scan skin copies (100) | GPU skinned copies share the rebuilt skin | order unobservable: each recopies its own |
| skin deformed list (skin controller) | after the controller deforms its mesh | the force-shell, shell and scan render steps (0) | as in the scene frame list | as above |
| game scene start (game) | at an arrival and at an arena reload | the controls' scene start (0) | `flow` | single member |
| audio scheduler (global) | in the frame tick and twice in the simulation's step | the audio environment's update (10) | `advance_audio_environment` (Update) | single member |
| AI manager list (global) | in the simulation's step | the AI manager's list swap (0) | the AI's per-tick lists | single member |
| window list (global) | in the window procedure | none | none | empty |
| GPU program reset (global procedure list) | when the GPU programs are freed | the shader units, terrain, VXGI, the powers' and effects' programs (100) | Bevy's `PipelineCache` | order unobservable |
| character texture list (global procedure list) | never | the character textures' reset (100, twice) | none | unreachable |

## The deferred-call rings

Two rings (`exa_game::frame::DeferredCalls`): the main thread's, run by the
frame tick and once at an arrival, and the simulation's, run by each step.
About sixty push sites post to them: the main ring takes equipment
finalisers, AI search jobs, the powers' finish and arrival callbacks, glow
pulses, the journey tick, trade and loot polls; the simulation ring takes
the controls' restores, hand plans, the rotator's timed return and the
control mind's step.

The run walks from the head up to the tail it found, the loop going on
while that tail is above the index as a plain integer. Once the queued
calls wrap past the 128th slot, only the call at the head runs in a pass:
the ones behind it wait one pass each until the head reaches the last
slot, and a head that asks to run again holds them all. This is reachable:
the tail passes the end every 128 posts in the life of the game, and many
posted calls ask to run again for many passes (the glow pulse over its
fade, the journey tick, the rotator's return until its deadline, a drop
job while its pack is locked, a vanish's finish while its fade lasts), so
at a wrap the ring often holds such a call ahead of others; two one-shot
posts in one pass into the last two slots are enough too (a tail of 0
already stops the walk), the second waiting a pass. Exarust posts the same work as
Bevy messages, systems and `sim_app::SimInbox`, which run every post in
one pass: the wrap's delay is not reproduced. Reproducing it needs every
original poster to post into one ring in the original's order: `SimInbox`
also carries Exarust's own spawn and despawn links, which the original
never queues, so giving it the ring's walk would delay posts the original
does not delay. The ring rows stay open until the posters share one ring.

## Content stream rings

The content stream's rings (`exa_assets::stream::RingQueue`) differ the
other way: the original's poster spins (yielding) while a ring is full
until the stream thread takes an entry; Exarust's ring grows by one
instead. Both keep insertion order, so the builds and results come in the
same order; the original's poster is only held. A spin is not adopted:
Exarust keeps every ring of the stream behind one lock
(`exa_assets::stream_worker`), and the stream thread pushes a result while
holding it, so a push waiting there for room would keep the main thread
from taking the result that makes room: it would wait for ever. The push rows stay
open on this difference (a timing one: no order or result changes).
