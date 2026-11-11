## ADDED Requirements

### Requirement: Form focus follows the pointer
In the hub's table-built forms the focus SHALL be on the record under the pointer. A text field or combo box gaining the focus SHALL select its whole text. After a text field's Enter gives the focus up, the focus SHALL come back only when the pointer moves.

#### Scenario: Hovering the name field
- **WHEN** the pointer rests on the emblem form's empty name field and r is typed
- **THEN** the field reads r without a click

### Requirement: Text field keys
A focused text field SHALL take each key through the text edit unit at its caret, with its limit and its span. Enter with a callback SHALL run it with the submitted bit and, with the release style, give the focus up. A digits-only field SHALL drop a typed non-digit. With the notify style the owner SHALL be called after every key.

#### Scenario: Editing in the middle
- **WHEN** ravns is typed, Left is pressed twice and e is typed
- **THEN** the field reads ravens with the caret after the e

### Requirement: Combo box keys
A focused combo box SHALL pass every key but Enter to the text edit unit, whatever its editable bits; Enter SHALL run its callback with state bits 0x300000.

#### Scenario: The merchant's box
- **WHEN** End and Backspace twice are pressed over Look for: and then Enter
- **THEN** the box reads Anythi and the stock type is re-stored from the box's selection

### Requirement: Text field caret placement
A left press on a hovered text field SHALL put the caret and the anchor under the pointer, measured from the field's own text origin; a drag while held SHALL move the caret only, selecting from the anchor.

#### Scenario: Press and drag
- **WHEN** a field with six 8-pixel bytes is pressed 20 pixels right of its origin and dragged to 39
- **THEN** the anchor is 3 and the caret 5

### Requirement: Colour swatch drag
A swatch held past the pointer's drag threshold SHALL start a colour drag carrying its colour, the press's offset and its inner size, drawn under the pointer at alpha 0xa0; the press SHALL not run the swatch's callback. A swatch released over SHALL take the colour and run its drop callback.

#### Scenario: Blue onto the design's second colour
- **WHEN** the background's first swatch is dragged onto the design's second
- **THEN** the design's second colour takes the first's word and no palette popup opens
