# Tasks

- [x] Trace the controls' per-frame pad routine in REA against the
  assembly. Cover its bit order, its gates and its callees (the pause
  menu's toggle, the notice window's close, the bar, the tree, the
  inventory, the skill screen, the dialogue opener with the screen-nearest
  character).
- [x] Add `pad_hotkeys` and dispatch its commands through the key
  handler's match.
- [x] Make the pad's pause toggle the menu.
- [x] Add the scripted pad's new names. Let the scripted pad win over a
  reported device.
- [x] Remove the uncalled screen-nearest duplicate.
- [x] Tests: the commands' order and gates.
- [x] GPU check in the grid: the pad opens the inventory and the menu,
  asks for the swap and the skills.
