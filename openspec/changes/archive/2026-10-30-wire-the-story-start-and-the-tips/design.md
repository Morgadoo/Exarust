# Design

**What a story start does, in order (REA).** The start clears the player's
roles and adds 7, 1; the unknown background appends item 0x10c with the
cell words (0x20, 0x80) and writes the power word 0x510000 and 4.0 beside
the power slots; the knight and proctor put item 0x1f in the off hand and
carry it with flags 0x100. It fills a spawn request: the record, a word
(0x200 unknown, 0 knight/proctor, 0x600 villager), the level-start point
(8200, 25, 3920) and a pointer to the 12-byte start record (the start
motion 0x1d, 0x13 or 0x1b, then zeros), hands it to the character spawner
and loads `exanima01`. After the load it builds the player from the
record, then for backgrounds 0 and 3 makes the torch: item 0x1f from the
game database, flags 0, at (8045, 4.7, 4075) turned (−48°, 110°, 88°)
(villager: (8000, 500, 4000), (8°, −150°, 50°)), its scene node raised to
resource level 256, added to the world. Then the motion, the lie-down
(3000 ms) or the villager's 0x600 words and turn factor, the placement and
the camera; Derrin last. The routine never calls the tip popup.

**Holding what follows the load.** The original's load is synchronous;
Exarust's is a request the viewer serves over many frames, and a new
character's body is spawned by it. `apply_steps` therefore applies a
batch up to its `LoadLevel` (and any `PlacePlayer`, a resource the viewer
consumes once the player stands in the level) and holds the rest in
`AfterLoad`. `release_after_load` runs before `apply_steps`: once the
loading markers (`LevelChangeReady`, `LoadingStatus`) have been seen and
then absent for two frames, a `PlayerBody` exists and no placement waits,
the held steps join the queue. A load that never shows releases after
600 frames. The flow's `tick` passes no player while steps are held, so the
story hook does not read the body before the start's writes.

**The spawn word.** The request's word matches the conditions the start
writes later (the unknown's lie-down 0x200, the villager's 0x600); it is
applied as the spawned body's condition [guess, labelled in code].

**The torch decision.** The owner has not decided whether Exarust's start
moves the torch from the hand to the floor. `StoryLight` therefore lays the
torch only when the player body carries none, and logs the original's pose
otherwise. `--no-torch` with `--open-creator begin:0` reproduces the
original's start.

**Tips.** `Tips::show` already modelled the selection; the lines (18 tips,
6 hints) are now `exa_ui::game_tips` tables, `Shown::text` maps a shown
entry to its line, and `help_live::paint_tip_banner` draws the banner from
`GameTips::draw` (box alpha 160) in the main font while no menu is open.

**Manual.** `help_live::read_manual` reads the pack's table of contents
for `parse_topics` and its records for `Manual`; the form is the original's
0x2fc × 0x1e6 at the 4-pixel unit, centred, with the 0x90-wide list and the
page box, painted after the menu.

Open questions: the save field that reads the level-6 stamp (no reader
found in the save unit's routines searched); the record field of the
float beside the power slots. Format page: `docs/formats/OTHER.md` (manual
pages), unchanged.
