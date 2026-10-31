# Design

**Figure hands.** The figure no longer takes the player's `Wielded`: every
hand model is the hand scene's. Each frame while the view is open the
targets are the record's out set (plus Exarust's carried torch, which is
outside the record) and the job is `scene_plan` against what the figure's
hand models show (`DollHands::shown`). Its steps run in the original's
order: a changed hand's old model is released (`release_model`: the
drag-off take-out's mode 7 when the hand is left empty, a plain take-out's
mode 0 when it takes another item), the new models are attached under the
hand bones, the figure's `Hands` follow the targets (the held moveset picks
the figure's clips: the torch raises the hand, a shield its stance), and
the stance restarts with blend 20. Placement: a weapon by its class's grip
in the hand's held slot, a light by its own grip (the torch's), a shield in
the left forearm's frame at the hand's rest offset turned -5° about z. The
models are plain scene nodes: the figure has no physics.

**Checkpoint zone.** The zone check is per character with its own checked
point (`ZoneCheckPoint`), the 3 cm gate and `exa_ai::golem::zone_entered`,
over the level's script zones in placement order. The point is the body
frame's anchor (root x and z, pelvis height): the shipped checkpoint zone's
box starts 10 cm off the floor, so the feet would miss it (seen on the GPU
before the change). The routine runs through the conversation engine; its
effects go out as `ScriptEffect`s. `run_checkpoint_scripts` sits in the save
chain before `run_save_steps`, so the call and its step land in one frame.

**Identity reset hook.** The hook the reset calls is the content stream's
character-ring post (installed by the stream unit at start-up), not a game
mode's. Exarust builds the loaded player synchronously from its record at
the next level load, which starts no AI, so the request
(`CharacterRequest` with `NO_AI_START`) is returned by both restores and
logged.

Open questions:

- The class word of the posted request: not traced (Exarust passes 0).
- The checked point at a body's creation: taken as the origin (as the alarm
  check does); a body spawned inside a zone runs it at once. Answer with a
  retail capture of a load inside the checkpoint zone.
- The buckler's look on the figure (edge-on in the idle pose): compare with
  a retail inventory shot.

Format page: `docs/formats/OTHER.md` (volume section).
