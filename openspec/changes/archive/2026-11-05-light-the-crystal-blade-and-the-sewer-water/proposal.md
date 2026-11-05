# Proposal

## Why

Wiring backlog systems S12 (item abilities) and S13 (water surfaces).

- S12: the flame row along a lit blade (`exa_game::effects::fire_row`) was
  ported but had no caller. The backlog took the "burning ability word" for
  a record field. It is not one: the shipped data never carries a burning
  state, and no routine writes one. The state is the item's first crystal
  socket, which the item GUI fills when a Power Crystal C is dropped in.
- S13: the wet steps were ported, but the levels were thought to have no
  water. The scene does keep a water level. Exarust had misread its source
  as a sun height and its value as a shadow distance. It is the terrain
  palette's wetness. The sewer palette `UWSwrA1` (0.5) floods its levels,
  `exanima05_sw` and `exanima07_sw`, at −25 cm.

Phase: Phase 5/6 combat and world milestones of docs/PLAN.md. This change
does not edit PLAN.md.

## Evidence

From REA (Ghidra on `Exanima.exe`), checked against the disassembly:

- the held-item ability install and its registry;
- the body tick's once-a-second ability tick;
- the type-1, type-2 and type-3 checks;
- the burning blade's lighting, putting out and hit callback;
- the flame row node and its built-in `_FlameMat01` record (`flameletsa3.rfi`);
- the moveset choice with its overlay argument;
- the player light switch's lit-blade rule;
- the item GUI's socket drop;
- the damage routine's call of the striker's hit callback;
- the scene environment's frame (the water level from the terrain wetness);
- the static terrain's load (the palette's wetness into the environment);
- the save load's reset;
- the walk's gait plant and the footwork's release step, each with its
  wet step.

Data: `objdb.rdb`, and every level's local database in `Resource.rpk`.

## What Changes

- `exa_game::item_ability` (new):
  - Every game second, each of the player's held items runs its ability
    check.
  - A type-3 blade with a Power Crystal C in its first socket gets a flame
    row with its light, its `FireS02` loop and its `_FlameMat01` ribbon.
  - Lighting it sets the "Flaming" moveset overlay and puts the player light
    out. Removing the crystal puts the blade out.
  - A hit with a burning blade sizzles.
  - Debug flag `--flame-sword`.
- `exa_character::held`: `HeldModifier`, the overlay bits the moveset choice
  ORs in.
- `exa_game::player_light`: the lit-blade rule is wired.
- `exa_physics::body`:
  - `WaterLevel`, `water_level` (−25 cm above a terrain wetness of 0.4,
    else −100000) and `gait_wet_step`, the walk's wet step;
  - `Footstep` carries the planted contact's height and the gait thrust;
  - `FootCue::StepEnd` carries its contact point.
- `exa_world`:
  - the terrain load hands its palette's wetness to the scene environment
    (`terrain_wetness`, which was `sun_height`);
  - the frame publishes `water_level` (which was `shadow_far`) into
    `WaterLevel`.
- `exa_audio::footsteps`: under the water, both step paths play `StpWet`.
