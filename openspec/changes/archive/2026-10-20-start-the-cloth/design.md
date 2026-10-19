# Design

## Behaviour

Start ORs 0x10000 into the flags and copies the seed from the start argument's 0x50 word. It copies a 4×4 identity matrix into the cloth's 0xa0 block. The step at 0x178 is the square root of the count word times 1e-6, computed in double and stored as a float. When the cloth's body has a 0x160 pointer whose 0xc word carries 0x10000000, bit 0 of 0x1b0 is set and the pointer is stored at 0x1b8.

Frame checks the pending word at 0x14. At most one makes two copies of the source buffer at 0xd8 into 0xe8 and 0xe0, clears the pending word and sets flag 1. More than one rotates the buffers: the old 0xe8 becomes 0xd8, the old 0xe0 becomes 0xe8, the old 0xd8 becomes 0xe0, and the argument stores at 0x10.

## Open questions

The buffer contents and the body pointer's type are not established.

## Docs

The two methods are 160 and 139 bytes. `TVCCloth` stays 70%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
