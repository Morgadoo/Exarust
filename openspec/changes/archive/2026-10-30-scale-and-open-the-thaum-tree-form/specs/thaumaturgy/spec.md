## MODIFIED Requirements

### Requirement: Tree window fades
Showing the tree SHALL start a 1000 ms fade: with `r = 1 − (1 − left·0.001)` (the inner difference in double, rounded to single), the window SHALL be `2·(1 − r²)` held to one and the domain wheel child's show value `1 − r²` (times the window while closing). Hiding SHALL cut the window by `left/256` and stop the draw at its stamp. A domain switch SHALL cut the content out over 256 ms, switch at the stamp and fade the new tree in over 1000 ms.

#### Scenario: Opening and closing
- **WHEN** the tree is shown at 1000 and drawn at 1001, 1500 and 2000, then hidden at 3000 and drawn at 3128 and 3256
- **THEN** the window is 0.003997922, then 1 with the wheel child shown at 0.75, then 1 with the fade over
- **AND** the window is 0.5 at 3128 and the screen has faded out at 3256

## ADDED Requirements

### Requirement: Bar fade independent of the wheel
The quick bar SHALL NOT take the domain wheel child's show value; its slots SHALL fade only with the bar's own opening and closing animation.

#### Scenario: The bar ignores the wheel's show value
- **WHEN** the steady bar is drawn while the wheel child's show value is zero
- **THEN** its slots are drawn at full alpha

### Requirement: Tree hit tests at the interface unit
The quick bar's slot hit test and the creation prompt's buttons SHALL be laid out at the live interface unit `u`: slot centres from `(width − 153u) >> 1` in steps of `17u` at a top of `8u`, within `7u` (or `12u` for a drop, nearest slot) of the pointer; BACK between `5u` and `37u` from the left and BEGIN between `width − 37u` and `width − 5u`, both between `height − 9u` and `height − 2u`.

#### Scenario: A larger interface unit
- **WHEN** the pointer is at 704, 64 on a 1000-wide form at unit 8
- **THEN** it hovers slot 7, and the same point hovers nothing at unit 4

### Requirement: Tree form opening words
Opening the tree from the character creator (no calling form) SHALL set the form's task word to `0x10`, its hint alpha to `0xff` and hand in no child panel; opening it from the game SHALL set the task word to `0x1010` and keep the calling form's child panel, whose paint alpha the draw SHALL write as the panel's shown value less twice the window's fade, unclamped.

#### Scenario: The game's opening at half fade
- **WHEN** a played tree's child panel is shown at one and the window is 0.25
- **THEN** the panel's paint alpha is 0.5

#### Scenario: The creation screen
- **WHEN** the creator opens the tree
- **THEN** the task word is `0x10`, the hint alpha `0xff` and there is no child panel
