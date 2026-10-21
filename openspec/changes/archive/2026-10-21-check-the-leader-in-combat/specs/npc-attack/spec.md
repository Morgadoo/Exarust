# Spec Delta

## ADDED Requirements

### Requirement: The follow band in combat
A fighting follower's band SHALL ease toward 700 cm by a hundredth a tick, or, while the leader sprints, toward 500 cm plus 100 cm × how far ahead of the leader it is by a fifth, in double precision; while following only 0.7 of it SHALL count.

#### Scenario: Easing and a sprint
- **WHEN** a band of 600 cm is checked once, and a band of 400 cm 300 cm behind a sprinting leader
- **THEN** the first becomes 601 and the second stays 400

### Requirement: Answering the leader
Inside the band a follower SHALL stop; outside it the follow SHALL run when the leader runs or is three bands away, else walk; a follower with a route SHALL answer the call, replanning when the leader is more than 100 cm from the route's end, and one without SHALL plan.

#### Scenario: Near and far
- **WHEN** a follower with a band of 600 cm and a route is 300 cm, then 2000 cm, then 500 cm from its leader (the route's end then 101 cm off)
- **THEN** it stops, then it answers running without a new route, then it answers walking with a new route
