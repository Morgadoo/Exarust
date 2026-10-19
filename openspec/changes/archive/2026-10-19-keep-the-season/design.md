# Design

## Context

The Exarust save restores the purse, the roster names, and the focus. On load it sets the day, the weeks, and the reward ids back to a new career. The night already fills those fields.

## Goals / Non-Goals

**Goals:**

- Write and read the day, the four week flags, the season and tournament counts, the reward count, and the reward ids.
- Refuse a week flag that is neither 0 nor 1.
- Let a career saved after two won weeks finish that season after it is loaded.

**Non-Goals:**

- The original's packages.
- Roster health, skill experience, and techniques.
- Emblems.

## Decisions

1. The new fields sit with the company, after the earned total and before the roster.
2. Each week is one byte, 0 or 1. Any other byte is a malformed file.
3. The reward ids are a counted list of 32-bit values, in the order the season granted them.
4. The version stays 1. A file that ends before the new fields is the same error as any short file.

## Risks / Trade-offs

- A file written before this change no longer reads. The tests write and read the current bytes.

## Migration Plan

No retail format page. The Exarust save gains fields; old Exarust bytes are short and refused.

## Open Questions

- None for this slice. The fields are the ones the night already stores.
