# Design

## Picker client

Every picker routine measures `right − left` and `bottom − top` of the
form's client rectangle, and the pointer reaches the pick already taken
to the client's corner. The form layout writes left and top as the
border (a picker has no title) and right and bottom as the size less the
border (no scrollbars). The border is the form constructor's 2: the
rescale keeps a border under four, and the frame stretch the picker's
style word selects (2 from the skin constructor) is not larger, so the
border is not widened. Exarust keeps the picker's `size` as the form
size and derives the client with `field_size`; the creator insets the
form rectangle by `FIELD_BORDER` for picks and draws.

## Compact tree

The pip painter copies the four-corner texture rectangle and rewrites
the second and fourth corners' u from the first and second corners' u;
both v words are untouched. The node painter loads the line colour, sets
its alpha byte to the byte times the fade (rounded to even), and for the
slot in training shifts that stored byte right by one, so the order is
scale then halve.

## Main menu

The end-page branch tests bit 4 of a word and stores the forfeit phase
0x10 into the same word when the bit is clear, then finishes the match.
That word is the session state (the in-game EXIT clears it to 0); the
game mode (practice 2, story 4, arena 8) is the word the in-game rows
compare.
