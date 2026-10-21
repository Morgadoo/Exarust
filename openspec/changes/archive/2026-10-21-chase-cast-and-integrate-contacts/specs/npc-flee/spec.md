# Spec Delta

## ADDED Requirements

### Requirement: Calming contacts
Each tick an active brain SHALL take a seen character's hostility down by `(1 − decay)·soothe·(1 − restraint)·5e-5` times its calming scale, raise each contact's concern by 0.5 % of the gap to its target (else decay it by 0.9997), decay hostility by 0.999, and raise the noticed level by 0.5 % of the sighting while below it (else lower it 0.01).

#### Scenario: Calming and rising
- **WHEN** a seen contact with soothe 20, concern 1 toward 2, sighting 2 and hostility 1 is integrated with no restraint and decay
- **THEN** its hostility is 0.999 × 0.999, its concern 1.005 and its noticed level 0.01

### Requirement: The integration's aggregates
The integration SHALL clear an unreachable mark whose time has passed; a passive brain SHALL only decay its contacts and clear this tick's sightings. The notice scale SHALL be `100·n/(100 + n)` of the noticed sum above 1, else 0.99, and the readiness SHALL rise by 1 % toward the interest or fall 0.0125 while above it.

#### Scenario: A passive brain
- **WHEN** a passive brain integrates a contact seen this tick with concern, noticed level and attention 1
- **THEN** this tick's sighting clears, the concern is 0.9997 and the noticed level and attention 0.999

### Requirement: The target pass's clean-up
The pass SHALL clear every contact's concern target and this tick's sighting before weighing, take a scripted focus outright, and clean every contact neither noticed nor the best of its level, concern target and sightings. A brain entering combat SHALL take the level (3 for 2 and above) and the target; one already in combat SHALL keep both.

#### Scenario: An unkept contact
- **WHEN** a pass sees nothing and holds a contact of level 2 with a noticed level of 0.7
- **THEN** the contact's level, concern target and sightings are 0 and its noticed level stays 0.7

### Requirement: The greeting and the refresh
A newly attended character SHALL be met, and greeted when the notice scale is not above 6, the readiness not above 1 and it lies within 600 cm, a NaN passing each gate. The round-robin cursor SHALL move to the next contact seen this tick, looking first two on.

#### Scenario: Gates and distance
- **WHEN** a noticed character is attended at 600 cm, at 601 cm, with a notice scale of 6.01, and with NaN gates
- **THEN** it is greeted at 600 cm and with NaN gates, and only met otherwise

### Requirement: The nearby list
The nearby list SHALL hold at most sixteen characters in sight within 3000 cm by squared distance, a new one going before every entry it is not farther than, a NaN distance first, a full list dropping its last.

#### Scenario: Ties and NaN
- **WHEN** two characters at the same distance and one at a NaN distance are inserted in that order
- **THEN** the list holds the NaN one, then the second, then the first

### Requirement: Courage and health
Each AI tick SHALL decay the stress by 0.01 and the alarm by 0.9999, lower the courage by `(r/r₀·0.5 + 0.5)·0.9` when the health ratio r (scaled by `0.4 + 0.2·confidence` while down) drops, and ease it off by 0.0001 above `(r + 1)(confidence + 1.1)/2`.

#### Scenario: A downed body
- **WHEN** a downed body of confidence 1 at full health drifts once
- **THEN** its health ratio is 0.6

### Requirement: Courage, panic and jitter
The courage SHALL drift toward `max(2·(stress + alarm), 2·total, confidence + 0.5)` at the easing factor × 0.0003 (0.00015 from 1 up), or decay by 0.999 at or over it; the panic SHALL take 0.001 of the stress over the courage and decay by 0.995; the jitter SHALL walk a hundredth of a centred draw (−0.003 while down) within ±1.

#### Scenario: Drift
- **WHEN** stress 1, alarm 1, courage 0.5 and confidence 0.5 drift once at full health
- **THEN** the courage rises by (3.9798 − 0.5)·1.6·0.0003
