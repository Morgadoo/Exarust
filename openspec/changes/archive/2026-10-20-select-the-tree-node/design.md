# Design

## Behaviour

When the new word and the current word are both non-positive the base result returns unchanged. In mode 3 a new or current word equal to 0x1000000 or 0x2000000 stores the new word and returns 3 without touching the focus.

Otherwise the candidate is zero when bit 0x400000 is set. A new word in 1..0xfffe gives its low 16 bits when the table entry's first word is at least 1, else zero. Any other word gives zero, except that a positive current word carrying 0x3000000 returns 3 with the current word kept.

The new word is stored. An equal last focus returns 0. An equal previous selection returns 3. A fresh candidate returns 3, stores the focus, sets the previous when the candidate is positive and stamps the clock.

## Open questions

The table and the base call are not in.

## Docs

This method is 248 bytes. `TThaumTree` moves from 19% to about 19.5%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
