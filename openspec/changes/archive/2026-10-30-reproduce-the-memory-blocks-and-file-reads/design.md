# Design

## Behaviour

The rules are in [docs/formats/SETTINGS.md](../../../docs/formats/SETTINGS.md)
(File plumbing, Memory blocks, the `KeyDef` paragraph).

- One `MemBlock::resize` stands for both original resize routines: they
  differ only in calling the allocator's fetch or reallocate for a general
  block, with the same bytes, sizes and result.
- Reads return `Result<MemBlock, FileIssue>`; the caller logs
  `FileIssue::message` (none for a search-path miss) instead of the parser
  logging.
- The original's `KeyDef` count is the length of its macro list, which only
  the unit's finalizer releases, so `Settings::hotkeys.len()` is the count.

## Choices

- The block copy into a resource destination that refuses the resize copies
  only the bytes both blocks hold; the original would copy the destination's
  size and can read past the source.
- `ExitHooks` is a tested model; the launcher calls `save_to` directly at
  exit, which gives the same file.
