# Design

## Behaviour

**Use gates.** Both uses refuse while the player possesses another body. With a player, the squared distance from the device's origin plus 16 cm on y to the player's point is compared with 10 000 by an unordered compare (only an ordered "greater" refuses, so NaN passes). The slot is the first of the device's children whose name starts with `[`; the object on it is the first scene object whose squared distance to the slot is under 1 (NaN is not near). The squared distance sums (x² + y²) + z².

**Teleporter.** The object's record id, whole, must be non-zero and 0x1AF or 0x58F, and its condition byte (bits 16–23 of the placement word) at most 0x54. The device's record is resolved before the open-screen check, so a missing record refuses even with a screen open; an open screen makes the use succeed without opening another. The screen walks the first four destination words (one with flag bit 2): a zero word, or one met when rows 1–3 are taken, sets row 0 to its index; any other takes the next row. Rows nothing reaches keep index 0. The atlas is `prtlsmbla.rfi`, loaded once by name.

**Reconstruction device.** The object must be 0x1F4 (no charge test). A scan effect already a child of the player makes the use succeed with nothing started. Otherwise the scan effect waits for the game clock plus 1600 ms (state 8), loops its close with the flash scale 3.0, and `devicescna01` plays at the player at 0.3, pitch 1. Then the latch is set to charging with the player record's identity word and a stored word cleared, and the glow pulse starts on the device: e = clock − start (32 bits); over when e > 2·ramp (doubled in 32 bits) + hold, writing 0; e < ramp gives e/ramp; e > ramp + hold gives 1 − (e − hold − ramp)/ramp; else 1; fractions in double. The pulse scales the device's light nodes' intensity (Exarust: its voxel lights' colour by the square root) and its meshes' emissive scale.

**Session end.** The player's red health widened to double: at least 0.01 (or NaN) is a normal end. Below it, in the story only, a latch that is not idle with a matching identity reconstructs: the first time (charging) the player is stripped; the latch settles on reconstructed, and later deaths reconstruct without a strip. Otherwise it is an ordinary death.

**Strip.** For an unshared record, a copy is made first: one use, status bit 0x40, identity 0, roles cleared then 1 and 0xC2 added, health and stamina words 0, registered in the character database under the first free id. The original record is then stripped: four hand words and the worn count 0, the underwear rule, one zeroed action, health and stamina words 0x7FFF. The character then points at the copy, which is the body left behind; the stripped record is the player's. A shared record is stripped in place and counts one more use.

**Power-down.** When the grab code lets an object go, every slot point within a squared 5 cm of where it sat names its owner; an owner whose operative kind is 18 plays `devicepwra01` at 0.1, pitch 1 at its position and clears the latch.

## Model

Pure rules in `exa_world::teleporters` (`use_portal`, `use_res_device`, `ScreenData::row_entries`, `scan_timing`, `pulse_step`, `Latch`, `session_end`, `Record::strip`, `released_from_slot`). `exa_game::teleporters` answers `UseDevice`, `SlotReleased` and `EndSession` messages on the effects' game clock, spawns `ScanEffect` children, steps `DeviceGlow` and writes `OpenPortalScreen`, `DeviceSound` and `SessionEnded`.

## Open questions

- The player's point the reach is measured from: answered by REA on the character's field writers (the character tick), outside this unit; Exarust uses the root.
- The latch's cleared word and its reader: answered by REA xrefs on it.
- Whether the pulse's light class is the voxel light node: answered by REA on the node class table (the 0x41 class's constructor).
- The role database's parent chains for roles 1 and 0xC2: answered by `exa-tool characters` on the role database.

## Docs

- [`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md), Door and operative sections.
