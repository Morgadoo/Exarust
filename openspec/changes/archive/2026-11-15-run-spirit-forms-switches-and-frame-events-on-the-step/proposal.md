## Why

Lead B0.6 of the phase B close, found by the mindend lane (round 206). With the simulation running free, Spirit Form's adoption (`adopt_spirit`, Update) and its way home (`spirit_home`, at the transition's black) still moved the player behaviour, the power caster and the player marker on the frame's copies. The next step packet in the same frame put the old values back, or the frame's copies went over the simulation's newer ones. A free-mode test of the old code loses the power list on the way home in 3 of 3 runs.

The same lane found a general hazard: an event the simulation sends to the frame (`FrameCommands::send`) that writes bridged data, run between two step packets applied in one frame, lost its writes. Each apply settles the frame's changes made before it as its own, so the simulation never saw the event's writes, spawns, despawns or messages, and a later step's value could put back a removed component or a despawned entity. A bridge test that spawns an entity and writes a message from each of 40 step events sees only 16 of them reach the simulation (3 of 3 runs).

## What the original does

REA on Exanima.exe 0.9.5, checked in the assembly:

- The Spirit Form cast is a cast table callback, run from the bar's key handler on the main thread. It calls the spirit body routine, which makes the record, builds the body and switches the player at once: the player pointer, the shared power list rebound to the spirit, the status bits and the held controls. The writes go straight to the bodies the simulation thread reads.
- The power's control (slot 14, in the power step) calls the end routine when the focus runs out or the form is ended. The end sets the spirit screen to state 2 and opens the transition with the way-home callback.
- The way-home callback runs on the main thread at the transition's black. It copies the focus, reserve and capacity change and the power list back to the body, rebinds the list, moves the player pointer back, clears the behaviour flag, points the camera at the body and removes the spirit from the scene. These writes also go straight to the shared bodies.

## What Changes

- `adopt_spirit` keeps the frame's part (the held body, the spirit screen, the rooms' anchor) and posts the switch (`enter_spirit`) to the next step. The switch holds the caster's behaviour, gives the spirit a player behaviour, the caster's power caster, the mind list and status bits, and moves the `Player` marker, on the world that walks the powers.
- `spirit_home` keeps the frame's part (forgets the held body and the screen, moves the anchor back) and posts the way home (`leave_spirit`) to the next step. The way home moves the caster, the list, the status word, the player behaviour and the `Player` marker back, and sends the spirit's removal to the frame.
- `HeldPlayer` and `SpiritOf` are bridged.
- In free mode the frame runs the events of all the step packets it applied in a frame after the last one, in send order.
- The contact mirror and the greeting's frame event write a script's concern and a line's hearing to the brain through `FixedCommands::write`. The brain is otherwise only read on the frame.

## Impact

- `exa_game::mind::runtime`, `exa_game::script_cast`, `exa_game::role_scripts`, `exa_physics::sim_app`.
- Lockstep and one world: the switch and the way home land at the start of the next fixed step instead of at once, one step later at most. In free mode a frame event runs after the frame's last step packet instead of right after its own step's packet, which can be a few steps later.
