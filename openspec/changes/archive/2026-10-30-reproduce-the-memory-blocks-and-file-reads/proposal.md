# Proposal

## Why

The settings and data I/O units still lacked the engine's memory blocks (the
kinds a whole-file read fills), the whole-file reads by reference and over
the search path with their failure messages, the executable's resource read,
the settings store's exit save and the `KeyDef` re-read rule. Earlier passes
named these out of scope.

## What Changes

- `exa_formats::mem_block`: `MemBlock` with its logical size, allocation
  size and kind; general memory resized exactly, global memory in `0x2000`
  steps, resource memory released only at zero, a borrowed view copied on its
  first resize; `copy_into` (resizing the destination by its own kind),
  `duplicate`, `make_owned`.
- `exa_formats::file_ref`: `read_file`, `read_referenced_file`,
  `read_from_search_path`, `write_file`, `write_referenced_file`,
  `file_exists`, `read_resource` (the `RT_RCDATA` payload, the platform's
  directory walk injected) and `FileIssue` with the original's log lines; a
  search-path miss logs nothing.
- `exa_formats::settings_store`: `SettingsStore::save_to` (the exit save,
  skipping an empty store) and `ExitHooks` (the chain the original registers
  it in, latest first).
- `exa_formats::settings_ini`: `Settings::reread` applies a file onto live
  settings; the macro list is kept, so a re-read's macros past the sixth stay
  unbound. Its number conversions, formats, substring search and splice now
  delegate to `exa_formats::text`.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-settings`: memory blocks, whole-file reads, the exit save and the
  `KeyDef` re-read.

## Impact

- `exa_formats` (new `mem_block`), `exarust::user_settings` (calls
  `save_to`). Docs: `docs/formats/SETTINGS.md`, the `exa_formats` and
  `exarust` READMEs.
- Evidence: REA/objdump on `Exanima.exe` 0.9.5: both block resize routines
  (kind switch on the low six bits: 0 general, 1 global, `0x32` resource,
  `0x38` borrowed; global size `(n + 0x1fff) & 0xffffe000` in 32 bits; the
  borrowed copy of `min(old, new)` bytes), the block copy (destination resized
  then its new size copied), make-owned, the whole-file read (open failure
  logged only while the error log is on, read failure always logged and the
  block cleared), the search-path read (no clear, no log on a miss), the
  write, the exists probe, the resource read (type 10, kind `0x32`), the
  store's save and exit hook, and the `KeyDef` parser (its count is the macro
  list's length, never reset).
- Out of scope: the Win32 resource-directory walk, the error log's switches,
  the GUI task's hotkey callback array; nothing in the game calls the
  resource read or `ExitHooks` yet.

## Phase

Phase 8, settings.
