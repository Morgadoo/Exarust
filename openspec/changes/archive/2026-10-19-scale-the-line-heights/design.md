# Design

## Behaviour

The method first calls the parent's scale with the two scales. It then stores three bytes: 80 times the vertical scale, 104 times it, and 128 times it. Each product rounds to nearest even and keeps the low byte. The two stored bytes that the method multiplies and discards are dead.

## Open questions

The parent's scale body is not in. The methods around this one are not in.

## Docs

This method is 181 bytes. With the frame, the span bar, the square, the chip, the capped span, the label and the rail, that is 4,395 bytes of the 24 KB class, which is 18%. `TGSLines` is 18%. `TGUIDrawSkin` stays 49%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
