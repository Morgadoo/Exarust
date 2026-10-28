## MODIFIED Requirements

### Requirement: Personality from the record
The skill share SHALL be the combat skill over 100, capped at 1. With t1 and t2 the second and third traits, the confidence SHALL be t1/100, the restraint −t2/100, the courage (confidence + 1)/2, the eagerness ((3 − restraint) + (3 + confidence))/8, and the blend `(1 − h) + h·((confidence + 2)·0.333)` with h = (1 − feral)², or 1 marked; the shares and the blend are formed in double. The jitter SHALL be the square of a draw over −1..1; the arousal SHALL start at 10.

#### Scenario: A half-feral record
- **WHEN** a record of combat skill 150, traits 20 and −30 and feral 0.5 is read with the generator at 0
- **THEN** the skill is 1, the confidence 0.2, the restraint 0.3, the courage 0.6, the eagerness 5.9/8, the blend 0.75 + 0.25·2.2·0.333 and the jitter about 1

#### Scenario: Shares in double
- **WHEN** a record of combat skill 5 and traits 5 and −5 is read
- **THEN** the skill, confidence and restraint are the f32 nearest 0.05 (bits `0x3d4ccccd`)

### Requirement: The ally-class threat
With a concern of at least 1, the ally sense SHALL take the largest threat to an ally contact. That threat is the contact's weight `(view·0.75 + 0.25)·closeness²` times the observed state's factor (4 attacking, 0.25 with the observed body's in-combat mark or 0.1, halved when down) times 1.2. When that threat is above the concern, the concern target SHALL become 0.3 of it.

#### Scenario: An attacker near an ally
- **WHEN** the observed character attacks, the concern is 1.5 and an ally contact has closeness 0.9 and view 1
- **THEN** the concern becomes 3 and the concern target 0.3·(0.81·4·1.2)

#### Scenario: The in-combat mark
- **WHEN** the observed character stands in its combat stance without attacking
- **THEN** its state factor is 0.25, and 0.1 out of the stance
