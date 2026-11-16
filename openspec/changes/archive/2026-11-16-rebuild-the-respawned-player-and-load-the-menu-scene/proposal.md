## Why

Roadmap task B4, the XA_SaveGames rows the saves3 lane left. The death
screen's CONTINUE kept the surviving body (the dead body holding the
dressed copy of the record), so the respawned player woke with the gear
the corpse also kept; the menu background scene was never loaded, so the
title showed over an empty world; and the level exit's transition save ran
a plain save then a checkpoint step that left an existing checkpoint alone,
where the original copies over it, and only for a destination the save
did not hold yet. The exit never asked whether its level exists.

## Evidence

REA (Exanima.exe), read in the assembly:

- The CONTINUE's request takes the current record, clears its race byte and
  posts it to the content stream with the word `0x1000` and the world scene
  while the stream's hook is set; the respawn saves, clears the world (the
  player's body word goes to 0), loads `exanima05` from the save and spawns
  the player from the current record.
- The menu scene load (after the splash and in the return to the title):
  the GUI and dialogue resets, the import of `menuscene.rfc`, then the camera
  node's field 60.0, its position (0, 500, 700) and its target node's
  (0, 0, −200), constants read from the data.
- The transition save tests the open package for an `exanima04` section,
  then for the destination's, and only with neither flushes, saves and
  copies the package file over the checkpoint with `CopyFileA(…, 0)`; the
  exit takes its plain save when it returns 0 and stamps the level-6 clock
  only when it returns 1. The exit first asks the level-available test
  (the save's section, then the archive) and does nothing without it.
- The between-round reload's first call is a GUI form focus, not a scene
  reset; the world clear is the scene's own clear call before the archive
  reimport.

## What Changes

- `Step::PlayerData` resets the player record's identity and posts its
  request (`LoadedPlayerStream`); `Step::RebuildPlayer` (in the respawn,
  after its save) hands the saved record to the next spawn (`SavedPlayer`)
  and despawns the old root (`save_live::RebuildPlayer`). The save steps run
  in the frame the flow applies them.
- `Step::MenuScene` in `after_splash` and `quit_to_menu`: the world reset and
  the `menuscene` load with `level_change::MenuSceneView`; the viewer spawns
  no player for it, the chase camera stands at the fixed view and the
  surviving player's body stays out of the scene.
- `Step::TransitionSave` / `SaveStep::TransitionSave` for a level exit that
  keeps a checkpoint; `save_live::level_available` asked by the exit zones
  and journeys.
