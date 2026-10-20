# Design

## Behaviour

Showing: when field `0x60` has bit `0x20`, the relayout `0x10022c950` runs. The state at `0x158` gains bit 1, and `0x164` stores the game clock plus the delay global at `0x1002ad5b0`. The global is zero in the file and set before use, so the delay is passed in.

Leaving: a set `0x1c4` forwards to the base `0x10002ac10`. A non-zero `0x1c0` returns. With state bit 0 set, the method returns while `0x160 − 0x190` is greater than the clock, so the transition's last 400 units are still open. Then state bit 1 returns. Otherwise it calls `0x10022a380` with zeroed arguments.

## Model

`show_skill_ui(relayout, state, clock, delay)` returns the relayout flag, the state with bit 1 set and the deadline. `leave_skill_ui(closing, page, state, deadline, clock)` returns which of the base and clear calls run.

## Open questions

The relayout, the base and the clear call are not in.

## Docs

These methods are 63 and 116 bytes. `TSkillUI` moves from 1% to about 2%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
