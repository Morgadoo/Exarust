## Why

The motileai7 lane wired several TMotileAI rules into the brain and the duellist: the tick's footprint step, the removal forget, the sense's familiarity write, the combat entry and leave, the one-time attach snap and the set-target routine. The review checked each against the assembly and found the footprint gate keyed on the wrong flag (and stamping a second time at the body origin), the familiarity write creating relations the original never makes, the entry stance named after the wrong motion, the set-target bearing carrying the estimate tick's rules, and the leave using the leave look instead of the leave-combat routine.

Phase 6 (AI) in docs/PLAN.md.

## What Changes

- Set target: the mind is fresh, committed, its overshoot mean and stuck count cleared, the estimate on the foe; the bearing is the flat length and the whole offset over it, and the aim takes its yaw. Live at the duellist's entry.
- Familiarity: the sense's drift is added only to an existing relation. Live in the perception pass.
- Entering combat sets the in-combat word; the Defence stance (motion 3) is the duellist's entry. Live.
- Leaving through the target pass takes the body's look yaw and draws the next sway 500 ms out. Live.
- The tick-end footprint gate reads the route-following flag, the locomotion's activity and its speed; tested, not wired (the game's footprint system keeps NPC footprints).
- A removed character is forgotten by every brain and combatant; a brain snaps its look to the body once. Live.

## Evidence

The motileai pack's assembly (the AI tick's tail and out-of-fight branch, the enter and set-target routines, the leave-combat and leave-look routines), `exe.asm` (the route flag's writers, the perception routine's drift call) and the relation writer's and flat-length helper's listings read through REA; the combat-stances note's motion numbers.

## Out of scope

Wiring the footprint gate (NPC bodies carry no locomotion request), the estimate routine's lead on the set-target path, the look yaw the set-target routine shares with the brain, the leave-combat routine's 3 s look timer, the relation list for flagged ids.
