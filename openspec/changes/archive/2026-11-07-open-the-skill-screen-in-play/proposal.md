## Why

PLAN Phase 7 (game modes and content). The component gap lane for `TThaumTree` and
`TSkillUI` listed both components routine by routine. Every power-tree
routine the shipped game reaches is ported and called; the skill screen's
29 routines were ported as tested functions, but no form mounted them: the
skills key (K) did nothing in play.

## Evidence

REA (Exanima.exe), read against the assembly:
- The player controls' key handler calls the skill unit's open routine on
  the skills key (permission bit 1, outside the journal-hiding modes) with
  the player's record and page 0. The open routine runs the unit's
  once-only setup, plays `SkillShow_01`, suspends the controls, builds the
  ring screen, stores the record, refreshes the ready flag and shows it;
  with the screen open it only closes it. The close routine frees the
  form, restores the controls, refreshes the body's technique word from the
  record and refits the character.
- The screen's pointer method: the right button calls the open routine's
  toggle (closing at once) unless the screen hides or sits on a page; with
  nothing under the pointer a pending row or dialog starts its 200 ms
  close; a click presses the node (learnt asks to forget, open shows the
  learning row) or answers the pending choice.
- The node hit test sets the group's code `(group + 1) << 8` for any node
  within 23 units and adds the level for one of its five satellites; the
  earlier port left the group out. The hover filter passes a group's own
  code without the grid and compares its elapsed time unsigned.
- The node tooltip, the learning row and the forget dialog are placed at
  the level's satellite (the satellite method), not the group's node, as
  the hit test also reads them.
- The unit's cell draw submits top-left, bottom-left, bottom-right,
  top-right with the top edge at the cell's far row: the earlier quad
  started at the top right and stood the icons on their heads.
- The glow boxes are the skin's rounded box: black, grown by `unit × 2 × 4`.

## What Changes

- exa_ui: `skill_node_hit` returns the group and level code from the
  ring's nodes with their groups; `skill_hover` passes group codes and
  compares unsigned; `SkillScreen::pointer` and `SkillPointer`; the skill
  draw places the node tooltip, learning row and forget dialog at the
  satellite, orders each box and label among the batches, and draws the
  icon cells upright.
- exa_game: `skill_screen` mounts the screen on K (through the hotkeys),
  cuts the two atlases at the interface unit, plays the reveal sound,
  steps the fades, the hit test, the hover filter and the buttons each
  frame, keeps the story's experience copy in step with the record, and
  rasterises the draw list (the four programs, the rounded boxes and the
  GUI font's glyphs) over the HUD; closing refreshes the technique word.

## Impact

Only the skills key changes behaviour. Scripted story levels and the arena
duel are unchanged tick for tick.
