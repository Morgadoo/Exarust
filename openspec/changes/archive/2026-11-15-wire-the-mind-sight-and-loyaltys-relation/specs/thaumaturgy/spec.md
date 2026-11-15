## ADDED Requirements

### Requirement: Sense Minds overlay start
A Sense Minds cast's answer SHALL start the sight overlay on the frame for the instant cast's duration, or with no end for the persistent one, registering its frame hook when it was off. The persistent power's end SHALL set the overlay to its fade.

#### Scenario: A cast from the bar
- **WHEN** the player's bar casts the instant Sense Minds with focus covering its cost
- **THEN** the overlay is fading in, its hook is registered and it ends the cast's duration later

### Requirement: Mind-sight frame
While its hook is registered the overlay SHALL fade in and out over 2 s, unregister its hook once faded out, run the tracker over every character but the player and the body it left, and draw each tracked mind whose body has a head part at the overlay's alpha less its fade.

#### Scenario: An instant overlay
- **WHEN** a 4000 ms overlay starts at 1200 ms beside another character
- **THEN** at 1704 ms that character's sphere is drawn at alpha 0.252 on its head, and at 7500 ms the overlay is off with no minds and no spheres

### Requirement: Mind-sight level and quit hooks
A new level SHALL drop the overlay's tracked minds and keep the overlay running; the story's quit (without the stripped mark) SHALL turn the overlay off, drop its minds and unregister its hook.

#### Scenario: A level change under the persistent overlay
- **WHEN** a level loads while the persistent overlay tracks a character that the change removed
- **THEN** the overlay tracks no mind

### Requirement: Mind-sight place
Each sphere SHALL sit on its body's head part frame, lifted by the race record's sight lift plus 15 cm, moved by its forward offset along the head's third axis and scaled by its three scales.

#### Scenario: The human place
- **WHEN** a human's head part is 2 m ahead of the origin at 1.6 m
- **THEN** its sphere's anchor is at (0, 160, 200) cm with lift and forward offset 0

### Requirement: Mind awakening glow
The mind awakening game event SHALL add a glow on the player; each frame every glow SHALL tick and draw its single entry at `1 − fade`, and a glow SHALL end after 3 s.

#### Scenario: A glow on the player
- **WHEN** a glow is added at 1200 ms
- **THEN** it draws at alpha 0 at 1200 ms, 0.804 at 2004 ms and is gone at 4200 ms

### Requirement: Bar toggle in a handed body
The bar's toggle on a body whose status has bit `0x20_0000` SHALL NOT toggle the bar; with a body left behind it SHALL end that body's Spirit Form or the Control Mind it runs.

#### Scenario: Toggle while possessing
- **WHEN** the player toggles the bar in a possessed body
- **THEN** the held body's Control Mind is ending and its bar is unchanged

## MODIFIED Requirements

### Requirement: Loyalty strength
Once 800 ticks have passed, a loyalty target's strength SHALL blend `((relation+100)·0.005)²·100`, kept in double until it is narrowed, at 0.05 with 0.95 of its old value. The relation SHALL be the marked body's last attitude toward its caster, as the Loyalty cast's own strength reads it.

#### Scenario: A loyalty strength in double
- **WHEN** a loyalty target past its 800 ticks with strength 0 ticks with relation −98.15
- **THEN** its strength has the bits 971000813

#### Scenario: The real relation
- **WHEN** a marker with strength 0 reaches its 800th tick on a body whose last attitude toward its caster is 50
- **THEN** its strength is 2.8125

### Requirement: Mind-sight noise
The sight's noise SHALL be a 256² field of four octaves at amplitude 1 with both plane-building weights 0.5, then set to four octaves, a 0.25 s period and target 1; the spirit screen's SHALL be 512² with a 0.8 s period. The overlay and the glows SHALL each sample their own field. Nothing SHALL be made or ticked while nothing is drawn.

#### Scenario: Nothing to draw
- **WHEN** no overlay entry and no glow is drawn
- **THEN** the draw makes no sphere and no noise picture
