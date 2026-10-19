# Design

## Behaviour

The state bit 0x40 selects the raised arm. With that bit clear, bit 0x10 selects the pressed arm. With both clear the idle arm is used. Each arm is two fill colours, two border colours and one outline colour.

The fill is two quads. The first runs (x+w-2, y+2), (x+2, y+2), (x+2, mid), (x+w-2, mid) with the second colour at the top and the first mixed toward the second by 0.6 at the middle. The second runs (x+w-2, mid), (x+2, mid), (x+2, y+h-2), (x+w-2, y+h-2) with the first colour at the middle and the second mixed toward the first by 0.6 at the bottom. The two midpoint pairs are not shared, so the gradient has a hard seam.

The border is six lines on the rectangle (x+1, y+1) to (x+w-1, y+h-1) moved in by half a pixel. Seven binds feed twelve vertices. The left seam is at the midpoint plus 0.5 and the right seam at the midpoint. The stops are 0.65 near the top and 0.35 near the bottom.

The outline is four lines on (x, y) to (x+w, y+h) moved in by half a pixel, using the outline colour already bound. Each batch is independent and counts one begin and one end. The fills bind four samples, the border seven, and the outline one. Totals are three begins, twelve binds and three ends.

Mix each RGBA channel as `A + (B - A) * t`, round to nearest even and saturate to 0..255. The x plus w sums are made in 64 bits before the float conversion.

## Open questions

The colour ramps that fill the arm inks are not in. The methods around this one are not in.

## Docs

This method is 775 bytes. With the span bar's 470, the square's 210 and the chip's 401, that is 1,856 bytes of the 24 KB class, which is 7%. `TGSLines` is 7%. `TGUIDrawSkin` stays 47%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
