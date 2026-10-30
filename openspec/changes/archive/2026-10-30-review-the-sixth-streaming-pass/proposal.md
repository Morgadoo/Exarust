# Proposal

## Why

The sixth DeepSeek pass on the content stream and texture units (lane
`streaming6`, docs/PLAN.md Phase 2, Static world: texture loading and the
content stream) added the reader's chunk-end stack, a reader for a level's
character section on the stream, and a class gate on the player test. Its
review checked each against the original's instructions:

- The chunk-end stack (open: remember the enclosing end, read tag and size,
  set the new end; close: jump to the end, restore the enclosing one) is
  right and kept as `exa_formats::chunk::ChunkCursor`.
- The character-section reader duplicated
  `exa_formats::characters::LevelCharacters::parse` and
  `exa_world::placed_characters`, and misread the `0xCECDDB00` chunk as the
  list's end: the original closes (skips) that chunk, the level's local
  character database, and then reads all n placements. It was removed.
- The class gate is right in shape (the level builder dispatches on the
  node's kind word: door `0x40000`, then operative `0x10000`, then volume
  `0x8000AC07`), but the lane derived the word with operative before door
  and gave the volume word to any record with a volume section. The
  database builder sets it from the tree: on a model tree a door section
  wins over an operative one (weapons and item bodies get none), the
  model-less volume record's container volume carries the volume word, and
  the placeholder the operative word.

## What Changes

- content-streaming: Player focus gates on the node kind; new Placed object
  node kind.
- Code: `exa_world::db_objects::node_kind`, used by the level pass;
  `character_entries` and its constants removed.

## Evidence

The instructions of the stream's level-object builder (three compares of
the node's kind word in the order door, operative, volume), the database
object builder (model section first, then the volume section's container
volume writing the volume word, then the operative placeholder writing the
operative word; on a model tree the record's section flags tested door then
operative) and the stream's character handoff (count; peek; on the database
chunk's tag a close, then the placement loop).

## Out of scope

Driving the requested-object, requested-character and streamed-character
builders from the stream; the texture unit is unchanged by this pass.
