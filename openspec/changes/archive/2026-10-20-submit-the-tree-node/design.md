# Design

## Behaviour

A state other than 0x21 appends nothing. In state 0x21 a positive node count appends one entry holding the value over 5.0, the second value over 5.0 and the count itself. Every call clears the state and the index and runs the base once, returning its result.

## Open questions

The node count and the base bodies are not in.

## Docs

This method is 277 bytes. `TThaumTree` moves from 20% to about 21%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
