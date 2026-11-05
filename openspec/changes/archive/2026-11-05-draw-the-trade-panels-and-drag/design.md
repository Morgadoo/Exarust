# Design

## The original's layout

The trade window's constructor makes the character view and four item
panels of the item panel class, then sets the page's border word to 8. The
panels' places add up to the page less that border: the stock's 0x234
height is the page's 0x244 less 16, and its x (the page's width less 0xe4
less 0x10) puts its right edge at the client's. So the places are in the
client, inside the border.

Each panel's required-flag pair (mask, value): pack and sell basket
(0x80000000, 0), buy basket and stock (0x80000000, 0x80000000). The buy
and sell baskets carry change callbacks that sum their items' value words
(the sell basket's then quartered by the balance) and redo the bottom row.

The layout loads the pack from the player's inventory list (stored places)
and the stock with each shelf id and the shop's flag, then lays the stock
out with the seeded grid pass; the hub's opener passes the day counter plus
one as the seed.

## The drag

The item panel's mouse slot starts a drag on a left press over an entry
with the grab at half the icon; the GUI task hands the release to the
panel under the pointer: its accept test, its drop (a drop on the own
panel repositions), then the source's drag end. A release anywhere else
cancels.

## Open questions

- The bottom row's record template is not transcribed (REA: the template
  the constructor hands its form; a later lane).
- The "Trade" button's callback is outside the window (REA: the hub's
  hooks on the window).
- Whether the loadout form's kit list accepts a drag from the arsenal box
  directly (only the drop on the preview is traced).

Docs: `docs/UI.md` (the hub's windows).
