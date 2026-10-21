# Proposal

## Why

The arena keeps a global list of twenty-byte records. One appender stores an id and one value, another an id and four words, both raising the count and growing the list. Exarust has the arena career maths but not the record log.

## What Changes

- Appending a value stores the id and the value in the first word.
- Appending words stores the id and all four words.
- Both raise the count by one and return the new slot.

## Capabilities

### New Capabilities

- `arena-records`: the arena's record log.

### Modified Capabilities

## Impact

- `exa_game::arena_log`, from `ArenaLog::append_value` and `ArenaLog::append_words`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `XA_Arena` stays about 34%.
- Evidence: the 113-byte appender: the record count, the twenty-byte list, the id and value stores. The 127-byte appender: the same count and list with sixteen bytes copied from the argument. Eleven live call sites. No new corpus files.
- Out of scope: the readers.

## Phase

Phase 6, game modes. The arena record log.
