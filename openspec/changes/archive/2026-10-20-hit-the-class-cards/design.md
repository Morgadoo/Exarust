# Design

## Behaviour

Let S be the clock scale. The band top is trunc((clock − 1024S) / 2) + trunc(140S) and the height is trunc(512S). A point must lie strictly between top and top + height.

The card count is 1 + bit0 + bit1 + bit2 of the available mask. For each card ordinal 0 through 3, an ordinal above zero needs bit ordinal−1. Available cards take successive slots; a card's left edge is screenWidth / 2 + trunc((slot − count × 0.5) × 288S), and its width is trunc(256S). A point strictly between the left and left + width, and inside the band, names ordinal + 1. Every other point names none.

## Open questions

The draw that uses the same layout and the screen object are not in. The scale is passed in already quantised.

## Docs

This method is 465 bytes. `SG_GameMain` is 15%. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
