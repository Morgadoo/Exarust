# Design

Format pages: none (behaviour only; `docs/UI.md` Character view describes
the view these callbacks drive).

## The inventory call

The original's inventory call takes a character. Its show rule is the
player's (ready, not dead), the finder matches an open, visible inventory
whose character is the one asked for, a filling one is left, an active one
closes only with the toggle, any other is focused; a new one is tagged
with the character's record id and the container bit, bound to the
character (window, pack owner, view's shown character), filled from that
character's look, and placed: the player's own at its remembered place
(unless never set), another character's at the screen width less the
window width less four GUI units, at the constructor's y. The close
remembers the place only for the player's own window.

Callers: the inventory key and the interaction on the player's own body
(toggle), the interaction on another living character (toggle), and the
`ShowInventory` builtin (no toggle, actor index clamped to 0).

Exarust keeps one inventory window: `GameScreens` records the owner and
its body, the consumers (pack bind, panel moves, character view figure,
hands, drop and drag off) read the bound body, and asking for another
character while one is open closes it first.

## Which shipped characters pass

`charroles.rdb` holds two interaction-condition sections: the Golem
Boss's toward the Player role (`result = Actor[1].HasItem($1DE);`) and
the Golem Servant's toward the Leader relation role (`result = 1;`). No
companion and not Derrin has one, so their inventory opens only through a
Loyalty or Command Dead marker the player cast, or the Ally role's
`Equipment` topic, whose nodes run `Actor[1].ShowInventory;`.

## The hub previews

Both are the character view. The trade window gives it no shown
character, no linked pack and sets its lock; its drop-back puts a
displaced piece that is neither the braies nor the shop's into the pack
at its centre, and its give-back does the same for a hand item the
take-in moved out. The view's release answers -1 for a shop item on a
locked view, so a tried-on shop item comes off wherever its drag ends.

The loadout form installs a pre-drop hook (a box item joins the kit list;
a kit-list drag's source is cleared, so the kit list keeps its entry), an
accept hook (a kit-list item needs more copies than the shown record's
two out hands hold; then the rank and points gate), a change hook (the
view's record is stored into the tab's loadout, the points line and the
marks are redone) and, on the roster page, a drop-back that hands nothing
back. The kit list's drop answers a drag off the preview with -1 and
clears its item (take off, no copy added). The arsenal box's drop takes
the item back like a kit-list return, removes the first kit-list copy,
runs the view's release with -1 and trims every loadout to the kit list's
remaining copies less one.

## Open questions

- Whether the trade window's stock keeps an item dragged onto the
  preview (Exarust's stock panel removes it, as it does for the buy
  basket). Not traced further.
