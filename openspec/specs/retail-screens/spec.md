# retail-screens Specification

## Purpose
TBD - created by archiving change model-the-retail-screens. Update Purpose after archive.

## Requirements

### Requirement: Tooltip timing and box
A tooltip SHALL fade in over 256 clock units, stay shown for 2500 and fade out over 256, with a background alpha of 153. Showing a tooltip while one fades out SHALL reverse the fade.

#### Scenario: Fade phases
- **WHEN** a tooltip is shown
- **THEN** it is fully faded in at 256 units, hidden after 2500 more and 256 of fade-out

### Requirement: Modal screens
The message box SHALL confirm on Enter or Space and cancel on Escape with result -1. The shader warm-up screen SHALL fade in over 200 units, step one cache entry a frame and fade out when the cache is warm.

#### Scenario: Escape cancels
- **WHEN** Escape is pressed in a message box
- **THEN** it closes with result -1

### Requirement: Game-state transition
The transition SHALL fade to black over 600 units (800, 300 or 2000 by the mode flags), call the switch, hold and fade back over 600 units.

#### Scenario: Default fade
- **WHEN** a transition starts with no mode flags
- **THEN** the fade to black takes 600 units

### Requirement: Story background picker
The picker SHALL lay out 256 by 512 portrait cards 288 apart, centred, only for the unlocked backgrounds (the first always).

#### Scenario: One unlocked
- **WHEN** only the first background is unlocked
- **THEN** one card is laid out

### Requirement: Dialogue reply list
The reply list SHALL answer the hovered entry's number from 1, ease its height over 500 units and fade both text colours while a chosen entry is held for a second.

#### Scenario: Hover
- **WHEN** the pointer is over the second entry
- **THEN** the hit test answers 2

### Requirement: Picker fields
A picker's move key SHALL add hundredths of a stick (worked in double) to the point and hold it to 0 through 1, leaving an unordered value unordered, and every key code SHALL answer handled.

#### Scenario: Held at one
- **WHEN** a stick of 1 moves a point at 0.995
- **THEN** the point is 1

### Requirement: Popups and dialogs
The item card SHALL be 268 by 260 with a 256-unit squared fade. The colour window SHALL be 402 by 282 with sixteen layout elements and five gradients from black. The four confirmations SHALL be 392 by 112.

#### Scenario: Sizes
- **WHEN** the constants are read
- **THEN** they equal those sizes

### Requirement: Item icon renderer
The icon renderer SHALL allocate renderbuffers and a first image at twice the icon size (160 by default) and a final image at the icon size, each image being the payload plus a 32-byte header.

#### Scenario: Default allocation
- **WHEN** the icon size is 160
- **THEN** the full image allocates 320 x 320 x 4 + 32 bytes

### Requirement: Body loot and item windows
The body loot window SHALL be 244 by 440 with a 228 by 180 pack at the bottom of its client area. The arsenal SHALL lay out up to five columns of 236 pixels, 60 units apart. A borrowed item SHALL stay marked 0x200 in its column and the inventory SHALL refuse it.

#### Scenario: Five columns
- **WHEN** the screen is wide enough
- **THEN** five columns are laid out

### Requirement: Script role commit
A role commit SHALL rewrite the values in place when the stored count matches the slots and otherwise rebuild the list from the last entry of the role whose count differs, placing the tail `stored * 4` bytes too far and reporting the overrun of the allocation.

#### Scenario: Shrinking the last entry
- **WHEN** the last entry of a role stored three values and two are committed
- **THEN** the entry holds two values, the overrun is 4 and the gap 12

### Requirement: Creator physique wait
The creator SHALL regenerate after a physique change only when the last regeneration plus 2 does not exceed the clock, compared in 64 bits.

#### Scenario: Too soon
- **WHEN** the last regeneration was at clock 10 and it is now 11
- **THEN** the change stays pending
