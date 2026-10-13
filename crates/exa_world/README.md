# `exa_world`

Levels in the Bevy world.

**Phase:** 2. Status: props, level scene nodes and lights spawn
(`spawn_level`, `spawn_model`); walls/floors from the tile grid, terrain,
sectors and portals are next (see [`docs/PLAN.md`](../../docs/PLAN.md)).

- `spawn_model(commands, model, placement)`: a cached model with its node hierarchy; a placement replaces the root transform, as level props do.
- `spawn_level(commands, loader, name)`: the level's own meshes and omni lights plus every prop from `Objlib.rpk`.
- `point_light(light)`: the original omni light approximated as a Bevy `PointLight`.
