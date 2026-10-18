# Design

## Context

The career, the story and a cast all live in memory. Nothing writes them.

## Goals / Non-Goals

**Goals:**

- Round-trip the session fields the modes write, and refuse a malformed file.

**Non-Goals:**

- Reading or writing the original's save packages.

## Decisions

1. The magic is `EXRS`, version 1, little-endian fields. It is not the original package magic.
2. The story level is stored as its index in the eight-level campaign.
3. A file that does not parse completely, including trailing bytes, is an error.

## Risks / Trade-offs

- The file holds the roster and the journal, not a copy of a level.

## Migration Plan

No data migration. Version 1 is the first version.

## Open Questions

None.
