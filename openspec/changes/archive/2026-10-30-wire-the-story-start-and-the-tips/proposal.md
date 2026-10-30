# Proposal

## Why

Phase 7 (Game modes and content, story act 1): the flow already emitted the
story start's steps and the help tips, but the running game only logged
them. `Step::Tip` never reached the prelude's popup, so the knock-out's and
death's lines ("You have been defeated.", "You have died.") never showed;
the menu's MANUAL rows warned "not wired"; and the story start's
`StoryPlayer`, `StoryLight`, `StartRecord` and `LevelTime` steps were
dropped. Worse, every step after the start's level load ran before the new
player existed (the creator despawns the old one), so the start's
knock-out, pose and motion fell on nothing. Backlog items U4 and G2.

## What Changes

- `exa_game::flow`: the steps that follow a level load in a batch are held
  (`AfterLoad`) until the load has shown, two quiet frames have passed, a
  player stands in the level and the carried placement has been applied;
  the flow's hook gets no player reading meanwhile.
- `StoryPlayer` writes the start's off-hand item and carried entry into the
  record the load spawns the player from (the power word only on a record
  that has a power block) and keeps the role list and spawn word;
  `StartRecord` gives the spawned player that word, the role list, the gear
  and the start motion.
- `StoryLight` is the start's torch item (0x1f) laid in the world at its
  pose. It is laid only when the player carries no torch (`--no-torch`), so
  the default start keeps the torch in the hand (the owner's decision is
  pending). `LevelTime` keeps the engine clock for the save unit.
- `Step::Tip` shows the prelude's line on the tip banner; the prelude's
  close and a level load hide it. The tip and hint lines are the banner's
  tables in `exa_ui::game_tips`.
- `exa_game::help_live`: the MANUAL rows open the manual (`manualkm.fds`, or
  `manualcn.fds` with the controller) over the covered menu; list clicks and
  the arrows pick topics, the wheel scrolls, Escape, F1 or a right click
  closes it. The menu's F1 and input yield to the open manual.
- `--open-creator begin:N`: the story creator for background N commits its
  opening record at once (an unattended new story, for GPU checks).

## Evidence

REA on `Exanima.exe` 0.9.5: the story start routine (record writes, the
spawn request with its word and start-record pointer, the character
spawner's request and the player build, the torch routine for backgrounds
0 and 3: the game database's item 0x1f made with flags 0, its position,
three-angle frame and node level 256, added to the world), the prelude's
tip routine and its four callers (knock-out, death, arena match end, end
menu), the 24-entry tip table, the menu's row handler (MANUAL opens the
manual with 4 and sets the menu form's help flag), the manual's open and
close. GPU run on the RTX 3090 in `exanima01`.

## Out of scope

- Moving the story torch by default (the owner decides); the torch's fall
  and settling (Exarust's floor torch is a static model).
- The arena's one-time help tips (Getting Started after a new career, the
  inventory tip in the arena) and the two-pane tutorial help screens.
- The save field that reads the level-6 clock stamp; the float the unknown
  background writes beside its power slots.
- The manual's `[rimg=…]` tutorial pictures.
