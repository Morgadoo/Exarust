# Proposal

## Why

Two lanes read `charroles.rdb`: the script lane (`RoleDb`, with the
dialogue graphs) and the roles lane (`RoleBook`, with the corpse script,
the share, the locale and the entry accessors). Both parsers read the same
records; keeping two readers of one file would let them drift. The
integration keeps one, the script lane's, and folds the roles lane's
rules into it.

The two disagreed on kind bit `0x20`. The script lane placed it after the
other sections as a guess ("game-event line"). The roles lane found the
original's corpse accessor, which reads it after the sections of bits
`0x1` to `0x8` only. The evidence wins.

## What Changes

- `exa_formats::roles` is the only reader. `RoleEntry::corpse` (was
  `event`) is read after the sections of bits `0x1` to `0x8`.
- `Role::share`, `Role::entry_for`, `RoleEntry::locale`,
  `RoleEntry::for_anyone`, `RoleEntry::script` and `locale_id` move over
  from the roles lane's parser.
- `exa_ai` reads `RoleDb`; its entry finder returns the entry rather than
  raw section bytes.

## Capabilities

### New Capabilities

### Modified Capabilities

- `role-database`: entry sections (the corpse script's place) and the
  level's locale.

## Impact

- `exa_formats::roles`, `exa_ai::{roles, activity, brain}`,
  `exarust::viewer`, `exa_script` corpus test.
- Evidence: the original's corpse accessor and section-table finder (REA),
  as recorded by the roles lane; the shipped database (one corpse entry,
  alone) parses the same either way.
- Out of scope: the duplicated role-list rules in `exa_script::cast`
  (a stand-in) and `exa_ai::roles`, and the init-script runner in
  `exa_ai::activity` next to `exa_script`'s interpreter.

## Phase

Phase 6, the NPC brain, and the script engine.
