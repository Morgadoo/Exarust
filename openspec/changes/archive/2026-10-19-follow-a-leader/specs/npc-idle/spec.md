# Spec Delta

## ADDED Requirements

### Requirement: Follow a leader
No leader: stand if there is no activity or panic is at least 1, otherwise run the activity. Not walking, inside (1 + facing toward the follower / 2) times 100 cm, SHALL close; at that distance SHALL not. The band is 400 cm, 280 while following, and 1200 while busy. Outside it, urgency is 0 up to 800 cm, 1 up to 1600 cm, and 2 past 1600 cm or when the leader sprints. More than 100 cm from the planned spot replans; exactly 100 cm does not. The plan stops at 50 cm.

#### Scenario: Facing sets the stop, distance sets the urgency
- **WHEN** a follower with no leader has no activity, another has panic 1, a third has panic 0.99 and an activity, a follower 149 cm in front of a leader facing it is not walking, one at 150 cm is not walking, one at 500 cm accepts a plan, one at 801 cm does not, one at 1601 cm does not, one at 500 cm whose leader sprints does not, a busy follower at 1000 cm is not following, an already-following busy follower at 300 cm is not walking, an already-following walker whose planned spot is 100 cm aside stays, one whose planned spot is 101 cm aside accepts a plan, and a hidden already-following follower is not walking
- **THEN** the first two stand, the third runs the activity, 149 cm closes, 150 cm runs the activity, 500 cm pursues at urgency 0 and stops the plan at 50 cm, 801 cm is urgency 1, 1601 cm and the sprint are urgency 2, the busy follower at 1000 cm runs the activity, the follower at 300 cm pursues at urgency 0, 100 cm does not replan, 101 cm does and remembers the leader, and the hidden follower stands
