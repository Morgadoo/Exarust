## Why

Round 32's forms lane (TLoadoutForm next pass) added a drop "sort" and a paint frame for the arena loadout editor. Review against the listing found the lane read the 2814-byte drop handler as an equip, when the constructor attaches it to the company arsenal: it takes an item back from the kit. The lane's gate also counted the arsenal's copies, and its match-mode release duplicated the existing one on the wrong container.

Phase 7 (game modes and content: the arena's loadout editor, UI) in docs/PLAN.md.

## What Changes

- The arsenal's drop handler is ported on the editor's list model (`exa_ui::loadout::return_to_stock`, `sort_drop`): the mask release for the form's category, the sort into the kit container the item left, the off-hand copy freeing the worn entry a hand holds, the preview-only removals and the per-container count written to the rank's record.
- The equip gate counts the weapon and worn containers only (`copies`), as before the lane.
- The lane's duplicate match-mode release is removed; `apply_drop` is again the list model's equip behind the gate, keeping the worn slot for an item found in neither list.
- The editor's screen (`exa_game::loadout_screen`) and the flat-fill paint frame stay as the lane wrote them.

## Evidence

The constructor's container slots (the arsenal 228 wide at (4, 180), the stock container on the right taking the 2814-byte handler on its drop-received slot, the preview taking the gate, the release and the 173-byte drop), the gate's count of the two kit containers, and the handler's branches, read in the forms pack's assembly.

## Out of scope

The arsenal's own lent-item query and quantity write (outside the pack: passed in as `lent`, reported as `zero_stock`), the record's hand-slot clearing and per-rank record walk (reported as `freed_hand` and `count`), the containers' generic drag and drop, the arsenal icon tree and the character preview. Nothing opens the loadout screen yet.
