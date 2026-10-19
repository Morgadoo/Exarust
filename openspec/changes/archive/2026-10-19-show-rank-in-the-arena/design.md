# Design

## Gates

`card_gates` keeps the rows when the next mode is the current mode. Practice is 2 and clears both rows unless the mode being left already has bit 2. Story is 4 and clears both. Arena is 8 and sets both. Any other mode leaves them.

## Rows

`shield_rank` reads the top three bits. Zero is absent. The names are Inept, Aspirant, Novice, Adept, Expert, Master, and a dash. Inept is the zero slot, so it is not shown. The seventh index has no name. `shield_points` multiplies the low five bits by one half as a float, and zero is absent. A closed gate hides both.

## Open questions

- Description phrases are not in.

## Docs

`docs/PLAN.md` and `docs/COMPONENT-MAP.md` rate `TGUIItemContainer` at 18%. The published headline stays 14.4% and 5.7%.
