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
A walker with no room or no exits SHALL not pick nor step the roll. A pick SHALL draw one exit, and draw once more (keeping that draw) when it leads to the avoided room. The goal SHALL be a quarter of the way from the door's near cell centre to the centre of the cell behind it, at height 0, stop 20 cm, radius 150 cm, ignoring height; the pick SHALL remember the room behind the exit and its place.

#### Scenario: A quarter of the way, unless the exit leads back
- **WHEN** a walker in room 0 draws its only door, from a near cell at the origin to a cell at 600 cm on x, leading to room 5, the roll starting at 1, and another walker whose first draw leads to the avoided room has a second door leading to room 7 from the origin to −600 cm on x
- **THEN** the first goal is 150 cm on x at height 0, stop 20 cm, radius 150 cm, room 5, place 0, with the roll at 134775814, and the second walker draws again, takes the second door and its goal is −150 cm on x

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

### Requirement: Witness memory
On the tick that drops the dormant flag, each seeing person SHALL update memory, and nobody else. A later tick SHALL leave it. A short count SHALL add one and keep counting. Meeting the need SHALL stamp the seconds plus the delay and set the high bit. A cooldown, a future last use, and never-again SHALL stay. Inside its window an activated stamp SHALL refresh to now, and at the window re-arm with the delay. A future stamp SHALL report updated and stay. Held-off SHALL leave an activated word.

#### Scenario: Who can see stores a word
- **WHEN** a knocked-out body is ticked at 1501 ms at second 100 with a seeing person who needs two hits and whose word is 0, a seeing person already counting 1 with a 2 second delay, a seeing person already counting bit 1 who is witnessed as bit 2, a seeing person whose last use is second 1000, a person who cannot see, and a non-person who can, then ticked at 1502 ms, and a knocked-out body is ticked at second 1009 and another at second 1010 with a ten-second cooldown and a last use of 1000 that needs two hits, and the shipped update is asked for a never-again nibble, an activated stamp one second inside a ten-second window, that stamp at the window with a 2 second delay, a future activated stamp, the held-off flag, and a never-again window at the last second with a 2 second delay
- **THEN** the short count is 1 and still counting, the met need stamps 102 with the high bit, the second bit stamps 100 with the high bit, the last use of 1000 stays, the person who cannot see and the non-person stay, the later tick leaves the count, second 1009 stays at 1000, second 1010 counts 1, never-again stays, the inside stamp refreshes to 109 with the high bit, the window re-arms at 112, the future stamp stays and reports updated, held-off stays, and the last second refreshes to itself with the high bit

### Requirement: Hold and weigh
When the clip base reaches 1 with a held record, that record SHALL be held again. A ghoul's arm stiffness SHALL be 1 and its hands SHALL weigh twice. A Fael's unarmed hands SHALL weigh four times, reach 150 cm, and set arm stiffness to one half.

#### Scenario: A ghoul weighs twice and a Fael weighs four times
- **WHEN** a knocked-out body whose clip base is 0.995 is ticked at 8001 ms holding fists as a ghoul, and another is ticked holding fists as a Fael whose unarmed reach is 150 cm
- **THEN** the ghoul's held balance is 1, its arm stiffness is 1, its hands weigh twice, and its reach is 50 cm, and the Fael's hands weigh four times, its reach is 150 cm, and its arm stiffness is one half

### Requirement: A knock-out is the unconscious state
A knock-out SHALL set the wake clock to the current clock plus the delay, wrapping. Flags SHALL lose bits 0xF0 and gain the dormant and waking flags. The condition SHALL be down, the statue mark SHALL clear, gait SHALL be 0, and tone SHALL be 0. The pin gain bits SHALL be 0x3da3d70a. A sleep countdown below 256 SHALL rise to 256, and one already at 300 SHALL stay. A blow SHALL NOT call this, and health SHALL NOT store a limb impairment.

#### Scenario: Delay 40 at clock 1000
- **WHEN** a statue with gait 2, tone 1, flags 0x80000150 and a sleep countdown of 300 is knocked out at clock 1000 with delay 40, and the same body with a lower countdown is knocked out at the maximum clock with delay 1
- **THEN** the first wakes at 1040, is down, has flags 0x80000130, is not a statue, has gait 0 and tone 0, has pin bits 0x3da3d70a, and keeps countdown 300, and the second wakes at the minimum clock with countdown 256

### Requirement: A dead body is knocked out
A dead body SHALL be knocked out at the current clock with no delay, down, with the dormant and waking flags. A walking body SHALL stay walking.

#### Scenario: Dead at clock 50, walking stays
- **WHEN** a dead body is entered at clock 50, and a walking body is entered at clock 90
- **THEN** the dead body wakes at 50, is down, and has the dormant and waking flags, and the walking body stays walking

### Requirement: A dead entry rewrites parts before the knock-out
A dead entry SHALL set a held part's mark to 0x2001 from 1, its gate to 6 from 15, its span bits to 0x40000000, and its identity turn to 0, 0, 0, 1. An unheld part SHALL keep mark 3 and span 9 and still take gate 6. A negative basis SHALL take turn 0, 0, 1, 0. A link of count 1 SHALL become kind 4 and bits 0x30, and count 0 SHALL stay. The body SHALL then be down at clock 50. A walking body SHALL keep its parts and stay walking.

#### Scenario: Held part, plain part, and a walking body
- **WHEN** a dead body with a held part, an unheld part, a negative basis, a counted link and an empty link is entered at clock 50, and a walking body is entered with those same parts
- **THEN** the held mark is 0x2001, the gate is 6, the span bits are 0x40000000, the identity turn is 0, 0, 0, 1, the unheld mark stays 3 and the span stays 9, the negative turn is 0, 0, 1, 0, the counted link is kind 4 and bits 0x30, the empty link stays, the dead body is down at 50, and the walking body keeps its parts and stays walking

### Requirement: A dead entry sets the race hands and the step
When both pair words are 0, class 0x800 and body kind 0x100 SHALL set the hands to firm and open, leave the bare mark on, clear the class word, and set the primary to -1. Class 0x500 SHALL mark the hands and clear the bare mark. A pair word of 1 SHALL leave the hands. Feral 0 and feral -1 with a base of 1 SHALL score reach bits 0x42a00000, and feral 1 SHALL score bits 0x42600000. Empty slots SHALL fill. An offset of 3, 4, 0 SHALL face along x. A walking body SHALL skip this.

#### Scenario: Firm and open hands, then a marked class
- **WHEN** a dead body with both pair words 0, class 0x800 and body kind 0x100 is entered, then class 0x500 at feral 1, then a pair word of 1 at feral -1, and a walking body is entered
- **THEN** the hands are firm and open with the bare mark on, the class word is 0 and the primary is -1, the reach bits are 0x42a00000, the slots are filled and the facing is along x, class 0x500 is marked with reach bits 0x42600000, the pair word of 1 leaves the hands and scores reach bits 0x42a00000, and the walking body is unchanged

### Requirement: Golem dormancy
The system SHALL scale stored values by 0.05 on fade-in and 0.02 on fade-out, in double precision.

#### Scenario: One scale each
- **WHEN** the source is 20
- **THEN** the fade-in value is 1
- **WHEN** the glow is 50
- **THEN** the fade-out value is 1

### Requirement: Golem homing
The system SHALL compute the homeward delay as distance + π + roll − 0.5 and the reach as 200 + offset.

#### Scenario: A centred roll
- **WHEN** the distance is 10 and the roll is 0.5
- **THEN** the delay is 10 + π and the reach for an offset of 5 is 205

### Requirement: Init assignments
Applying roles SHALL start from Activity 1 and IdleActivity 0 and run each role's init script through the script interpreter, newest role first, so an older role wins; the script SHALL see the two activity slots and the held role's variables (0 for a variable the held role stores none of). Statements the interpreter reports an error for SHALL be returned with the role.

#### Scenario: Older wins
- **WHEN** Activity Wait is newer than Activity Patrol, a role's script adds a concern, and another names an unknown class
- **THEN** the character patrols, the concern statement runs without effect, and the unknown class is reported as "Script Error: Invalid class identifier."

#### Scenario: The shipped init scripts
- **WHEN** every init script of the installed `charroles.rdb` runs with its variables at 0 and at 9
- **THEN** the activities equal the scripts' plain assignments and nothing is reported

### Requirement: Activity to idle state
An active controlling effect SHALL force Activity 4 unless resisted. Activity 1 SHALL roam, 2 wait, 3 sleep, 4 follow (joining the party with a 400 cm band), 5 stand sentry and 7 patrol; 0, 6 and any other SHALL wait. A follower that no longer follows SHALL leave the party.

#### Scenario: Control
- **WHEN** a patrolling character carries a controlling effect, with and without resistance, having followed before
- **THEN** it follows without resistance, and with it patrols and leaves the party

### Requirement: Idle-activity registry
The idle activity SHALL be created from the registry when there is none and replaced when its id changed. The registry SHALL refuse a taken id and hold the search as 6.

#### Scenario: The explorer
- **WHEN** an explorer's Activity variable is 4 with IdleActivity 6, and it later holds no such role
- **THEN** it follows with a 400 cm band and gets the search, which then goes

### Requirement: The wait and the home-side point
The wait SHALL stand after the disturbance and alert checks. The home-side point SHALL be the exit of the current room toward the room the last wander point aimed at, at height 0, mode 3, stop 20 cm and radius 150 cm, ignoring height; no room or no such exit SHALL pick nothing.

#### Scenario: Back to the doorway
- **WHEN** a roam in a room with exits toward rooms 4 and 9 last wandered toward room 9, and a waiting character hears a disturbance of 0.31
- **THEN** the home-side point is the exit toward room 9 with mode 3, and the waiting character walks to the noise

### Requirement: The level-leave reset
When the level is written out, an AI character that is not collapsed, dead or dormant SHALL return its current health to its lasting pool when that pool is over 0.01, and SHALL forget its contacts and target. A roaming AI that is not a statue SHALL also, over two rounds, take a wander endpoint whose path spans more than 150 cm, followed by a home-side endpoint over the same span, and SHALL leave combat. Entering the scene SHALL NOT apply this reset.

#### Scenario: A roam is put down elsewhere
- **WHEN** a roaming AI with lasting health 0.6 and current 0.2 is reset, and the wander plans span 173 cm
- **THEN** its current health is 0.6, its contacts are empty, it leaves combat and it ends at the second wander endpoint

#### Scenario: A collapsed body
- **WHEN** a collapsed AI is reset
- **THEN** nothing changes

### Requirement: The idle object search
The idle driver's object search SHALL run over every object of the room, and the last usable one SHALL become the goal.

#### Scenario: Two usable objects
- **WHEN** a room holds two usable objects and two unusable ones
- **THEN** the later usable object is the goal

### Requirement: Idle attention
An idle state SHALL attend the contact the target pass attends when its notice score `(concern + hostility)·6 + (noticed − attention)` is not under 1 (a NaN score attends). The look SHALL take the yaw to the contact when the wrapped gap from the look passes the window `clamp(1 − 0.1·score, 0.25, 0.7)·π` (50° while the scripted focus holds). Attending SHALL turn the body and put a roam or patrol back in its wait.

#### Scenario: Window
- **WHEN** a contact with score 5 lies 1 rad from the look, then 2 rad
- **THEN** the first leaves the look and the second takes it, both attend, and a score of 0.5 does not attend

### Requirement: Idle head
Every waiting brain SHALL first run the alert look-around (panic at least 0.2 with the body's in-combat mark), then the attention; only when neither takes the tick SHALL it clear the in-combat mark and run its phase. Fighting SHALL set the mark.

#### Scenario: Calm brain
- **WHEN** a waiting brain has panic 0 and nothing attended
- **THEN** its mark is cleared and its roam phase runs

### Requirement: Roam planning in play
A roaming or patrolling brain in the scene, standing, out of combat and not fleeing SHALL plan on its turn of the one-planner-a-frame turn, picking its goal only then. A turn SHALL end the planning phase whether or not the route is followed; a wander route SHALL be followed only past 150 cm end to end and a home-side route always.

#### Scenario: Short route
- **WHEN** a patrol's wander route is 50 cm long
- **THEN** it is not followed, the walk ends on the next step and the patrol pauses 0.5 to 2.5 s

### Requirement: Roam walk in play
The followed route SHALL be walked at the walk speed toward its carrot with the walk clip and SHALL end within 100 cm of its last point; a leg's end SHALL settle the look before the pause is drawn.

#### Scenario: Through the door
- **WHEN** a roaming brain in a room whose only door leads east waits out its deadline and gets its turn
- **THEN** its goal is a quarter of the way through the door, it follows a route there, and after the heading eases it walks east with the walk clip

### Requirement: Entering walker facing
A brain the scene takes back while its walk follows a route SHALL face the route's goal; without a route, or collapsing, it SHALL keep its facing.

#### Scenario: Mid-leg
- **WHEN** a brain following a route ending 500 cm along x enters the scene
- **THEN** it faces +x
