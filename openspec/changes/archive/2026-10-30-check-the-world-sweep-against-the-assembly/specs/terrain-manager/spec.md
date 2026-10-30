# Spec Delta

## ADDED Requirements

### Requirement: Tile read-back hand-off
Queuing a tile's bake SHALL clear the record's bit 0x100. With a pending record, the hand-off SHALL copy the 244² height plane (59 536 bytes) when the renderer's feature bit 0x2000 is set and the 240² × 3 packed plane (172 800 bytes) when bit 0x1000 is set, then clear the record's bit 0x2000, set its bit 0x4000 and empty the pending slot; with none pending it SHALL do nothing.

#### Scenario: Both planes
- **WHEN** a pending record is handed off with both feature bits set
- **THEN** 0xe890 height bytes and 0x2a300 packed bytes are copied and the record gains 0x4000 and loses 0x2000

### Requirement: Block node
The terrain manager's record SHALL hold one whole-tile block descriptor, as the original's record allocates a single childless block node.

#### Scenario: Update
- **WHEN** the height bounds change
- **THEN** only the whole-tile descriptor is recomputed
