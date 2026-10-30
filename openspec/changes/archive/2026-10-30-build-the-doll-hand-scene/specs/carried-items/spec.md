# Spec Delta

## MODIFIED Requirements

### Requirement: Re-dress apply order
Applying a re-dress plan SHALL detach each changed hand whose body model is present, then clear the body's weapon part once when its weapon class word has none of the `0x30` bits, then attach the new models, then refresh the body when both target item ids the plan read are 0, then recompute the body's grip, then restart the stance unless the body is in its swap state, clear the attach flag and, for a swap plan, exchange the sets.

#### Scenario: An empty stowed set on a body without weapon bits
- **WHEN** a swap plan whose targets are both 0 changes the main hand of a body holding no models, in its swap state with class word 0x40
- **THEN** the steps are clear the weapon part, refresh the body, recompute the grip, clear the attach flag and exchange the sets

## ADDED Requirements

### Requirement: Doll hand scene
The doll's hand-scene builder SHALL mark every hand whose target item differs from the item it shows as changed and SHALL build a model only for a non-zero modelled target, attached as a weapon when the item has a weapon section (as a light when it is the off hand and the item also lights), else as a shield, else as a light, else as a weapon. A weapon-section node SHALL settle at -1 and a shield's at the largest word.

#### Scenario: A shield joins the off hand
- **WHEN** the main hand keeps item 5 and the off hand's target becomes a shield
- **THEN** only the off hand changes and it is attached as a shield whose node settles at 0x7FFFFFFF

### Requirement: Hand take-in record flags
A weapon or light taken into a hand SHALL write only the top bit of the dragged item's flags into the record; a shield SHALL keep all its flags.

#### Scenario: A sword with flags 0x82000000
- **WHEN** a sword flagged 0x82000000 is taken in while a torch is the main hand
- **THEN** the record's entry is written with 0x80000000

### Requirement: Released hand model
Releasing a hand's old model SHALL detach its scene node, SHALL refresh the body when both record hands name no item, and SHALL restart the stance when the take-out job's mode has bit 1.

#### Scenario: Scheduled take-out
- **WHEN** a scheduled take-out (mode 7) empties both hands
- **THEN** the node is detached, the body refreshed and the stance restarted
