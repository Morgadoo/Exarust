# Design

## Behaviour

The handler starts with result 0. When both selections are non-positive it goes to the key-`0x100` block; otherwise bit 0 selects the move path and any other key the base/hint path.

The move path uses a high selection for simple steps: back one on key portion `0x1001`, forward on `0x2001` while the index is under two. Without a high selection, `0x1001` scans backwards from index − 2 and takes the slot after the first zero; `0x2001` scans forwards from index − 1 while the index is at most seven. A moved index above zero and different from the old is published.

The base/hint path calls the base on bit `0x100`, or the hint on bit `0x200` with a hide of 200 when hidden and bit `0x40` cleared, then publishes the high selection if positive else the low one.

Key `0x100` handles the repage: a result of 1 sets the high slot to the index and publishes 1; a result of 2 sets the low slot to the index, fills the grid, selects the slot after the first zero (or zero when none) and publishes it. Any other result ends. Both successful results then hint, set the hide to 1, the deadline to the clock plus 300 and bit `0x40`, and fall into the closing step.

The closing step returns on a closing word of 1. Otherwise a key with bit 0 reads the cursor and publishes a non-zero hit, and a key with bit `0x200` while state bit 1 is clear runs the clear call.

## Model

`skill_screen_key(input)` takes the key, selections, index, state, hide, deadline, page, closing, clock, grid, repage, hit and flags, and returns the index, selections, publications and effects.

## Open questions

The base, hint, cursor, hit and clear bodies are not in.

## Docs

This method is 864 bytes. `TSkillUI` moves from 12% to about 16%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
