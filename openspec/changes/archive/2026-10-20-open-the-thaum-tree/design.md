# Design

## Behaviour

Close clears the global qword at 0x56a590, runs the inner reset once, clears the byte at 0x56a3a0 and runs the base once.

Show checks the focus global's 0x70 count; when positive it calls the focus with the low three bits of the global's 0x40 word OR 0x800. When the pane's 0x60 word carries 0x20 it refreshes once. It ORs bit 2 into 0x21d, writes the clock plus the 0x2ad5b0 offset to 0x224 and stores the 0x1bc word at 0x2e8790.

## Open questions

The focus and refresh bodies are not in.

## Docs

The two methods are 59 and 121 bytes. `TThaumTree` moves from 17% to about 17.5%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
