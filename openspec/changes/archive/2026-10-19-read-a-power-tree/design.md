# Design

## Context

The power table already prices each power. The tree file is a separate graph: a header, a count at byte 52, then nodes. After the nodes are read, every node but the first subtracts the first node's x and y.

## Goals / Non-Goals

**Goals:**

- Refuse a bad magic and a file that ends before the count.
- Read each node's id, two words, place, and 16-byte links.
- Store no links when the count is below 1.
- Shift later nodes by the first node's place.

**Non-Goals:**

- The text library that may follow the nodes.
- Drawing, zooming, and learning a node.
- The meaning of the two words beside the power id.

## Decisions

1. The count is a signed word. A negative count is an error. A count whose nodes cannot fit is an error before any node is stored.
2. A link count below 1 stores an empty list and reads no further link bytes.
3. The shift is a float subtraction of the first node's x and y. The first node stays where it was read.

## Risks / Trade-offs

- The header bytes between the magic and the count are skipped, not interpreted.

## Migration Plan

No data migration. `docs/formats/OTHER.md` gains the node layout.

## Open Questions

- None for the node layout. The text library's records were not read.
