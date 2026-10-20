# Design

## Behaviour

The attachment search runs only when `kind & 0x89 == 0x89` and the wanted count at `0x1c8` is not negative. It looks at up to that count entries of the list at `0x1d0` and matches the first whose `0xd0` word shifted right 16 equals `0xfca0`.

Whatever the attachment result, the search then walks the children at `0x120`, up to the count at `0x118`, and returns the first recursive match. A node with neither an attachment match nor a matching descendant is false.

## Model

`PartNode` carries the kind, the wanted count, the attachment tags and the children. `has_creation_part(&node)` returns the boolean. The part objects are reduced to their tags.

## Open questions

The part objects themselves are not in.

## Docs

This method is 173 bytes. `SG_Character_Create` stays about 12% (about 12.5% by bytes). The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
