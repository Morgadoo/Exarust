# Tasks

- [x] Trace `CheckpointSave`, the checkpoint routine, the identity reset and its hook, the zone check and its routine runner, and the body frame's anchor in REA.
- [x] Scan the shipped levels' volume sections for script zones (eleven enter zones; `exanima01`'s `Checkpoint` holds `CheckpointSave(1);`).
- [x] `exa_formats::objdb`: the volume routine; tests on synthetic bytes.
- [x] `exa_world::script_volumes`; a real-data test (skipped without the game).
- [x] `exa_script`: the effect's player flag, the argument read, `run_zone_script`; tests.
- [x] `exa_game::zone_scripts` live; `save_live::run_checkpoint_scripts`; the load's request; tests.
- [x] `exa_game::doll_view` hand scene and the list `--doll-drop`; tests.
- [x] docs/formats/OTHER.md, docs/UI.md, crate READMEs.
- [x] GPU check: the figure with a shield, two weapons and the torch; a load inside the checkpoint zone writing the save and then its copy.
- [x] fmt, clippy -D warnings and tests for `exa_formats`, `exa_script`, `exa_world`, `exa_game`, `exa_character`, `exarust` on the worker.
