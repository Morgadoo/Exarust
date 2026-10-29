## ADDED Requirements

### Requirement: Tangle break
Each tick of a living body (muscles on) that is neither down nor dying SHALL run the tangle counter outside a parry: it falls by one (floored at 0), then each hand part inside the box y −5…60 cm, z < 0, |x| < 20 cm of the character frame adds 2; past 60 the counter clears and the Break Tangle clip starts from where the body is, returning to the stance and ending an attack in progress. A parry SHALL leave the counter as it is.

#### Scenario: Hands behind the back
- **WHEN** both hands stay behind the back of a standing fighter
- **THEN** the counter reads 4 after one tick, 58 after 19, and the 20th tick clears it and starts Break Tangle

#### Scenario: A parry holds the counter
- **WHEN** the fighter is parrying
- **THEN** the counter does not change
