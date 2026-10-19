# Design

## Behaviour

The stored word is shifted left by two in 32 bits, then sign-extended. That side is S. The inset is the logical shift right, by one, of S minus 2. The point moves back by that inset. The quad is then (x, y+S), (x, y), (x+S, y), (x+S, y+S), converted to floats.

A mode call with argument 2, one preparation, one begin, one sample bind, four colour writes, and one end call follow. The begin clears the cursor, so the four marks start at 0 even when the incoming cursor is not 0. A clear bit 4 stores no mark and still finishes at 4.

The image word is 4, so the side is 16 and the inset is 7. A word of -1 makes S minus 2 negative, and the logical shift does not preserve the sign.

## Open questions

The begin call also rebuilds the view scales. That rebuild is not in. The other line-class methods are not in.

## Docs

This method is 210 bytes. With the span bar's 470, that is 680 bytes of the 24 KB class, which is 2%. `TGSLines` is 2%. `TGUIDrawSkin` stays 47%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
