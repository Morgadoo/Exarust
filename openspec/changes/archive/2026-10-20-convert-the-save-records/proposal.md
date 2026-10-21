# Proposal

## Why

The save loader decides whether an old save needs conversion, then writes its record table as a count followed by each record's kind and two qwords. Exarust has the settings chunk but not the version gate or the record table.

## What Changes

- Conversion needs mode 4, no pending work, and either the legacy entry or a read record whose tag is not `0xDBAC0D06`.
- Writing the record table writes the count, then each record's kind and two qwords.

## Capabilities

### New Capabilities

### Modified Capabilities

- `save-games`: the version gate and record table.

## Impact

- `exa_formats::save_chunk`, from `save_needs_conversion` and `write_save_records`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `XA_SaveGames` moves from 5% to 6%.
- Evidence: the 180-byte conversion: the mode global, the pending count, the two entries, the record and the tag `0xdbac0d06`. The 146-byte record list: its count and the 24-byte records with their kind, qword and qword. No new corpus files.
- Out of scope: the archive and stream routines.

## Phase

Phase 6, game modes. The save version gate and record table.
