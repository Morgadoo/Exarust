# Proposal

## Why

The game-main unit's fourth pass (lane gamemain4) extended `exa_game::flow`
with the story start's hand-over data, the first wake-up's end on the
animator word, the respawn and revive menu closes, the next practice
fight's condition clear, the sixth level's checkpoint stamp, the death
screen's record flag, a NaN health's alive tip and the launch's unlock
reads. The review against the assembly kept these, but found the exit
nudge's sign test inverted for an unordered offset and the respawn's
player-controls re-attach mapped to taking the controls away.

Phase 7 (Game modes and content), the game-mode flow milestone.

## What Changes

- `flow`: the story start records the level-start point (8200, 25, 3920),
  the pre-load view (forward +z, looking at (8200, 0, 4000)) and the
  twelve-byte start record (the start motion, then two zero words); the
  practice start's view is forward −z looking at (0, 20, 500).
- The first wake-up animation runs while the player's animator word
  (`WakeFlags`, written by the wake and story-pose steps) holds 0x20.
- The revive closes the prelude first; the story respawn hides the end
  menu and closes the prelude after its save, and keeps the controls.
- The next practice fight writes a zero condition word (applied to the
  player's `exa_combat::Condition`).
- A level exit that makes the sixth level's checkpoint adds `LevelTime`.
- The death screen's CONTINUE writes the player back with flags 0x100.
- A NaN health takes the death screen's alive tip.
- The boot applies the settings store's `[1, 0x6431ac01]` (expert) and
  `[1, 0x6431ac02]` (backgrounds) words; the app reads them from the
  user's store.
- Review fix: the exit nudge negates the push for a zero or unordered
  offset (only a strictly positive one is pushed forward).

## Evidence

The game-main unit's story start, practice start, story hook, revival,
story respawn, continue-after-death, next practice fight, death path,
level exit, exit nudge, unlock reads and boot, read in the assembly.

## Out of scope

- A motion that clears the animator word's 0x20, so the first wake-up
  ends on its first frame and a later revival after a get-up keeps the
  animation flag set; the start record's consumer; the level time's value;
  the player-data record's consumer.
- Mapping the combat-mode setter the revive and practice calls make (the
  flow still names it as controls).
