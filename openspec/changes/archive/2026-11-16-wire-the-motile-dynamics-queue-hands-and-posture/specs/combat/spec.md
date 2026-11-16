## ADDED Requirements

### Requirement: Light blow step stamp
A blow on the flesh path whose impact is not over 0.5 (a float compare, so a NaN impact too) SHALL take no knockback step and SHALL date the struck body's step start 1000 ms back, so every window measured from it (100, 200 and 300 ms) has passed.

#### Scenario: A slow cut
- **WHEN** a slow cut of impact under 0.5 strikes a stepping body
- **THEN** no step starts and the running step reads at least 1000 ms old

### Requirement: Attack arms before the stall
The attack arms SHALL read the swing strength before the attack routine drains it by the stall, and the drain SHALL floor the strength at 0 by an ordered compare, so a NaN strength stays.

#### Scenario: A stalling swing
- **WHEN** a swing of strength 1 stalls by 0.5
- **THEN** its arms are those of strength 1 and its strength is 0.5 after the tick
