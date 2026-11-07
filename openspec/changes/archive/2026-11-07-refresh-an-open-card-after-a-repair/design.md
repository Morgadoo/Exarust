## Context

The card refresh is the only caller of the sentence builders outside the
card builder. Exarust's card keeps the built rows (`BuiltCard`), so the
refresh writes into them by control id instead of into form controls.

## Decisions

- The refresh is a message (`CardRefresh`) from the kit tick to the card
  systems, so the card reads the record after the copy pass of the same
  frame.
- The weapon builder's crafted-name register is an address's low byte in
  the refresh; Exarust takes it as non-zero (the great table), which an
  address on a 256-byte boundary would not give. [guess]
- Exarust's card does not keep the weapon card's flip state, so the
  refresh reads striking side 0, which is what the built card shows.
- The original sets the refreshed text without the condition-block and
  pronoun expansion; Exarust does the same.
