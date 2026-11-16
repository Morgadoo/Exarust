## ADDED Requirements

### Requirement: Shock charge and release sounds
Shock SHALL play `thmnrg_chrga` at its caster's right hand at volume 0.2 on its first control tick and `thmnrg_shock` at volume 1 as its burst fires, each at pitch `0.9 + 0.2·r` with the runtime stream's next draw, the release's draw taken before the burst's.

#### Scenario: A shock from charge to burst
- **WHEN** a player's Shock charges and fires on its 101st charging tick
- **THEN** the charge plays on the first tick, nothing while it charges, and the release with the burst

### Requirement: Displacement start and come-back sounds
Vanish's start, a player's Long Door at its cast and an AI's Long Door when its grid finds the place SHALL play `thmdsp_vns` at the body's place; both come-backs SHALL play `thmdsp_apr` on a voice that follows the body. Each SHALL be at volume 0.4 and pitch `0.9 + 0.2·r` with the engine generator's draw, a start's after the shell's and the image's draws.

#### Scenario: A vanish
- **WHEN** a player casts Vanish, holds it and lets it go
- **THEN** the start plays `thmdsp_vns` and the come-back `thmdsp_apr` on the body's following voice at the drawn pitch

#### Scenario: An AI's Long Door
- **WHEN** an AI's Long Door's grid finds its place and the door later comes back
- **THEN** the find plays `thmdsp_vns` and the come-back plays `thmdsp_apr` at the destination

### Requirement: Force wave voices
A force wave SHALL sound as the blast does: its start SHALL take the runtime stream's draw after the ring's for its charge loop's start, `length·0.8·r` into the buffer in double precision, and its throw SHALL play `thmfrc_wav` at the pitch the throw draws; the frame SHALL level both voices by the blast's rule.

#### Scenario: A thrown wave
- **WHEN** a paid wave charges and is thrown
- **THEN** its charge roll is the stream's draw after the ring and its release pitch lies between 0.9 and 1.1

### Requirement: Force Strikes swing sound
While Force Strikes runs, its caster's swing voice SHALL start on `ThmFrc_Swing` and its attack ticks' volume SHALL take the swing factor 3; its end SHALL give the body back `SwingHA` and the factor 1.

#### Scenario: A force swing
- **WHEN** a fighter under Force Strikes swings a sword into the swing's region
- **THEN** the voice starts on `thmfrc_swing` and the tick's volume is the swing sound at loudness 3

### Requirement: Throw and kinesis failure sounds
A throw cast whose pick finds nothing SHALL answer 1 and play `ThaumFail01`; a left press kept for combat or out of reach while a kinesis waits for its pick SHALL play `ThaumFail01` and end the kinesis.

#### Scenario: A refused kinesis pick
- **WHEN** the hand refuses a press while the player's kinesis waits
- **THEN** the failure sound plays and the kinesis leaves the active list

### Requirement: Mind powers mark their bar slots
A player's Mind power that ends or answers 4 SHALL mark the bar slot holding its id, and opening the bar SHALL mark the running Mind powers: Spirit Form, Control Mind and the constant Sense Minds and Mask Mind light it, the constant Loyalty fades it with the mark as it ends, the instant Sense Minds and Mask Mind fade it on their clocks, and the other classes take the base rule.

#### Scenario: Sense and Loyalty
- **WHEN** the player's instant Sense Minds passes its end and the constant Loyalty ends
- **THEN** the sense's slot fades from its end over 2 s and the loyalty's fades with the mark over 200 ms

### Requirement: Barricade ended by an event
A barricade's power that ends, by an event or out of range, SHALL tell its wall to fall.

#### Scenario: A collapse broadcast
- **WHEN** the caster's status broadcast tells its running barricade 2
- **THEN** the power leaves the list and its wall is told to end

### Requirement: Cast end to the stored stance
The cast end SHALL return a body to its stored stance over 30 frames, linear; a body without one SHALL keep the combat idle or the defence stance it played, else Idle.

#### Scenario: A body in the combat idle
- **WHEN** a cast ends on a body whose stance is the combat idle
- **THEN** it plays `Cmbt - Idle` with a 30-frame blend

### Requirement: Tree domain on opening
The power tree SHALL open on the domain it last showed when the same player body opens it again, and on the Mind domain for another body.

#### Scenario: Another character
- **WHEN** the tree last showed the Force domain and a new player body opens it
- **THEN** it opens on the Mind domain, where the old body would have reopened on Force

### Requirement: Mind mark leaves the party
A Loyalty or Command Dead mark that ends on a body SHALL take the body out of the experience system's party, as the original's mark end calls the party's removal.

#### Scenario: A loyalty mark ends
- **WHEN** the Loyalty mark on a target is told to end
- **THEN** the walk sends the target's party leave
