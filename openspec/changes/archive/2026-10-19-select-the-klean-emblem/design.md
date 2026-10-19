# Design

## Context

Company creation can store a name. Typing `klean` fills an emblem: four layer bytes, background 52, and design 81. Each picture name is a fixed stem, a hundreds digit that stays 0, then the index divided by 10 and the remainder.

## Goals / Non-Goals

**Goals:**

- Return that emblem for the exact name `klean`.
- Return nothing for any other name.
- Name background 52 `emblembkg052` and design 81 `emblemdes081`.

**Non-Goals:**

- Drawing the picture.
- The dword cleared before the pictures are chosen. Its role was not read.
- A candidate's listed price.

## Decisions

1. The match is the exact characters `klean`.
2. The four bytes stay in the order they are stored: 7, 0, 0, 6. What each byte selects was not read.
3. The picture name writes only the tens and ones. The hundreds character stays 0.

## Risks / Trade-offs

- An index of 100 or more would write a tens character past a digit. The preset uses 52 and 81.

## Migration Plan

No data migration. No format page: no field changes.

## Open Questions

- None for this slice. The name, the four bytes, and the two picture numbers were read before this change.
