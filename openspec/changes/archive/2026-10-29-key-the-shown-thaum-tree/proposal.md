# Proposal

## Why

docs/PLAN.md, Phase 6 (the power tree screen, thaumtree lane). The rating
left one gap in TThaumTree: the GUI form glue and the base form's key
handlers. The running screen read only `P`, the arrows and Escape while the
tree was shown, and the tree's key rule named its toggle "open", although
with the tree on screen it closes it.

## What Changes

- `exa_ui::form::form_key` models the base form's key handler: Tab among
  the records (Shift backward), the focused record's answer (both low bits
  take the key and ask for a redraw), else the parent's key slot unless the
  key repeats or was typed.
- The tree's key rule names its result `toggle` (the open-or-close routine,
  which closes the shown tree) and reports `base` for a screen that is not
  the play screen; the form byte's two bits are the opening and closing
  fades.
- While the tree is shown, the running screen sends every pressed key to
  the tree's key rule: the arrows step the bar, any other key closes the
  tree, except in the opening fade's first 50 ms and while it fades out.
- READMEs and docs/THAUMATURGY.md describe the keys.

## Evidence

REA (Ghidra) on Exanima.exe 0.9.5: the tree form's key slot, its
open-or-close routine and close, the draw head clearing the two fade bits,
the base form's key handler, its control index and redraw request, and the
window procedure's key words (1 for a repeat, 2 for a typed character),
read in the instructions.

## Out of scope

Key focus between the play screen and the tree form beyond the shown tree
(the player's polled movement keys), the skin panels.
