# object-physics Specification

## Purpose
Props, items, doors, levers and pressure plates are rigid bodies in the
original, held by links and moved by the hand, by other bodies and by
signals. This capability covers their data (bodies, cloth markers, door and
operative records) and, as later changes land, their simulation.

## Requirements

### Requirement: Rigid bodies of models
The system SHALL decode the rigid body a mesh node carries as the original stores it: class word, mobility, the four rest points and point masses, the collision spheres, the links (points on the body and on the parent, slack, strength, smoothing, and a slider's axis, whose stored slack is its travel squared) and the scrape and roll sound slots, in metres. It SHALL hold for every chunk tree of `Objects.rpk`, `Objlib.rpk`, `Characters.rpk` and `Resource.rpk`.

#### Scenario: Shipped bodies
- **WHEN** every model of the installed `Objects.rpk` is decoded
- **THEN** 1 163 of the 1 253 models carry a body, all 1 230 bodies decode, 69 models carry links and 65 of them hang from the world

#### Scenario: A door
- **WHEN** the body of `door uw01a` is decoded
- **THEN** its mobility is 0.016, it has 1 899 spheres, a hinge pin 2.429 m up and a swing stop with 1.3404 m of slack

#### Scenario: Sliders
- **WHEN** the sliders of `pressplateu01` and `portcullis uwa` are decoded
- **THEN** the plate travels 0.04 m (stored 16) along its axis and the portcullis 1.70 m (stored 28 900), each equal to its axis's length

#### Scenario: Malformed body
- **WHEN** a body chunk is truncated or holds a count it cannot fit
- **THEN** decoding returns an error kept on the node, which still counts as carrying a body, and nothing panics

### Requirement: Cloth marker
The system SHALL report whether a mesh node carries the cloth marker, which makes the original simulate the mesh as cloth in the game's scene. It SHALL hold for every chunk tree of `CharExt.rpk`, `Objlib.rpk`, `Resource.rpk` and `Objects.rpk`.

#### Scenario: Shipped cloth
- **WHEN** the installed packs are decoded
- **THEN** the six long hair models carry the marker on two meshes each, four banners of `Objlib.rpk` and the wraith's robe on one, and no `Objects.rpk` model does

### Requirement: Door records
The system SHALL read an object's door section: its flags (latches, locked, not clickable, motorised, fast, inverted, frozen), the object it is linked to (its key, or the relay or lever that moves it) and the letters naming its lock, slam, hinge and unlatch sounds (`door`, `a` + letter, `_`, suffix). It SHALL hold for `objdb.rdb` and every level's own database.

#### Scenario: Doors of the first level
- **WHEN** the database of `exanima01` is read
- **THEN** it holds 43 doors, whose linked objects are its keys, levers and relays but for one id it lacks

#### Scenario: A portcullis
- **WHEN** a door section with flags `0x254` and sound letter 5 is read
- **THEN** the door is locked, motorised and fast, and slams with `doorf_slm`

### Requirement: Operative records
The system SHALL read an object's operative section: class (key, lever, push surface, job, relay), kind, flags, value, a lever's return time in 100 ms steps and two targets. It SHALL hold for `objdb.rdb` and every level's own database.

#### Scenario: Operatives of the first level
- **WHEN** the database of `exanima01` is read
- **THEN** it holds 72 operative records: 40 levers, 12 push surfaces, 6 fire jobs, 8 relays and 6 keys

#### Scenario: A returning lever
- **WHEN** a lever section with return time 50 is read
- **THEN** the lever returns after 5 000 ms

### Requirement: Mechanism of a model
The system SHALL give the body class the original gives a record's model: a lever for operative class 3, a push surface for class 5, none for any other operative class, else a door when the record has a door section. It SHALL hold for `objdb.rdb` and every level's own database.

#### Scenario: First level
- **WHEN** the records of `exanima01` are classified
- **THEN** 43 make doors, 40 levers and 12 push surfaces

#### Scenario: Operative class first
- **WHEN** a record holds a key's operative section and a door section
- **THEN** its model is no mechanism
