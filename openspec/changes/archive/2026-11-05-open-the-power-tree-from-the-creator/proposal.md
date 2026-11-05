## Why

TThaumTree's remaining gap (rated 82%) named the creator page that opens the tree: the tree's creation mode (its prompt "Choose which new power you wish to learn first.", BACK and BEGIN, the task word 0x10, the hints at 0xff, no calling form) was ported and tested but nothing opened it. The creator's record never got the thaumaturge's starting powers either, so the story's unknown background committed straight away and started with no powers.

Phase 6 (thaumaturgy), milestone: the power tree screen. No docs/PLAN.md edit in this change (the main session owns it).

## What the original does

Evidence: REA (Ghidra on `Exanima.exe`), checked against the decompiler output of the creator's final button, its two tree callbacks, the tree's opening routine, its close and the story start.

- **The unknown background's powers.** Loading the unknown background (its preset or a new record) ends with the creation defaults: the ten slots cleared, two known powers `0x101` and `0x201` with `0x5FFF`, slot 1 `0x100101`, slot 2 `0x201`, the potential 4.0 and the record's power word `0x510000`. The story start writes the same power word and potential again.
- **The final button.** On the equipment page (page state 1) a record with known powers sets the page state to 2 and queues the tree's opening; the button's label is not written for such a record. In any state but 1 the button runs the appearance page's branch (name check, page state 1, the equipment page rebuilt), and the creator's BACK in any state but 1 leaves the creator.
- **The opening.** The queued routine closes the game's panels and any shown tree, hides the creator's form, opens the tree with no calling form (creation mode 3, task word 0x10, hints 0xff) and installs two callbacks on it: cancel and commit.
- **Cancel (BACK, or the form's 0x200 word).** Queued: the tree closes (its vtable close, the bar stored, the shown flags cleared), the creator's form is shown again and given the focus. The page state stays 2.
- **Commit (BEGIN, or the form's 0x100 word).** The preset is saved, the record's worth stored, the reload flag cleared and the game start queued, whatever the mode. The tree's clicks have already written their pairs into the creator's record (the tree works on the record itself).

## What Changes

- `create::apply_creation_defaults` writes the defaults into a record; the creator's entry record takes them for the unknown background.
- The creator offers the record's known powers to its final button, keeps the button's label for a record with powers, and on the tree commit hides its form, sets its page state 2 (`Creator::powers_page`) and asks for the tree (`CreatorAction::OpenPowerTree`).
- `power_screen::CreatorPowerTree`: the request opens the tree in its creation mode on the creator's record (no bar); the prompt's BACK and BEGIN, Enter (0x100) and Escape (0x200) answer, the tree leaves and fades as its close does, and the game's own record is put back.
- `Creator::tree_answered`: BACK shows the form (state 2 kept); BEGIN writes the tree's pairs, slots and potential into the record and commits as the story's BEGIN (story) or CREATE (other modes) (`create::tree_begin_kind`).
- `PowerRecord::of_character` and `write_into` convert between the character record and the tree's record.
- The story start writes the power word into the record's power word and the potential into the power block (it wrote the word into the potential).
- `--open-creator powers`: the unknown background's creator, named if unnamed, presses its final button on both pages (a debug tool).
- The tree's texts (prompt, tooltip, labels, the wheel's label) were never drawn on screen, for two reasons found with the installed packs: the tree loaded its GUI font atlas at the start's default interface unit (5, from a 900-pixel design height) and a 720-pixel window runs at unit 4, where another level's atlas gives no glyphs; and the skin's text program sampled the atlas through the sheets' flipped rows, where the glyph cells address it from its first row as the GUI's own text draw does. The atlas now follows the unit, and the text program reads the atlas as the GUI text draw does.
- The tree's invented `TreeScreen::commit` (turning a new pair into a learnt and picked one for the creator) is removed: the creator's BEGIN saves the record as the clicks left it.

## Out of scope

- The hand-listing step the creator's own commit runs before saving (the tree's BEGIN skips it in the original too; Exarust does not model it either way).
- The preset save and worth store: the existing commit path's behaviour is kept.
- SG_Thaum_DIsp's remaining gap.
