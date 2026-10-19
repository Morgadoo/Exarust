# Design

## Behaviour

The span is four times the unit, or six times it when the wide switch is set. The trim test runs when the place's flag word masked with 0x30 equals 0x10 and the point's y is at or above the span. Inside it, control bit 2 with x at or past the right limit minus the span returns -4. Otherwise control bit 1 returns -3.

When the trim test does not run, control bit 0x200 with x past the left limit runs the strip test. A y above the top limit returns -8 when it is below the bottom limit, and -10 when the bottom limit plus the span is above it. A y at or below the top limit returns -9 when the top limit minus the span is below it.

Any other point returns 0. The codes are the original's and their meaning is not established.

## Open questions

The caller that uses the codes is not in. The methods around this one are not in.

## Docs

This method is 224 bytes. With the frame, the span bar, the square, the chip, the capped span, the label, the rail and the heights, that is 4,619 bytes of the 24 KB class, which is 18%. `TGSLines` stays 18%. `TGUIDrawSkin` stays 49%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
