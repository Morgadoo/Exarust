//! Welding duplicate stream elements and compacting a mesh's channels, as
//! the original's mesh tools do it before a mesh is converted to render
//! form.
//!
//! The original runs this on character meshes after skinning and on merged
//! item and grid meshes: every stream that owns its channel sorts its
//! elements by their first component, merges the later element into an
//! earlier one when their values lie within a small distance, and then
//! renumbers the surviving elements from zero in their old order. The
//! positions stream's compaction also rebuilds the incidence; streams that
//! share its channel (colours, skin weights) take its new count, and a
//! character pass folds the skin weights of a welded-away position into
//! the survivor.
//!
//! Lengths are metres here and centimetres in the original. The two
//! tolerances are converted: its 0.01 cm step is 1e-4 m and its 0.0001 cm²
//! squared distance is 1e-8 m², compared from the single-precision sum as
//! the original converts its result to double.
//!
//! Evidence: the original's mesh-tools unit (the weld, its element-size
//! helper, the channel compaction and the skin-influence merge), read
//! against the decompilation and its assembly.

use exa_formats::rfc::SkinWeights;

use crate::mesh_edit::{
    Corner, EditMesh, FIXED_STREAM, Incidence, KEY_MASK, MeshError, SHARES_CHANNEL, Smoothing,
    StreamData,
};

/// The squared distance under which two positions (a 0.01 cm reach) weld.
const POSITION_REACH_SQUARED: f64 = 0.0001;
/// The squared distance under which two texture coordinates weld.
const TEXTURE_REACH_SQUARED: f64 = 1e-9;
/// The difference in the sort key past which no later element can weld
/// (0.01 cm, in metres).
const POSITION_KEY_STEP: f32 = 0.01 * 0.01;
/// The same for texture coordinates (0.001).
const TEXTURE_KEY_STEP: f32 = 0.001;
/// How many times a metre fits a centimetre squared, for the comparison of
/// the single-precision squared distance with the original's centimetres.
const CM2_PER_M2: f64 = 10_000.0;
/// The skin format's low key word: its elements are influence weights.
const SKIN_KEY: u32 = 0x1000;

/// The stream's element size in bytes, as the original's helper computes
/// it from the format word: the component count times a component's size
/// (8 bytes for the skin word, 1 for a colour byte, 4 for a float).
pub fn stream_element_size(format: u32) -> usize {
    let unit = match format & 0x00F0_0000 {
        0x00A0_0000 => 8,
        0x00B0_0000 => 1,
        0x00F0_0000 => 4,
        _ => 0,
    };
    (((format & 0x000F_0000) >> 16) as usize) * unit
}

/// The channels the source ordinates of a stream are sorted by, when its
/// elements can weld at all.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Vec3,
    Vec2,
    Other,
}

fn kind_of(data: &StreamData) -> Kind {
    match data {
        StreamData::Vec3(_) => Kind::Vec3,
        StreamData::Vec2(_) => Kind::Vec2,
        _ => Kind::Other,
    }
}

/// Moves the element at `from` to `to` (the destination is never past the
/// source, so a forward walk is safe).
fn move_element(data: &mut StreamData, from: usize, to: usize) {
    if from == to {
        return;
    }
    match data {
        StreamData::Vec3(v) => {
            if let Some(x) = v.get(from).copied()
                && let Some(slot) = v.get_mut(to)
            {
                *slot = x;
            }
        }
        StreamData::Vec2(v) => {
            if let Some(x) = v.get(from).copied()
                && let Some(slot) = v.get_mut(to)
            {
                *slot = x;
            }
        }
        StreamData::Rgba8(v) => {
            if let Some(x) = v.get(from).copied()
                && let Some(slot) = v.get_mut(to)
            {
                *slot = x;
            }
        }
        StreamData::Skin(v) => {
            if let Some(x) = v.get(from).copied()
                && let Some(slot) = v.get_mut(to)
            {
                *slot = x;
            }
        }
    }
}

/// Compacts a stream's element values by `remap` (old element to new, -1
/// for a dead one) and truncates it to `count`.
fn compact_data(data: &mut StreamData, remap: &[i64], count: usize) {
    for (old, &to) in remap.iter().enumerate() {
        if 0 <= to {
            move_element(data, old, to as usize);
        }
    }
    data.resize(count);
}

/// The sort key an element is ordered by: its first component. The
/// texture-coordinate accessor sorts by x too (its separate helper differs
/// from the vec3 one only in the element stride); that is a guess, since
/// the tiny accessor at the start of the tool unit is not part of the
/// exported functions.
fn key(data: &StreamData, element: usize) -> f32 {
    match data {
        StreamData::Vec3(v) => v.get(element).map_or(0.0, |e| e[0]),
        StreamData::Vec2(v) => v.get(element).map_or(0.0, |e| e[0]),
        _ => 0.0,
    }
}

/// Whether two elements lie within the stream's weld distance.
fn within(data: &StreamData, a: usize, b: usize) -> bool {
    match data {
        StreamData::Vec3(v) => {
            let (Some(p), Some(q)) = (v.get(a), v.get(b)) else {
                return false;
            };
            let d = (p[0] - q[0]) * (p[0] - q[0])
                + (p[1] - q[1]) * (p[1] - q[1])
                + (p[2] - q[2]) * (p[2] - q[2]);
            f64::from(d) * CM2_PER_M2 < POSITION_REACH_SQUARED
        }
        StreamData::Vec2(v) => {
            let (Some(p), Some(q)) = (v.get(a), v.get(b)) else {
                return false;
            };
            let d = (p[0] - q[0]) * (p[0] - q[0]) + (p[1] - q[1]) * (p[1] - q[1]);
            f64::from(d) < TEXTURE_REACH_SQUARED
        }
        _ => false,
    }
}

/// Sorts `order` by the stream's key with the original's middle-pivot
/// quicksort (not stable: the order of equal keys is the algorithm's).
fn sort_order(order: &mut [u32], data: &StreamData) {
    let n = order.len();
    if n < 2 {
        return;
    }
    let mut stack = vec![(0i64, n as i64 - 1)];
    while let Some((lo, hi)) = stack.pop() {
        if lo >= hi {
            continue;
        }
        let pivot = key(data, order[((lo + hi) / 2) as usize] as usize);
        let (mut i, mut j) = (lo, hi);
        loop {
            while i <= hi && key(data, order[i as usize] as usize) < pivot {
                i += 1;
            }
            while lo <= j && pivot < key(data, order[j as usize] as usize) {
                j -= 1;
            }
            if i > j {
                break;
            }
            order.swap(i as usize, j as usize);
            i += 1;
            j -= 1;
            if i > j {
                break;
            }
        }
        // The original recurses into the left run first; the runs are
        // disjoint, so their order does not matter.
        if i < hi {
            stack.push((i, hi));
        }
        if lo < j {
            stack.push((lo, j));
        }
    }
}

/// Splits the rings into the surviving elements: sorts the element indices
/// by key, merges every later element within reach of an earlier live one
/// into it (appending its ring), and returns the rings with merged ones
/// emptied and the old-to-survivor map (-1 when nothing merged it).
fn merge_rings(data: &StreamData, rings: &mut [Vec<Corner>]) -> Vec<i64> {
    let mut remap = vec![-1i64; rings.len()];
    if kind_of(data) == Kind::Other || rings.is_empty() {
        return remap;
    }
    let mut order: Vec<u32> = (0..rings.len() as u32).collect();
    sort_order(&mut order, data);
    let step = if kind_of(data) == Kind::Vec3 {
        POSITION_KEY_STEP
    } else {
        TEXTURE_KEY_STEP
    };
    for oi in 0..order.len() {
        let a = order[oi] as usize;
        if rings[a].is_empty() {
            continue;
        }
        for &b in &order[oi + 1..] {
            let b = b as usize;
            if rings[b].is_empty() {
                continue;
            }
            if within(data, a, b) {
                let head = rings[a][rings[a].len() - 1];
                let mut joined = Vec::with_capacity(rings[a].len() + rings[b].len());
                joined.extend_from_slice(&rings[a][..rings[a].len() - 1]);
                joined.extend_from_slice(&rings[b]);
                joined.push(head);
                rings[a] = joined;
                rings[b].clear();
                remap[b] = a as i64;
            } else if key(data, a) < key(data, b) - step {
                break;
            }
        }
    }
    remap
}

/// Renumbers the live rings from zero in old element order and rewrites a
/// channel's corners. Returns the old-to-new map (-1 for dead elements)
/// and, when `master`, the rebuilt incidence (the rings' corner order).
fn compact_channel(
    mesh: &mut EditMesh,
    channel: usize,
    rings: &[Vec<Corner>],
    master: bool,
) -> (Vec<i64>, Option<Incidence>) {
    let mut corners = mesh.channels[channel].corners.clone();
    let mut remap = vec![-1i64; rings.len()];
    let mut flat = Vec::new();
    let mut starts = vec![0u32];
    let mut next = 0u32;
    for (old, ring) in rings.iter().enumerate() {
        if ring.is_empty() {
            continue;
        }
        remap[old] = i64::from(next);
        for c in ring {
            if let Some(t) = corners.get_mut(c.face as usize)
                && let Some(v) = t.get_mut(c.corner as usize)
            {
                *v = next;
            }
            flat.push(*c);
        }
        next += 1;
        starts.push(flat.len() as u32);
    }
    mesh.channels[channel].corners = corners;
    let inc = master.then_some(Incidence {
        corners: flat,
        starts,
    });
    (remap, inc)
}

/// Folds a welded-away skin element's influences into its survivor: an
/// influence already there keeps the larger weight, the rest fill the
/// survivor's free slots up to four, and every element's weights are then
/// divided by their sum when it has more than one slot.
fn merge_influences_into(dest: &mut SkinWeights, src: &SkinWeights) {
    let Some(slot) = (0..4).find(|&k| (dest.bones[k] as i32) < 1) else {
        return;
    };
    let mut write = slot;
    for s in 0..4 {
        if (src.bones[s] as i32) < 1 {
            break;
        }
        let mut found = false;
        for t in 0..slot {
            if dest.bones[t] == src.bones[s] {
                if dest.weights[t] <= src.weights[s] && src.weights[s] != dest.weights[t] {
                    dest.weights[t] = src.weights[s];
                }
                found = true;
                break;
            }
        }
        if !found && (src.bones[s] as i32) > 0 {
            dest.bones[write] = src.bones[s];
            dest.weights[write] = src.weights[s];
            write += 1;
            if write > 3 {
                break;
            }
        }
    }
}

/// Divides an element's weights by their sum, from the second slot on.
fn normalise_influences(w: &mut SkinWeights) {
    let mut sum = 0.0f32;
    let mut last: i32 = -1;
    for k in 0..4 {
        if (w.bones[k] as i32) < 1 {
            break;
        }
        sum += w.weights[k];
        last = k as i32;
    }
    if last > 0 {
        let inv = 1.0 / sum;
        for x in w.weights.iter_mut().take(last as usize + 1) {
            *x *= inv;
        }
    }
}

/// Merges the influences of every element the position weld removed into
/// its survivor, then normalises every element.
fn merge_influences(data: &mut StreamData, remap: &[i64]) {
    let StreamData::Skin(weights) = data else {
        return;
    };
    for old in 0..weights.len() {
        let Some(&survivor) = remap.get(old) else {
            continue;
        };
        if survivor < 0 {
            continue;
        }
        let dest = survivor as usize;
        if dest >= weights.len() || dest == old {
            continue;
        }
        let src = weights[old];
        merge_influences_into(&mut weights[dest], &src);
    }
    for w in weights.iter_mut() {
        normalise_influences(w);
    }
}

impl EditMesh {
    /// Welds duplicate elements of every stream and compacts the channels,
    /// as the original does on merged item and grid meshes: positions and
    /// texture coordinates within 0.01 cm weld, streams sharing a channel
    /// are compacted with their owner. Influences are not merged; see
    /// [`EditMesh::weld_influences`] for the character pass.
    pub fn weld(&mut self) -> Result<(), MeshError> {
        self.weld_impl(false)
    }

    /// The same weld with the character pass's skin-weight merge: the
    /// influences of every welded-away position are folded into the
    /// position that absorbed it, then the stream takes the new count.
    pub fn weld_influences(&mut self) -> Result<(), MeshError> {
        self.weld_impl(true)
    }

    fn weld_impl(&mut self, merge_skin: bool) -> Result<(), MeshError> {
        self.validate()?;
        self.incidence = None;
        let mut position_remap: Option<Vec<i64>> = None;
        for i in 0..self.streams.len() {
            if self.streams[i].id & SHARES_CHANNEL != 0 {
                continue;
            }
            let channel = self.streams[i].channel;
            let elements = self.streams[i].data.len();
            let inc = Incidence::build(&self.channels[channel].corners, elements)?;
            let mut rings: Vec<Vec<Corner>> = (0..elements)
                .map(|v| inc.corners[inc.of(v)].to_vec())
                .collect();
            // A fixed stream keeps every element; the rings still compact.
            let welded = if self.streams[i].format & FIXED_STREAM != 0 {
                vec![-1i64; elements]
            } else {
                merge_rings(&self.streams[i].data, &mut rings)
            };
            if i == 0 {
                position_remap = Some(welded);
            }
            let (remap, rebuilt) = compact_channel(self, channel, &rings, i == 0);
            let count = remap.iter().filter(|&&r| r >= 0).count();
            compact_data(&mut self.streams[i].data, &remap, count);
            if let Some(incidence) = rebuilt {
                self.incidence = Some(incidence);
            } else if channel == 0 {
                self.incidence = None;
            }
            // Streams that share the channel take its new element count.
            let shares: Vec<usize> = (0..self.streams.len())
                .filter(|&k| {
                    self.streams[k].id & SHARES_CHANNEL != 0 && self.streams[k].channel == channel
                })
                .collect();
            let owner_count = self.streams[i].data.len();
            for k in shares {
                if merge_skin
                    && self.streams[k].format & KEY_MASK == SKIN_KEY
                    && let Some(position_remap) = &position_remap
                {
                    merge_influences(&mut self.streams[k].data, position_remap);
                }
                compact_data(&mut self.streams[k].data, &remap, owner_count);
            }
        }
        self.smoothing = Smoothing::Unset;
        Ok(())
    }
}

#[cfg(test)]
#[path = "mesh_weld_test.rs"]
mod tests;
