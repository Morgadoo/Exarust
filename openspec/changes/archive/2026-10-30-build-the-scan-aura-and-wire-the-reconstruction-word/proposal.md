# Proposal

## Why

docs/PLAN.md Phase 7 (game modes and content), the teleporter unit (`SG_Teleporters`, 2 513 bytes). Earlier passes left the reconstruction device's scan without its aura, the device's latch apart from the word the story session's death handler reads, the strip a logged step the app dropped, and the destination screen's "already open" answer unread.

## What Changes

- The scanner device builds the scan node's aura: the data library's `_ScanAura01` material with its 184-byte parameter block (starting with the grey, opaque colour 0.25, 0.25, 0.25, 1.0), state bits 0x9000, one sub-object of buffer usage 35044 whose parent is the node, the material bound to it and a shader object made once for the run, under the effect render's scene lock. The scan start adds material flag 0x20 and binds the shared `_ForceNoise` picture; the scan node stores the two unit draws its constructor rolls.
- A scan writes the story session's reconstruction word to rebuildable and the scanned record identity beside it; taking the crystal off clears the word.
- The play flow's strip step strips the live player's body (hands and worn data cleared, the underwear rule, yellow and red health at the 0x7FFF share).
- A teleporter use while the destination screen shows opens nothing.

## Capabilities

### Modified Capabilities

- `level-devices`: the scan's aura, the reconstruction word and the live strip.

## Impact

- `exa_game::effects::{SceneLock, scan}`, `exa_game::teleporters`, `exa_game::flow`; the `exa_game` and `exa_world` READMEs.
- Evidence: REA (Ghidra) pseudo-code and listing of the aura builder, the scan start, the reconstruction-device use and power-down, the scan effect's constructor (two `Random` draws narrowed to single precision), the story death handler (compares the reconstruction word with zero, the dying record's identity with the word beside it, strips only at 1, then writes 2); the aura block's 184 bytes read from `Exanima.exe`.

## Out of scope

- Drawing the aura, its shader signature and deferred release; the scan's per-frame mesh copy callback.
- The power-down trigger from the hand's release (nothing sends it yet).
- The corpse's record copy and the undress of the drawn figure; the device record's level-local copy.
