## MODIFIED Requirements

### Requirement: Reply panel form
The reply panel SHALL be made on the first talk, 400 by 120 pixels, centred across its parent and 120 + 32 pixels above the parent's bottom, and reused afterwards; it SHALL be shown while the engine's panel is open, also with an empty list, and hidden when that closes, keeping the rows it shows until the next open rebuilds them. The engine's pick turning on SHALL hold the picked entry for 1000 ms.

#### Scenario: First talk
- **WHEN** the player first talks on a 1280 × 720 window and the list needs 224 pixels
- **THEN** the panel sits at x 440, top 568, and its height eases from 120 toward 224, reaching 172 half way through the 500 ms

#### Scenario: Close keeps the rows
- **WHEN** the engine closes a panel showing one row
- **THEN** the panel is hidden with that row kept, and the next open lays its list out again even when it is the same

### Requirement: Session player
The player body's character id SHALL be the session's player. A new player's id SHALL be the original's new-character id: 1, the first free id of the emptied world character database, with the world database's tag, `0x4000_0001`; a loaded save's player SHALL keep the number its record was saved under with that tag. The player's id SHALL never equal a level character's.

#### Scenario: A shared id
- **WHEN** a level character carries id 1 and a new player body is spawned
- **THEN** the player is `0x4000_0001` and the cast holds both, each with its own name and roles

## ADDED Requirements

### Requirement: Uncreated story player
A story level started with no committed character and no save SHALL take the creator's opening record for the unknown background when the user folder's settings store holds its preset (keys `0x6431BC02`, `0x6431CC01`): the stored character record with its race byte, known powers and power slots cleared, its hands and lists dropped to the braies, at full health, its name kept. Without that preset there SHALL be no such record.

#### Scenario: The stored preset names the player
- **WHEN** the store holds a record named "Tester" of race 3 under the unknown background's keys
- **THEN** the player is "Tester", of race 0, wearing only the braies

#### Scenario: Another background's preset
- **WHEN** the store holds a preset only under the knight's keys
- **THEN** no opening record is made

### Requirement: Creator opens on the saved preset
The creator SHALL open a story background on its saved preset when the store holds one: the unknown background's preset with its gear dropped, a named background's preset with its template's hands, worn and carried items.

#### Scenario: A knight's preset
- **WHEN** the knight's preset "Tester" wears one item and the knight template wears two
- **THEN** the creator opens on "Tester" wearing the template's two items

### Requirement: Pad step mode
While the reply panel is open the gamepad reader SHALL turn the move stick's vertical axis into steps: up over 0.1, down under −0.1, repeating every 300 ms while held, with the stick giving no movement.

#### Scenario: A held stick
- **WHEN** the panel is open and the stick is held down
- **THEN** a step down is pressed on the first frame and again 300 ms later

### Requirement: Pad dialogue control
With the panel open and the power bar closed, the pad's newly pressed Select SHALL pick the highlighted entry when it is one of the list's; otherwise a step down SHALL move the highlight forward while it is below the count (from none to the first); otherwise a step up SHALL move it back while it is above the first.

#### Scenario: Stepping and picking
- **WHEN** a pad-opened three-entry panel shows entry 1 highlighted and the pad steps down three times, up once, then presses Select
- **THEN** the highlight goes 2, 3, 3, 2 and entry 2 is picked

#### Scenario: The power bar takes the stick
- **WHEN** the power bar is open and Select is pressed
- **THEN** the panel is left alone

### Requirement: Pad talk
The pad's Talk SHALL close an open panel, as the keyboard's Talk does, and otherwise talk to the target query's character in the pad mode.

#### Scenario: Talk twice
- **WHEN** the pad's Talk is pressed facing Derrin, then pressed again
- **THEN** the panel opens in the pad mode and then closes

### Requirement: Pad talk highlight
A panel opened by the pad's Talk SHALL highlight its first entry on the open and on every layout of a list with entries; one opened by the keyboard's Talk or by the narrator SHALL not.

#### Scenario: A new list
- **WHEN** a pad-opened panel's list changes to two entries after a pick
- **THEN** entry 1 is highlighted until the pointer moves or the pad steps

### Requirement: Reply panel fade
The talk's show SHALL start the base form's show fade when the panel is hidden, and the close its hide fade: 256 ticks of the GUI clock (system milliseconds), the alpha rising from 0 to 1 or falling from 1 to 0 with the ticks left; a hide that ends SHALL hide the panel. A show asked for while the panel is still visible inside its hide fade SHALL not restart the fade.

#### Scenario: Fading in and out
- **WHEN** the panel is shown at tick 1000 and hidden at 6000
- **THEN** it draws at 0.25 at tick 1064, at 1 from 1256, at 0.75 at 6064, and not at all from 6256

#### Scenario: A show inside the hide
- **WHEN** a show is asked for at tick 6100 during that hide
- **THEN** the panel still draws at 0.25 at 6192 and is gone at 6256

### Requirement: Reply panel fade alpha
While a fade runs the panel SHALL draw its box at `round(alpha × 160)` and its inks with the line skin's palette alphas scaled by the alpha, and the scales SHALL be 1 again after the draw.

#### Scenario: Half way in
- **WHEN** the panel draws at alpha 0.5
- **THEN** its box alpha is 80 and its label ink's alpha is the line skin's scaled by 0.5
