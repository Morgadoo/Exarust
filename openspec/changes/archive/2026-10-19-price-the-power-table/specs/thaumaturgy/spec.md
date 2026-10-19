# Spec Delta

## ADDED Requirements

### Requirement: Power prices
The power table SHALL record each listed id and its focus cost, including the four effects already cast. A sustained cost SHALL be that cost times the tick in seconds, and half of that when the sustain technique is known. Repeating the same instant power with the repeat technique SHALL halve the cost. Blast and bolt SHALL spend 1.2 times the cost, barrier twice, a mind power 0.3 times, vanish 80 times, and the long door once.

#### Scenario: Barrier tick and a repeated blast
- **WHEN** barrier's 0.07 is priced for a 500 ms tick, then again with the sustain technique, and blast's 0.10 is repeated with the repeat technique
- **THEN** the costs are 0.035, 0.0175 and 0.05, and calm spends 0.3 times its cost while the long door spends its cost once
