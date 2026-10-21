//! The runtime race: the per-tick gait table and the voice templates a
//! character's body and sounds take from its race.
//!
//! The race database (`races.rdb`, [`exa_formats::races`]) stores gaits per
//! second and second squared; the original's race build converts them once
//! per race into the locomotion routine's per-tick rows (1/83, 1/83²), puts
//! the loader's built-in brake distances in column 2, leaves the boost as
//! stored and negates the backpedal row's thrust and clock. It also keeps
//! the two gender voice templates, from which each (voice index, gender,
//! slot) sound set's four names are built on demand.
//!
//! The built-in human stands in for an id the database lacks, as the
//! original's race lookup falls back. Evidence: REA on the race loader,
//! the runtime-race build and the voice-set builders; values checked
//! against the assembly.

use std::collections::HashMap;
use std::sync::Arc;

use exa_formats::races::{Race, RaceVoice, VoiceNames, VoiceSlot};

use crate::body::locomotion::GaitRow;

/// A race's runtime data: four per-tick gait rows and its voice templates.
#[derive(Debug, Clone, PartialEq)]
pub struct RaceRuntime {
    pub id: u32,
    pub rows: [GaitRow; 4],
    pub voice: RaceVoice,
}

impl RaceRuntime {
    /// Builds the runtime race: each gait row scaled by the loader's
    /// per-tick factors ([`GaitRow::scaled`]), the backpedal's thrust and
    /// clock negated.
    pub fn of(race: &Race) -> Self {
        Self {
            id: race.id,
            rows: rows_of(&race.body),
            voice: race.voice.clone(),
        }
    }

    /// The four sound names of voice `index` for a gender and template,
    /// falling back between the groans when one is missing.
    pub fn voice_names(
        &self,
        index: u32,
        female: bool,
        slot: VoiceSlot,
        exists: impl Fn(&str) -> bool,
    ) -> VoiceNames {
        self.voice.names(index, female, slot, exists)
    }
}

/// The four per-tick gait rows of a race's body parameters.
pub fn rows_of(body: &exa_formats::races::RaceBody) -> [GaitRow; 4] {
    std::array::from_fn(|i| {
        let g = body.gaits[i];
        GaitRow::scaled(i, g.thrust, g.push_scale, g.clip_rate, g.boost_below)
    })
}

/// The race runtimes, built once per race id (the original's cache).
#[derive(Debug, Clone)]
pub struct RaceCache {
    runtimes: HashMap<u32, Arc<RaceRuntime>>,
    builtin: Arc<RaceRuntime>,
}

impl Default for RaceCache {
    fn default() -> Self {
        Self::new()
    }
}

impl RaceCache {
    /// The cache with the built-in human, which a missing id takes.
    pub fn new() -> Self {
        Self {
            runtimes: HashMap::new(),
            builtin: Arc::new(RaceRuntime::of(&exa_formats::races::builtin_human())),
        }
    }

    /// The runtime of `race`, built the first time.
    pub fn runtime(&mut self, race: &Race) -> Arc<RaceRuntime> {
        self.runtimes
            .entry(race.id)
            .or_insert_with(|| Arc::new(RaceRuntime::of(race)))
            .clone()
    }

    /// The runtime of the race with `id`, or the built-in human's.
    pub fn by_id(&mut self, db: &exa_formats::races::RaceDb, id: u32) -> Arc<RaceRuntime> {
        let race = db.get(id);
        if race.id == 0 {
            return self.builtin.clone();
        }
        self.runtime(race)
    }

    /// The built-in human's runtime.
    pub fn human(&self) -> Arc<RaceRuntime> {
        self.builtin.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_formats::races::{Gait, RaceBody, RaceDb, RaceStats};

    fn gait(thrust: f32, push: f32, clip: f32, boost: f32) -> Gait {
        Gait {
            thrust,
            push_scale: push,
            brake_distance: 0.0,
            clip_rate: clip,
            boost_below: boost,
        }
    }

    fn race() -> Race {
        let mut body = RaceBody::FALLBACK;
        body.gaits = [
            gait(83.0, 6890.0, 83.0, 1.0),
            gait(166.0, 0.0, 166.0, 2.0),
            gait(249.0, 0.0, 249.0, 3.0),
            gait(83.0, 0.0, 83.0, 4.0),
        ];
        Race {
            id: 3,
            name: "Test".into(),
            flags: 0,
            stem: "test".into(),
            movesets: ["Default".into(), "Default".into()],
            health: 1.0,
            natural: Default::default(),
            body,
            stats: RaceStats::FALLBACK,
            voice: RaceVoice::default(),
        }
    }

    /// The loader's per-tick rows: 1/83 and 1/83², the built-in stops and
    /// the backpedal's signs.
    #[test]
    fn the_runtime_rows_are_per_tick() {
        let runtime = RaceRuntime::of(&race());
        let walk = runtime.rows[0];
        assert!((walk.thrust - 1.0).abs() < 1e-6);
        assert!((walk.push - 6890.0 / (83.0 * 83.0)).abs() < 1e-6);
        assert!((walk.clock - 1.0).abs() < 1e-6);
        assert_eq!(walk.stop, 0.05);
        assert_eq!(walk.boost, 1.0);
        let sprint = runtime.rows[2];
        assert!((sprint.thrust - 3.0).abs() < 1e-6);
        assert_eq!(sprint.stop, 0.25);
        let back = runtime.rows[3];
        assert!((back.thrust + 1.0).abs() < 1e-6);
        assert!((back.clock + 1.0).abs() < 1e-6);
        assert_eq!(back.stop, 0.05);
        assert_eq!(back.boost, 4.0);
    }

    /// The cache builds a race once and falls back on the built-in human.
    #[test]
    fn the_cache_builds_once_and_falls_back() {
        let race = race();
        let mut cache = RaceCache::new();
        let a = cache.runtime(&race);
        let b = cache.runtime(&race);
        assert!(Arc::ptr_eq(&a, &b));
        assert_eq!(a.id, 3);
        let db = RaceDb::new(vec![race]);
        assert!(Arc::ptr_eq(&cache.by_id(&db, 3), &a));
        let human = cache.by_id(&db, 99);
        assert_eq!(human.id, 0);
        assert!(Arc::ptr_eq(&human, &cache.human()));
    }

    /// A voice set from the runtime's templates numbers and shares as the
    /// race data says.
    #[test]
    fn the_runtime_builds_the_voice_names() {
        let mut race = race();
        race.voice = RaceVoice {
            first: exa_formats::races::GenderedName {
                male: "humanm".into(),
                female: "humanf".into(),
            },
            second: exa_formats::races::GenderedName {
                male: "goblin".into(),
                female: "goblin".into(),
            },
            first_numbered: true,
            second_numbered: false,
        };
        let runtime = RaceRuntime::of(&race);
        let names = runtime.voice_names(2, true, VoiceSlot::First, |_| true);
        assert_eq!(names.grunt, "humanf03_GruntS");
        let names = runtime.voice_names(2, false, VoiceSlot::Second, |_| true);
        assert_eq!(names.grunt, "goblin_GruntS");
    }
}
