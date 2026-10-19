# npc-idle Specification

## Purpose
How an idle character roams, patrols or stands sentry when nothing else has its attention.

## Requirements

### Requirement: Idle cycles
A roam SHALL wait, walk to a wander point, wait 500 ms plus a draw of 2000 ms, walk to a home-side point, wait 1000 ms plus a draw of 4000 ms, and repeat. A patrol SHALL wait 500 ms plus a draw of 2000 ms between wander walks and SHALL NOT walk home. A sentry SHALL hold the guard pose. A deadline equal to the clock SHALL keep the wait. During a wait, a disturbance above 0.3 SHALL be walked to without advancing the phase.

#### Scenario: Out and back
- **WHEN** a roam whose wait ended at 1000 ms is stepped at 1001 ms, accepts a path, arrives at the wander point at 2000 ms, accepts the home path, and arrives home at 3000 ms, the generator having started at 0, and a patrol does the same arrival at 10 ms, and a waiting roam hears a disturbance of 0.31
- **THEN** the roam's first pause ends at 2500 ms and its home pause ends at 4125 ms, the patrol's pause ends at 510 ms and its next plan is another wander, and the noise leaves the roam waiting

### Requirement: Wander point
A walker with no room SHALL not pick. A pick SHALL be one exit, drawn by the roll, and a second draw only when that exit leads back to the room the walker came from. The goal SHALL be a quarter of the way from the exit toward the room point, at height 0, with a stop of 20 cm and a radius of 150 cm, ignoring height. A second draw of the same exit SHALL still be kept.

#### Scenario: A quarter of the way, unless the exit leads back
- **WHEN** a walker with no room is asked to pick, a walker draws the only exit from the origin toward a room point 100 cm ahead, a walker draws an exit 100 cm deep toward a room point at the origin, a walker whose first exit leads back draws the next exit 200 cm ahead, and a walker whose only exit leads back draws again
- **THEN** the walker with no room picks nothing and the roll stays, the ahead goal is 25 cm forward at height 0 with stop 20 cm and radius 150 cm after one step of the roll, the deep goal is 75 cm deep at height 0, the retry uses the second exit at 150 cm and steps the roll twice, and the only exit is kept after two steps

### Requirement: Follow a leader
No leader: stand if there is no activity or panic is at least 1, otherwise run the activity. Not walking, inside (1 + facing toward the follower / 2) times 100 cm, SHALL close; at that distance SHALL not. The band is 400 cm, 280 while following, and 1200 while busy. Outside it, urgency is 0 up to 800 cm, 1 up to 1600 cm, and 2 past 1600 cm or when the leader sprints. More than 100 cm from the planned spot replans; exactly 100 cm does not. The plan stops at 50 cm.

#### Scenario: Facing sets the stop, distance sets the urgency
- **WHEN** a follower with no leader has no activity, another has panic 1, a third has panic 0.99 and an activity, a follower 149 cm in front of a leader facing it is not walking, one at 150 cm is not walking, one at 500 cm accepts a plan, one at 801 cm does not, one at 1601 cm does not, one at 500 cm whose leader sprints does not, a busy follower at 1000 cm is not following, an already-following busy follower at 300 cm is not walking, an already-following walker whose planned spot is 100 cm aside stays, one whose planned spot is 101 cm aside accepts a plan, and a hidden already-following follower is not walking
- **THEN** the first two stand, the third runs the activity, 149 cm closes, 150 cm runs the activity, 500 cm pursues at urgency 0 and stops the plan at 50 cm, 801 cm is urgency 1, 1601 cm and the sprint are urgency 2, the busy follower at 1000 cm runs the activity, the follower at 300 cm pursues at urgency 0, 100 cm does not replan, 101 cm does and remembers the leader, and the hidden follower stands

### Requirement: Sleep
A sleeper SHALL become dormant and set the dormant flag without clearing the others. Its tone and tone target SHALL become 0, it SHALL be a statue, and its gait SHALL be 2. The tick installed afterwards SHALL change nothing. A second sleep SHALL keep that flag and the dormant condition.

#### Scenario: A walking body lies dormant
- **WHEN** a walker whose condition is walking, whose flags are the shock stagger, whose tone is 1 and whose gait is 0 sleeps, its tick runs, and it sleeps again
- **THEN** the condition is dormant, the flags are the stagger and the dormant flag, the tone and the tone target are 0, it is a statue with gait 2, the tick leaves all of that as it was, and the second sleep keeps it

### Requirement: Wake a sleeper
A sleeper whose flags are exactly the dormant flag, and that has seen a contact, SHALL drop role 44 and be knocked out at the current clock. It SHALL be down, keep the dormant flag, gain the waking flag, clear the statue mark, set gait 0, set tone to 0 and the pin gain to 0.08, and raise its sleep countdown to 256. Any other flags, or no contact, SHALL leave it sleeping.

#### Scenario: A contact knocks the sleeper out
- **WHEN** a sleeper with only the dormant flag sees no contact, then sees one at clock 2500, and a sleeper whose flags also include the shock stagger sees a contact
- **THEN** the first stays dormant with role 44 until the contact, which drops role 44, knocks it out at 2500, leaves it down with the dormant and waking flags, clears the statue, sets gait 0, tone 0 and pin gain 0.08, and raises the countdown to 256, and the stagger sleeper stays dormant with role 44

### Requirement: Wake tone
While not waking, a tick SHALL change nothing. Through 1500 ms tone and target SHALL stay 0, the clip base SHALL be 0.4, and the condition SHALL be down with both flags set. At 1501 ms tone SHALL gain 0.001, or 0.002 getting up at phase 0, and the target SHALL stay 0. The dormant flag SHALL clear unless held. At 8000 ms the clip base SHALL stay. At 8001 ms it SHALL gain 0.01, and at 1 tone SHALL be 1, the target the stance factor, and flags SHALL clear unless held.

#### Scenario: Tone stays 0 for 1500 ms, then rises
- **WHEN** a knocked-out body is ticked at 1500 ms, at 1501 ms, and at 1502 ms, a getting-up body at phase 0 is ticked at 1501 ms, one at phase 1 is ticked at 1501 ms, a held body is ticked at 1501 ms, a body is ticked at 8000 ms, a body whose clip base is 0.995 is ticked at 8001 ms, a held body at that clip base is ticked at 8001 ms, and a body that is not waking is ticked at 9000 ms
- **THEN** 1500 ms stays at tone 0 with clip base 0.4 and both flags, 1501 ms is tone 0.001 with the dormant flag cleared and the target still 0, 1502 ms is tone 0.002, phase 0 is tone 0.002, phase 1 is tone 0.001, the held body keeps the dormant flag, 8000 ms does not move the clip base, 0.995 finishes at tone 1 with the stance factor as the target and the flags cleared, the held finish keeps the waking flag, and the body that is not waking is unchanged

### Requirement: Awaken and refresh
The tick that drops the dormant flag SHALL tell each person who can see the body, and nobody else. A later tick SHALL tell nobody. When the clip base reaches 1 the reach SHALL be the greatest weapon reach, or the race reach when no hand bit is set. No weapon SHALL prefer thrust 0. A zero slash, crush and pierce sum SHALL prefer 10. Otherwise the preference SHALL be the rounded thrust share times 5, plus 1, and at most 9.

#### Scenario: One witness hears, then the weapons refresh
- **WHEN** a knocked-out body is ticked at 1501 ms with one person who can see, one person who cannot, and one non-person who can, then ticked again at 1502 ms, and a body whose clip base is 0.995 is ticked at 8001 ms with a 40 cm weapon of equal slash and thrust and an 80 cm weapon, and another such body is ticked with no hand bit and a weapon whose slash, crush and pierce are 0
- **THEN** one witness hears at 1501 ms and none at 1502 ms, the armed finish reaches 80 cm and prefers thrust 6, and the empty-handed finish uses the race reach of 50 cm and prefers 10
