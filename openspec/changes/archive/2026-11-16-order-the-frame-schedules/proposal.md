## Why

Lane `ambiguity2`. Round 290 ordered every conflicting pair of the fixed
step and of `PreUpdate`, and left the frame's `Update` (5835 pairs) and
`PostUpdate` (275 pairs) with a TODO count. The frame's systems run on the
thread pool, so each such pair ran in whichever order the threads gave it,
which could change from frame to frame and moved whenever a lane added a
system. The original runs one frame routine in a fixed order: the GUI's
input and windows, the game task's frame list (the session's frame, the
player controls' frame, the scene task with the camera, the spirit screen),
the modal list (the transition screen), then the render frame
(`docs/re/hook-lists.md`).

## What changes

- `exa_physics::frame_phase`: `FramePhase` (`Input`, `Gui`, `Hub`,
  `Session`, `Controls`, `Camera`, `Hand`, `Cursor`, `World`, `Audio`,
  `Draw`, `Capture`), chained in `Update`, and `FramePostPhase` (`Prepare`,
  `Placed`, `View`, `Trace`), chained in `PostUpdate` and placed against
  Bevy's sets (`Prepare` before the transform propagation, the camera
  update, the UI and the asset events; `Placed` after the propagation and
  before the bounds, the frusta, the visibility, the light bounds and the
  sun's cascades). `configure_frame_phases` is called by the game's plugin.
- Every `Update` and `PostUpdate` system of the game's plugins sits in its
  phase. Inside a phase, each conflicting pair has its own `before`/`after`,
  in the order the frame's executor tended to give it before (the schedule's
  topological order on main), about 350 edges in all. Pairs that main's
  order only held through Bevy's shared sync points are explicit too.
- By sense: every GUI paint runs after the game screens' draw, which clears
  the GUI queue (F565: the tips' banner, the loadout form, the destination
  and the saves' paints had been ordered before it); the settings form draws
  after its open and input; the spirit screen, the game task list's last
  entry, is in `Hand`; the mind's hand-back transition (`run_mind_fade`, an
  exclusive system) runs after `Hand` and before `Cursor`, as the modal list
  runs after the game task list, and before Bevy's screenshot, tilemap and
  light-probe systems.
- `exa_script::ScriptSystems` names the scripts' frame; the game places it in
  `Session`.
- The hook-list test chains the phases and checks the game task list's
  order through them.
- Systems now named across modules or crates were made `pub(crate)` or
  `pub`, with the resources and system parameters their signatures name.
  No system's body changed. The character creator's screen and view systems
  (`creator_screen`, `creator_view`) are in `Gui`, and `tag_parts` in
  `Prepare`; their logic is untouched.
- `exarust::schedule_ambiguity_test`: the TODO counts are gone; the allowlist
  matches a module path as well as a system and holds seven entries, each a
  pair with one of Bevy's private systems that no public set or path names
  (the audio output, the light and skinned-bounds gizmos, the sun map size
  check, the order-independent transparency check), each with its reason.

## Evidence

The check passes: every schedule has no unordered conflicting pair outside
the allowlist (`Update` 1 allowed, `PostUpdate` 14 allowed). Against main's
sorted order (round 290), 1971 of `Update`'s 5835 former pairs and 12 of
`PostUpdate`'s 275 take the other side: pairs that had no fixed order on
the frame's threads. The lane is merged with rounds 291 and 292. Golden suite and determinism results: `context/artifacts/2026-11-16-ambiguity2/`.
