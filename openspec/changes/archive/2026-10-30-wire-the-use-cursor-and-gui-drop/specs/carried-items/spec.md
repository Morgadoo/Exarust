# Spec Delta

## ADDED Requirements

### Requirement: Light toggle reads the held part
The light toggle SHALL read whether the light burns from the first held item's part mark, which a weapon or light take-in sets from the item's full flags, and SHALL fall back to the lit bit of the record hand holding that item when the part carries no mark of its own; a successful toggle SHALL turn the mark over.

#### Scenario: A lit torch dropped onto the doll
- **WHEN** a torch flagged 0x2000000 is dropped onto the doll (its record entry written with only the top bit) and the light is toggled
- **THEN** the light goes out: the record's lit bit stays clear and the next toggle sets it
