# npc-steering Specification

## Purpose
How an NPC follows a path: the heading and desired velocity it writes, the same fields the player writes.

## Requirements

### Requirement: Path order
Following a path SHALL write a heading and a desired velocity. Along a straight path the heading already faces, the velocity SHALL be the player's walk speed, or the player's jog speed when the urgency is at least 1. A path of one point SHALL leave the velocity at zero.

#### Scenario: Straight walk
- **WHEN** the path runs 8 m forward and the heading already faces it
- **THEN** the velocity is 1.4 m/s along that heading

#### Scenario: Urgent
- **WHEN** the same path is followed with urgency 1
- **THEN** the velocity is 3.6 m/s along that heading

### Requirement: Duellist uses the order
A duellist closing on its target SHALL set its body's desired velocity to the order for the straight path from itself to the target.

#### Scenario: Closing in
- **WHEN** a duellist 8 m from its target has entered combat
- **THEN** its desired velocity equals that order and it plays the combat walk

### Requirement: Door on the path
A walker with no path, a cursor at the end, or no door SHALL keep walking. A door more than 0.7 open SHALL be left alone. A door at 0.7 or below SHALL hold the walker. If that door is not already toggling, the walker SHALL toggle it. A door already toggling SHALL stay as it is.

#### Scenario: A shut hinge holds, an open slider does not
- **WHEN** a walker with no path meets a shut hinge, a walker meets a slider at travel 0.29, a walker meets a shut hinge, a walker meets a slider at travel 0.3, and a walker meets a shut hinge that is already toggling
- **THEN** the first two keep walking and those doors stay as they were, the shut hinge holds the walker and aims open for 100 ticks, the slider at 0.3 holds the walker and opens for 60 ticks, and the hinge already toggling holds the walker without a new aim

### Requirement: Door wait
A walker not at a door SHALL stay as it is, door included. A door that lets the walker through SHALL move it past the door and let the search run. At exactly ten seconds the wait SHALL still hold. One millisecond later the walker SHALL seek again, clear the avoided node, and skip the cursor's low five bits. When both happen, seeking SHALL win and the search SHALL already be allowed. A start near the top of the clock SHALL not give up at a small now. Heading sway is not in this rule.

#### Scenario: Ten seconds still waits, the next millisecond seeks
- **WHEN** a walker already seeking meets a shut hinge one millisecond after ten seconds, a waiting walker meets a shut hinge at cursor 3 at exactly ten seconds and again one millisecond later, a waiting walker meets a slider at travel 0.29 at the start of the wait, that slider is met again at cursor 32 one millisecond after ten seconds, and a wait that started near the top of the clock meets the same slider at a small now
- **THEN** the seeking walker and its hinge stay as they were, the ten-second hinge is toggled open and the wait still holds with the search held and the avoided node kept, the next millisecond seeks with the avoided node cleared and bit 3 skipped while the search stays held and the hinge is not toggled again, the early slider moves the walker past the door and lets the search run without changing the door, the late slider seeks with the search allowed, the avoided node cleared and bit 0 skipped, and the wrapped clock moves past the door without seeking

### Requirement: Span arrival
A walker not approaching a span SHALL stay as it is. While the clock is still at the stamp and the walker is busy, the approach SHALL stay as it is. Otherwise a missing span, or a horizontal distance past 250 cm from the midpoint, SHALL send the walker back to seeking. The first miss SHALL set only the high skipped bit. A later miss SHALL also skip the cursor's low five bits. A distance of 250 cm SHALL start the door wait at the current clock. The aim snap after the choice is not in this rule.

#### Scenario: Two hundred fifty centimetres starts the wait
- **WHEN** a walker already at a door is stepped far past a span, a busy approach at its stamp is stepped past the span, an approach at its stamp that is not busy is stepped at exactly 250 cm, a busy approach one millisecond later meets no span at cursor 3, that approach meets a point 251 cm out again, and an approach meets the midpoint one millisecond later while busy
- **THEN** the walker at the door stays there, the busy approach at its stamp stays there with no skip, the point at 250 cm starts the door wait at that clock, the missing span seeks and sets only the high bit, the later miss also skips bit 3, and the midpoint starts the door wait at the later clock

### Requirement: Wander look
A look that is not wandering SHALL stay as it is. A wandering look SHALL copy the heading and start from the extracted angle. At the clock the offset and the roll SHALL stay, and the look SHALL ease toward the heading. Behind the clock the offset SHALL be the unit times 3.6 minus 1.8, the roll SHALL step, and the next timer SHALL add 4000 plus the high part of that step times 4000. An aim past half a turn SHALL wrap once. Posture 1 SHALL ease by 0.03. The flag call is not in this rule.

#### Scenario: At the clock the offset stays, behind it the offset is drawn
- **WHEN** a look that is not wandering is stepped behind its timer, a wandering look at heading 0 is stepped at its clock from extracted angle 1, the same look is stepped one millisecond later from unit 1, and a heading of 4 at posture 1 is stepped at its clock
- **THEN** the other look stays as it was, the on-time look is 0.91 with its offset and roll kept, the late look has offset 1.8, roll 134775814, timer 5126 and look 0.162, and the heading of 4 eases to about −0.0685

### Requirement: Aim snap
An aim snap SHALL copy the body's place and set the heading, the stored heading and the base heading to the yaw of the body's heading, ignoring its height. The roll SHALL step. The timer SHALL be that step's high part times 3000, plus the clock, plus 1000. The pace and the marker SHALL clear. The stance SHALL become idle: the old clip starts Idle over 30 frames, another clip stays, and Idle playing does not restart. A wander look at that clock SHALL keep its offset.

#### Scenario: A defence clip becomes idle, a walk does not
- **WHEN** a defence stance playing its own clip is snapped from a heading whose height is 9 and whose forward part is 1, at clock 2000 with roll 1, and a wander look is then stepped at that clock, and a defence stance playing a walk is snapped from a heading along the side, and an idle stance already playing Idle is snapped from a heading straight back
- **THEN** the place is copied, the yaw is 0, the roll is 134775814, the timer is 3094, the pace and the marker are clear, Idle plays over 30 frames, the look keeps its offset, the side heading's yaw is about 1.5708 and the walk clip stays, and the backward yaw is about 3.1416 with Idle left as it was

### Requirement: Start a long path
A path of fewer than two points SHALL NOT be followed, even when forced. Two points SHALL be followed when forced, or when the first and last are more than 150 cm apart. Exactly 150 cm SHALL NOT be followed.

#### Scenario: Exactly 150 cm stays, one centimetre more follows
- **WHEN** an empty path is forced, a single point is forced, two points 150 cm apart are not forced, two points 151 cm apart are not forced, and two points on top of each other are forced
- **THEN** the empty path and the single point are not followed, the 150 cm path is not followed, and the 151 cm path and the forced path are followed

### Requirement: Path carrot
A ground point SHALL project onto the nearest path segment in 3-D and keep the unclamped fraction. The goal SHALL be 100 cm further along that path, measured from the unclamped foot. An equal distance SHALL keep the earlier segment. A segment with no horizontal length SHALL be skipped. Exactly 100 cm left SHALL spill onto the next segment, and a step of no length SHALL stop the route.

#### Scenario: One hundred centimetres past the foot
- **WHEN** a ground point 100 cm along a 150 cm segment, 20 cm off it, is projected onto a path that then turns
- **THEN** its fraction is two thirds and the goal lies 50 cm along the next segment

### Requirement: Out of scene
Out of the scene a step SHALL advance 2 cm along the segment. The standing height SHALL add the pelvis base, and the ground point SHALL keep the path height. A length not strictly past the fraction times the length plus 2 SHALL cross once: with `a = 1 − (1 − f)·L`, the new fraction SHALL be `a − (f·a + 2)` over the next segment's flat length. No horizontal length SHALL count as 0.1.

#### Scenario: Two centimetres, then a crossing
- **WHEN** a body out of the scene steps twice from the start of a 3 cm segment with a 400 cm segment after it
- **THEN** the first fraction is two thirds, and the second step crosses onto the next segment at the original's sum

### Requirement: Run and stop
Urgency 0 SHALL set the run bit until the carrot fraction, as a double, is past 0.99 on the last segment; past that the run bit SHALL clear and a ground point under 30 cm of the carrot (3-D) SHALL soft-stop. Urgency above 0 SHALL clear the run bit and soft-stop under 100 cm of the last point (3-D).

#### Scenario: The run bit clears, then a close body stops
- **WHEN** a walk at urgency 0 stands 120, 104 and 20 cm short of its route's end, and a hurried walk 101 and 99 cm short
- **THEN** 120 cm keeps the run bit, 104 cm clears it without stopping, 20 cm soft-stops, 101 cm goes on and 99 cm soft-stops with the goal pulled to 20 cm ahead

### Requirement: Line replan
In the scene, a blocked line from the ground to the carrot SHALL clear the run bit and ask for a re-plan, and SHALL leave the carrot. A clear line SHALL leave the run bit and the re-plan flag. Out of the scene, and a higher scene bit alone, SHALL not walk.

#### Scenario: A wall asks for a re-plan
- **WHEN** an open line is walked in the scene with the run bit set, the same line with a floor of 40 at cell 4 is walked in the scene, out of the scene, and with only a higher scene bit while a re-plan was already asked
- **THEN** the open line keeps the run bit and does not re-plan, the walled line clears the run bit, asks for a re-plan, and keeps the carrot, the out-of-scene line keeps the run bit and does not re-plan, and the higher bit keeps the run bit and the re-plan that was already asked

### Requirement: The plan turn
Only one brain SHALL plan a route a body tick: another brain asking SHALL record its wait (ticks since its last plan) when it is longer than the longest so far, and the longest waiter SHALL plan next. A plan SHALL be followed when the route has at least two points and is forced or its ends lie more than 150 cm apart across the ground.

#### Scenario: Waits and spans
- **WHEN** two brains ask on ticks 1 and 2 and a third asks with an equal wait; and a route 5.1 m long and one 30 cm long are planned
- **THEN** the second brain plans next, the long route is followed and the short one only when forced

### Requirement: Presence in the scene
An AI character's presence SHALL be checked each body tick unless its condition carries bit 0x800 or its state flags carry 0x10. After a change it SHALL hold 80 ticks. Out of the activation range a present character SHALL leave the scene, and in range a dormant one SHALL enter it. A body out of the scene SHALL not be solved, ticked or drawn.

#### Scenario: A far character leaves and comes back reset
- **WHEN** a casting duellist stands 300 m from the centre of a 200 m area, and after it has gone the centre moves onto it
- **THEN** it leaves on the first tick and is out of the scene and hidden on the next; it stays dormant 80 ticks, then enters with its condition normal and an entry message, and is present and in the scene a tick later

### Requirement: Entering resets the body
A character entering the scene SHALL have its condition set normal, take its stance, and have its cast hook asked event −1 and cleared. Its body SHALL stop: every velocity zero; balance, tone and energy 1; the stepping episode, blow sag and posture wait cleared; the locomotion's moving, step, flags, free walk, goal and look cleared; the locomotor's speed, run and drives zeroed, its facing kept.

#### Scenario: A tired walker enters
- **WHEN** a dormant duellist with tone 0.2, energy 0.3, balance 0.4, moving, stepping, flags 0x4, a free walk, speed 3 and run 0.5 enters
- **THEN** tone, energy and balance are 1, it is not moving and has no step, its flags are 0, the free walk is off, and its speed, run, drive and going are 0

### Requirement: A hidden character
While an AI character is not present, its AI SHALL still run its combat state, choose its stance and start attacks, but its body SHALL neither turn nor move under it, and it SHALL advise, start and release no power.

#### Scenario: A hidden duellist
- **WHEN** a duellist out of the scene faces its target within reach
- **THEN** its combat state still decides, its body stays where it was, and no power starts

### Requirement: The walk driver's arithmetic
The walk SHALL scale the flat goal offset by its reciprocal length, use the facing at length 1 when the squared length is 1 cm² or less or NaN, form the short facing threshold as length·0.007 in double, take the near branch when the threshold is not below the facing dot (NaN included), and, with no foe looked at, zero the head offset's target before the offset eases 0.02 toward it.

#### Scenario: A NaN goal
- **WHEN** the step goal has a NaN coordinate
- **THEN** the walk moves without running or steering

### Requirement: The alert look-around's draws
With the body's in-combat mark set and a panic not under 0.2 (a NaN passes), the alert look SHALL step the generator for the wide pan (one in 300) and the narrow pan (one in 75) every tick before reading their timers; the wide pan spreads an f32 draw over ±π, the narrow one spreads the centred f32 draw times the double π. With a foe more than 45° (in double) from the look, the look SHALL take the foe's yaw, and the body turn SHALL run either way.

#### Scenario: The narrow draw is made with its timer pending
- **WHEN** the wide pan draws on a tick whose narrow timer has not passed, from a generator at 0
- **THEN** the generator ends three steps on, at 3698175007

### Requirement: The body in the way
The body-in-way test SHALL skip dead bodies, the body already remembered as in the way and the carried body, shift each other body 20 cm along the walk direction and return the first the steer line along that direction reaches.

#### Scenario: The remembered body is skipped
- **WHEN** bodies stand 100 cm and 150 cm ahead and the first is remembered
- **THEN** the second is returned

### Requirement: Look writes on leaving and settling
Leaving combat SHALL set the look, the heading and the eased offset to the body's look yaw and keep the head look, its target and the sway timer. The settle after a room walk SHALL set the look, the heading and the head look to the yaw of the body's heading direction, keep the eased offset and draw the next sway 1–4 s out. The aim snap SHALL copy the body's facing vector into the body's look, look smoothing and heading directions, and take its heading as the yaws.

#### Scenario: Leaving combat
- **WHEN** combat is left with the body's look yaw 0.75
- **THEN** the look, heading and offset are 0.75 and the head look and the sway timer are unchanged

#### Scenario: The snap copies the facing
- **WHEN** the aim snap takes a body facing along yaw −0.5
- **THEN** the body's look, smoothing and heading directions are that facing vector and the AI's yaws its heading

### Requirement: The idle room walk
The room walk SHALL mark the current cell visited; the candidates are the exits not leading to the previous cell nor to an exhausted cell. With no unexcluded candidate and the current cell not exhausted it SHALL mark the cell exhausted when there is no candidate at all, clear the exclusion word and go back through the previous cell's exit. Otherwise it SHALL draw among every unvisited unexcluded exit (the previous cell's included), or among the candidates when none is unvisited.

#### Scenario: Only the way back is unvisited
- **WHEN** both new cells beside the current one are visited and the previous cell is not
- **THEN** the exit to the previous cell is drawn

### Requirement: The idle driver's walk and use states
The driver's walk state SHALL walk while the clock is not past its deadline; failing the exit test, the first avoided exit SHALL set the marker bit and later ones their exit's bit. The use state's timeout SHALL fire when the arrival time plus 10000 ms, summed without a wrap, is below the clock, even on the tick the use completes, and SHALL set the exit's own bit.

#### Scenario: The use completes as the timeout passes
- **WHEN** the use completes 10001 ms after the arrival with no exit avoided yet
- **THEN** the driver plans again and the exclusion word holds the exit's bit only

### Requirement: The investigation's gates
The investigation SHALL compare the gap to the kept value with 0.02 and the disturbance with 0.3 in double precision; the same cell within the gap SHALL only stamp a value over 0.5, and a changed cell or value over 0.3 SHALL be kept and walked to.

#### Scenario: The f32 0.3 walks
- **WHEN** a changed disturbance of the f32 0.3 is sampled
- **THEN** the investigation walks to it

### Requirement: The entering body's placement
A collapsed entering body SHALL keep its stored placement. Any other SHALL take its current point and keep its stored facing, or, while following a path, face its path goal from its reference point on the ground plane; a goal straight above or below keeps the stored facing.

#### Scenario: A walking body enters
- **WHEN** a body following a path to a goal 300 cm across and 400 cm along from its reference enters
- **THEN** it faces (0.6, 0, 0.8) at its current point

### Requirement: The grabbed flag's timeout
The AI tick's grabbed flag SHALL stay set while the body reports the grabbed condition, unless the brain is not in the combat state and the body's hook answers event 0x10 under zero, which SHALL release the grip and clear the flag. Outside that condition the flag SHALL clear when the grab time plus 500 ms, summed without a wrap, is below the clock.

#### Scenario: Exactly 500 ms
- **WHEN** a grab began at 1000 ms and the clock reads 1500 ms outside the grabbed condition
- **THEN** the flag holds, and at 1501 ms it clears

### Requirement: The investigation's goal
The investigation's goal callback SHALL take the pathfinder's nearest walkable point to the body as the goal, with motion mode 20 and a 100 cm radius.

#### Scenario: A walkable point
- **WHEN** the pathfinder answers (10, −5, 2)
- **THEN** the goal is (10, −5, 2) with mode 20 and radius 100 cm

### Requirement: The AI walks on its locomotion
A character whose body has muscles SHALL walk its route through the AI's one walk, which writes its body's locomotion request each tick, and its character SHALL take no walk velocity; its body's movement decision SHALL turn and walk it on its feet. A body without muscles SHALL be steered toward the follower's goal by the stand-in, with its walk clip.

#### Scenario: A roaming NPC
- **WHEN** a roaming NPC with muscles follows a route
- **THEN** its request moves toward the route's carrot and its body walks with its feet

### Requirement: The driver's request
While the body steps the walk SHALL write nothing. Otherwise it SHALL ease the heading 0.1 toward the goal's direction, ask only to move (moving past half the gate) while the pelvis's alignment with the goal is not over the gate (0.7 past 100 cm, else `0.007·|d|` in double) or the route is no longer followed, and otherwise put the goal 150 cm out, move, and run at urgency above 0 when the grid's cost at the goal is not over 0.5 and the alignment passes 0.9, with the sprint latch at urgency 2.

#### Scenario: Urgency on cheap ground
- **WHEN** a route runs 10 m ahead at urgencies 0, 1 and 2 with no grid
- **THEN** the goal is 150 cm out and the flags are move, move and run, and move, run and sprint

### Requirement: The soft stop
A soft stop SHALL end the route, clear the run bit, set the stopping bit, pull a goal farther than `8·speed + 20` cm to that distance along the frame's forward, and walk the line from the ground point to the goal through the grid, which clamps it where the grid blocks.

#### Scenario: A far goal
- **WHEN** a route soft-stops with the goal 100 cm ahead at 0.5 cm a tick
- **THEN** the goal is 24 cm ahead and the stopping bit is set

### Requirement: The out-of-scene slide
While a character is out of the scene and follows a route, whichever state planned it, its standing point SHALL slide 2 cm a tick along the route with no physics, and the route SHALL end by the follower's own stops.

#### Scenario: A walker leaves the range
- **WHEN** a walker leaves the activation range mid-route
- **THEN** its standing point moves 2 cm a tick along the route until the route ends

### Requirement: The re-entry placement
When the scene takes a character back, its clip SHALL snap to its stance and its balance radius base SHALL be 60 cm with its counter 0, its standing point SHALL move to the nearest walkable point, and its body SHALL be placed on the ground there facing its body's yaw, or its route's goal while it follows one, its frame taken afresh and its yaw, heading and look set to the placed yaw. A standing character's turn SHALL go through its heading at the body's turn rates.

#### Scenario: A slid walker comes back
- **WHEN** a slid walker re-enters the range
- **THEN** its body stands at the slid point and does not move back to where it left

### Requirement: The driver stops for a body in the way
After the driver writes its request, a live nearby character that the walk reaches within its 120 cm steer line (its point moved 20 cm along the walk) SHALL stop the walk and end the route, once until the next planning turn.

#### Scenario: A character ahead
- **WHEN** a character stands 1 m ahead on the walk
- **THEN** the walk soft-stops and the route ends; the next check skips it

### Requirement: One AI state drives the walk
When combat, a flight or the scene takes a roaming character's body, a leg of its idle walk that is followed SHALL end with the soft stop once; otherwise the idle walk SHALL NOT touch the character's route, so a fighter's chase keeps its own.

#### Scenario: A fighter chasing
- **WHEN** an engaged fighter's driver is driving and its route is not the roam's
- **THEN** after the idle walk's tick the driver still drives and the stopping bit is not set

### Requirement: The AI reads body directions through the engine
The AI's look, turn, snap, settle, leave and search routines SHALL take the yaw of a body direction by the engine's fast heading, build a direction from a yaw by its fast sine and cosine, and take the idle look's yaw by its fast two-argument arctangent of the look's x and z (the heading while a look target still runs).

#### Scenario: A body facing one radian
- **WHEN** the AI reads a body facing along yaw 1.0
- **THEN** it reads the engine's heading of that direction, within 5e-4 of 1.0, and a direction it builds from a yaw equals the engine's direction of that yaw

### Requirement: A standing body keeps its facing
The stand-in that turns an NPC's model to the AI's eased yaw SHALL remember that yaw, and the facing the AI reads SHALL be that yaw while the model keeps the rotation the stand-in wrote, and the engine's heading of the model's forward once anything else moved it.

#### Scenario: An idle brain stands for 200 ticks
- **WHEN** an idle brain placed facing yaw 1.0, with no sway, ticks 201 times
- **THEN** its model faces the engine's heading of the placed facing after the first tick and the same yaw, within 1e-6, after the last

### Requirement: One route per character
A character SHALL keep one route: its points, the follower's segment and fraction, whether it is followed, whether a plan is wanted, the walk's urgency and the after-route count. Every state that walks SHALL plan into that route, and a plan the start test refuses SHALL stop it.

#### Scenario: A roam plans its leg
- **WHEN** a roaming character plans a leg on its turn
- **THEN** its route holds the leg's points, is followed at urgency 0 and belongs to the roam

### Requirement: The walk follows the state
The AI's walk SHALL run once a body tick after every state of the brains' tick, for each character whose route is followed, and SHALL not run for a character whose route is not followed.

#### Scenario: The leg is walked in the tick it is planned
- **WHEN** a roaming character on its locomotion plans its leg facing the route
- **THEN** in the same tick its request's goal lies 150 cm out along the follower's carrot, its flags ask only to move, it moves, and the AI's look yaw is the eased heading's

### Requirement: The walk writes its flags after the follower
The walk SHALL write the request's flags after the follower ran, so a soft stop the follower made at the route's end SHALL leave the request asking only to move toward the pulled-back goal, the route not followed and the after-route count at 80.

#### Scenario: The leg's end
- **WHEN** the roaming character stands on its route's last point
- **THEN** its route is no longer followed, the after-route count is 80, its request asks only to move toward that point and does not move, and the roam settles on the next tick

### Requirement: The after-route count
A walk asked while no route is followed SHALL answer 2 and count down while the after-route count is above 0, and SHALL answer 0 once it is 0.

#### Scenario: Counting down
- **WHEN** the walk is asked twice after its route ended, then with the count at 0
- **THEN** it answers 2 with the count 79, then 0

### Requirement: The head looks at the look target
While walking, the head SHALL look at the AI's look target when it lies within a quarter turn of the look (the offset easing 0.02 toward its yaw), and otherwise SHALL ease its offset 0.02 toward 0 in double; the cursor SHALL be the look yaw plus the offset, wrapped once.

#### Scenario: A target in front, then none
- **WHEN** a walk looks at a point 45° to the side, then walks with no target
- **THEN** the offset's target is that yaw and the offset 0.02 of it, then the target is 0 and the offset eases back 0.02 of the way

### Requirement: The follower asks no end test
The path follower SHALL end a route only by its own stops (fewer than two points, a step of no length, the soft stops at its end); the goal record's end test SHALL be asked only by the planner.

#### Scenario: A route's end within 150 cm
- **WHEN** a roam's route, whose goal record holds the flat test at 150 cm, is walked to 120 cm from its last point
- **THEN** the route is still followed

### Requirement: A downed body stops its route
The AI tick SHALL stop the route of a character whose condition has the down bit, whichever state planned it.

#### Scenario: Knocked down mid-leg
- **WHEN** a roaming character is knocked down while its leg is followed
- **THEN** its route is no longer followed

### Requirement: The walk's mode word and its passed body
The route SHALL keep the walk's mode word (the follow's 4 while following its leader, 6 while closing on it), the leader's point the follow planned from and the body the walk lets pass; the route stop and the soft stop SHALL clear the mode word, and the in-the-way test SHALL skip the passed body. The follow state SHALL read and write its mode and planned point there and SHALL keep its route while the combat state fights.

#### Scenario: A stop clears the mode
- **WHEN** a follower's walk to its leader is soft-stopped
- **THEN** its route is not followed and its mode word is 0

### Requirement: The head looks at the attended contact
The AI's look target the walk's head look reads SHALL be the contact the brain's target pass attends; a walker without a brain SHALL keep the character's own look target.

#### Scenario: Attending, then not
- **WHEN** a walking brain attends a character 45° to its right, and another attends no one
- **THEN** the first head's offset target is that yaw and the second's 0

### Requirement: Jolt soft stop
An AI body whose last balance tick was jolted while it moved SHALL be soft-stopped at the next AI tick, before the combat states: its route ends and its request takes the stopping bit.

#### Scenario: A jolted walker
- **WHEN** a route-following body's balance tick reports a jolt
- **THEN** at the next AI tick its route no longer follows and its request is stopping
