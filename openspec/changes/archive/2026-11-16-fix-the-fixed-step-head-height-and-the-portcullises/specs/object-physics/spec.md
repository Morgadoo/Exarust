## MODIFIED Requirements

### Requirement: Shipped portcullises
exanima01's 37 sliding leaves SHALL lie at the open end of level rails at load, standing in their arches, and read open; woken and left alone, each SHALL creep less than 2 cm and sleep; the router's close of an unlatched fast leaf SHALL slide it 1.7 m into the slot in its wall, where it reads shut and its arch's navigation cells are freed.

#### Scenario: Signalled shut
- **WHEN** the unlatched fast portcullises of `exanima01` are toggled by signal
- **THEN** each travels 1.7 m along its rail and reads shut

## ADDED Requirements

### Requirement: Sliding leaf reading
A sliding leaf SHALL read open at its placed pose (fraction 0) and shut at its rail's far end (fraction 1), as the original's door does; on the shipped portcullises' rails the placed pose is in the arch and the far end in the wall, and the leaf SHALL report which of the two it stands nearer.

#### Scenario: Closed and opened on level rails
- **WHEN** a motorised leaf at its placed pose on 1.7 m level rails takes a close, then an open
- **THEN** it travels 1.7 m, reads shut and is no longer at its placed pose, and after the open it reads open again

### Requirement: Sliding leaf picked as a door
The hand's pick SHALL name a sliding leaf by its door record as it names a hinged one: a hinge pick, or nothing when the record bars clicks, whatever the leaf's mobility.

#### Scenario: A portcullis under the cursor
- **WHEN** the pick hits a sliding leaf of mobility 0.048 whose record allows clicks
- **THEN** the pick is a hinge, and the same body without its door record picks nothing
