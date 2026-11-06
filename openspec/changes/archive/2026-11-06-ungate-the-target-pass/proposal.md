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

The player behaviour tick also sets the dialogue unit's end flag when the
player acts (buttons over the game view, movement keys, stick past 0.1)
while the narrator's panel offers choices; the dialogue update then ends
every narrator conversation. Exarust's engine had the flag with no setter.

## What Changes

- exa_ai: `Duellist::hold_until_ms` and its gate removed.
- exa_game: match fighters spawn without the hold; `dialogue_live::narration_input`
  sends `exa_script::EndNarration`.
