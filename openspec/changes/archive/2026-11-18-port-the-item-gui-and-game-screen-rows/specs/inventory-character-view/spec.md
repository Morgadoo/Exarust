## ADDED Requirements

### Requirement: Locked trade preview drag
The trade window's preview SHALL start its drags as the locked character view: a shop item (flag 0x80000000) SHALL carry the payload no panel takes, so it comes off the preview wherever the drag ends and lands in no panel; any other item carries the item payload. An item whose icon does not resolve starts no drag.

#### Scenario: A tried-on shop helmet dragged to the pack
- **WHEN** the player drags a shop helmet off the trade preview onto the pack panel
- **THEN** the pack does not take it and the helmet comes off the preview

### Requirement: Frozen loadout preview drag
The loadout form's preview SHALL start its drags as the frozen character view: only an item the pick finds in a hand starts a drag; a press on a worn piece starts none.

#### Scenario: A press on the preview's armour
- **WHEN** the player presses on a worn piece of the loadout preview
- **THEN** no drag starts and the piece stays worn
