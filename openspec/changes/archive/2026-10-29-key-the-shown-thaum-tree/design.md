# Design

## The tree form's key slot

In play mode the slot ignores keys during the opening fade's first 50 ms
(its stamp less 950 ms still ahead), steps the bar on the right and left
arrows unless the key is a typed character, and otherwise calls the tree's
open-or-close routine unless the closing fade runs. That routine, with the
tree present, closes it (hide, bar close, the player controls' flag). Any
other mode goes to the base form's handler.

## The base form's handler

A positive focus code naming a record: Tab moves the focus by the existing
`tab_target` rule; another key goes to the record's handler with message
0xa0, and an answer with both low bits takes the key and sets the GUI's
redraw bits. Otherwise a key whose flags have neither low bit goes to the
parent's key slot. `form_key` returns which of these happened; the caller
does the focus change, the redraw or the forwarding.

## Running screen

`power_screen` maps each just-pressed Bevy key to its virtual-key code and
feeds the tree's key rule with the guard from `FADING_IN`, the closing bit
from `closing()` and the stamp `open_at`. Escape and `P` now close the tree
by that same rule. Escape with the tree closed still puts the bar away.
