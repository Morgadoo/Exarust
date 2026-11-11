## Why

Round 184's exitstall lane wired the flee start, but every target pass
logged `most concern 0.000`: no brain's contact ever gained concern or a
contact alarm, so neither of the classifier's flee routes could fire.
Docs/PLAN.md, Phase 6 (AI), TMotileAI, TCombatant, TPlayerBehaviour.

## Evidence (REA, the assembly of `Exanima.exe` 0.9.5)

- The contact's concern target and its alarm (the threat word the
  classifier reads) are written by the perception's threat estimate, which
  walks the observed character's own contact list (its closeness and view
  words). The player's behaviour has no perception of its own: its
  contact routine (called from the player tick) rebuilds a 12-entry list
  of the characters within 12 m with the closeness, the facing weight as
  the view and the list's score in the concern word. The player's
  behaviour table points its perception slot at the shared perception, so
  every NPC reads that list when it perceives the player. Exarust's player
  had no contact list the perception could read, so the estimate was 0.
- The routine subtracts the other's position from the player's, so a
  character straight ahead weighs 1; Exarust had the sign reversed.
- One contact record holds concern and hostility. The hit reaction, the
  script's `AddConcern` (called by the blast, torrent, wall and stamped-body
  routines) and the perception write it in place. Exarust's brain keeps the
  integration's values apart and its AI tick copied them over the
  combatant's contact each tick, so every such write was lost.
- The target pass's flee branch ends the body's conversations (every live
  conversation it speaks in, as owner or partner, ended as the engine's
  `leave` ends one), attends the pick, runs the flee start (body target for
  a character, object focus otherwise, the flee state replacing the combat
  state) and keeps the flight's alarm. The flee state's head ends a flight
  whose threat is not fresh and is in neither the brain's room nor its
  previous one, or with no target and no focus: level 0, a 10-20 s clock,
  back to the idle state.

## What Changes

- `exa_ai::combatant::PlayerContacts`: the player's contact list as the
  perception reads it; `exa_game::player_practice` writes it each player
  tick; `brain::perceive_all` reads it for a character without a combatant.
- `exa_game::player_behaviour::contacts`: the delta is player minus other.
- `Contact::mirrored` and `brain::adopt_contact_writes`: a concern or
  hostility written on a combatant contact since the AI tick's copy goes
  into the brain before it integrates.
- The flee outcome sends `exa_script::EndConversationsOf` (bridged), sets
  the duellist's target to the threat and leaves the combat state, or
  keeps an object threat as `Npc::flee_focus` (the flight walk runs from
  it); at level 4 the pass passes the threat as its foe.
- `brain::flee_walk` runs `flee::tick`'s end (the escape and no-target
  ends).
- Debug: the `exa_ai::pass` line adds the most concern target, the most
  contact alarm and the room's exits; `exa_ai::flee_way` logs the way-out
  inputs and each flight plan.

## Impact

- NPCs now notice the player as a threat: a duel brain reaches concern 1
  and enters combat at level 1 on tick 82 instead of level 3 on tick 226;
  hits accumulate concern (13.6 after two blows against 0 to 4 on main);
  with Fear and a player attacking, a story NPC with an open exit flees.
- Lockstep: exanima01 (700 frames) and the arena duel without a brain are
  identical to main; the brain duel first differs at tick 83 (the earlier
  combat entry). Two runs of each are identical.
