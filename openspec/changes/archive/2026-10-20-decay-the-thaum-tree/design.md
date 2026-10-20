# Design

## Behaviour

With mask 0x10 the two fields at 0x17c and 0x180 lose `(2 + base) × first × 4` and `(2 + base) × second × 4`; the first start sets its byte and clock, and the method returns 3. With mask 0x20 the 0x180 virtual takes the runtime value times −10, rounded to nearest, and the result is 3. Both masks may act in one call.

In mode 3, mask 0x200 calls the 0x250 callback when present and mask 0x100 calls 0x248. Outside mode 3 a mask carrying 0x300 dims once unless the dim bit is set.

## Open questions

The virtual and callback bodies are not in.

## Docs

This method is 316 bytes. `TThaumTree` moves from 22% to about 23%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
