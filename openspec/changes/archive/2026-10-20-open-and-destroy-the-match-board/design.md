# Design

## Behaviour

Showing writes zero to the stored pointer and byte, the seed to its global, and each list's child count less one to three more globals. Five panels are cleared only when their count is positive. A non-null held pointer is cleared. The base show returns the result, and a non-null global callback is called with minus one.

Destroying calls the base destructor through its slot when the flags are positive, clears the stored pointer, releases the board's list, calls the base, and frees through the free slot when the object and flags are both present.

## Model

`show_match_board` returns the stored seed and columns, the five cleared flags, the pointer and global actions and the base result. `destroy_match_board` assumes the object is present and returns the four actions from the flags.

## Open questions

The panel, base and global routines are not in.

## Docs

These methods are 379 and 121 bytes. `TMatchBoard` moves from 0% to about 3%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
