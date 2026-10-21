# Design

## Context

The keep-off clamp builds its last test vector with its first component
taken from the step's z and leaves the second unwritten; the compares
that follow are unordered (a cosine that is not a number cuts the step).
The AI manager had its own copy that tested the whole vector.

## Goals / Non-Goals

**Goals:** one keep-off implementation with the original's compares; the
estimate's unwritten direction; leaving on a dead foe.

**Non-Goals:** the can-act test's grip and awareness.

## Decisions

1. `manager::clamp_approach` calls `combat::keep_off` on the flat points;
   the unwritten second component stays 0 [?], as in the combat state.

## Open Questions

- What the original's stack holds in the unwritten component: unknown;
  0 is the stand-in, labelled in the code.

## Docs

`crates/exa_ai/README.md` (unchanged rows).
