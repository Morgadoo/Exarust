## Why

Task B1 of the phase B close, lane `wiredyn`: the open rows of the dynamics and intersection units. The physics-06 audit found four defects there (F328 to F331): no placed character got the settling state, the freeze set one bit, a held item was not parked or unparked with its holder, and the barricade's dismissal pick used the slab test where the original uses the separating-axis test.

## What the original does

REA on Exanima.exe 0.9.5, checked in the assembly:

- The settling routine gives every living body down a node tree the settling bit and a tick count. Four callers: the AI tick when the scene takes a character back (83 ticks, skipped for a dormant statue or golem), the story respawn after it places the player at the portal 15 cm up (16), the save load of a save written while a respawn was pending (transition word 2: the word becomes 1, the player goes to the level's portal 15 cm up, 16), and Spirit Form on its new body once it is in the scene (16).
- The freeze clears the asleep and deep bits, sets the frozen and kinematic bits, places the parts at a matrix when given one (both callers, the item cursor's lock, pass none), copies the particle sets into each other, empties the two contact offset buffers, rebuilds the matrices and sets the countdown to 2. The thaw clears the frozen and kinematic bits. The level broad phase leaves out a body with the asleep, parked or kinematic bit.
- The wake and the park marks walk the character's node tree, where its held items hang.
- The barricade's cast tests the nearest pick's segment against a running barricade's node sphere, then its box by the separating-axis routine.

## What Changes

- `Ragdoll::settle_for` is called by the scene's take-back (`exa_ai::presence::check_presence`, 83 unless dormant), the story respawn (`Step::SettlePlayer(16)` after `Step::PlacePlayer`, carried across the load by `PlacedSettle` and applied in the step's placement), the story load of a pending respawn (`save_live::take_pending_respawn`, `settle_after_placement`, the placement at `Flow::portal_placement` when a portal is known) and Spirit Form's body (`limp::SettleWhenBuilt`, applied when the body is built).
- `Ragdoll::freeze_at` is the original's freeze; the item cursor's lock release calls it. `Ragdoll` keeps a kinematic flag: set by the freeze, cleared by the thaw (`freeze(false)`), and a kinematic body skips the level detection.
- `grip_solve::share_holder_park` gives each linked held item its holder's park mark before the solver. The grip's mark already wakes a held item to the full count every tick, which is what the tree wake leaves on it.
- `cast_pick::segment_meets_wall` takes `segment_obb_overlap` (the separating-axis test).

## Impact

- `exa_physics` (body, grip_solve), `exa_ai` (presence), `exa_game` (flow, level_change, save_live, limp, hand, park, cast_pick), `exarust` (viewer placement, Spirit Form spawn).
- Lockstep: a character the scene takes back now settles for 83 ticks (gravity without inertia), so scenes where a dormant-marked character re-enters the scene change from that tick; the story respawn, a pending-respawn load and Spirit Form settle 16 ticks after their placement. The order is the original's. A frozen item no longer meets the level.
