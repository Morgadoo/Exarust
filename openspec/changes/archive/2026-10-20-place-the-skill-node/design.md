# Design

## Behaviour

With a positive index the method calls the layout virtual at slot `0x1c8` with the row and the column, reads the two floats it returns and rounds them to the nearest even integer with `cvtss2si`.

With a zero or negative index, a set scaled flag forwards to the base method. Otherwise the x is half of the form's width less its left field and the y is half of its height less its top field, both truncating toward zero through the original's sign-corrected shift.

## Model

`skill_position(index, bounds, scaled, base, layout)` returns the point. The layout is a closure because the virtual is out of scope; the base is a supplied pair for the same reason.

## Open questions

The layout virtual and the base method are not in.

## Docs

This method is 186 bytes. `TSkillUI` moves from 0% to about 1%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
