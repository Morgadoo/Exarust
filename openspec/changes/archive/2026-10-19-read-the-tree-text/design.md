# Design

## Context

The node reader stops at the end of the nodes. A text chunk may follow. Its first word selects it. The reader then skips five words, reads a 16-byte head, an offset table, and a byte buffer.

## Goals / Non-Goals

**Goals:**

- Read each string as a length word and the following bytes, at the offset in the table.
- Treat a length below 1 as empty.
- Leave the texts empty when there is no chunk.
- Refuse a chunk that ends early.

**Non-Goals:**

- Drawing the tree.
- Learning a node.
- The meaning of the five words before the 16-byte head.

## Decisions

1. The chunk is present only when the word after the nodes is the text magic. Any other word leaves the texts empty.
2. The string count is the last word of the 16-byte head. The offset table has one more entry than that count.
3. A negative count is an error. A length or an offset that runs past the buffer is an error.

## Risks / Trade-offs

- The five words before the head are consumed and not interpreted.

## Migration Plan

No data migration. `docs/formats/OTHER.md` gains the text chunk.

## Open Questions

- None for this slice. The length word and the offset table were read before this change.
