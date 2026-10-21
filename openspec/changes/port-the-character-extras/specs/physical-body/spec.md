## ADDED Requirements

### Requirement: Electric shock window and sound
A shock SHALL do nothing while the body's next allowed shock is more than 150 ms ahead. A shock that acts SHALL play `elechit` with the pitch 0.4·u + 0.8 from the first draw and the gain 0.1·v + 0.3 from the second, in double precision.

#### Scenario: The window
- **WHEN** a body at 1000 ms allows its next shock at 1151 ms
- **THEN** the shock does nothing, and at 1150 ms it acts

### Requirement: Electric shock by race
A race with bit 0x1000 of its race word SHALL, unless the body is collapsing (condition bit 0x800), put the body's owner into the shocked state and allow the next shock 500 ms on; race index 1 SHALL only ring, next shock 500 ms on; every other race SHALL ring with the next shock 500 ms plus (seed × 200) >> 32 ms on, the game's generator advanced once, and, unless collapsing, burn.

#### Scenario: The longest random delay
- **WHEN** a human is shocked at 1000 ms with the generator's new seed 0xFFFFFFFF
- **THEN** its next shock is allowed at 1699 ms

### Requirement: Electric burn
A burn SHALL take the shock's amount off yellow health, setting it to zero only when the result is ordered below zero, set the flag word's shock-stagger bit 0x100, leave the condition normal, start motion 3 without a blend, stop locomotion, start a kind-2 stagger step of 1.5 × the step reach toward the standing point plus the shock's direction × 200, and start a pain reaction on part 2, or part 1 when the grip list holds a second entry, with (0.2·v + 0.4, 0.2·u + 0.8) from two more draws.

#### Scenario: A burn below zero
- **WHEN** a body with yellow 0.3 and step reach 80 takes 0.5 along +z from (10, 0, 20)
- **THEN** yellow is 0, the step is 120 long toward (10, 0, 220)

#### Scenario: A NaN health
- **WHEN** the yellow health is NaN
- **THEN** it stays NaN

### Requirement: Shock force release
The force release SHALL end a stagger step in progress, reset a swing in the combat condition (the swing flags gain 0x6), leave the body busy for 600 ms and set its condition to shocked (0xC0).

#### Scenario: Released in combat
- **WHEN** a stepping body in condition 2 is released at 2000 ms
- **THEN** the step ends, the swing is reset, it is busy until 2600 ms and shocked

### Requirement: Bone body flag word
The physical body reader SHALL keep the body chunk's first word as the part's flag word and SHALL read the stored collision spheres when a body carries them; for every shipped `humanoid.rsq` part it SHALL find a `0x1240_0000`-family word and no stored spheres.

#### Scenario: The shipped humanoid
- **WHEN** `humanoid.rsq` is read from the installation
- **THEN** its 18 bodies hold no stored spheres
