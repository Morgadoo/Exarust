# Design

## Context

`exa_script` is an empty plugin. The story levels exist as files the world can load, with no campaign order.

## Goals / Non-Goals

**Goals:**

- Advance the eight main levels, keep one chosen reply, and store a journal page.

**Non-Goals:**

- Catacombs, sewers and gardens, and the role-script interpreter.

## Decisions

1. The campaign is the eight `exanima0N` names, in order. The last one does not advance.
2. Choosing a topic records its title. A later pick of that title is refused.
3. The journal is a list of titled pages. Writing replaces the open page's text.

## Risks / Trade-offs

- The reply is not yet shown in a window. The function returns the line the window would show.

## Migration Plan

No data migration.

## Open Questions

None.
