# Design

## Where the check runs

The original's body tick calls the check from three places: the combat
idle (a), the Fend clip (b, not modelled) and the end of the tick for a
parrying fighter (c). (a) needs muscle tone of 0.85, no motion in the
attack range and the combat idle itself, a target attacking, the
permission, and the auto-parry deadline (a signed 32-bit compare against
the unsigned clock) passed. After a success at (a), (c) runs the check
again in the same tick, which is why the counter moves twice. A fighter
not parrying lets the counter drift to 0 at (c).

`run_parries` reproduces (a) and (c) after `run_swings`, for the ticked
body only, in three phases so the ECS borrows stay disjoint: it reads its
own state, scans the foes once (the nearest attacking foe that passes the
trigger, no allocation, no sort), then mutates. A body with nothing to
check or settle returns before the scan and flags nothing as changed.

## The check

Clear conditions come first (helpless, the break-tangle clip, yellow under
0.01 in double). A fighter without the permission only drops the flag.
Otherwise the trigger runs against the foe; a failure clears. A success
raises the flag, stamps the clock, re-picks when the clip is outside the
parry clips or the foe's motion differs from the countered one (storing
both), advances the counter from the stored clip, and starts the picked
clip with `round_ties_even(speed * 20)` frames and the fast ease when it
differs from the one playing.

## Ending

The clear returns to the Defence stance if yellow is under 0.01 or the
fighter is dazed, else the combat idle, starting that clip with 40 frames
smooth unless it already plays, and not at all when the condition's clip
stands still. A parry the hit path ended leaves its clip playing; the
non-parrying branch finds it and does the same restart.

## Faithful limits

An armed fighter's speed and grip come from the weapon's handling; that
rule exists (`armed_grip`) but its inputs (the hand links) do not, so an
armed fighter does not parry rather than parry with the wrong speed.
