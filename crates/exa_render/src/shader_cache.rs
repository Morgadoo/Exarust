//! The original's program cache: one bucket of compiled programs per
//! material and shader word pair, one entry per object and environment word
//! pair, and the material's use of the cache as it draws.
//!
//! A draw interns its material and shader words (compared as one 64-bit
//! key: the bucket), then asks for the program
//! of its object and environment words; a program it has not built yet is
//! compiled from `shading_v` plus the set's `<set>_f` fragment entry and
//! stored in the bucket, whether the compile succeeds or not. The engine
//! keeps the bucket array and each bucket's entry array across frames; a
//! second use of the same word pair only raises the bucket's reference
//! count. Rebuilding the shader libraries (or changing a setting) frees every
//! program and empties the cache. The last bound program and the
//! "uniforms are stale" flag are part of the drawing state.
//!
//! Exarust's renderer runs one WESL program with run-time flag words, so Bevy
//! owns the pipelines; this module keeps the cache's rules and order for the
//! warm-up and for tests (`docs/RENDERING.md`, Shader builder).

use crate::shader_program::ProgramKey;

/// The four words a program is built from: the material and shader words
/// pick the bucket, the object and environment words the entry in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProgramWords {
    pub object: u32,
    pub environment: u32,
    pub material: u32,
    pub shader: u32,
}

impl ProgramWords {
    /// The words of a warm-up or material key.
    pub fn of(key: &ProgramKey) -> Self {
        Self {
            object: key.flags.object,
            environment: key.flags.environment,
            material: key.flags.material,
            shader: key.flags.shader,
        }
    }

    /// The entry key inside a bucket: the object word and the environment
    /// word, compared as one 64-bit pair.
    pub fn key(&self) -> (u32, u32) {
        (self.object, self.environment)
    }

    /// The bucket key: the material word and the shader word, compared as
    /// one 64-bit pair.
    pub fn bucket(&self) -> (u32, u32) {
        (self.material, self.shader)
    }
}

/// One compiled program of a bucket.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProgramEntry {
    pub object: u32,
    pub environment: u32,
    /// The program handle; 0 when the build failed (kept, so the next draw
    /// does not try again).
    pub program: u32,
}

/// The bucket of one material and shader word pair.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProgramBucket {
    pub material: u32,
    pub shader: u32,
    pub programs: Vec<ProgramEntry>,
    /// How many draws have interned this word (the original counts them and
    /// never reads the count back).
    pub references: u32,
}

/// The program cache and the drawing state that reads it.
#[derive(Clone, Debug, Default)]
pub struct ProgramCache {
    buckets: Vec<ProgramBucket>,
    active_bucket: Option<usize>,
    active_entry: Option<usize>,
    /// The program last bound (`state.latest_program`).
    bound: u32,
    /// The gathered lights changed since their records were uploaded.
    uniforms_stale: bool,
}

impl ProgramCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// The buckets, in slot order.
    pub fn buckets(&self) -> &[ProgramBucket] {
        &self.buckets
    }

    /// The bucket the last draw selected.
    pub fn active_bucket(&self) -> Option<&ProgramBucket> {
        self.active_bucket.and_then(|i| self.buckets.get(i))
    }

    /// The entry the last draw selected.
    pub fn active_entry(&self) -> Option<ProgramEntry> {
        let bucket = self.active_bucket?;
        let entry = self.active_entry?;
        self.buckets.get(bucket)?.programs.get(entry).copied()
    }

    /// The number of buckets in use.
    pub fn len(&self) -> usize {
        self.buckets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buckets.is_empty()
    }

    /// The program handle last bound.
    pub fn bound(&self) -> u32 {
        self.bound
    }

    /// Whether the light records must be re-uploaded before the next draw.
    pub fn uniforms_stale(&self) -> bool {
        self.uniforms_stale
    }

    /// Clear the stale flag, as a light upload does.
    pub fn take_uniforms_stale(&mut self) -> bool {
        std::mem::take(&mut self.uniforms_stale)
    }

    /// Intern a material and shader word pair: the index of its bucket.
    ///
    /// An existing bucket only raises its reference count. A new pair takes
    /// the first empty slot (both words zero) met while scanning, or is
    /// appended; an equal bucket further on still wins over the empty slot,
    /// because the scan keeps looking.
    pub fn intern(&mut self, material: u32, shader: u32) -> usize {
        let mut empty = None;
        for (index, bucket) in self.buckets.iter_mut().enumerate() {
            if (bucket.material, bucket.shader) == (material, shader) {
                bucket.references += 1;
                return index;
            }
            if empty.is_none() && (bucket.material, bucket.shader) == (0, 0) {
                empty = Some(index);
            }
        }
        let index = empty.unwrap_or_else(|| {
            self.buckets.push(ProgramBucket::default());
            self.buckets.len() - 1
        });
        let bucket = &mut self.buckets[index];
        bucket.material = material;
        bucket.shader = shader;
        bucket.programs.clear();
        bucket.references = 1;
        index
    }

    /// The program of `words` in the bucket of `key`, building it with
    /// `compile` when the bucket has no entry for its object and environment
    /// words.
    ///
    /// A build that fails (returns 0) is stored and returned as 0, so the
    /// caller draws nothing and does not retry.
    pub fn program(
        &mut self,
        key: usize,
        words: ProgramWords,
        compile: impl FnOnce(&ProgramWords) -> u32,
    ) -> u32 {
        let Some(bucket) = self.buckets.get_mut(key) else {
            return 0;
        };
        if let Some(index) = bucket
            .programs
            .iter()
            .position(|entry| (entry.object, entry.environment) == words.key())
        {
            self.active_bucket = Some(key);
            self.active_entry = Some(index);
            return bucket.programs[index].program;
        }
        let program = compile(&words);
        bucket.programs.push(ProgramEntry {
            object: words.object,
            environment: words.environment,
            program,
        });
        self.active_bucket = Some(key);
        self.active_entry = Some(bucket.programs.len() - 1);
        program
    }

    /// The program of `words` if the bucket already has it; a missing entry
    /// is not built.
    pub fn lookup(&mut self, key: usize, words: ProgramWords) -> Option<u32> {
        let bucket = self.buckets.get_mut(key)?;
        let index = bucket
            .programs
            .iter()
            .position(|entry| (entry.object, entry.environment) == words.key())?;
        self.active_bucket = Some(key);
        self.active_entry = Some(index);
        Some(bucket.programs[index].program)
    }

    /// Make `program` current. Returns whether it differs from the last one
    /// bound (the original's "program changed" flag).
    pub fn bind(&mut self, program: u32) -> bool {
        if program == self.bound {
            return false;
        }
        self.bound = program;
        self.uniforms_stale = true;
        true
    }

    /// Note that the gathered lights moved or changed.
    pub fn mark_uniforms_stale(&mut self) {
        self.uniforms_stale = true;
    }

    /// Free every program and empty the cache, as rebuilding the shader
    /// libraries does. The reference counts and the drawing state go too.
    pub fn clear(&mut self, mut release: impl FnMut(u32)) {
        for bucket in &mut self.buckets {
            for entry in &mut bucket.programs {
                release(entry.program);
            }
            bucket.programs.clear();
            bucket.material = 0;
            bucket.shader = 0;
            bucket.references = 0;
        }
        self.buckets.clear();
        self.active_bucket = None;
        self.active_entry = None;
        self.bound = 0;
        self.uniforms_stale = false;
    }

    /// The programs the cache holds.
    pub fn program_count(&self) -> usize {
        self.buckets.iter().map(|b| b.programs.len()).sum()
    }

    /// Build every key the warm-up lists, as the engine's warm-up step does,
    /// and report what the cache holds afterwards.
    pub fn warm(
        &mut self,
        keys: &[ProgramKey],
        mut compile: impl FnMut(&ProgramWords) -> u32,
    ) -> CacheStats {
        let mut stats = CacheStats::default();
        for key in keys {
            let words = ProgramWords::of(key);
            let bucket = self.intern(words.material, words.shader);
            stats.interned += 1;
            let before = self.buckets[bucket].programs.len();
            self.program(bucket, words, &mut compile);
            if self.buckets[bucket].programs.len() != before {
                stats.compiled += 1;
            } else {
                stats.reused += 1;
            }
        }
        stats.buckets = self.buckets.len();
        stats.programs = self.program_count();
        stats
    }
}

/// What a warm-up pass left in the cache.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CacheStats {
    /// Word pairs interned (the reference count is incremented once per
    /// key).
    pub interned: usize,
    /// Programs built.
    pub compiled: usize,
    /// Keys that found a program already built.
    pub reused: usize,
    pub buckets: usize,
    pub programs: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shader_program::{GeometryOptions, SceneOptions, program_keys, warmup_keys};
    use exa_formats::material::{Material, TextureLayer, TextureSlot};

    fn words(material: u32, object: u32, environment: u32) -> ProgramWords {
        ProgramWords {
            object,
            environment,
            material,
            shader: 1,
        }
    }

    #[test]
    fn interning_returns_one_bucket_per_word_and_counts_uses() {
        let mut cache = ProgramCache::new();
        assert_eq!(cache.intern(0x122_0007, 1), 0);
        assert_eq!(cache.intern(0x1A2_0007, 1), 1);
        assert_eq!(cache.intern(0x122_0007, 1), 0);
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.buckets()[0].references, 2);
        assert_eq!(cache.buckets()[1].references, 1);
        // A pair that is not a bucket is not confused with the empty slot's
        // zero words.
        assert_eq!(cache.intern(0, 0), 2);
        assert_eq!(cache.buckets()[2].material, 0);
        // The empty bucket is reused for a new pair before appending.
        assert_eq!(cache.intern(0x55, 1), 2);
        assert_eq!(cache.len(), 3);
    }

    #[test]
    fn the_shader_word_is_part_of_the_bucket_key() {
        // The engine compares the material and shader words as one 64-bit
        // key, so the same material under another shader word is another
        // bucket with its own programs.
        let mut cache = ProgramCache::new();
        assert_eq!(cache.intern(0x7, 0x1), 0);
        assert_eq!(cache.intern(0x7, 0x101), 1);
        assert_eq!(cache.intern(0x7, 0x1), 0);
        assert_eq!(cache.buckets()[0].references, 2);
        let built = std::cell::Cell::new(0u32);
        cache.program(0, words(0x7, 0, 0), |_| {
            built.set(built.get() + 1);
            1
        });
        cache.program(1, words(0x7, 0, 0), |_| {
            built.set(built.get() + 1);
            2
        });
        assert_eq!(built.get(), 2);
    }

    #[test]
    fn an_empty_slot_does_not_beat_an_equal_bucket_further_on() {
        let mut cache = ProgramCache::new();
        let first = cache.intern(0x10, 1);
        cache.buckets[first].material = 0; // as a cleared slot looks
        cache.buckets[first].shader = 0;
        assert_eq!(cache.intern(0x20, 1), 0);
        assert_eq!(cache.intern(0x30, 1), 1);
        assert_eq!(cache.buckets[0].material, 0x20);
        assert_eq!(cache.buckets[1].material, 0x30);
    }

    #[test]
    fn a_program_is_built_once_per_object_and_environment_pair() {
        let mut cache = ProgramCache::new();
        let key = cache.intern(3, 1);
        let builds = std::cell::Cell::new(0u32);
        let build = || {
            builds.set(builds.get() + 1);
            builds.get() * 10
        };
        let a = cache.program(key, words(3, 0x1000, 0x10), |_| build());
        assert_eq!(a, 10);
        let b = cache.program(key, words(3, 0x1000, 0x10), |_| build());
        assert_eq!(b, 10);
        assert_eq!(builds.get(), 1);
        // A different environment word is another program under the same
        // material.
        let c = cache.program(key, words(3, 0x1000, 0x1100), |_| build());
        assert_eq!(c, 20);
        assert_eq!(cache.buckets()[key].programs.len(), 2);
        assert_eq!(
            cache.active_entry(),
            Some(ProgramEntry {
                object: 0x1000,
                environment: 0x1100,
                program: 20
            })
        );
    }

    #[test]
    fn a_failed_build_is_kept_and_not_retried() {
        let mut cache = ProgramCache::new();
        let key = cache.intern(7, 1);
        let builds = std::cell::Cell::new(0u32);
        let fail = || {
            builds.set(builds.get() + 1);
            0
        };
        assert_eq!(cache.program(key, words(7, 1, 2), |_| fail()), 0);
        assert_eq!(cache.program(key, words(7, 1, 2), |_| fail()), 0);
        assert_eq!(builds.get(), 1);
        assert_eq!(cache.buckets()[key].programs.len(), 1);
        assert_eq!(cache.lookup(key, words(7, 1, 2)), Some(0));
        assert_eq!(cache.lookup(key, words(7, 9, 9)), None);
    }

    #[test]
    fn binding_marks_the_uniforms_stale_only_on_a_change() {
        let mut cache = ProgramCache::new();
        assert!(cache.bind(5));
        assert_eq!(cache.bound(), 5);
        assert!(cache.take_uniforms_stale());
        assert!(!cache.take_uniforms_stale());
        assert!(!cache.bind(5));
        assert!(!cache.uniforms_stale());
        assert!(cache.bind(6));
        assert_eq!(cache.bound(), 6);
    }

    #[test]
    fn clearing_releases_every_program_and_forgets_the_buckets() {
        let mut cache = ProgramCache::new();
        let key = cache.intern(1, 1);
        cache.program(key, words(1, 0, 0), |_| 11);
        cache.program(key, words(1, 1, 0), |_| 22);
        let key2 = cache.intern(2, 1);
        cache.program(key2, words(2, 0, 0), |_| 33);
        cache.bind(22);
        let mut released = Vec::new();
        cache.clear(|id| released.push(id));
        released.sort_unstable();
        assert_eq!(released, vec![11, 22, 33]);
        assert!(cache.is_empty());
        assert_eq!(cache.bound(), 0);
        assert_eq!(cache.program_count(), 0);
        assert!(cache.active_entry().is_none());
        assert!(!cache.uniforms_stale());
    }

    fn material() -> Material {
        Material {
            kind: 1,
            flags: 0x7 | 0x20_000 | 0x4_0000,
            class: 0x100,
            opacity: 1.0,
            layers: vec![
                TextureLayer {
                    slot: TextureSlot::Diffuse,
                    texture: "d".into(),
                    parameter: 0,
                    amount: 0.0,
                },
                TextureLayer {
                    slot: TextureSlot::Specular,
                    texture: "s".into(),
                    parameter: 0,
                    amount: 0.0,
                },
            ],
            ..Material::default()
        }
    }

    #[test]
    fn the_warm_up_builds_one_program_per_row_and_light_count() {
        let keys: Vec<ProgramKey> = warmup_keys(5, 4, 3, false)
            .into_iter()
            .map(|key| key.key)
            .collect();
        assert_eq!(keys.len(), 29 * 7);
        let mut cache = ProgramCache::new();
        let stats = cache.warm(&keys, |_| 99);
        // The bucket is the material and shader word pair and the entry the
        // object and environment pair, so a row that repeats an earlier
        // row's material under another shader word builds its own programs.
        let mut triples = std::collections::HashSet::new();
        let mut materials = std::collections::HashSet::new();
        for key in &keys {
            let f = key.flags;
            materials.insert((f.material, f.shader));
            triples.insert((f.material, f.shader, f.object, f.environment));
        }
        assert_eq!(stats.interned, 29 * 7);
        assert_eq!(stats.compiled, triples.len());
        assert_eq!(stats.reused, keys.len() - triples.len());
        assert_eq!(stats.buckets, materials.len());
        assert_eq!(stats.programs, triples.len());
        assert!(stats.buckets < 29);
    }

    #[test]
    fn equal_rows_share_a_bucket_and_a_program() {
        let scene = SceneOptions::default();
        let keys = program_keys(&material(), GeometryOptions::default(), &scene);
        let mut cache = ProgramCache::new();
        let stats = cache.warm(&keys, |_| 1);
        assert_eq!(stats.interned, 7);
        assert_eq!(stats.buckets, 1);
        assert_eq!(stats.compiled, 7);
        // The same keys again only raise the bucket's reference count.
        let again = cache.warm(&keys, |_| 2);
        assert_eq!(again.compiled, 0);
        assert_eq!(again.reused, 7);
        assert_eq!(cache.buckets()[0].references, 14);
    }
}
