# Design

## Behaviour

The hover index is 1-based (`0` none, `−1` untested) and maps to an entry index. A right-button event falls through to the base form. A 0x4000 event starts a drag only when the panel allows it and an entry is hit; the payload's initial grab is the entry's centre, then it is reset to the cell's centre, so the icon is centred under the cursor. A 0x8000 event uses the item, or opens it when the item record has a container section. A 0x2000 event shows the item's card.

On drag end the source finds the entry carrying the drag flag. A cancelled drag clears the flag and keeps the entry. A refused drag-out restores the entry's flags. A drop into the linked panel shifts the array left without changing the bulk; any other drop removes the entry, subtracting its bulk and shifting the tail.

## Model

`mouse_intent(button, hit, allow_drag, is_container, cell) -> MouseIntent`; `drag_grab(cell)`; `drag_end(dragged, cancelled, allowed_out, linked) -> Option<DragEnd>`.

## Open questions

The callbacks themselves, the popup placement, the item cards and the icon cache are separate.

## Docs

`TGUIItemContainer` is **25%**. The headline rises slightly.
