# Proposal

## Why

The original's teleporter unit (`SG_Teleporters`, ten routines, 2 513 bytes) runs the two gameplay devices of the later levels: the teleporter pedestal, whose crystal slot opens the destination screen, and the reconstruction device, which scans the player so that a death in the story brings them back stripped while their corpse keeps their gear. A first pass reproduced the gates and data; a review against the assembly and the routines' call sites found the crystal id masked, the screen's row walk stopping early, the glow's hold read as its fall, the latch consumed by a level load instead of the session end, the strip applied to the wrong record, the power-down tied to the device leaving the world, and the scan replaced by a timed marker on a 12 ms clock.

## What Changes

- A device's crystal is compared by its whole record id; the teleporter still refuses a crystal spent 0x55 or more, the reconstruction device reads no charge.
- The destination screen's rows follow the original's walk (an empty entry, or one past the free rows, goes to row 0, without stopping), with the atlas the use loads by name.
- A reconstruction-device use puts the effects' scan on the player (waiting 1600 ms of game time, closing with the 3.0 flash), refuses to start a second while one is on the player, plays its sound at the player and glows the device's voxel lights over a 1500 ms rise, 3800 ms hold and 1500 ms fall.
- The session end turns a story death of the scanned identity into a reconstruction; the first strips the player and leaves a dressed copy of the record as the corpse.
- Letting an object go within 5 cm² of a reconstruction device's slot point plays the power-down sound and clears the latch.
- The use gate is the player's possession of another body.

## Capabilities

### New Capabilities

- `level-devices`: the teleporter and reconstruction devices and the reconstruction at the session end.

### Modified Capabilities

## Impact

- `exa_world::teleporters` (rules), `exa_world::PlacedItem` / `TeleportDevice`, `exa_formats::objdb::Operative` (atlas name and destination ids), `exa_game::teleporters` (the Bevy bridge), `exa_game::effects` (the scan it starts).
- `docs/formats/OTHER.md` (operative section: the device class and the teleporter's fields); the crate READMEs.
- Evidence: REA (Ghidra) pseudo-code and assembly of the ten routines; at their call sites the session end, the glow pulse task, the screen's row fill and journey start, and the grab release that powers a device down; `objdb.rdb` from the installed build for the shipped teleporter and reconstruction-device records.

## Out of scope

- The destination screen (drawing, hover, choice), its journey (the bubble, `teleportf01`, the crystal spent, the 1000/1400 and 1500/7000 ms glows) and the level change.
- The object-use and grab input that would send the requests; the possession that shuts the gate.
- The corpse record's registration in the character database, the role database's parent roles, the record copy's serialisation, the visual undress and the emissive half of the glow.
- The scan's mesh copy and draw, and audio playback.

## Phase

Phase 2, static world: the level's devices, next to its mechanisms (M35).
