## ADDED Requirements

### Requirement: Creation mode switch
Switching SHALL store the object as the global and call its routine in mode 2. Any other mode SHALL clear bit `0x10` of the looked-up widget for a positive mode and set it otherwise.

#### Scenario: Special and widget
- **WHEN** the mode is 2
- **THEN** the global is stored and the special call runs
- **WHEN** the mode is 1
- **THEN** the widget's bit is cleared
- **WHEN** the mode is 0
- **THEN** the widget's bit is set

### Requirement: Creation mode toggle
Toggling SHALL leave mode 2 for mode 1 with flag bit 1 and enter mode 2 from any other mode with flag bit 2, refreshing. Without the bit nothing SHALL change.

#### Scenario: Down, up and refused
- **WHEN** the mode is 2 and bit 1 is set
- **THEN** the mode becomes 1 and the refresh runs
- **WHEN** the mode is 1 and bit 2 is set
- **THEN** the mode becomes 2 and the refresh runs
- **WHEN** the needed bit is clear
- **THEN** the mode does not change and no refresh runs
