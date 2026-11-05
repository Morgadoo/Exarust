//! `--sim-digest <file>`: a debug aid that writes, after every fixed step, a
//! hash of every bridged data type's values (`crate::sim_bridge`), so a run
//! with the fixed step in the frame's world and one behind the bridge can be
//! compared tick by tick and type by type.
//!
//! The digest runs in its own schedule ([`SimDigestStep`]) after `FixedLast`
//! in the fixed step's order: in one world it digests the frame's world, and
//! behind the bridge it moves with the fixed step and digests the
//! simulation's world at the same point. Each line is `step, body tick,
//! type, values, hash`; components are taken in entity order, each value as
//! its `Debug` text. `EXA_DIGEST_DUMP=<step>` (or `<first>-<last>`) also
//! writes every value of those steps, to find the field that differs.

use std::any::type_name;
use std::fmt::{Debug, Write as _};
use std::fs::File;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{BufWriter, Write as _};
use std::path::PathBuf;

use bevy::app::FixedMainScheduleOrder;
use bevy::ecs::query::QueryState;
use bevy::ecs::resource::IsResource;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

use crate::sim_bridge::{SimBridgeAppExt, SimBridgeTypes};

/// One bridged type's digest of a world.
pub trait Digester: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    /// The values' hash and count; with `dump`, every value written there.
    fn digest(&mut self, world: &mut World, dump: Option<&mut String>) -> (u64, usize);
}

struct ComponentDigest<T: Component + Debug> {
    query: Option<QueryState<(Entity, &'static T), Without<IsResource>>>,
}

/// A digester of component `T` (registered with the bridge).
pub fn component<T: Component + Debug>() -> Box<dyn Digester> {
    Box::new(ComponentDigest::<T> { query: None })
}

impl<T: Component + Debug> Digester for ComponentDigest<T> {
    fn name(&self) -> &'static str {
        type_name::<T>()
    }

    fn digest(&mut self, world: &mut World, mut dump: Option<&mut String>) -> (u64, usize) {
        let query = self.query.get_or_insert_with(|| world.query_filtered());
        // Entity ids are left out: a load spawns in a varying order, so
        // they differ from run to run. The values are taken sorted.
        let mut values: Vec<(String, Entity)> = query
            .iter(world)
            .map(|(e, v)| (without_ids(&format!("{v:?}")), e))
            .collect();
        values.sort_unstable();
        let mut hasher = DefaultHasher::new();
        for (v, e) in &values {
            v.hash(&mut hasher);
            if let Some(out) = dump.as_deref_mut() {
                let _ = writeln!(out, "  {} {e} {v}", self.name());
            }
        }
        (hasher.finish(), values.len())
    }
}

struct ResourceDigest<T: Resource + Debug>(std::marker::PhantomData<fn() -> T>);

/// A digester of resource `T` (registered with the bridge).
pub fn resource<T: Resource + Debug>() -> Box<dyn Digester> {
    Box::new(ResourceDigest::<T>(std::marker::PhantomData))
}

impl<T: Resource + Debug> Digester for ResourceDigest<T> {
    fn name(&self) -> &'static str {
        type_name::<T>()
    }

    fn digest(&mut self, world: &mut World, dump: Option<&mut String>) -> (u64, usize) {
        let Some(value) = world.get_resource::<T>() else {
            return (0, 0);
        };
        let text = without_ids(&format!("{value:?}"));
        if let Some(out) = dump {
            let _ = writeln!(out, "  {} {text}", self.name());
        }
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        (hasher.finish(), 1)
    }
}

/// `text` with every entity id (`12v0`) replaced by `E`.
fn without_ids(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        let boundary = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
        let mut j = i;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        if boundary && j > i && j < bytes.len() && bytes[j] == b'v' {
            let mut k = j + 1;
            while k < bytes.len() && bytes[k].is_ascii_digit() {
                k += 1;
            }
            let end = k >= bytes.len() || !bytes[k].is_ascii_alphanumeric();
            if k > j + 1 && end {
                out.push('E');
                i = k;
                continue;
            }
        }
        let c = text[i..].chars().next().unwrap_or(' ');
        out.push(c);
        i += c.len_utf8();
    }
    out
}

/// The digest's schedule, after `FixedLast` in the fixed step's order.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SimDigestStep;

/// Where to write the digest (`--sim-digest`).
#[derive(Resource, Clone, Debug)]
pub struct SimDigestPath(pub PathBuf);

/// The open digest: moved to the simulation's world with the fixed step.
#[derive(Resource)]
pub struct SimDigest {
    out: BufWriter<File>,
    digesters: Vec<Box<dyn Digester>>,
    steps: u64,
    dump: Option<(u64, u64)>,
}

/// Writes the digest when a [`SimDigestPath`] is set.
pub struct SimDigestPlugin;

impl Plugin for SimDigestPlugin {
    fn build(&self, app: &mut App) {
        app.init_schedule(SimDigestStep)
            .add_systems(SimDigestStep, write_digest)
            .add_systems(PostStartup, open_digest)
            .bridge_move_resource::<SimDigest>();
        app.world_mut()
            .resource_mut::<FixedMainScheduleOrder>()
            .insert_after(FixedLast, SimDigestStep);
    }
}

fn open_digest(world: &mut World) {
    let Some(path) = world.get_resource::<SimDigestPath>().cloned() else {
        return;
    };
    let digesters = world
        .get_resource::<SimBridgeTypes>()
        .map(|t| t.digesters())
        .unwrap_or_default();
    let file = match File::create(&path.0) {
        Ok(f) => f,
        Err(e) => {
            error!("cannot write the digest to {}: {e}", path.0.display());
            return;
        }
    };
    let dump = std::env::var("EXA_DIGEST_DUMP").ok().and_then(|v| {
        let (a, b) = v.split_once('-').unwrap_or((&v, &v));
        Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
    });
    info!(
        "sim digest: {} types to {}",
        digesters.len(),
        path.0.display()
    );
    if world.contains_resource::<crate::sim_bridge::FrameBridge>() {
        let frame_path = path.0.with_extension("frame.tsv");
        if let (Ok(file), Some(types)) = (
            File::create(&frame_path),
            world.get_resource::<SimBridgeTypes>(),
        ) {
            let digest = FrameDigest(SimDigest {
                out: BufWriter::new(file),
                digesters: types.digesters(),
                steps: 0,
                dump,
            });
            world.insert_resource(digest);
        }
    }
    world.insert_resource(SimDigest {
        out: BufWriter::new(file),
        digesters,
        steps: 0,
        dump,
    });
}

fn write_digest(world: &mut World) {
    let Some(mut digest) = world.remove_resource::<SimDigest>() else {
        return;
    };
    write_into(world, &mut digest);
    world.insert_resource(digest);
}

/// Behind the bridge, the frame's own copy of the bridged types after each
/// step's copy out (`<file>.frame`): what the frame's systems will read.
#[derive(Resource)]
pub struct FrameDigest(SimDigest);

/// Writes the frame's digest after a lockstep step (a no-op without one).
pub fn write_frame_digest(world: &mut World) {
    let Some(mut digest) = world.remove_resource::<FrameDigest>() else {
        return;
    };
    write_into(world, &mut digest.0);
    world.insert_resource(digest);
}

fn write_into(world: &mut World, digest: &mut SimDigest) {
    digest.steps += 1;
    let step = digest.steps;
    let tick = world
        .get_resource::<crate::body::BodyTicks>()
        .map_or(0, |t| t.0);
    let dumping = digest.dump.is_some_and(|(a, b)| (a..=b).contains(&step));
    let mut text = String::new();
    let mut dump = String::new();
    for d in &mut digest.digesters {
        // The wall clock differs from run to run.
        if d.name().contains("time::Time<bevy_time::real::Real>") {
            continue;
        }
        let (hash, n) = d.digest(world, dumping.then_some(&mut dump));
        let _ = writeln!(text, "{step}\t{tick}\t{}\t{n}\t{hash:016x}", d.name());
    }
    let _ = digest.out.write_all(text.as_bytes());
    if dumping {
        let _ = digest.out.write_all(dump.as_bytes());
    }
    if step.is_multiple_of(100) {
        let _ = digest.out.flush();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn entity_ids_are_left_out() {
        assert_eq!(
            super::without_ids("Foo(12v0, x3v1, 2.5, [7v3])"),
            "Foo(E, x3v1, 2.5, [E])"
        );
    }
}
