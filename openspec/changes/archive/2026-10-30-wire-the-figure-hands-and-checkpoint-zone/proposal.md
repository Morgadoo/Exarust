# Proposal

## Why

Wiring backlog items P3 and G6.

- **P3.** The inventory's character view copied only the main-hand weapon
  onto the figure, although the hand-scene job of the character view
  (`exa_items::doll::scene_plan`, `rebuild_steps`, `release_model`,
  `hand_node`) was ported: a shield, a second weapon and the torch never
  showed, and nothing followed the record while the view was open.
- **G6.** `exa_script` pushed `CheckpointSave` with nothing reading it, the
  save unit's checkpoint step (save, then copy) had no script caller, and
  the identity reset's hook after a load was not modelled. Tracing the
  caller found the shipped one: `exanima01`'s `Checkpoint` trigger volume
  runs `CheckpointSave(1);` through the AI manager's zone check, which
  Exarust did not run for script zones at all.

Phase: the gameplay milestones of docs/PLAN.md (inventory and story
progress); no PLAN.md edit in this change.

Evidence (REA, Ghidra on `Exanima.exe`): the `CheckpointSave` global (the
first actor slot compared with the player, its argument read and dropped,
the checkpoint routine only while checkpoints are on), the checkpoint
routine (checkpoint name, the existence check, the pre-save world sync,
the save, the copy), the identity reset (race byte cleared, then the
request word `0x1000` and the world scene stored on the character and the
content stream's character-ring hook called; the stream unit's start-up
installs that hook), the AI manager's frame (3 cm step) and zone check
(radius, box distance 0.1 now and before, class `0x100`; low byte `0x10`
runs the routine for anyone; low byte 2 is the golem alarm; bit `0x20` posts
the level exit with the zone's two words, `0x21` flagged), the zone routine
runner (one actor, the `ZoneID` local, the text after the section's fixed
words), the body frame's anchor (root x and z, pelvis height). Corpus scan
of the shipped levels' volume sections: eleven enter script zones
(`exanima01` six, `exanima02` one, `exanima03` one, `exanima05_sw` three),
the `Checkpoint` zone among them.

## What Changes

- `exa_formats::objdb`: a volume section's routine (`Item::volume_script`,
  `Volume::script`), `Volume::runs_script`, `Volume::zone_id`.
- `exa_world`: `script_volumes` and `LevelInfo::script_volumes`.
- `exa_script`: `Effect::CheckpointSave { player }` (the first actor slot is
  the player), the argument read, `Engine::run_zone_script` (one actor, the
  `ZoneID` local).
- `exa_game::zone_scripts` (new, live): the AI manager's zone check for the
  script zones, for every character, at the body frame's anchor.
- `exa_game::save_live`: `run_checkpoint_scripts` turns a player's call
  with checkpoints on into the checkpoint step; the identity reset's
  content-stream request (`loaded_player_request`, request word `0x1000`)
  on both loads, logged.
- `exa_game::doll_view`: the figure's hand scene (`DollHands`,
  `rebuild_doll_hands`): models for both hands by the scene plan and its
  steps, the release of an emptied hand, the figure's held moveset and
  stance restart; Exarust's carried torch on the figure; `--doll-drop`
  takes a list and a near-hand `+`.

## Out of scope

- The zone check's stay zones (class `0x200`, the memory fragments), its
  level-transition kinds (Exarust's own exit path) and the alarm zones
  (already live in `exa_game::golem_alarm`); fixing `exa_ai::manager::enter`,
  whose kind 2 is named a level change but is the alarm zone.
- The checkpoint routine's pre-save world sync of placed objects (no live
  object database, S2).
- Queueing the loaded player in a live content stream (S5); the request is
  modelled and logged.
- The fire of a lit item on the figure; drag off the figure (P4).
