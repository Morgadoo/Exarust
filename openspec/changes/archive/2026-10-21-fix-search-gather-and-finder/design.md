# Design

## Context

The flood visits a cell's character list, then its object list. For each
character the original tests the kind bits of the character's object
before the character-class and dead tests.

## Goals / Non-Goals

**Goals:** the branch order of the character list and the finder's table
walk.

**Non-Goals:** what makes a character's object container-like in the
shipped data (Exarust's scene says so through the object's kind).

## Decisions

1. The character test reads the kind of the object the scene gives the
   character ((`Corpse::object`)); a container-like one goes to the same
   object filter as the object list.
2. The finder returns nothing for the corpse bit rather than special-casing
   the corpse accessor, as the table walk has no corpse entry.

## Docs

`crates/exa_ai/README.md` needs no change (it does not describe the
branch).
