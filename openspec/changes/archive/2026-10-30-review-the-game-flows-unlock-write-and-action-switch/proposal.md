# Proposal

## Why

The game-main unit's sixth pass (lane gamemain6) wrote the expert-ladder
unlock into the user's settings store with the original's once-only flag
and applied the flow's requested motions to the player's animator, which
were emitted but dropped. It also moved the practice spawn to the origin.
The review against the assembly kept the unlock write and the motion
wiring, but found the spawn move wrong: the placement reads the spawn
table's x and y as one 8-byte word and z on its own, and only y is
overwritten, so the player stays at (0, height, 500). It also found the
end menu's practice branch gated on the practice mode, and every flow
motion to be the body's unforced action switch.

Phase 7 (Game modes and content), the game-mode flow milestone.

## What Changes

- `flow`: a novice practice win in the practice mode shows tip 5 and, while
  the stored expert word was clear, writes `[1, 0, 0, 0]` under settings
  path [1, 0x6431ac01] once (`Step::SettingsWrite` → `SettingsEntry`);
  the expert ladder shows tip 6 and writes nothing; outside the practice
  mode neither runs.
- `exarust::user_settings::write_flow_settings` writes and flushes each
  entry into the user's store.
- `Step::RequestAction` starts the numbered action on the player's
  animator unless the body is already in it
  (`exa_anim::Motions::start_requested`, shared with the cast pipeline).
- The practice start and each next fight place the player at (0, height,
  500) — unchanged from main.

## Evidence

- The practice start and next fight copy the 12-word spawn table
  (rotation, then 0, 0, 500) and overwrite its eleventh word only; the
  placement setter reads the position as an 8-byte (x, y) read and a
  4-byte z read.
- The end menu tests the practice mode word (2) and phase 4 before the
  ladder test, writes the unlock only while its flag is zero and then sets
  it; the boot seeds that flag from the stored word.
- The story start (0x1d/0x1b/0x13), wake (0x20), respawn (0x2f) and
  practice (3) motions all call the body's action switch with the force
  flag clear, which returns early when the body is already in that action.

## Out of scope

- The `GameEvent` handler's consumer (and so the background unlock write).
- The end menu's flag tests the whole stored word while the main menu
  tests bit 0; Exarust keeps one bit for both.
- The action switch's other resets (timers, blend words) beyond the
  animator start.
