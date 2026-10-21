# Design

## Behaviour

The record stores four {id, flags} pairs after the profession word and the
last locale; the reader now keeps both words of each pair, and the whole
profession word.

## Model

`CharacterRecord::hand_flags` and `CharacterRecord::profession`;
`Possessions::of_record` takes them as the out and stowed hands' flags and
the possessions' record flags.

## Open questions

None left. Across the 477 level records no held item carries a flags word
and none has the second-set bit; saves are where they appear.

## Docs

`docs/formats/RFC.md` (character records).
