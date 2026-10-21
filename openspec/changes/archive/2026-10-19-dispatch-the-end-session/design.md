# Design

## Behaviour

The dispatch reads the global page and the action. Page 2 action 1 calls the end page's continue; action 2 shows the main menu. Page 4 action 1 reloads the stored level when the form page is 4, otherwise ends the session to the main menu; action 2 ends the session to the main menu. Page 8 action 1 with form page 2 hides the kept form, makes a second call and prepares the character record, then shows its form; with any other form page it shows the next practice fight with the last-round flag. Page 8 action 2 shows the main menu. Any other page calls the registered handler whose key equals the page when one exists.

Every path clears the action and field 0x204 and returns 0.

## Open questions

The form constructors and the game calls are named only by address. The handler table's row layout is not needed beyond its key and call.

## Docs

This method is 477 bytes. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
