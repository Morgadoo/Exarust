## Why

The NPC combat state weighs its footwork places with four answers from the original's installed pathfinder: each place's ground cost, the line walk that pulls a place back from a wall, the obstacle test that hides the target in the chase check, and the snap. Exarust answered all four with the abstract pathfinder's neutral values, so the duellist stepped through walls, found every place equally cheap, and saw its foe through walls. The level's navigation grid now answers the same queries for the player and the planner.

Phase 6 (NPC AI) in docs/PLAN.md: the combat state of TMotileAI.

## What Changes

- A new `exa_ai::ground` module answers the combat state's ground cost (the grid's cost sample through the combatant's cost formula), line walk clamp and obstacle test in centimetres, each the abstract pathfinder's answer without a grid.
- The duellist's field asks it, so places near walls and other characters' footprints cost more, places and the long step's continuation stop at walls, and a foe behind a wall counts the fighter as stuck toward its chase.
- Around its combat ticks the duellist lifts its own footprint and stamps it again, as the original's AI tick does around its state.
- The snap stays a no-op: the level's pathfinder never overrides it.

## Evidence

REA/Ghidra on Exanima.exe: the combat state's calls through the installed pathfinder's line walk, obstacle, cost sample and snap slots (the slot numbering in the movement notes), the combatant's cost routine (`b·40 + ((a² + a)·120 + 1)` from the sample's two words), and the AI tick's footprint lift before and stamp after its state call.

## Out of scope

The stab's head tracking call (its aim point feeds the held-weapon handling pushes, which are not wired), the can-act gate's grip and awareness terms, the scene-node origins, and the idle and roam walk (another lane).
