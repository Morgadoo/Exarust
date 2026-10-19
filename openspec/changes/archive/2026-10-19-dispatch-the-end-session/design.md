# Design

## Behaviour

The dispatch reads the global page and the action. Page 2 action 1 calls the resume routine; action 2 shows form 0x1001243c0. Page 4 action 1 calls 0x100124650 when the form page is 4, otherwise 0x100124820; action 2 calls 0x100124820. Page 8 action 1 with form page 2 calls 0x100132250, 0x10017b480 and 0x10012c2c0, then shows form 0x100124800; with any other form page it shows 0x100123d30 with the last-round flag. Page 8 action 2 shows 0x1001243c0. Any other page calls the registered handler whose key equals the page when one exists.

Every path clears the action and field 0x204 and returns 0.

## Open questions

The form constructors and the game calls are named only by address. The handler table's row layout is not needed beyond its key and call.

## Docs

This method is 477 bytes. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
