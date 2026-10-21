## ADDED Requirements

### Requirement: Perception range
One character's look at another SHALL skip an invisible character and clear the contact's threat first. The distance SHALL ignore vertical offsets under 150 cm. The contact SHALL be tracked inside 1200 cm, halved asleep, cut for a hidden character by `1 − 0.9·(1 − v)` (its inner radius by `1 − 0.8·(1 − v)`), 600 cm more while tracked. Outside both radii it SHALL be lost, raising the alert of an observer it was the target of to 40.

#### Scenario: At the radius and past the tracked one
- **WHEN** a character is looked at from 1200 cm, then from 1800 cm and 1799 cm while tracked and targeted
- **THEN** 1200 cm is not seen, 1800 cm is lost and raises the alert to 40, and 1799 cm is seen and tracked

### Requirement: Noticeability and view
Closeness SHALL be `1 − d/1200`, at most 0.99. Noticeability SHALL be `3c²` dead or asleep (capping the noticed level and clearing concern), `24c` down (an unconscious character's concern decaying by 0.99), otherwise `(f²/2 + 1)·((g + 2)·(2c⁴ + (1 − c²)c))` from the other's look f and facing g toward the observer, plus 10 attacking. The view weight SHALL be `0.25·((1 − look·u)/2)² + 0.75·(1 − fwd·u)/2`.

#### Scenario: At 600 cm
- **WHEN** a character facing the observer at 600 cm is looked at standing, attacking, dead with a noticed level of 5, and asleep
- **THEN** closeness is 0.5 and the noticeability 2.25, 12.25 attacking, 0.75 dead (capping the noticed level at 0.75 and clearing concern) and 0.75 asleep, with a view weight of 1

### Requirement: Group blend
For a character neither dead nor asleep the attitude SHALL become `a = ((100 − att)·calm·0.25 + att)/100`, moved halfway toward the best `trust·liking/100` over the third parties the other likes and the observer trusts, and stored as 100a. The observer itself SHALL count trust 1 but give no candidate; a third party dead for over 5 s SHALL count trust 0.

#### Scenario: A trusted friend
- **WHEN** the other likes a friend at 80 whom the observer trusts at 50, then likes only the observer at 80
- **THEN** the attitude moves from 0 to 20, and then stays 0

### Requirement: Threat estimate
Each of the other's character contacts SHALL be weighted by its closeness and the other's view of it (32 while the other attacks, 2 down, else 16 or 8), raised to the eighth power, against the share `liking − blend(a/2 + 1/2)`. The threat SHALL be `(own/(own + 0.5) − crowd·(1 − base³))·0.85·m` over 10, quadrupled for a fast body. A mid-swing attack with a threat over 16 and concern over 1 SHALL raise the quick-swing event.

#### Scenario: An attacking foe
- **WHEN** an attacking foe disliked at −100 stands at phase 0.52 with the observer close and in its view while the observer's concern is 2
- **THEN** the threat is bits 0x3fe81b50 with the observer selected, the quick-swing event is raised and the concern target is 27.2

### Requirement: Crowd penalty and concern target
A negative share SHALL feed the crowd penalty, kept as the largest seen across passes, and SHALL raise the concern target of the observer's contact with that third party to the term times the multiplier. The contact's own concern target SHALL rise to the estimate of the largest term.

#### Scenario: A threatened stranger
- **WHEN** an attacking character the observer likes at 100 is close to a stranger the observer does not trust and to the observer
- **THEN** the crowd penalty is 0.5 and is kept on a later pass without the stranger, and the observer's contact with the stranger gets a concern target of 16

### Requirement: Attitude powers
The effective attitude SHALL apply the observer's powers in order, kinds compared under the kind mask: frenzy SHALL make it half the negated aggression and stop; binding or command sourced on the other SHALL make it 100, or average it with 100 under the resistance technique; a charm aimed at the other SHALL make it 100; suspicion SHALL take 50, not below −100. No case SHALL exist for oneself.

#### Scenario: The powers on an attitude of 20
- **WHEN** an attitude of 20 meets frenzy at aggression 4, binding with and without resistance, a charm, suspicion, suspicion on −80, and binding followed by suspicion
- **THEN** the attitudes are −2, 100, 60, 100, −30, −100 and 50

### Requirement: Trust powers
Trust SHALL use the attitude's overrides except that frenzy SHALL make it 0.

#### Scenario: Frenzy and suspicion
- **WHEN** a trust of 20 meets frenzy, then suspicion
- **THEN** it is 0, then −30

### Requirement: Concern
The script's concern call SHALL set the contact's flags word to 3 and, below 15, raise the concern by `h²·((1 − g)·a⁸ + a·g)·gain` with `f = att·0.005 + 0.5`, `g = 1 − blend(f, p)` and `h = g/2 + 1/2`, capped at 15. Reaching 1.2 from below SHALL raise one event bit; reaching 4 from at least 1.2 SHALL raise the other.

#### Scenario: Steps of 0.5625
- **WHEN** concern is added with amount 1 and gain 1 at attitude 0 eight times, and once with amount 2 on a fresh contact
- **THEN** the concern runs 0.5625, 1.125, 1.6875 (first event), up to 3.9375 and then 4.5 (second event), and the jump goes to 15 with only the first event
