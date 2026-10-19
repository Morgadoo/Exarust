# Design

## Behaviour

The rebuild checks that the object is its own root. When it is not, nothing happens. A root copies the object's 0x280 pointer plus 0x10 into the cloth's 0xf0. When the object has an override at 0x2b0 that differs from that origin, the rebuild stops. Otherwise it takes the count at 0x280 plus 0xc, transforms each point at 0x280 plus 0x10 into a fresh buffer using the object's 0x40, and submits the buffer.

## Open questions

The transform and submission are not in. The pointers are named only by offset.

## Docs

This method is 224 bytes. `TVCCloth` stays 72%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
