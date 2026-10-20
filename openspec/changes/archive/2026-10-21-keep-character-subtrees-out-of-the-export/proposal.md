# Proposal

## Why

A review of `exa_formats::rfc_write` against the original's export walk
found two departures. The walk returns as soon as it files a character:
the original never descends into a character's subtree, so those nodes stay
unexported and unnumbered. Exarust's walk descended into them and exported
them as scene nodes. Also, a keyframe track without keys was an error,
where the original writes it with −1 as its last key's index and no keys.

## What Changes

- The walk does not visit a character's children. They keep their flags
  and numbers and are not written.
- A track without keys stores the last-key index 0xFFFFFFFF and no keys.
- The keyframe track rule moves into its own requirement (the light and
  body requirement grew past the strict length limit).
- Spec wording: the spot values of a light depend on `flags2 & 0x300`
  being `0x200`, as the code and the format page already say.

## Capabilities

### New Capabilities

### Modified Capabilities

- `chunk-tree-export`: the walk skips character subtrees, and tracks
  without keys are written.

## Impact

- `exa_formats::rfc_write` (`Walk::visit`, `write_track`), its tests,
  and `docs/formats/RFC.md` (Writing chunk trees).
- Evidence: REA (Ghidra 12.1.4) assembly of the original's collection
  routine. The character branch appends the node and jumps straight to the
  routine's exit, past the child loop that the prop and scene-node
  branches reach. The keyframe writer stores the track's last index and
  (last + 1) × 16 bytes of keys, which is no keys at −1.
- Out of scope: everything the archived change `write-a-chunk-tree`
  listed as out of scope.

## Phase

Phase 7, save games: the level export a save stores visited levels with.
