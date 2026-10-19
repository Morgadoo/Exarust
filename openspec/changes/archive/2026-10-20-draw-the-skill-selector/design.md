# Design

## Behaviour

The draw begins once, binds the skin's 0xe8 texture and takes the selection's low 16 bits less one.

On page 2 it selects the text font and draws one line at 12 units across, the place width less twice the unit, zero, 20 units down and flag 1.

Otherwise, when the index is 0 through 7, it selects the icon font, measures the row's icon, centres it across the place width less twice the unit and draws it 16 units down. It then selects the text font and draws the row's text at 18 units across, the same reduced width, zero, and the place height less 20 units, with flag 1.

Any other index draws nothing. The begin and bind still run.

## Open questions

The row table and the font measurement are not in. The page field is named only by offset.

## Docs

This method is 424 bytes. `SG_GameGUI` is 6%. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
