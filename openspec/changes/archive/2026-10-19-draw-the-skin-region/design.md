# Design

## Behaviour

The place carries an origin, an inset, an extent, two scroll totals, two fractions, the mode word, the flags, whether the translation applied, whether the global clip accepted the region, whether the child prepare succeeded, buffered quads and child visibility.

The first begin runs in every case. When mode bit 0 is set the method returns after it, with one begin, one prepare call and no clip. When the clip is rejected it returns the same way. The clip is origin + inset with width extent.x − inset.x and height extent.y − inset.y.

Otherwise a second begin runs. The scroll is zero when flag 0x800 is set; otherwise it is round-to-nearest-even((total − clip span) × fraction) on each axis. Every buffered quad is replayed as four vertices and the line end runs once when there is at least one quad. Every child whose visibility is true is visited. The scissor is set and released, two calls.

The detail runs when flag 0x80 or 0x100000 is set: at the scroll offsets with the clip span, mode 0x30 and offset one unit for 0x80, or mode 0x50 and offset four units for 0x100000. The 0x80 arm wins when both are set.

The translation, when applied, moves by −origin.x and +origin.y.

## Open questions

The global clip object and the scissor calls' bodies are not in.

## Docs

This method is 965 bytes. `TGUIDrawSkin` is 55%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
