# Proposal

## Why

Two parts of the game GUI units that the shipped game reaches were still
missing:

- the item card's coin control drew its counts but no coin icons, and the
  coin sheet was never loaded (the item GUI's sprite-sheet loader reads it);
- the character view's preview bars (the HUD's health and focus bars drawn
  at the bottom of a character preview) were modelled but never drawn. The
  arena hub's loadout form turns them on whenever it binds a person.

Tracing the rest of the in-game GUI unit also showed that its power-slot
control, its segmented counter control, its bar control and the technique
selector they open belong to one control-record table that no form ever
uses: they are unreachable in 0.9.5 and stay out.

Phase: Phase 6 interface milestones of docs/PLAN.md (game GUI). This change
does not edit PLAN.md.

## Evidence

REA (Ghidra on `Exanima.exe`), checked against the disassembly:

- the coin control's procedure and draw, the sprite-sheet loader (the coin
  manager's cell side is `4 · unit`, the icons stand on the text baseline);
- the preview-bars switch, its draw callback, and its two callers (the
  loadout form's bind and the character creator's equipment page);
- the in-game unit's control-class registration, its control-record table
  and every reference into it (only the unit's finaliser);
- the HUD draw's message layer and the references to its flag (one read).

The label-group ink offsets follow from the skin image draw and the socket
row (both already ported).

## Out of scope

- The loadout form's own size: Exarust builds it at 512 × 384 units, which
  overflows a 16:9 window, so its preview's bottom edge (and the bars) are
  off-screen unless the window is taller than the form.
- The creator's equipment page's preview bars (its view values are not
  traced).
- The remaining SG_ItemGUI gaps: the icon mesh subdivision, the streamed
  icon wait, the scene-bounds helper.
