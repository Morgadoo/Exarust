## MODIFIED Requirements

### Requirement: Sleeping limp bodies
The system SHALL sleep a limp body by the original's countdown: 256 when woken, then each tick, by its fastest part's last motion, back to 256 over 40 cm/s, held at 30–32 between 20 and 40 cm/s, and two off under 20 cm/s; at 1 it sleeps. Its first tick asleep SHALL take its velocity away; later ticks SHALL do nothing. A living body SHALL never sleep. A blow or going limp SHALL wake a body; level contacts SHALL NOT. A body woken by a blow SHALL start without its earlier contacts.

#### Scenario: Corpse at rest
- **WHEN** a limp body's parts have moved under 20 cm/s in every tick for 128 ticks since it was woken
- **THEN** it falls asleep in that tick and is not simulated until a blow lands on it

#### Scenario: Woken corpse
- **WHEN** a sleeping corpse on the floor is shoved
- **THEN** it wakes with none of its earlier contacts and is tested afresh where it lies

#### Scenario: Creeping body
- **WHEN** a limp body's fastest part keeps moving between 20 and 40 cm/s
- **THEN** it never falls asleep

#### Scenario: Deep sleep
- **WHEN** a body has slept through one tick
- **THEN** it has no velocity and its particles stay exactly where they are in every later tick

#### Scenario: Living body
- **WHEN** a living body stands, lies fallen or gets up for any time
- **THEN** it stays awake, and it sleeps by its speed only once it goes limp
