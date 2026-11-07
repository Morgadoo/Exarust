## MODIFIED Requirements

### Requirement: Support pin pull
In each iteration a supported living body SHALL pull each part's support pins right after that part's own links, parts children first: the thorax's two pins in order, and the pelvis origin twice, last, each pull reading the pushes before it, toward the carry's goals. One pull, in cm: 0.2·e·|e|/60 under 60, else 12 along e, less this tick's horizontal displacement, capped at 8, times S²·(G·(P·F)), G the pin-gain (from 0.08), F the pin factor. A gap under 1.19e-7 cm², walking, or off its feet (down, getting up, dormant or dead) pulls nothing.

#### Scenario: Pelvis origin twice
- **WHEN** a supported pelvis origin and a thorax pin are each 10 cm from their clamped targets and have not moved this tick
- **THEN** the thorax pin takes one application and the pelvis origin takes a second after the first push has moved it

#### Scenario: Pins after the part's links
- **WHEN** a supported body's thorax link to the spine and its thorax pin both correct the thorax in one iteration
- **THEN** the pin's pull reads the thorax as its link left it, and the pelvis pins come after every other part

#### Scenario: One application inside 60 cm
- **WHEN** a supported pin is 10 cm from its clamped target, its horizontal displacement this tick is 1 cm along that error, and the body is an alert human (S = 1, pin-gain 0.08, T = 0.5, E = 1, pin factor 1.25) that is not getting up
- **THEN** one application is (0.2·10·10/60 − 1) cm along the error, times 0.05, which is −0.0333 cm

#### Scenario: One application past 60 cm
- **WHEN** the same pin is 80 cm from its target and has not moved this tick
- **THEN** one application is 12 cm along the error, capped at 8 cm, times 0.05, which is 0.4 cm

#### Scenario: A tiny gap
- **WHEN** a supported pin's horizontal gap is 0, then under 1.19e-7 cm², and it moved 1 cm this tick
- **THEN** one application is zero both times

#### Scenario: Walking or off its feet
- **WHEN** a supported body with its chest 10 cm off the feet centre is walking, then off its feet while getting up, then walking in a shock stagger
- **THEN** the pins do not move it

#### Scenario: Collapsing
- **WHEN** that body is collapsing instead, and not walking
- **THEN** one application is the standing pull

#### Scenario: Chest returns
- **WHEN** a supported standing body's chest is held about 10 cm off the feet centre
- **THEN** later ticks of the real step bring the chest back toward the feet centre

#### Scenario: Goals once a tick
- **WHEN** a body is carried while a step leads its thorax, and its pins are then pulled
- **THEN** the pulls move it toward the goals the carry led, and the goals do not change between iterations
