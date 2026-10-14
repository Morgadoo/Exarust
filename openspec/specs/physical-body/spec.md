# physical-body Specification

## Purpose
TBD - created by archiving change add-ragdoll. Update Purpose after archive.

## Requirements

### Requirement: Body data
The system SHALL decode each skeleton bone's physical body chunk: its links to the parent part (flags, a point on the part and one on the parent, slack, stiffness, smoothing, and the axis of sliders), its foot contact points and its pins.

#### Scenario: Knee pivot
- **WHEN** `humanoid.rsq` is parsed
- **THEN** each lower leg has two links to its femur, one of them with zero slack

### Requirement: Limp bodies
The system SHALL simulate a limp body as the original does. Each part is a rigid tetrahedron of four particles laid over its mesh, integrated at 83 Hz under gravity with speed-banded velocity retention. Links between parts are hard radii in the parts' rest frames, solved 20 times per tick. Sphere colliders are pushed out of static geometry 8 times per tick with friction.

#### Scenario: Body at rest
- **WHEN** a body goes limp above a floor
- **THEN** its parts fall, come to rest on the floor, and every link stays within its slack

### Requirement: Going limp
The system SHALL turn a posed body asked to go limp into a ragdoll from its current pose, stop animating it and remove its character controller.

#### Scenario: Corpses
- **WHEN** a level places a dead character
- **THEN** its body collapses where it was placed

#### Scenario: Character view
- **WHEN** `exarust --limp character` runs
- **THEN** the body drops to the floor in a sprawl with its parts joined

### Requirement: Muscles
The system SHALL drive a living physical body toward its animated pose. Each particle is pulled toward its place in the pose with damping relative to the pose's own motion. The gain is 0.084 per tick (0.07 for hands) times tone² times energy⁴, and gravity is compensated by tone times energy. The body is carried along with its character controller's motion.

#### Scenario: Driven walk
- **WHEN** `exarust character` plays the walk on a physical body
- **THEN** the body walks with its parts within a centimetre of the animated pose

### Requirement: Blows and daze
The system SHALL let a blow change the velocity of the struck part and take a share of the body's energy (at most 90 %, never below 0.05), which recovers by 0.005 per tick.

#### Scenario: Stagger
- **WHEN** a body's chest is shoved at 3 m/s with daze 0.7
- **THEN** the chest is pushed off its pose, the body sags, and both recover within about a second
