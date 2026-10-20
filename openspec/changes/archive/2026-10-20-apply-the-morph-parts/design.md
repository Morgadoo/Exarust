# Design

## Behaviour

Each node is tested: the low twelve bits of the kind word must be `0x89`, and the `0xC00000` bits of the `0x204` word must be `0x400000`. An eligible node runs the apply call on its `0x20` child. The walk then recurses into every child in the list at `0x120`, up to the count at `0x118`.

## Model

`MorphPart { kind, flags, children }` and `apply_morph_parts(&node) -> usize`, returning the eligible count. The apply call is out of scope.

## Open questions

The apply call is not in.

## Docs

This method is 154 bytes. `RF_Morphing` moves from 5% to about 6%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
