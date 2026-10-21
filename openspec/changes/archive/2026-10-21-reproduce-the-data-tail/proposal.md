# Proposal

## Why

The original's data layer has a tail of small helpers Exarust lacked: the
sorted id database behind the locale table, the in-memory named-block
archive, the project file's path rules, the debug command registry, the
object-library search order, the string library's clear and the modifiable
class pack open. A DeepSeek lane ported them; the review against the
original's assembly corrected several rules and removed invented code.

## What Changes

- `exa_formats::uid`: a sorted id index (`UidIndex`) with the original's
  whole-block length, fix-up on removal and delete as an empty block.
- `exa_formats::locales`: the locale table, looked up by a 16-byte name
  compared raw against the lower-cased stored name.
- `exa_formats::colour`: luma and desaturate helpers, reused by hair.
- `exa_formats::objstrings`: the table's clear.
- `exa_rpk::mem_archive`: the named-block archive (add, replace, delete,
  read an entry); the byte writer is Exarust's addition, labelled as such.
- `exa_rpk::project`: resolving a path against the project folder and the
  user folder choice.
- `exa_rpk::vfs`: `GameData::open` takes the Resource pack by its literal
  stem, skips unknown pack names and errors on a known pack that is
  missing.
- `exa_game::commands`: the debug command registry (first match by name,
  trailing arguments), the queue and the hotkey commands; `exarust`
  registers `cshd` (reload shaders) and `msm` (logs only).
- `exa_render::shader_reload`: the twelve embedded shader assets that
  reload.
- `exa_assets::noise` (total weighted plane sum), `exa_assets::model`
  (the placeholder node flag), `exa_world` props taking the pack from the
  object name.

## Capabilities

### New Capabilities

- `data-tail`: the id database, locale table, memory archive, project path
  rules, command registry and pack open rules above.

### Modified Capabilities

## Impact

- Crates: `exa_formats`, `exa_rpk`, `exa_game`, `exa_render`, `exa_assets`,
  `exa_world`, `exarust`; READMEs of the first five;
  `docs/formats/OTHER.md`, `docs/formats/RPK.md`, `docs/DEBUGGING.md`.
- Evidence: REA (Ghidra 12.1.4) on the data library, the memory block
  library, the project manager, the debug command registry, the locale
  lookup, the object library manager and the string library. Tests use
  synthetic bytes; no corpus file is needed.
- Review corrections: the sorted index stores the whole-block length
  (first word includes itself); removal fixes later offsets; the Resource
  pack opens by its literal stem; the title stays "Exarust"; the
  window-title, `set_name`, `first_name`, exporter clear and "seen in
  shipped models" claims were invented and removed.
- Not wired: the id index, the memory archive and its writer, the stream
  thread, the weighted plane sum, model release and the placeholder swap
  are library code with tests only; `msm` logs and does not subdivide.
- Out of scope: the 23 procedural material presets (the existing
  `exa_formats::procedural` data and `exa_assets::paint` stay as they are),
  a project file writer, a console UI, and pack writing from the archive.

## Phase

Phase 1, complete asset decoding (data-library helpers); the command
registry belongs to Phase 8, polish and debug tools.
