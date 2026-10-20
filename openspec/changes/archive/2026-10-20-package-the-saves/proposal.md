# Proposal

## Why

The retail save package is a documented container, but Exarust only has its own `EXRS` format. The package header, 24-byte directory, 0x2000-aligned sections, chunk framing and both player-data headers were missing.

## What Changes

- The story and arena magics.
- The 8-byte header, 24-byte directory entries and 0x2000 section alignment.
- Exact 16-byte section names and the sentinel entry.
- The `id, len, payload` chunk framing.
- The 48-byte story/arena player-data headers.
- Package build, parse and section lookup.

## Capabilities

### New Capabilities

### Modified Capabilities

- `save-games`: the retail package and chunk framing.

## Impact

- `exa_formats::save_package`, from `MAGIC_STORY`, `MAGIC_ARENA`, `Section`, `Package`, `PackageError`, `parse`, `write_package`, `Chunk`, `parse_chunks`, `write_chunk`, `Flavour`, `PlayerHeader`, `parse_player_header` and `name`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `XA_SaveGames` becomes **12%** in the source rating. The headline rises.
- Evidence: `SG_SaveWorld` (`0x10026f950`, `0x10026fb40`, `0x10026fde0`, `0x10026fe80`, `0x10026fd10`, `0x100270160`), the chunk framing (`0x1001cc1a0`/`0x1001cc1e0`) and the two player-data headers (`0x100182d10`/`0x100182850`, `0x100183100`/`0x1001838e0`). No new corpus files.
- Out of scope: the scene/world exporters, DB blocks, checkpoint lifecycle and file naming.

## Phase

Phase 7, save games. The package container.
