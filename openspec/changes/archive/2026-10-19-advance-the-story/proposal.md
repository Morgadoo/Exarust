# Proposal

## Why

Phase 7 (M8 "story act 1") does not advance `exanima01` through `exanima08`, and it has no dialogue choice or journal. `docs/PLAN.md` lists those as still open.

## What Changes

- A new story loads `exanima01` and each advance loads the next level through `exanima08`.
- A topic offers its choices once. The pick is that line.
- The journal opens on a blank page titled "New Entry" and stores what is written there.

## Capabilities

### New Capabilities

- `story-progress`: the campaign order, one dialogue choice, and the journal page.

### Modified Capabilities

## Impact

- `exa_script::story`.
- `docs/PLAN.md`.
- Evidence: the story level ids, the dialogue choice list and topic memory, and the journal window's first entry. No format field changes, so the corpus step is a skip.
- Out of scope: side levels, the script interpreter, role graphs, and the journal window itself.
