# Design

## Context

Kinds 0, 2, 3 and 5 already run. Kind 1 walks every listener and forwards the same value.

## Goals / Non-Goals

**Goals:**

- Select the listeners in list order and forward the relay id with the original value.

**Non-Goals:**

- Carrying out the close.

## Decisions

1. Only an id whose high bits are the object tag is considered.
2. A door link wins. A match includes the source. A miss does not fall through to the operative target.
3. Without a door link, the operative target must equal the relay, and the source's own id is skipped.

## Risks / Trade-offs

- The caller supplies the listener list. The function is the selection rule.

## Migration Plan

No data migration.

## Open Questions

- None for this slice. The broadcast was read before this change.
