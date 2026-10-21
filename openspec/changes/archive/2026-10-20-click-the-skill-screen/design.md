# Design

## Behaviour

The handler starts at result 16. An index below −1 returns the base result at once. The point is transformed through the view's two transforms.

Mask bit 1 leaves when the page is positive. With a closing word it returns on 1 and otherwise finishes; without one it clears when state bit 1 is clear and returns 1. The clear runs through the screen's clear routine.

Otherwise an index at or below zero with a selection hides for 200 and hints. Index `0x10000` needs mask bit `0x2000`: state byte 4 shows for 500 at `0x160`, state bit 1 returns, and otherwise the state gains bit 1 and `0x164` takes the clock plus the delay.

A selected index with flag `0x1ea` clear and mask bit `0x2000` set either repages (1 sets the high slot, 2 sets the low slot and fills the grid, anything else returns 16) or, with selections, selects on index 1, or when the used byte is clear runs the selection result and writes the row and column slot, then hints with a 200 hide.

## Model

`handle_skill_input(input)` takes the index, mask, page, closing, state, selections, hide, flag, clock, grid, used bytes, repage, selection, base, delay and the three deadlines, and returns the result, changed state and the action flags.

## Open questions

The called bodies are not in.

## Docs

This method is 826 bytes. `TSkillUI` moves from 16% to about 20%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
