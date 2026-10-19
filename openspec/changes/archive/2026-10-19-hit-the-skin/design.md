# Design

## Behaviour

The place carries x0, y0, a size, three limits (0x3c, 0x40, 0x44), the word at 0x5c and the flags at 0x64. The edge pass starts from zero:

- flags & 0x4000: size at or above x returns 0x80000011; otherwise x at or above x0 − size − 1 returns 0x80000012.
- flags & 0x8000: size at or above y ORs 0x80000014; otherwise y at or above y0 − size − 1 ORs 0x80000018.

A non-zero result returns immediately.

Otherwise, with flags bit 0 set, the threshold is 0x40 when flags & 0x800030 equals 0x20 or the 0x5c word has bit 0x80, and 0x40 + unit otherwise. When the threshold is above y, codes −4, −5 and −10 are tried under flag bits 1, 2 and 3; a hit returns its code and otherwise −3. The same checks are repeated for y below 0x40, which the threshold invariant makes unreachable.

With flag 0x200 and x at or above 0x44, codes −9 then −10 are tried, then −8. With flag 0x100 and y at or above 0x3c, codes −14 then −13 are tried, then −12. Otherwise the result is 0.

A code hits when the rectangle the caller returns for it contains x and y inclusively on both axes. A code with no rectangle does not hit.

## Open questions

The virtual that supplies each code's rectangle is not in; the caller supplies it. The limits at 0x3c, 0x40 and 0x44 are named only by offset.

## Docs

This method is 807 bytes. `TGUIDrawSkin` is 44%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
