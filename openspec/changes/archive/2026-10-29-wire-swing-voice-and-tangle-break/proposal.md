## Why

Round 28's TMotileDynamics audit (`docs/re/motiledyn-map.md`) found routines ported and tested but never called. Two of them can be wired without new physics: the swing tick's voice (Exarust played one whoosh when the strike window opened instead of the original's voice that follows the swing's speed) and the body tick's tangle break.

Phase 4 (combat and items) and Phase 5 (visual and audio fidelity) in docs/PLAN.md.

## What Changes

- The swing tick keeps a swing voice per fighter and cues its start (silent, 0.4 × W26) on the first tick in the swing's region, its volume and pitch every attack tick from the tracked speed, and the body tick's fade on every tick that is not an attack; `exa_audio` plays them on the fighter's first held item's voice with the clothing rustle, replacing the window-open whoosh.
- The body tick's readiness stage runs the tangle break on living bodies that are neither down nor dying: hands behind the back build the counter and start the Break Tangle clip, ending the attack.
- The routine map's live column and totals are updated; the pieces still tested only (swing pushes, bite link, clinch clear, armed parry, struck-item sound) say why.

## Evidence

The swing routine's voice start, volume write and rustle, the body tick's held-voice fade and Break Tangle call and the tangle routine itself, read in the motiledyn pack's assembly; the constructor's 1.0 for the swing loudness and sound gain.

## Out of scope

The swing's weapon and hand pushes, the bite link, the clinch clear's grip links, the armed parry and the struck item's blow sound (see the map for why each stays unwired).
