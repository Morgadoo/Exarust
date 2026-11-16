## Why

Roadmap task B4 (lane `charview2`): the character view's open rows. The
view's player-pack answer had no caller, so the game unit's crystal drain
drained the record while the open inventory's pack kept the old charges
(and wrote them back on its next save); the view's shown item (the core
crystal over the figure), its icon draw and its hover test were never
driven; and the view's image ignored the settings' supersampling.

## Evidence

REA (Exanima.exe 0.9.5), read in the assembly where order and constants
matter:

- The inventory source's drain asks the character view for the player's
  first view: with one, every crystal entry (item 499) of its pack panel
  loses its charge and has its icon released and resolved again for item
  499 with the new flags; without one the record's carried list drains.
- The view's bind ends with the first worn item that is one of crystals
  A, AL, B, C and AR shown over the figure (nothing shown without one).
- The show resolves the item's icon; an item whose icon resolves is kept
  with its flags and adds the view's draw callback, anything else clears
  it and removes the callback. The callback draws the icon at (unit, view
  height less unit) by the icon draw's origin rule, under the view's
  scissor.
- The hover pick answers the shown item and its flags first when the
  pointer, taken into the icon's space from the same point, meets an
  opaque pixel of the icon.
- The drag-over test takes a core crystal only while nothing is shown; the
  drop of an item with no apparel, weapon or shield section that the body
  takes as its core shows it, clears the shown body's mark and ends the
  source's drag; the record's lists do not take it.
- The release of a drag whose icon is the shown one takes the shown item
  off the record's worn list, sets the body's mark, runs an effect on the
  body and shows nothing.
- The constructor's first choice of super sampling: 1 at or below 0, the
  setting plus 2 for 1 to 3, 4 above.

## What Changes

- `GameScreens::player_pack_mut` answers the player's own open inventory's
  pack; the crystal drain's inventory source drains it while it is shown
  (`crystal_drain::drain_open_pack`) and the carried entry each panel entry
  was loaded from takes the same flags.
- The inventory's character view keeps a shown item (`InventoryDoll::shown`,
  `doll_view::show_on_view`), bound from the worn crystals when its scene
  is made, drawn at the view's lower-left by the inventory's painter, picked
  first by the figure's hover pick, taken off by a release of its drag, and
  set by a core-crystal drop (`figure_accepts_shown` with the shown icon).
- The inventory view's image renders at `doll_view::doll_scale` of the
  settings' super-sampling switch (setting 4 on, 0 off).

Not changed: the body mark the core's drop clears and its take-off sets,
the take-off's effect and part refresh (not modelled in Exarust); the
creator's own view.
