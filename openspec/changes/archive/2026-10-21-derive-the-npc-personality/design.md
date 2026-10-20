# Design

## Context

The AI constructor fills a handful of floats from the record before the
attach method picks the contact sense and applies the roles.

## Goals / Non-Goals

**Goals:** the constructor's arithmetic and the attach method's branches,
with the engine's draw (the generator over 2³²).

**Non-Goals:** morale, the combat state and the effect the attach sets.

## Decisions

1. Exarust names the shares by formula (confidence, restraint) and labels
   the names a guess.
2. The race word the attach reads is an input; where its AI and statue
   flags come from is not traced here.
3. Placed characters draw their jitter from a generator started at 0 per
   spawn; the engine's shared generator state is not kept.

## Open Questions

- The effect level the attach writes (0 or 1) and what reads it.

## Docs

`crates/exa_ai/README.md`.
