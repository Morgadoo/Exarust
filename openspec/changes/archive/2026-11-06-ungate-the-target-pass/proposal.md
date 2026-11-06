## Why

Round 81 playerctl lane: round 80's hold on the target pass's combat entry
was a guess. REA shows the AI timers the match spawners set do not gate
the pass.

## Evidence

REA and the assembly: the AI tick's target pass step runs the pass when
the brain has the turn and its active word is 1, with no timer test. The
spawners' and round reset's timers are the idle look's: its glance check
reads the first (due → a new random look heading, the timer re-armed
3000 ms and the second 1000 ms ahead), the second gates its next step,
and the third (now + 10000 + a 0..10000 draw) is written but never read.

## What Changes

- exa_ai: `Duellist::hold_until_ms` and its gate removed.
- exa_game: match fighters spawn without the hold.
