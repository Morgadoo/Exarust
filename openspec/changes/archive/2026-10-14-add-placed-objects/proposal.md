# Proposal

## Why

Story levels place hundreds of object-database objects (tools, cups,
tubs, chests, doors) through their local database. None appeared.

## What Changes

- `exa_formats::rfc`: placed objects (`DBC0` → `DBCE` → `DB01`) on `Scene`.
- `exa_world`: each placed object spawns as its item's model (`Objects.rpk`,
  else `Objlib.rpk`) at its transform. `LevelInfo` counts them.

Out of scope: physics for loose objects, containers' contents, articulated
parts, scripted objects.

## Capabilities

### New Capabilities

### Modified Capabilities
- `level-decoding`: placed objects.

## Impact

`crates/exa_formats`, `crates/exa_world`, `crates/exarust` (log), docs.
