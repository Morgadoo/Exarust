# Proposal

## Why

Review of `add-roles-and-search` against the original found two places
where Exarust differs:

- Gathering weighs each character of a flooded cell as a dead character
  only. The original first asks whether the character's object is
  container-like and, if so, weighs it as a container or loose item; only
  otherwise does the dead-character test apply.
- The entry finder returns the corpse script when asked for it. The
  original's finder reads the section through the section table, which
  holds bits 1, 2, 4, 8, 0x10 and 0x40 only, so a request for the corpse
  section finds nothing; only the dedicated corpse accessor reads it.

## What Changes

- Gathering weighs a character whose object is container-like as an object.
- The entry finder gives no corpse section.

## Capabilities

### New Capabilities

### Modified Capabilities

- `npc-search`: gathering.
- `npc-roles`: the entry finder.

## Impact

- `exa_ai::search`, `exa_ai::roles`.
- Evidence: the original's gathering routine (its two cell lists and the
  kind test before the dead test) and its entry finder and section-table
  accessor (REA). No format field changes, so the corpus step is a skip.
- Out of scope: everything else in the search and the roles.

## Phase

Phase 6, the NPC brain.
