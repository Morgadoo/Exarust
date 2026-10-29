## Why

The character creator's forms (the creation form, the body and hair editors, the pickers) existed only as tested functions: the menu's BEGIN started the unknown background directly, and the practice, new-career and versus rows opened nothing. The original routes every new character through the background picker and the creator.

Phase 7 (game modes and menus) in docs/PLAN.md.

## What Changes

- The menu's BEGIN opens the story background picker; its confirm opens the creator for that background. The practice rows, the in-game NEW GAME, NEW CAREER and VERSUS open the free creator in their game modes.
- The creation form fills the window and lays out the original's three control-record tables (the creation form's seven, the body editor's twelve, the hair editor's seven) at unit 4, scaled to the live GUI unit, drawn through the GUI path with the shown palette and the line skin, with the pickers, the hair style list and the preview viewport.
- The controls run the creator's callbacks into the record; NEXT asks for a name and shows the equipment page; its final button writes the record, which the story or practice start then spawns the player from.
- The preview draws the creation floor and the record's body in an off-screen view under the GUI canvas.
- `--open-creator <mode>` opens it once loaded, for scripted checks.

## Evidence

The creation form's, body editor's and hair editor's constructors and their 0x68-byte record arrays, the edit page's callbacks (sex, height, age, pitch, voice, hair set, facial hair), the preview viewport's builder, read with REA in the charcreate, charextra and screens packs.

## Out of scope

The saves' character screen (CONTINUE, screen mode 12), the emblem generator before a new career, the arena career, versus lobby and recruit commits, the power tree, the voice previews, the starting kit lists and the skills panel.
