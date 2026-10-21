## ADDED Requirements

### Requirement: Power focus and ending
A running power SHALL hold the focus it takes in the body's reserve and spent focus, three quarters of the cost with the reserve technique, and SHALL return the held amount to the reserve only. A control event 2 SHALL mark a power ending and answer 0; the next tick SHALL return its focus and answer 2. A persistent power SHALL publish HUD word 2 while it runs and word 12 with the clock and a 200 ms fade once it ends.

#### Scenario: A persistent power ends over two ticks
- **WHEN** a power takes 0.5 of focus, receives event 2 and then ticks again
- **THEN** the event tick answers 0, the next answers 2, the reserve is back to 0 and the spent focus stays 0.5

#### Scenario: The reserve technique
- **WHEN** a body with the reserve technique gives a power a cost of 0.4
- **THEN** the power holds 0.3 and the reserve and spent focus each grow by 0.3

### Requirement: Timed Mind powers
Confusion and Enrage SHALL clear their status bit and answer 2 once the unsigned 32-bit elapsed game time passes 15 s or the power is ending, without writing the state. Trust and Suspicion SHALL mark themselves ending once the elapsed time passes 20 s.

#### Scenario: Confusion expires
- **WHEN** a confusion applied at 100 ms ticks at 15 100 ms and at 15 101 ms on a body that is confused and masked
- **THEN** the first tick keeps both bits and answers 0, the second clears the confusion bit only and answers 2, and the state stays 0

### Requirement: Instant Mask Mind
Instant Mask Mind SHALL keep the masked bit for 15 s with intensities `0.9·(1−((t−5 s)/10 s)²)` after 5 s and `0.8·(1−((t−10 s)/5 s)²)` after 10 s, worked in double, answering 4 as the second fade begins and 2 once it expires.

#### Scenario: The mask's two fades
- **WHEN** an instant mask applied at 1000 ms steps at 11 500 ms
- **THEN** the first intensity is 0.62775, the second 0.792, and the step answers 4

### Requirement: Loyalty strength
Once 800 ticks have passed, a loyalty target's strength SHALL blend `((relation+100)·0.005)²·100`, kept in double until it is narrowed, at 0.05 with 0.95 of its old value.

#### Scenario: A loyalty strength in double
- **WHEN** a loyalty target past its 800 ticks with strength 0 ticks with relation −98.15
- **THEN** its strength has the bits 971000813

### Requirement: Control Mind binding
Control Mind SHALL count its binding down by the double 0.002 and take the single 0.002 of focus each tick, and SHALL hand control over once 2 s have passed. Taking control back SHALL write the caster's status, less the control bits, over the released body.

#### Scenario: Control comes back
- **WHEN** control is taken back from a body while the caster's status holds the handover bit and bit 0x10
- **THEN** the released body's status is the handover bit and 0x10, and the caster keeps 0x10 only

### Requirement: Force shell and domes
Shell SHALL set the body's hit damping to 0.5 and clear it at the end. A bubble or barricade power SHALL end on an event with bit 1 or 3, and SHALL mark itself ending once its effect is more than 20 m from the caster.

#### Scenario: A dome left behind
- **WHEN** a bubble at the origin ticks with its caster 2000 cm and then 2000.1 cm away
- **THEN** it keeps running at 2000 cm, marks itself ending at 2000.1 cm and answers 2 on the next tick

### Requirement: Force Strikes
Strikes SHALL set the strike factor 1.0 and the swing factor 3.0, keep at most four weapon auras, and ripple the aura of a swung weapon at half the swing's strength.

#### Scenario: A swing
- **WHEN** a weapon with an aura is swung at strength 3
- **THEN** its aura ripples at 1.5

### Requirement: Kinesis steering
Kinesis SHALL steer its body at its reach (half its radius plus one) along the unit direction to the aim, or the direction times 0.02 within 50 cm or for a distance that is not a number, and SHALL turn it by `reach²` times the double 0.001 a tick, each key pair tested if/else.

#### Scenario: Both turn keys
- **WHEN** a kinesis body of radius 100 has all four turn keys held
- **THEN** it turns forward on both axes by 2601 times the double 0.001, narrowed

### Requirement: Throw flight
Throw SHALL hold no reserve. Its lift SHALL aim 100 cm above the body and spend tick ms × cost × 0.00125 every steered tick at `mass/(0.1+mass)` of the unit direction; 800 ms after the lift it SHALL drop the shell and coast at six times that share toward the caster's aim point, ending at once when that point is within 100 cm, and restore the body's materials 200 ms after the drop.

#### Scenario: A throw's flight
- **WHEN** a throw of cost 0.1 lifts a body of mass 0.9 at 1000 ms with a 12 ms tick and its caster aims 500 cm away along x
- **THEN** it aims 100 cm above the body, spends 0.0015 on each steered tick, drops the shell at 1801 ms, coasts at six times the mass share along x and restores the materials at 2002 ms

### Requirement: Energy power classes
Shock's first control tick SHALL install its cast hook and play the charge; each later tick SHALL add the charge rate, spend the tick's focus while charging when the charge is not past 1, and past 1 fire one arc burst and clear the hand's arcs, the cast hook and the cast id; a second charge past 1 SHALL answer 2. Shock Weapons SHALL start cast motion 0x25 and clear its weapon arcs when it ends.

#### Scenario: One burst
- **WHEN** a shock with rate 0.5 ticks four times after its first tick
- **THEN** the second and third ticks spend focus, the fourth fires the burst and clears the cast id, and a later charge past 1 answers 2

### Requirement: Vanish phases
The vanish effect SHALL wait 400 ms spending the power's tick cost, hide the body with its vanished bit, fade it out over 300 ms spending the tick cost, hold (Vanish until released, spending half its cost; the Long Door 400 ms, spending nothing), move it, fade it in over 400 ms with the share narrowed before it is taken from 1, and finish 300 ms later.

#### Scenario: A vanish
- **WHEN** a vanish begun at 0 ms with a tick cost of 0.001 is released at 1101 ms
- **THEN** it spends 0.001 until the body vanishes at 400 ms, is half faded at 550 ms, holds from 700 ms spending 0.0005, moves at 1101 ms, is 0.25 shown at 1201 ms and is done at 1801 ms

#### Scenario: The Long Door holds by itself
- **WHEN** an AI's long door finds its place at 20 ms
- **THEN** the body vanishes at 420 ms, the hold spends nothing and the body moves at 1120 ms without a release

### Requirement: Constant Displacement and the Summon
Constant Displacement's hum SHALL rise by the double 0.002 each control tick while its volume is under the double 0.12. The summon SHALL wait 2 s after its demon is created, fade the caster in over 3 s and finish 1 s later.

#### Scenario: The summon's fade
- **WHEN** a summon's demon is created at 4 ms
- **THEN** the fade starts at 2004 ms, the caster is half shown at 3504 ms and the summon is done at 6005 ms

### Requirement: Power lists and dispatch
The known-power search SHALL return the index of an id in a record's known powers, or none; ending by id SHALL mark the first running power with that id ending. AI evaluation SHALL dispatch on the id's domain byte (0x200 Force, 0x400 Energy, 0x600 Displacement) and answer 0x400_0000 for any other.

#### Scenario: A Mind id
- **WHEN** an AI evaluation asks for id 0x2010C
- **THEN** it answers 0x400_0000

### Requirement: Saved power rebuild
A saved Force power SHALL be rebuilt with its raw id; with the reserve technique its held focus SHALL be taken down by 0.75 once more after the rebuild's own three quarters.

#### Scenario: A rebuilt shell with the technique
- **WHEN** a body with the reserve technique loads a saved Force Shell of cost 0.35
- **THEN** the reserve grows by 0.2625 and the rebuilt power holds 0.196875

### Requirement: Power Glean
Power Glean SHALL teach Vanish, or else Shock, from a dying shaper's known powers to a player who knows at least one power and none of that domain, unless the death is more than 20 m away; a distance that is not a number SHALL pass.

#### Scenario: Glean prefers Displacement
- **WHEN** a shaper knowing Shock and Vanish dies 1 m from a player who knows only a Force power
- **THEN** the player learns Vanish with word 0x9fff and `Power_Glean` shows for 3 s
