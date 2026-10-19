# Design

## Behaviour

The gate is `(word & 0xf00) == 0x100`; it returns before any particle work. For each particle, the normal's squared length decides: under 1e-6 the normal becomes zero and the offset −1; otherwise the normal is normalized and the offset is its dot with the edge plus the length. The edge is `e0 − d8` with its component along the normal removed. When that squared length exceeds 0.2 the edge is added to d8 scaled by the negative square root of 0.1 over it; otherwise it is subtracted. The normal then gains d8 times the negative length.

After the loop, the coupled calls run `coupled − 1` times when the coupled word is at least 2. All normals are zeroed and the byte at 0x1fe clears.

## Open questions

The coupled update's body is not in.

## Docs

This method is 809 bytes. `TVCCloth` is 77%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
