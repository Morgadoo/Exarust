## Why

Lead B0.3 of the phase B close. With the simulation running free, the frame still wrote the duellist and the player behaviour over newer simulation steps when Control Mind ended. The bridge3 sweep (round 201) logged "exa_ai::Duellist 1; exa_game::player::PlayerBody 1" in the mind2 control-mind scene: the frame's copies, based on step 1491, undid step 1494's duellist roll and player aim.

The cause was in both halves of the control. The handover (`possess`) ran on the frame, sent there from the step. When several step packets arrived in one frame, the next packet's values put back what the handover had removed on the frame: the caster's player behaviour and the target's duellist. So in free mode both bodies kept a player behaviour, and the possessed body's AI kept running in the simulation. The hand-back (`release`) also ran on the frame, at the transition's black. It wrote the frame's held copies (a held duellist, a held player behaviour) over the simulation's values.

## What the original does

REA on Exanima.exe 0.9.5, checked in the assembly:

- The swap runs inside the Control Mind power's step on the simulation thread. It sets the status bits and the possession marker, holds the behaviours and installs the player behaviour there. It then posts its frame callback (GUI sweep, controls, camera target) to the main thread's queue.
- The release transition's callback runs on the main thread at the black. It closes the GUI's forms, resets the controls, sets the GUI mode and the camera target, then posts the hand-back to the simulation thread's posted-call ring. The hand-back writes the status words, moves the player pointer back, restores the behaviour, tells the scene's behaviours of the swap and frees the player behaviour. It runs at the start of the next step.

## What Changes

- `possess` runs on the world that walks the powers. `tick_minds` queues it there instead of sending it to the frame. It writes the minds at once and holds both behaviours. It installs the player behaviour, moves the `Player` marker, and sends its frame part (`possess_view`: the GUI sweep, the rooms' anchor, `MindControl::held`) to the frame.
- `release`, at the transition's black on the frame, keeps only the frame's part (the GUI sweep, the rooms' anchor, `MindControl::held`). It posts `hand_back` (status words, behaviours, `Player` marker) to the next step through `FixedCommands`.
- A free-mode test runs the bridge with a step that changes the duellist and the player behaviour every step. The test fails on the old code: the frame keeps the caster's player behaviour after the handover.

## Impact

- `exa_game::mind::runtime`.
- Lockstep and one world: the hand-back lands at the start of the next fixed step instead of at the frame's black, one step later at most, which matches the original's posted call. The handover's minds land in the step that decides the swap instead of the next one.
