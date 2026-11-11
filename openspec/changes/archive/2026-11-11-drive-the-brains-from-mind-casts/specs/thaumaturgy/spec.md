## ADDED Requirements

### Requirement: Control minds written on the power walk
Behind the simulation bridge, Control Mind's handover SHALL write the caster's handover bit, the target's control bits and the target's possession marker on the world that walks the powers, after the frame's changes of that frame, and SHALL write no mind on the frame. In one world the writes SHALL run at once.

#### Scenario: A slow frame at the handover
- **WHEN** the frame's copy of the target's mind lags the simulation's when the handover runs
- **THEN** the simulation's mind takes the marker and the control bits, and the frame's copy writes nothing over it

### Requirement: Hand-back minds written on the power walk
Behind the simulation bridge, Control Mind's hand-back SHALL write the caster's status word less the control bits over the released body's, then clear the caster's handover bit, both on the world that walks the powers; Spirit Form's end SHALL clear the body's handover bit there too, and the way home SHALL give the caster's body the body's status word there after the frame's changes.

#### Scenario: Spirit Form ends in free mode
- **WHEN** the spirit's form ends while the simulation runs free
- **THEN** the left body's handover bit clears in the simulation and the caster that comes home carries the cleared word

### Requirement: Mind casts change the live brain
A Mind cast's changes to its target's AI SHALL be recorded in order and run on the target's live brain on the world that runs the brains, at the start of its next step: Calm, Enrage, Confusion, Trust, Fear, Embolden, the settled contact of Loyalty and Command Dead, and the re-arm. A body without a brain SHALL keep no such values.

#### Scenario: Fear on a brain
- **WHEN** Fear starts on a living target with a brain whose alarm is 30
- **THEN** at the next step the brain's alarm is 40

### Requirement: Mind cast AI values
The casts SHALL move the brain's values the original's offsets name: Calm moves each contact's concern and hostility into its noticed level and lowers the panic by 10, the readiness by 50 and the alarm by 1, each floored at 0, and sets the calm to 1; Enrage and Confusion add 50 panic; Confusion clears the body's target, the brain's level and each contact's noticed, sighting, attention, concern, concern target, soothe and hostility.

#### Scenario: Calm
- **WHEN** Calm starts on a brain whose contact has concern 12, hostility 4 and noticed 1, with panic 5 and readiness 60
- **THEN** the contact has concern 2, hostility 0 and noticed 43, the panic is 0, the readiness 10 and the calm 1

### Requirement: Bound contact and re-arm
Trust SHALL halve the concern and hostility of the target's contact toward the caster and set its noticed level to 30; Loyalty and Command Dead SHALL clear them and set it to 30, and set the brain's re-arm byte. Fear SHALL add 20 alarm, 40 at most; Embolden 50 courage, 80 at most.

#### Scenario: Command Dead's contact
- **WHEN** Command Dead raises a corpse whose brain has a contact toward the caster
- **THEN** that contact has no concern or hostility, a noticed level of 30, and the brain is re-armed

### Requirement: Re-armed roles read the powers
A brain whose re-arm byte is set SHALL apply its roles again at its next tick though they did not change, reading the codes of its body's mind powers and whether the body resists mind control, and SHALL clear the byte: a Loyalty or Command Dead marker on a body that does not resist SHALL make its activity follow.

#### Scenario: Loyalty bound
- **WHEN** Loyalty binds a brain that does not resist
- **THEN** its roles are applied again and its activity is follow

### Requirement: Marker end re-arms the brain
The end of a Loyalty or Command Dead marker SHALL set the bound brain's re-arm byte on the world that walks the powers, so its roles are applied again without the marker.

#### Scenario: Loyalty ends
- **WHEN** the Loyalty marker on a following brain ends
- **THEN** its roles are applied again and it no longer follows

### Requirement: Revival on the revived body
A revival SHALL write, after the revive muscles the body again, the body's pins factor (the gait scale `1.25 − f²/4`), its most energy and its feral level, whose step reach is the revival's stamina, and its root's link-iteration count (the pace `round((1 − f/4)·1660/83)`).

#### Scenario: A raised corpse
- **WHEN** Raise Dead revives a corpse without decay
- **THEN** its body has pins 1.24, most energy 0.98, feral 0.2 and 19 link iterations

### Requirement: Dead powers read the profession word
Before a Mind cast, the target's character flags SHALL be its record's profession word: its `0x40` bit SHALL refuse Raise Dead and Command Dead, and a race byte other than 0 SHALL keep the corpse from the revival.

#### Scenario: A skeleton corpse
- **WHEN** Raise Dead starts on a dead body whose profession word has race byte 3 (a living kind)
- **THEN** the body is raised without a revival
