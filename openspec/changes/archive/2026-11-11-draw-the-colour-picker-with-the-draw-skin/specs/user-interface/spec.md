## ADDED Requirements

### Requirement: A form keeps the skin its maker gives it
A form made without an owner SHALL draw with the draw skin's default instance and its own scheme unless its maker sets another skin; a form with an owner SHALL take its owner's skin. The colour picker SHALL draw with the default instance.

#### Scenario: Opening the colour picker
- **WHEN** the hub opens the colour picker on a swatch
- **THEN** its frame is the default background with a title bar in the default instance's palette, and its records sit under the title bar

### Requirement: Records on the default instance
On a form that keeps the default instance, a button SHALL draw the skin's shell and then its caption through the skin's label slot, and a trackbar SHALL draw its style frames, the skin's slider and its caption through the plain label draw.

#### Scenario: A picker's channel trackbar
- **WHEN** the R trackbar of a picker on white is drawn
- **THEN** its trough is striped from cyan to white and a one-unit bar stands at its right end
