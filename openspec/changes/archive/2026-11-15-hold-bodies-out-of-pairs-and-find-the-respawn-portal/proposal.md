## Why

Task B4 of the phase B close, lane `holdtree`: the dynamics unit's held-tree trio and its tree disable had no live caller (round 210, lane `wiredyn`), and finding F542 showed the story respawn's portal was only ever set by a test.

## What the original does

REA on Exanima.exe 0.9.5, checked in the assembly:

- The hold sets the held state bit (0x400) and empties the world contact count down a body's node tree; the pair drop walks the solver's kept pair list (the one a tick's detection leaves for the next tick's iterations) and drops each pair with a held body, copying the last pair into its place and testing that place again; the release clears the bit down the tree. Only the pair drop reads the bit.
- Callers: the story revive pass (each character not dead, after its brain reset; then the drop over every sector and the release over every character), the alarm return (the guardians it sends home, the player it puts outside; released by the revive pass), the player behaviour's third-tick spawn snap (hold, ground placement, drop, held items put back, release), the teleport (the same order; Vanish's come-back to the body's own standing point and the Long Door's move), and the item cursor's deferred hold-out (posted to the simulation step: bits 0x805 and 0x400, drop, 0x400 cleared).
- The tree disable sets the disabled bit (0x800) down the tree. Spirit Form disables its new body right after the handover. The body-to-body broad phase skips a disabled body; the level broad phase tests only the asleep, parked and kinematic bits (0x105).
- The story respawn sets the session's arrival word to the session-start word before it loads `exanima05`, so the level's reconstruction device (operative kind 18) becomes the player's object; after the load it stands the player at that object's frame 15 cm up, or at the identity frame 15 cm up without one. The story save reader sets the same word, and a pending respawn's load places the player at that object the same way and settles it 16 ticks.

## What Changes

- `exa_physics::body::HeldBodies`, `Pairs::remove_held`, `hold_out_of_pairs` and `hold_entities_out_of_pairs`: the hold, the drop and the release.
- Callers: `revive_pass::hold_revived` (from `reset_brains`, with the alarm return's `RevivedHolds`), `golem_alarm::return_from_alarm`, `limp::place_on_ground` for the spawn snap (`GroundPlacement::held`), `thaum::cast::apply_otherwhere` (the teleport) and `hand::press_hand` (posted to the step).
- Vanish's come-back writes the teleport to the body's standing point.
- `Ragdoll::detect` gates on the kinematic flag only; Spirit Form's body is built disabled (`limp::DisabledWhenBuilt`).
- `Step::PlaceAtPortal` and `level_change::PortalArrival`: the respawn loads take the session-start arrival word; the content stream keeps the player object's frame (`LevelInfo::player_frame`); the viewer places the player there and records `Flow::respawn_portal`.
- `Step::CloseSave` is applied: no save file stays open between operations.

## Impact

- `exa_physics` (body, body_pairs), `exa_assets` (content stream), `exa_world` (level info), `exa_game` (flow, level_change, save_live, revive_pass, golem_alarm, limp, player, hand, thaum cast), `exarust` (viewer).
- Lockstep: the player's spawn snap now drops the kept pairs with the player's body and empties its world contacts for the next tick's iterations, so every story and arena scene changes from the snap tick on; Spirit Form's body no longer meets other bodies; the revive pass, the alarm return, a teleport and an item lifted by the cursor drop their pairs; the story respawn stands the player at `exanima05`'s reconstruction device.
