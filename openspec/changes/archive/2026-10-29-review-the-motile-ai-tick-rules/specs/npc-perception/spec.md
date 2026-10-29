## ADDED Requirements

### Requirement: Set target
Taking a character target SHALL mark the mind fresh, set its commitment to 1, clear its overshoot mean and stuck count, place the estimate on the target, and measure the bearing from the chest as the flat length of the offset with the whole offset divided by it as the direction; the aim SHALL take the direction's yaw. No height rule and no 1 cm floor SHALL apply.

#### Scenario: A target high above
- **WHEN** a target stands 500 cm up and 30 cm ahead of the chest
- **THEN** the distance is 30 cm and the aim is straight ahead

### Requirement: Familiarity drift
The perception's familiarity drift SHALL be added to the observer's relation with the character it saw only when that relation exists.

#### Scenario: A stranger
- **WHEN** a brain sees a character it has no relation with
- **THEN** no relation is made for it

### Requirement: Footprint gate
The AI tick's footprint gate SHALL register the footprint unless the character follows a route, its locomotion is active and its speed is at least 1 cm a tick; a NaN speed SHALL skip the stamp.

#### Scenario: A walker on a route
- **WHEN** a character follows a route at 250 cm a tick
- **THEN** the gate does not stamp

### Requirement: Combat entry and leave
A brain entering combat SHALL set its body's in-combat word and its duellist SHALL take the Defence stance through the set-stance helper. Leaving through the target pass SHALL give the brain's look the body's look yaw and draw the next sway 500 ms out.

#### Scenario: An engagement
- **WHEN** a hostile brain's target pass enters combat
- **THEN** its combatant is in combat and its duellist is engaged
