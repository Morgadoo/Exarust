# Design

Both read data the parsers already decode (`exa_formats::roles::RoleDb`,
`CharacterRecord::powers`). An attitude entry's target is a role id, or one
character in one locale when its condition marks it so; a target of 0 is
anyone. Formats page: none changes.
