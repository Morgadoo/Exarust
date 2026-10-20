# Spec Delta

## ADDED Requirements

### Requirement: Personality from the record
The skill share SHALL be the combat skill over 100, capped at 1. With t1 and t2 the record's second and third traits, the confidence SHALL be t1/100, the restraint −t2/100, the courage (confidence + 1)/2, the eagerness ((3 − restraint) + confidence + 3)/8, and the blend `(1 − h) + h·(confidence + 2)·0.333` with h = (1 − feral)², or 1 for a marked body. The jitter SHALL be the square of a draw spread over −1..1, and the arousal SHALL start at 10.

#### Scenario: A half-feral record
- **WHEN** a record of combat skill 150, traits 20 and −30 and feral 0.5 is read with the generator at 0
- **THEN** the skill is 1, the confidence 0.2, the restraint 0.3, the courage 0.6, the eagerness 5.9/8, the blend 0.75 + 0.25·2.2·0.333 and the jitter about 1

### Requirement: Attaching the brain
Without the race word's AI flag no contact sense SHALL be installed. With it, the sense SHALL be the ally sense for race class `0x12xx` and the standard one otherwise; a dead character SHALL get the effect at 0 and its roles SHALL NOT be applied; a statue race SHALL stand sentry with the effect at 0; any other SHALL get the effect at 1.

#### Scenario: Ally, dead and statue
- **WHEN** brains are attached to an ally-class body, a dead body and a statue body, all with the AI flag
- **THEN** the first has the ally sense and the effect at 1, the second the effect at 0 and no roles applied, and the third is a statue with the effect at 0
