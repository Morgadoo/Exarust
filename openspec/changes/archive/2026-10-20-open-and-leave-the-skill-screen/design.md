# Design

## Behaviour

Showing: when the style word has bit `0x20`, the relayout runs. The state gains bit 1, and the second deadline stores the game clock plus the delay global. The global is zero in the file and set before use, so the delay is passed in.

Leaving: a set closing field forwards to the base. A non-zero page returns. With state bit 0 set, the method returns while the first deadline less 400 is greater than the clock, so the transition's last 400 units are still open. Then state bit 1 returns. Otherwise it calls the clear with zeroed arguments.

## Model

`show_skill_ui(relayout, state, clock, delay)` returns the relayout flag, the state with bit 1 set and the deadline. `leave_skill_ui(closing, page, state, deadline, clock)` returns which of the base and clear calls run.

## Open questions

The relayout, the base and the clear call are not in.

## Docs

These methods are 63 and 116 bytes. `TSkillUI` moves from 1% to about 2%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
