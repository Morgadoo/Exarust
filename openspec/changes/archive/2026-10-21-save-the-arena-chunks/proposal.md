# Proposal

## Why

An arena career's save keeps three chunks of its own beside the player
data: the company (name, emblem, the lifetime records), the day (today's
matches and money, the special matches, the company log) and the
tournament (the event rows, the rival field, the events, the season and
the prize history). The save container and chunk framing are read; these
payloads are not.

## What Changes

- `exa_formats::arena_save`: read and write the company, day and
  tournament chunk payloads, total on any input.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the arena save chunks.

## Impact

- New `exa_formats::arena_save` (Bevy-free).
- Docs: `docs/formats/OTHER.md` (Saves).
- Evidence: REA/Ghidra on `Exanima.exe` 0.9.5: the three writers and three
  loaders, the stream's word, byte-run and string primitives and the chunk
  begin and end. No shipped file holds these chunks; tests use synthetic
  bytes.
- Out of scope: the hire list chunk (it embeds character records), the
  employee and merchant chunks, wiring the chunks into a save the game
  writes, the loader's re-roll of a special match.

## Phase

Phase 7, M7 "arena season": keeping a career.
