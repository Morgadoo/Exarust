## Why

The original's post-step hands a character's pending hit to its damage routine every 12 ms step, on the simulation thread (REA: the post-step tests the pending kind, calls the damage routine, clears it). Exarust ran the hit chain once a drawn frame, from blade and part frames the frame had propagated, with the frame's delta as the blade's time step.

## What Changes

- Strikers are tracked from the bodies each tick (`exa_combat::striker_frame`, `Blade::frame`): a held item with a body of its own from its first part (the blade's entity), a striker carried by a part of its wielder (fists, a held model with no body) from that part times its placement; the tip's velocity over one body tick.
- The hit chain runs in the fixed step's post-step (`BodyPostStep`), every tick: tracking after the solver and before the held bodies' classifier, then the bodyless strikers' sweep against the target's body parts where the tick left them, the damage, the bites; then the brains' reaction to blows and the debug pauses.
- Waiting wounds count down before the characters' ticks (`BodyPreStep`), reading the striker where the tick before left it.
- `body::deal_shoves`, first in every tick, deals the blows the last post-step and the frame sent (push, daze, sag, knockback step), so the next tick's character tick sees them, as the original's post-step writes them before the next body tick. `SimFrameStart` keeps only the per-frame tick count reset.
- The swing routine's edge alignment, overhead pull, homing and stab drive read the same tracked frames; "a tick ago" is the sweep's last tip.

Phase 3 (bodies: skeleton, physics, control) in docs/PLAN.md: the simulation thread.

Evidence: REA, the post-step and the damage routine (`context/re/dyn-damage.md`, `body-pairs.md` §11, `master-combat.md` §5); the knockback writes the step target only; the Force Strikes push moves the struck part's particles directly in the post-step. Rule change: hits land at step resolution.

## Out of scope

- The free-running loop; weapons as body–body contacts for the bodyless strikers; the brains' threat estimate still reads the attacker's root transform.
