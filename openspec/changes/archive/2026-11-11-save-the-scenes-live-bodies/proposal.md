## Why

PLAN Phase 7 (saves), XA_SaveGames and SG_GameMain. Round 144 wrote the
scene's characters into the story save's level section, but kept a
character's stored pose, motion and AI home: a character who died after the
load had no pose, so the section did not export at all ("a posed character
without a pose") and a reload brought the dead back; the follow a recruit
took was lost on a load, as the flush kept the record's stored roles. The
save unit's scene node hook (531 B) had no live caller. In SG_GameMain the
end menu's pad confirm played no click, and the help-closed and GUI-scale
notifications had no live caller.

## Evidence

REA (Exanima.exe), checked against the assembly:

- The exporter's character section, per character after the place, the
  heading and the condition: a dead body (`0xA00`) writes its pose tree
  (`0xCAC0EB00`: each part node's world matrix, its bone word and its part
  children, depth first); in save mode, for the character controller AI, a
  race word whose `0xff00` pair is `0x1000` with the AI's statue byte clear
  writes the AI home (`0xCACE0F77`: home, heading, 0); an AI in its attached
  mode 1 writes the pose tree and the motion state (`0xCAC0EA00`: the current
  motion, its progress, 0); otherwise the posed bit `0x10` writes the pose.
- The character builder scans the saved chunks; with a motion state the
  body's motion is the saved one (5 to 12 become 3 and the progress 0),
  switched with no blend and forced, and the progress is put back; with an
  AI home the AI's home and heading are set and its statue byte cleared.
- The save unit's start sets a node-added hook on the game scene; it is
  called for each class-1 node and fills three lists: volumes of kind pair
  `0x1100`/`0x1200`, bracket-named nodes without a record and volumes of
  pair `0x2200`, and operative objects of class 2.
- The end menu's confirm (called by the controls' pad routine) plays the
  `MenuClick` buffer the title menu loads, pitch 0.8 + 0.4 × the RTL draw,
  then confirms the row. The GUI scale callback (registered at boot) takes
  the scale from the window height and lays both menus out again when it
  moved.

## What Changes

- exa_game `level_save`: `LiveCharacter` carries the live pose tree
  (`live_pose_tree`, from the ragdoll's parts), the motion state (the
  animator's action number and progress, for a character with a brain) and
  an awake guardian's AI home; `characters_after` writes them
  (`live_state`); `SavedCharacterState` and `restore_saved_state` put a
  loaded motion (`loaded_motion`) and home back.
- exa_game `save_live`: `scene_character_record` writes the live roles into
  the record (`record_with_roles`).
- exa_game `save_level`: `scene_lists`, `SceneContentLists` and
  `track_scene_content` (the node hook's lists at every load).
- exa_game `end_menu_screen`: the pad confirm plays the click.
  `help_live`: the close goes through `Flow::help_closed`. `flow`:
  `notify_gui_scale` and the `GuiScale` step lay the title menu out again.
- exa_world `PlacedCharacter`: `motion` and `ai_home`; exa_anim
  `Motions::action_index`; exa_character `pose::frame_of_affine`.

## Impact

A story save after a kill now exports its level section, and the load
lays the corpse on its live pose where it fell; NPCs resume their saved
motion; a follower saved and loaded follows again. Not covered: Exarust's
living spawn ignores a saved pose tree (the original restores it before
registering the body); an AI home is written only once a wake gave the
guardian one; the scene lists' consumers keep their own queries.
