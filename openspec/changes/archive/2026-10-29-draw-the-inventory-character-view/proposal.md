## Why

The inventory window drew its frame, doll panel, label, Primary button and pack, but not the player's figure in the character view, not its second record and not the equip-by-drop onto the figure. Phase 6 (GUI) in docs/PLAN.md.

## What Changes

- The character view draws the player's dressed figure: a copy of the player's body on its own render layer, drawn by an off-screen camera at the view's pose into a small image shown under the GUI canvas only while the inventory is open; the figure turns toward the pointer.
- The inventory's second record (class 0xac01) is the item GUI's rating-dots control and shows ten times the player's encumbrance with the dot sheet's whole and half dots. The routines that write it were read as a camera refit; they are renamed.
- A drop on the figure wears apparel through the equip conflict pass, or takes a weapon into the hands by the take-in plan, or a shield into the off hand.
- The view draws no preview bars: only character creation and the arena switch them on.

## Evidence

The inventory constructor, bind and encumbrance callback; the character view's constructor, background, task and drop; the item GUI's control table and rating-dots draw; the preview-bar switch and its two callers.

## Out of scope

The view's kept still behind the figure (taken as the window's own client), the torch on the figure, drags off the figure, the hover pick and the card from the figure, the mind-cost checks of a controlled character's view.
