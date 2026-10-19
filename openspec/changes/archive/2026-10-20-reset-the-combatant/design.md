# Design

## Behaviour

The reset writes zero to the count at 0x288, the qwords at 0x770, 0x788 and 0x798, the words at 0x200 and 0x158, the byte at 0x87e, and the body's 0xe38 and 0xe50 pointers, and writes −1 to 0x7a0. A marker at 0x7d0 has its 0x10 and 0x18 words zeroed. A reference at 0x7c8 has its 0x1bc count decremented and the pointer zeroed. The measure pair stores the given reach at 0x7b0 and the body's 0x990 value times 1.0 at 0x7b8.

## Open questions

The base call and the reach function are not in.

## Docs

The two methods are 216 and 62 bytes. The combat-rule figure stays about 29%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
