# Design

## Behaviour

The method takes the unit integer, an origin, a span, and a stored word. It always counts one preparation, then binds a 32-byte middle sample and draws one quad, then binds a 32-byte cap sample and draws the left cap, then binds that cap sample again and draws the right cap. There is no branch.

The thickness is the stored word sign-extended and shifted left by one, then converted to a float. The image's stored word is 4, so the thickness is 8. A stored word of -3 becomes -6.

At unit 5, origin 10, 20 and span 100 the middle is (15, 20), (15, 28), (105, 28), (105, 20). The left cap is (15, 20), (15, 28), (10, 28), (10, 20). The right cap is (105, 20), (105, 28), (110, 28), (110, 20).

## Open questions

The preparation call switches on a material. Its body is not in this slice. The vertex helper adds a view origin and multiplies by a view scale, then stores a colour word. Those three are shared and are not in.

## Docs

`TGSLines` is 1%: 470 bytes is past 1% of the 24 KB class and under 2%. `TGUIDrawSkin` stays 47%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
