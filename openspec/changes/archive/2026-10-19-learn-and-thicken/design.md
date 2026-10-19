# Design

## Marks

`learn_marks` walks the nodes. Each known pair is an id and a mark. The id matches when bits 20 to 23 are cleared, and a later pair replaces an earlier one. The first node then sets bit 0x10000.

A mark that already has bit 0x1000 keeps bit 0x4000 and sets bit 0x10000. Any other mark clears bit 0x4000. It sets bit 0x10000 when the power's top nibble is not 7 and every link's third word, a one-based end, has bit 0x1000. An end of zero or past the nodes does not.

## Thick joint

`thick_joint` takes the previous point, the joint and the next point. The offset is the chord from the first to the third, turned to the left and scaled to the width. The two edge points are the joint minus and plus that offset. The join factor is the difference of the offset's dots with the incoming and outgoing steps, times −0.09, divided by the squared width, in double precision. A zero-length chord stays on the point.

## Open questions

- Charge-up and the other effect meshes are not in.

## Docs

`docs/formats/OTHER.md` gains the mark and joint rules. `docs/PLAN.md` and `docs/COMPONENT-MAP.md` leave the component at 17%.
