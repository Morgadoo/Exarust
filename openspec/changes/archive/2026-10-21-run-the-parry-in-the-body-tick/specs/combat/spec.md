## ADDED Requirements

### Requirement: The parry check in the body tick
The system SHALL run the parry check for the ticked fighter after its swing routine: from the combat idle when its muscle tone is at least 0.85, its auto-parry deadline has passed and its hands hold something, and on every tick it already parries. A parry that starts SHALL advance the counter twice in that tick. A fighter holding a weapon SHALL NOT parry until the armed grip is modelled.

#### Scenario: A mid swing in front
- **WHEN** an unarmed fighter in the combat idle at game time 12 000 ms faces a foe 1 m ahead and 0.5 m higher attacking with Attack Mid at clock 0.5
- **THEN** it parries with Parry Left, blend 20 frames, fast ease, last-check time 12 000 and counter 2

#### Scenario: The deadline is strict
- **WHEN** the last attack tick was at 11 600 ms
- **THEN** the idle starts no parry at 12 000 ms, and one at 11 599 ms does

#### Scenario: Tone and hands
- **WHEN** the muscle tone is 0.84, or the hands are empty, or a weapon is held
- **THEN** the idle starts no parry

### Requirement: The parry trigger's window
The trigger SHALL skip the foe's clock window (0.3 through 0.9) only for a swing carrying the continuation flag; an open strike window SHALL NOT skip it.

#### Scenario: Clock edges
- **WHEN** the foe's clock is 0.29, 0.3, 0.9 and 0.91
- **THEN** the parry starts at 0.3 and 0.9 only

### Requirement: The parry's re-pick and counter
The check SHALL store the countered motion and the picked clip whenever it picks again, advance the counter from the stored clip on every checked tick, and let the counter of a fighter not parrying drift one step a tick toward 0.

#### Scenario: A new motion
- **WHEN** the foe changes from the main side to the other side while the fighter parries a level foe
- **THEN** the countered motion becomes 8, the clip Parry Rgt Dn with blend 20

### Requirement: The parry's end
A parry that ends SHALL return to the Defence stance when yellow health is under 0.01 or the fighter is dazed, else to the combat idle, with a 40-frame smooth blend, unless the condition's clip stands still. A fighter without the permission SHALL only drop the parrying flag.

#### Scenario: Dazed
- **WHEN** the foe stops attacking and the fighter's energy is 0.1 under its most
- **THEN** the parry ends into the Defence stance

#### Scenario: Shocked
- **WHEN** the fighter is shocked
- **THEN** the parry ends and its clip and stance stay

### Requirement: The parry guard
The fighter's guard SHALL be the parry's only while it parries or counters a motion, SHALL NOT replace a shield's guard, and SHALL open when the hit path ends the parry.

#### Scenario: A shield held
- **WHEN** a fighter holding a shield parries
- **THEN** its guard stays the shield's
