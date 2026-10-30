# Spec Delta

## ADDED Requirements

### Requirement: Breaking a mind hold
After a perception pass sets the attitude, when the contact's concern is over 1.5 and the raw role attitude plus 0.1 is under the effective attitude (compared in double precision), the pass SHALL set the state of the observer's first power that is a binding sourced on the observed character, or a charm aimed at it, to 2. A command SHALL NOT be broken, and the attitude this pass SHALL stay the effective one.

#### Scenario: A worried observer bound to its foe
- **WHEN** an observer with concern 2 and raw attitude 0 holds a binding sourced on the observed character
- **THEN** the pass reports the mind check, the attitude is 100 and the binding's state is 2

#### Scenario: Calm or already fond
- **WHEN** the concern is 1, or the raw attitude is 100
- **THEN** the binding's state stays 0

#### Scenario: Only the first binding or charm
- **WHEN** the powers are a charm on a third character, a frenzy, a binding sourced on the character and a command sourced on it
- **THEN** only the binding is broken; a command alone is not
