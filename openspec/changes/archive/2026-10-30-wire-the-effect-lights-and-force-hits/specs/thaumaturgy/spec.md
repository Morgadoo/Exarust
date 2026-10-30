# Spec Delta

## MODIFIED Requirements

### Requirement: Force character knock
A struck character SHALL be knocked by its gathered impulse, normalised, capped at 30 and doubled, when the impulse's squared length (summed `z² + (y² + x²)`) is above 1, and its support-pin gain SHALL be set to `(1 − k)·0.08` computed in double precision, 0 below 0 and 0.008 above 0.08 (compared in double), a NaN kept. Its instability counter SHALL rise by one and its concern toward the caster by 1 at gain 0.01.

#### Scenario: A heavy impulse
- **WHEN** a character of mass 80 gathers an impulse of 100 along x
- **THEN** it is knocked 60 along x with a pin gain of 0.078

#### Scenario: The length's sum order
- **WHEN** a character of mass 80 gathers an impulse of (0.2, 0.2, 1.3)
- **THEN** the knock is twice the root of 1.7699999, not of 1.7699997

## ADDED Requirements

### Requirement: Force hit on the struck body
In the running game, a character a force blast or torrent strikes SHALL have its body's support-pin gain replaced by the knock's pin gain and its balance's landing momentum weight set to 5, after the knock. The gain SHALL then recover by 0.001 a tick while under 0.08 and the landing weight fade by 0.05 a tick, as the body tick does for any body.

#### Scenario: A 50 kg body struck by a blast
- **WHEN** a blast strikes a 50 kg character standing at the living gain 0.08
- **THEN** its pin gain is 0.0768, its landing weight 5, and four body ticks later its gain is back above 0.08

#### Scenario: A torrent hit
- **WHEN** a torrent strikes a 50 kg character
- **THEN** its pin gain is `(1 − 1/50)·0.08` = 0.0784 and its landing weight 5

### Requirement: Force effect light lifetime
In the running game, a force blast, a force torrent and a teleporter bubble SHALL each register one GI shape while they live and refresh it every frame. The blast's shape SHALL be freed once its frame builds nothing and never registered again; every effect's shape SHALL be freed when the effect leaves the scene.

#### Scenario: A spent blast
- **WHEN** every chunk of a thrown blast is spent
- **THEN** its shape leaves the GI's list on that frame and is not registered again

#### Scenario: A blast leaving the scene
- **WHEN** a lit blast's entity is removed
- **THEN** its shape leaves the GI's list

### Requirement: Force effect light values
The effect lights SHALL take the manager's light scale 0.09. The blast's SHALL be a multi-point light with one point per chunk at the chunk's light point (the second sample of its bent ribbon spine), its colour following its charge or life curve; the torrent's one point per gathered light point, following its charge; the bubble's its shell with the teleporter's colour, fade, pulse and phase.

#### Scenario: A charging blast at charge 0.1
- **WHEN** a blast charges at 0.1
- **THEN** its shape holds 16 points, each coloured (0.16, 0.08, 1)·0.09·0.6·(0.6 − 0.1)·0.1·11.11, with attenuation 5000 and a box of the points plus 120 cm

#### Scenario: A torrent at charge 0.5
- **WHEN** a torrent at charge 0.5 has gathered a point of weight 2
- **THEN** that point is coloured (0.12, 0.06, 1)·0.09·0.5·0.75·1.2 at attenuation 2000

### Requirement: Effect frame inverse
An effect's frame SHALL take a world point into its own space by removing its translation and taking the dot product with each basis column, so that the frame applied to the result gives the point back for any turned frame.

#### Scenario: A camera turned a quarter about x
- **WHEN** a frame with columns (1, 0, 0), (0, 0, −1), (0, 1, 0) at (10, 500, −20) takes the point (30, 120, 40)
- **THEN** the point in its space is (20, −60, −380), and the frame applied to that gives (30, 120, 40)
