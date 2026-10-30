# Design

The camera pair lives on `Session` so both writers stamp it from one place;
the readers leave the session's defaults because the original's readers
discard the pair. The host owns the value through an optional `SaveCamera`
resource, absent today, so saves keep writing the defaults Exarust wrote
before.

The disable flag is the existing `LiveTransition::saves_disabled`, which
the viewer inserts after every in-process level change. The new-slot save
and the flow's save step return before anything else, as the writer's
first check does. The checkpoint step hands the flag to the save sections'
`disabled` switch so `Slots::checkpoint_full` skips the save and still
copies, reproducing the original's stale checkpoint.

`session_from_player_data` is a separate reader so the strict Exarust
payload reader keeps refusing a retail file, as the session spec requires;
the load path falls back to it only when the strict reader fails.

The record repair reuses `arena_save::repair_converted_record` and runs on
the record the placement names, gated by `LoadedSave::converted`, which is
set only by the data-version conversion.
