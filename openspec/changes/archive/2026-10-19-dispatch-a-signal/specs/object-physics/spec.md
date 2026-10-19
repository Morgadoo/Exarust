# Spec Delta

## ADDED Requirements

### Requirement: Signal door
A missing target SHALL do nothing. A door SHALL close on 0 and open on any other value, and -1 SHALL toggle: an unlatched leaf closes and a latched leaf opens. An inverted door SHALL flip that 0 and 1, and -1 SHALL still toggle. A door SHALL win over an operative record on the same object.

#### Scenario: Close, open, and a toggle
- **WHEN** a missing door is signalled 1, a door that also carries a fire job is signalled 0, then 1, then 2, then -1, and the same door inverted is signalled 0 and then 1
- **THEN** the missing door does nothing, the door closes, opens, opens, and toggles, an unlatched leaf closes, a latched leaf opens, and the inverted door opens and then closes

### Requirement: Signal job
A fire job SHALL start, at its stored size, only when the value is not 0. A light job SHALL set the stored bit to the value's low bit. A relay of kind 0, 1, 2, 3 or 5 SHALL be named, and any other kind SHALL be ignored. A plate SHALL be pushed exactly when the value is not 0.

#### Scenario: A fire, a light bit, a relay, and a plate
- **WHEN** a fire of size 200 is signalled 0 and then 1, a light whose stored word is 16 is signalled 1 and then 2, a relay of kind 4 is signalled, and a plate is signalled 0 and then -1
- **THEN** the fire does not start and then starts at size 200, the stored word becomes 17 and then 16, the relay is ignored, and the plate is free and then pushed
