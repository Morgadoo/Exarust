# npc-perception Specification

## Purpose
How an NPC rates one other character and chooses to notice, watch, engage or flee.

## Requirements

### Requirement: Hostility
With neutral traits, hostility toward a negative attitude SHALL settle near 0.0375 times that attitude's size. An attitude of −100 SHALL engage. An attitude of −20 SHALL only be noticed. Tolerance 100 SHALL keep the −100 contact from engaging.

#### Scenario: Demon and zombie
- **WHEN** hostility has settled for attitudes −100 and −20, and the −100 contact is given tolerance 100
- **THEN** the first engages, the second is noticed, and the tolerant one is noticed

### Requirement: Choice
Concern above 1, with no heat past the threshold, SHALL watch. Stress above 3 and alarm above 0.75, with a flee route, SHALL flee.

#### Scenario: Watch or flee
- **WHEN** concern is 1.5, then stress is 4 and alarm is 0.8 with a flee route
- **THEN** the first choice is watch and the second is flee

### Requirement: Range and view
Closeness at the observer SHALL be 0.99. It SHALL be 0 at 12 m and beyond, unless the contact is already tracked, which extends the range to 18 m. Asleep detection SHALL be 6 m. A hidden contact in no light SHALL be seen only to 1.2 m. The view weight SHALL be 1 straight ahead and 0 straight behind.

#### Scenario: Ahead and out of range
- **WHEN** the other stands on the observer, then 12 m away, then straight behind
- **THEN** closeness is 0.99, then 0, and the view weight is 1 ahead and 0 behind

### Requirement: The duellist engages
A level character's duellist SHALL take the guard when its brain's target pass enters combat against the duellist's target, and SHALL leave the fight when a pass calms down while it fights. The mind's own rating SHALL NOT start the fight.

#### Scenario: Hostile contact
- **WHEN** a brain whose roles hate the player sees the player two metres away for 700 frames
- **THEN** its target pass enters combat and its duellist is engaged

#### Scenario: Neutral role
- **WHEN** a brain with no hostile role sees the player as long
- **THEN** its pass stays at level 0 and its duellist waits

### Requirement: Personality from the record
The skill share SHALL be the combat skill over 100, capped at 1. With t1 and t2 the second and third traits, the confidence SHALL be t1/100, the restraint −t2/100, the courage (confidence + 1)/2, the eagerness ((3 − restraint) + (3 + confidence))/8, and the blend `(1 − h) + h·((confidence + 2)·0.333)` with h = (1 − feral)², or 1 marked; the shares and the blend are formed in double. The jitter SHALL be the square of a draw over −1..1; the arousal SHALL start at 10.

#### Scenario: A half-feral record
- **WHEN** a record of combat skill 150, traits 20 and −30 and feral 0.5 is read with the generator at 0
- **THEN** the skill is 1, the confidence 0.2, the restraint 0.3, the courage 0.6, the eagerness 5.9/8, the blend 0.75 + 0.25·2.2·0.333 and the jitter about 1

#### Scenario: Shares in double
- **WHEN** a record of combat skill 5 and traits 5 and −5 is read
- **THEN** the skill, confidence and restraint are the f32 nearest 0.05 (bits `0x3d4ccccd`)

### Requirement: Attaching the brain
Without the race word's AI flag no contact sense SHALL be installed. With it, the sense SHALL be the standard one for race class `0x12xx` and the ally sense otherwise; a dead character SHALL get the effect at 0 and its roles SHALL NOT be applied; a statue race SHALL stand sentry with the effect at 0; any other SHALL get the effect at 1.

#### Scenario: Ally, dead and statue
- **WHEN** brains are attached to a class-`0x12xx` body, a class-`0x04xx` body, a dead body and a statue body, all with the AI flag
- **THEN** the first has the standard sense and the effect at 1, the second the ally sense, the third the effect at 0 and no roles applied, and the fourth is a statue with the effect at 0

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

### Requirement: The stress flight
The stress SHALL be the stress base plus the stress less the courage, at least 0. With stress above 3, an alarm above 0.75 (0 for the contact already fled, its alarm raised to the one it fled at) and a way out, the contact SHALL be fled, scored by its alarm, before any other test.

#### Scenario: Stress 4 and alarm 0.8
- **WHEN** an alarm of 0.8 at stress 4 is classified with and without a way out, and the fled contact's alarm of 0.1 with the flight's alarm 2
- **THEN** the first flees scored 0.8, the second is noticed, and the fled contact keeps the flight scored 2

### Requirement: The engagement threshold
The threshold SHALL be `7b² + (2 + b)` for a tolerance b above 0 and `2 + b` otherwise; concern plus hostility above the threshold plus the stress, or above the threshold with an alarm above 1.5, SHALL engage at level 3, or 2 for an unreachable contact, scored by concern plus hostility.

#### Scenario: Thresholds
- **WHEN** concern 1 and hostility 1.5 are classified at tolerance 0, the same contact unreachable, concern 1 and hostility 3 at tolerance 0.5, then hostility 3.3, and hostility 1.6 at tolerance −0.5
- **THEN** they engage at level 3 scored 2.5, engage at level 2, are noticed, engage at level 3, and engage

### Requirement: Watching and noticing
Short of an engagement, a concern above 1 SHALL flee when it passes 8 less the stress and a way out exists, and SHALL watch, scored by the concern, when not; anything else SHALL be noticed with the score `6·(concern + hostility) + noticed − attention`, 3 more for the contact already attended.

#### Scenario: Concern 4.5 and a notice
- **WHEN** a concern of 4.5 at tolerance 1 is classified at stress 4 and at stress 0, and an attended contact of concern 0.5, hostility 0.25, noticed 1 and attention 0.25
- **THEN** the first flees, the second watches scored 4.5, and the notice scores 0.75·6 + 0.75 + 3

### Requirement: The contact list
A brain SHALL keep twelve contacts at most; with all twelve taken a new contact SHALL replace the one of least weight (ten times its sighting plus its concern, the first contact's sighting by size) unless it weighs less itself.

#### Scenario: A full list
- **WHEN** a full list of contacts weighing 5, one weighing 1, meets contacts weighing 2 and 0.5
- **THEN** the first replaces the light one and the second is not kept

### Requirement: The target pass's choice
The pass SHALL start from the foe it flees (at the alarm it fled at), or the foe it fights while concern plus hostility pass the raw stress plus 2. A higher level SHALL win, its current pick SHALL be renewed at any level, and at the same level the higher score SHALL win (the notice score kept apart at level 0). A flight SHALL flee the pick; nothing above a notice SHALL calm the brain toward the best noticed; anything else SHALL fight at the watch, or at level 3 for levels 2 and up.

#### Scenario: Notices, watches and a flight
- **WHEN** notices scored 2 and 3, watches scored 1 and 1.5, an engagement at level 2 and a flight are weighed in turn
- **THEN** the pass calms toward the notice scored 3, fights the watch scored 1.5, fights at level 3, and flees

### Requirement: The alarm mean and waking
The pass's alarm mean, its sum over the root of its count, doubled, SHALL raise the stress; a sleeper that finds anything above a notice SHALL wake.

#### Scenario: Alarm sum 3 over 4
- **WHEN** an alarm sum of 3 over 4 meets a stress of 1, and a sleeper finds a watch
- **THEN** the stress becomes 3 and the sleeper wakes

### Requirement: Perception fills the brain's contacts
Once a body tick the brain on its target-pass turn SHALL clear its contacts' concern targets, closeness, view and noticed flag, then perceive, through the sense it holds, every character within the largest perception reach that the level's sight test lets it see and every character it already knows, and SHALL write the perception's sighting, closeness, attitude, threat and concern target into its contact for that character. A second frame on the same body tick SHALL perceive nothing.

#### Scenario: Visible and unseen
- **WHEN** the player stands two metres ahead, then is made unseen
- **THEN** the contact's closeness is 1 − 200/1200, then its sighting and closeness are 0

#### Scenario: Once a tick
- **WHEN** a contact's noticeability is cleared after a perception and the frame repeats on the same body tick, then the tick advances
- **THEN** it stays 0 on the same tick and is written again on the next

#### Scenario: One brain a tick
- **WHEN** two brains face the player and the turn rotates each tick
- **THEN** neither perceives on the first tick, one of them on the second and the other on the third

### Requirement: Dead, knocked-out and lost characters
A dead, asleep or removed character's concern and hostility SHALL be cleared on the brain; only a knocked-out character SHALL be skipped by the calming. A tracked target that moves out of reach SHALL raise the brain's alert to the lost floor.

#### Scenario: Dead and downed
- **WHEN** a contact with concern 3 and hostility 5 is dead, or only down
- **THEN** both are cleared for the dead one and kept for the downed one

#### Scenario: Lost target
- **WHEN** a tracked target moves 25 m away
- **THEN** the contact is no longer tracked and the alert is 40

### Requirement: The AI tick
Once a body tick after the perception each brain SHALL decay its group alert, stop there when its body is out of the fight (condition bit `0x800`), and otherwise drift its morale and integrate its contacts; on its target-pass turn it SHALL also run the target pass over every character seen this tick, each contact's way out answered from the brain's room, exits and fled rooms. It SHALL keep the report and copy its concern and hostility to the combatant's contacts.

#### Scenario: One pass a tick
- **WHEN** a brain holds a seen contact with hostility 5 for the player
- **THEN** one tick's pass enters combat at level 3 and attends the player, and a cooled contact the next tick leaves it at level 0

### Requirement: Calming from the perceived attitude
The integration SHALL raise a seen contact's hostility by `(1 − decay)·attitude·(1 − restraint)·5e-5·scale` subtracted, only when the perceived attitude is below zero.

#### Scenario: Hostile and friendly
- **WHEN** a seen contact's attitude is −20, then +20
- **THEN** its hostility rises above its 0.999 decay, then only decays

### Requirement: The AI tick's room cell and alert
Every frame each brain SHALL link itself into the 600 cm room cell of its body: the floored position over 600 cm on each ground axis, offset by half the grid. Off the grid its link SHALL stay unchanged. The group alert SHALL decay one step every 10 s, down to 1.

#### Scenario: Off the grid
- **WHEN** a brain in a 4 × 4 grid moves 100 m along x
- **THEN** it stays linked in its old cell

### Requirement: The idle turn
While not engaged, a brain's model SHALL turn to the out-of-combat eased yaw. The yaw starts at the body's facing and eases toward the look plus a sway by `0.09 − 0.06·encumbrance`. The sway is `u·3.6 − 1.8`, redrawn every 4000 ms plus up to 4000 ms.

#### Scenario: Easing toward the look
- **WHEN** an idle brain faces yaw 0 and looks along π/2 with no sway and no encumbrance
- **THEN** its model turns to 0.09·π/2 that frame

### Requirement: The ally-class sense range and fellows
The ally-class sense SHALL raise a concern between 1 and 3 to 3. It SHALL track a character within 1200 cm, or 1800 cm while tracked, ignoring height differences under 150 cm. A fellow AI character SHALL get attitude 100 and no concern.

#### Scenario: A fellow AI
- **WHEN** the observed character's race has the AI flag
- **THEN** its attitude is 100, its concern 0, and the sense returns −1

### Requirement: The ally-class threat
With a concern of at least 1, the ally sense SHALL take the largest threat to an ally contact. That threat is the contact's weight `(view·0.75 + 0.25)·closeness²` times the observed state's factor (4 attacking, 0.25 with the observed body's in-combat mark or 0.1, halved when down) times 1.2. When that threat is above the concern, the concern target SHALL become 0.3 of it.

#### Scenario: An attacker near an ally
- **WHEN** the observed character attacks, the concern is 1.5 and an ally contact has closeness 0.9 and view 1
- **THEN** the concern becomes 3 and the concern target 0.3·(0.81·4·1.2)

#### Scenario: The in-combat mark
- **WHEN** the observed character stands in its combat stance without attacking
- **THEN** its state factor is 0.25, and 0.1 out of the stance

### Requirement: The sight test
A brain SHALL see a character in a level without a grid always; in a grid level when both stand in the same room, or when a door between their rooms, open in the room tracker, has its point (¾ of the way into the cell behind it) within 2 m of the other character. A position in no room SHALL be taken as seen.

#### Scenario: Through a door
- **WHEN** a brain in a two-room grid looks at a character in its own room, one 1.5 m from the door's point behind it, one two cells past the door, and the first again with the door shut
- **THEN** it sees the first two, not the third, and not the second once the door is shut

### Requirement: The level room
The AI tick SHALL read the brain's level room from the level's cell-to-room table. A cell without a room SHALL set the room to −1 and keep the previous room; a new room SHALL make the old one the previous room only when the old one was a room.

#### Scenario: Through a roomless cell
- **WHEN** a brain enters room 2, a roomless cell, room 5 and room 3
- **THEN** the previous room stays −1 until room 3, which remembers 5

### Requirement: The out-of-combat look chain
Out of combat, a body stepping or attacking SHALL copy its look into its look smoothing and heading. Otherwise the look yaw SHALL turn a tenth of the gap to the eased offset (held within a quarter turn), the pitch ease 1 % toward the target pitch, the smoothing ease from the facing by 0.4 of the gap (0.64 of its sign past 1.6 rad), and the gait helper SHALL rewrite the body's heading when it lags the eased look by more than 30°.

#### Scenario: A tenth and 0.4
- **WHEN** a body looking along yaw 0 with facing 0 has an eased offset 0.3 and a target pitch 0.5
- **THEN** the look yaw becomes 0.03, the pitch 0.005 and the smoothing yaw 0.012

### Requirement: Set target
Taking a character target SHALL mark the mind fresh, set its commitment to 1, clear its overshoot mean and stuck count, place the estimate on the target, and measure the bearing from the chest as the flat length of the offset with the whole offset divided by it as the direction; the aim SHALL take the direction's yaw. No height rule and no 1 cm floor SHALL apply.

#### Scenario: A target high above
- **WHEN** a target stands 500 cm up and 30 cm ahead of the chest
- **THEN** the distance is 30 cm and the aim is straight ahead

### Requirement: Familiarity drift
The perception's familiarity drift SHALL be added to the observer's relation with the character it saw only when that relation exists.

#### Scenario: A stranger
- **WHEN** a brain sees a character it has no relation with
- **THEN** no relation is made for it

### Requirement: Footprint gate
The AI tick's footprint gate SHALL register the footprint unless the character follows a route, its locomotion is active and its speed is at least 1 cm a tick; a NaN speed SHALL skip the stamp.

#### Scenario: A walker on a route
- **WHEN** a character follows a route at 250 cm a tick
- **THEN** the gate does not stamp

### Requirement: Combat entry and leave
A brain entering combat SHALL set its body's in-combat word and its duellist SHALL take the Defence stance through the set-stance helper. Leaving through the target pass SHALL give the brain's look the body's look yaw and draw the next sway 500 ms out.

#### Scenario: An engagement
- **WHEN** a hostile brain's target pass enters combat
- **THEN** its combatant is in combat and its duellist is engaged

### Requirement: A duellist's foe is its target's fighter
A brain-bearing duellist aimed at a character SHALL take as its foe the fighter of that character (the aimed entity when it fights, else the fighter it carries), so the perception's contact, the target pass, the entry into combat and the combat state's contact all name that fighter.

#### Scenario: Aimed at the player's root
- **WHEN** a brain whose roles rate the player hostile duels the player's root, whose child is the player's fighter
- **THEN** the target pass enters combat against that fighter, the duellist engages, its combatant targets the fighter with a hostile contact, and the root has no contact

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

### Requirement: The race word picks the per-contact sense
A brain placed from a character record SHALL read the record's profession word as its race word. A race word with the AI bit `0x1000` SHALL install the standard sense when its class byte is `0x12`, and the ally sense otherwise; a race word without it SHALL install no sense and keep the base perception.

#### Scenario: Guard, suit and human
- **WHEN** brains are placed from records of race byte 16, race byte 18 and race byte 0
- **THEN** the first takes the ally sense, the second the standard sense and the third the base perception

### Requirement: Statues and fellow AIs
The statue bit `0x08000000` with the AI bit SHALL mark the brain a statue, whose ally sense clears the contact's noticed level and concern and ends. The ally sense SHALL take an observed character for a fellow AI only when its race word carries the AI bit.

#### Scenario: A statue guard and a human
- **WHEN** brains are placed from a record of race byte 16 with the statue bit and from a human record
- **THEN** the first is a statue carrying the AI mark and the second carries no AI mark

### Requirement: The standard sense's range
The standard sense SHALL clear the contact's threat word, end with 0 for a contact that is not a character, and end with −1 for a character that cannot be perceived. Out of 1200 cm (1800 cm while tracked, a strict squared compare ignoring height differences under 150 cm) it SHALL clear the tracked flag and end with −1. In range it SHALL set the tracked and noticed flags and write the closeness `1 − d/1200`, at most 0.99 compared in double.

#### Scenario: Exactly at the range
- **WHEN** an untracked character stands exactly 1200 cm away
- **THEN** the sense ends with −1 and the contact's flags stay 0

### Requirement: The standard sense's sighting
A dead or asleep body, or any body seen by an observer standing as a statue, SHALL get noticeability 0, its noticed level capped at 0.5 and its concern, hostility and selected character cleared. A down body SHALL get 24·closeness, its concern times 0.99 in double while knocked out. A live one SHALL get `((look·u·0.5 + 0.5)²·0.5 + 1)·((facing·u·0.5 + 0.5) + 2)·(2c⁴ + (1 − c²)c)`, 10 more while it attacks. The view SHALL be `(look·u·−0.5 + 0.5)²·0.25 + (forward·u·−0.5 + 0.5)·0.75`.

#### Scenario: A body facing the observer at 600 cm
- **WHEN** a live body 600 cm away looks and faces straight at an observer that looks straight at it
- **THEN** the closeness is 0.5, the view 1 and the noticeability 2.25

### Requirement: The standard sense's threat
The player's body SHALL clear the concern and set the attitude to 1. For any other live body the sense SHALL weigh each character contact of the observed, softened by `min((concern + hostility)·0.15, 1)`. The heaviest SHALL select the observer's own contact at that entry's position. Only a third party that is the observer (times the multiplier and 0.5) or the player (times the multiplier) SHALL raise the concern target to the largest such value.

#### Scenario: A threat to the observer
- **WHEN** the observed body's only contact is the observer, at closeness 0.5, view 1 and concern 2, with multiplier 8
- **THEN** the concern target is 0.102589 and the selected character is the observer's contact at position 0

### Requirement: The ally sense's entry
The ally sense SHALL clear the contact's hostility and threat words before anything else.

#### Scenario: A non-character contact
- **WHEN** the ally sense runs on a contact with hostility 2 that is not a character
- **THEN** its hostility and threat are 0 and the sense ends with 0

### Requirement: The sense the perception runs
The perception pass SHALL run, for each brain, the sense its race installed: the base perception (the role database's attitude, the group blend, the threat estimate and the familiarity drift) for a race without the AI bit, the standard sense for an AI race of class `0x12xx`, and the ally sense for the other AI races.

#### Scenario: A Golem Suit sees the player
- **WHEN** a brain holding the standard sense, with concern 2 for the player, perceives the player standing 200 cm ahead and looking away
- **THEN** the contact is tracked with attitude 1 and concern 0, closeness 1 − 200/1200 and noticeability `2·(2c⁴ + (1 − c²)c)`

#### Scenario: A human keeps the base perception
- **WHEN** a brain is placed from a human record
- **THEN** it installs no sense and perceives through the base perception

### Requirement: An out-of-fight brain senses nothing
A brain whose body is out of the fight (condition bit `0x800`) SHALL perceive nothing in the perception pass.

#### Scenario: Knocked out
- **WHEN** a brain whose body is dead faces the player two metres ahead, then stands again
- **THEN** it holds no contact for the player, then tracks the player

### Requirement: The target pass classifies before the drift
Within the AI tick, the brain SHALL take its aim point and freshness, then run the target pass on the contacts as the perception left them, and only then drift its contacts' hostility, concern, noticed level and attention, with the level, attention and stress the pass wrote.

#### Scenario: Classified before the drift
- **WHEN** a brain with no tolerance and no stress holds a seen contact with hostility 2.0015
- **THEN** the pass engages at level 3 and the drift then leaves the hostility under 2

### Requirement: The standard sense's statue observer
The standard sense SHALL take its lying branch for an observer standing as a statue, and SHALL NOT take it for an observer that is merely asleep.

#### Scenario: A statue observer
- **WHEN** an observer standing as a statue sees a live body facing it at 600 cm with noticed level 1, and an asleep observer that is no statue sees it
- **THEN** the first contact's noticed level is 0.5 and its noticeability 0, and the second keeps noticed level 1 with noticeability 2.25

### Requirement: The target pass's turn
At the start of each tick the turn SHALL pass to the brain the last tick named, clearing the named brain and its wait, and the tick counter SHALL step. Each brain in the scene and the fight SHALL then ask for the turn: the brain on its turn SHALL stamp the counter as its last pass and run the target pass; any other SHALL be named for the next tick when its wait is strictly longer than the longest so far this tick. A brain out of the scene SHALL neither run the pass nor wait.

#### Scenario: Three brains
- **WHEN** three brains ask for the turn every tick from the first
- **THEN** none runs the pass on the first tick, and they run it in turn from the second, the first brain first

#### Scenario: A lone brain
- **WHEN** a single brain asks for the turn every tick
- **THEN** it runs the pass every other tick

#### Scenario: A dormant brain
- **WHEN** a dormant brain stands beside a present one for six ticks
- **THEN** it never perceives or runs the pass, and the present brain runs it on ticks 2, 4 and 6

### Requirement: The wait for the target pass
A brain's wait SHALL be the tick counter less its last pass, an unsigned difference of the wrapping 32-bit counter, and the whole counter for a brain that never had a pass.

#### Scenario: A newcomer
- **WHEN** a brain that never had a pass asks beside two that alternate
- **THEN** it runs the pass on the next tick after the current one, ahead of the brain that waited one tick

### Requirement: Between target passes
On a tick without its target pass a brain SHALL keep the last pass's report and the per-pass words its last perception wrote, its concern drifting toward the concern target that perception left, and SHALL neither enter nor leave combat.

#### Scenario: The hostile still engages
- **WHEN** a hostile and a neutral brain share the turn for 700 ticks
- **THEN** each has the pass every other tick and the hostile brain engages the player

### Requirement: Asleep is the dormant flag
The perception SHALL read a body as asleep when its state flags carry the dormant flag (0x10), whatever else they carry: an asleep observer's reach SHALL halve and an asleep observed body SHALL be weighed as lying. A sleeper and a knocked-out body both carry it.

#### Scenario: Sleeper and knocked-out body
- **WHEN** an observer with flags 0x10, and one with flags 0x30, look at a body with flags 0x30
- **THEN** both observers read themselves as asleep and the body as asleep
