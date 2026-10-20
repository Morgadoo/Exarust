# Design

## Behaviour

Starting at the given node, the original reads the next node at `0x128`, then its record at `0x160`. A missing record stops the walk and the current node is returned. Otherwise the record's flags at `0xc` are masked with `0x12000000`; when both bits are set and the word at `0x468` is negative, the walk stops. Any other node becomes the next one.

## Model

`first_settled_view_node(nodes, start)` walks `ViewNode { next, record }` and returns the stopping node, or `None` when the chain ends without one.

## Open questions

The node and record are the character view's own; no other user is known.

## Docs

No published percentage moves. The headline stays 14.4% and 5.7%.
