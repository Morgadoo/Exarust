## Why

Round 182's sweep left two frame writers that undo a free-running simulation's newer steps. The power bar's casts ran in the frame and wrote the player's whole caster and both bodies' minds. A frame copy based on an older step then won over the simulation's newer focus, active list and mind state ("applied: MindBody 3; PowerCaster 2" in three Mind casts). The flame rows and the arcs took frame writes for drawing (the loop's pitch, the flickers, the light points) and frame game-logic writes (the item abilities' extinguish and arcs' end), so they could not be drawn copies. In the burning-blade Shock scene, round 182's build applied six stale Flame copies, and main's build applied about 240 a thousand steps along with up to 173 LiveArcs.

## What the original does

The bar's cast entry runs from the player controls' key handler, a GUI event on the main thread. It writes the body's power state directly, and the simulation thread reads it in its next step. The posted-call ring's posters do not include it. The item list runs in the body tick, on the simulation thread. The arcs' and flame rows' frame parts run on the main thread against the objects the simulation thread steps, sharing the row's random stream.

## What Changes

- A bar Use reads the frame's picks and posts the cast to the step (`cast_from_bar`), where the front ends, the active list, the focus and the Mind casts' writes run on the simulation's values. The cast draws from the step's effect streams. The GUI's fail sound, the mind sounds and music, the spirit body's request and the control's end come back through `FrameCommands`. The bar's toggle, pick and step go through `FixedCommands::write`. In one world the cast runs at once, as before.
- Flame rows: the loop's pitch is frame-only (`FlamePitch`). The frame's draw from the row's stream is queued for the simulation's copy. The removal is the frame's despawn.
- Arcs: the flickers and light points are frame-only (`ArcGlow`). The draw reads each tendril's origin, and its write of the first point is queued for the simulation's copy (`settle_origins`).
- The item abilities' extinguish and arcs' end go through `FixedCommands::write`.
- The feature sweep with the bridge's write log on found three more small frame writers, now routed through `FixedCommands::write`: the fight start (the duellists' engage, the player's combat mode), the stay check's latch clear (one check per lapsed request), and the held moveset choice's switch of the animator's config.

## Impact

- `exa_game::thaum::{cast, arcs, live}`, `exa_game::effects::fire_row`, `exa_game::item_ability`.
- Lockstep: the moved writes land at the start of the next step, where the frame's whole copy used to land. Traces are compared with main in the lane's evidence.
