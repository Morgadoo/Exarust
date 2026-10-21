# Design

## Behaviour

The band is the record's appearance word masked with `0xff00`. A zero band takes HF when bit 0 is set and HM otherwise. The other bands are fixed: 0x200 gives HAM, 0x300 gives OM and 0x400 gives ML. Everything else gives no label and passes a null string to the text set.

## Model

`creation_label(word)` returns a `CreationLabel`; `code()` gives the literal the original stores, or `None`. The five literals are read from the executable's read-only data.

## Open questions

What the codes abbreviate is not established.

## Docs

This method is 158 bytes. `SG_Character_Create` moves from 2% to about 2%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
