# Spec Delta

## ADDED Requirements

### Requirement: Story events on the live player
The running game SHALL answer a story script's `GameEvent` with the live player's memory role, class word, sex, potential, voice volume and pitch, and the story's fragments seen and background.

#### Scenario: A player reading
- **WHEN** a code arrives for a female player holding the memory role, class word 0x510000 and potential 4, with fragments 0b11 seen
- **THEN** the handler reads the memory role, the class word 0x510000, the female voice, potential 4 and fragments 0b11

### Requirement: Story event effects in order
The running game SHALL apply the handler's effects in order: the requested action through the unforced action switch, the voice line through the body's voice request, the narrator topic after its delay, the potential and class word on the record, and the sounds at the player (or the origin for the wail) with the handler's voice flags.

#### Scenario: A first memory fragment
- **WHEN** fragment 1 arrives for a player holding the memory role with potential 4 and no fragment seen
- **THEN** the head-pain action starts, the heavy groan plays, the narrator says `XU_Memory` 2.5 s later, the fragment word is 1 and the potential is 5

#### Scenario: The same fragment again
- **WHEN** fragment 1 arrives a second time
- **THEN** nothing happens

### Requirement: Raise-dead grant
The raise-dead narration SHALL, on the unknown background, first ask the power grant for Raise Dead on the player's known powers with the mind tree, and SHALL pick the successful line when the grant wrote the power and the failed line otherwise.

#### Scenario: A player without the grant technique
- **WHEN** the raise-dead code arrives for a player whose Insight lacks the grant technique
- **THEN** no power is written and the narrator says `RVL_RaiseDead_F` a second later

### Requirement: Story session words
The story SHALL keep its start background and the fragments seen: a new story SHALL clear the fragments and keep its background, a fragment SHALL or its bits in, an awakening SHALL set the word to 0xFFFF, a story save SHALL write the background and a non-zero fragment word, and a load SHALL restore both. The story events SHALL act on the raise-dead, Corlian and awakening codes only on the unknown background.

#### Scenario: An awakening after three fragments
- **WHEN** the mind awakening arrives with fragments 0b111 seen and potential 4 on the unknown background
- **THEN** the potential becomes 11, the fragment word 0xFFFF and the class word takes 0xD10000 under 0xFF00FFFF
