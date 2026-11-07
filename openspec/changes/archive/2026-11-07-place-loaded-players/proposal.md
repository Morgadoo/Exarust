## Why

PLAN Phase 2/7 (walkable world, saves). Reloading a scratch story save
removed the save file. The question was whether the original consumes a
story save on load (a rule against save-scumming) or Exarust lost data.

## Evidence

REA (Exanima.exe), the story slot load (XA_SaveGames):
- The load opens the save, or the checkpoint first when asked; when the
  checkpoint was the one read it copies the checkpoint over the save. It
  never deletes or renames the save, and marks nothing as in use.
- The only story-save deletion outside the menu's Delete is the death rule
  (a dead player's save is deleted; the slot leaves the list without a
  checkpoint), which Exarust already follows.

Exarust (GPU grid, scratch save dirs only): after the load the player was
placed at the saved spot on the frame side only. Behind the simulation
bridge the player's root and body transforms are simulation data, so the
simulation kept the player at the level start with its parts apart from
its root; the body found no ground under its pelvis, fell to its death,
and the death rule deleted the save. The load itself did not.

Also checked: under `--story-session` the scripted A and D keys do not
walk. Standing, they turn the heading by a quarter turn (once per 1 s
window), as the original's exploration steering does; only W and S walk,
and the earlier scripted walks met walls within a cell.

## What Changes

- exarust: the level placement is also posted to the next fixed step,
  which moves the player's root, turns its body and moves the body's
  parts by the root's offset, keeping their pose.

## Out of scope

The checkpoint-first load from the menu (unchanged).
