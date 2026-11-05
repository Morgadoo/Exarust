# Design

Formats page: `docs/formats/OTHER.md` (object database sections): the
operative row now names the use script at `0x38`/`0x3C` and its one
shipped record.

## Findings (REA, against the assembly)

- **Object use.** Type 3: the 2400 ms timer, then the script run, then the
  pulse (1200, 0), then the sound at 0.3 with flags `0x400`. The run sets
  the object script context's actor count to 1 when there is a player
  (else 0) with the player in slot 0, its owner to the object, and runs the
  text the operative section's accessor gives: the length word at body
  `0x38` compared signed against 1, the text at `0x3C`. Nothing writes the
  context's running-role word: object scripts run with role 0.
- **WitnessRole.** The wrapper checks that the first argument is the
  9-byte `Operative`, reads the second as an id, and calls the grant with
  the AI manager and the owner's position (the object's place). The grant
  walks the manager's characters: squared distance at most 3 240 000 cm²
  and the level's sight callback from the operative's place, then AddRole.
  Shipped callers (grep of the packs): the Pedestal Hand's use script only.
- **StartDialogue.** The roles unit registers it (then `ChangeRole`). The
  builtin ignores the calling script's actors: it copies a static record
  (owner, partner, role, partner role) and the first argument as a 16-byte
  name into a request and calls the dialogue's AI entry with arousal limit
  10. That entry refuses an owner above 10 or in any conversation's actor
  list (any state), then searches both ways with the owner, the partner and
  the topic (the roles are not passed); the callback sets the calm filter,
  starts the named node, and begins or frees the conversation. The only
  writer of the static record is the encounter: owner and partner, the
  running role per role, the partner role only on an entry matched on one
  of the partner's roles (never reset). Shipped callers: one dialogue node
  action (`Wary`: `StartDialogue(Greeting);`), no encounter script.
- **Interaction hook.** On a living character other than the player:
  dead mode `0xA00` loots; condition bit `0x800` or an active possession
  stops; else the condition test: the entry finder for section `0x40` of
  the character toward the player, none fails, length 0 passes, else the
  text runs on its own context with the character and the player as
  actors 0 and 1 and passes on a signed result above 0. A failed test
  walks the character's power list for `0x10010A` or `0x10010D` cast by the
  player. Either success opens the character inventory window, the same
  call the player's own body makes.

## Open questions

- The window for another character's inventory: no Exarust view yet
  (logged).
- The encounter's running-role word: no writer found by xrefs; a write
  through a pointer is not excluded. Exarust keeps passing the role.
