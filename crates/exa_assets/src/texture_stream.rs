//! The texture loader's thread, on Bevy's task pool.
//!
//! The original decodes one queued image at a time on a loader thread, hands
//! the decoded image to the main thread through a pending-upload list, and
//! waits for that list to drain before it starts the next request (unless a
//! material promoted one). [`crate::stream::TextureStream`] holds those
//! rules; this module runs them: a request is decoded by a task on Bevy's
//! async compute pool, the main thread collects the result, and the image is
//! put under the placeholder handle the texture manager handed out when the
//! record was created. The upload pass drains every ready entry, as the
//! original's does; a frame's cost is one decode in the background and the
//! image insertions, not the decode itself.
//!
//! The manager's streaming path is off unless something turns it on
//! ([`crate::texture_cache::TextureCache::set_streaming`]); the running game
//! turns it on in [`crate::AssetsPlugin`]. Off, a loader decodes on the
//! calling thread.

use bevy::image::CompressedImageFormatSupport;
use bevy::prelude::*;
use std::collections::VecDeque;

use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, poll_once};

use crate::Game;
use crate::model::{self, ModelCache};
use crate::stream::{TextureRequest, TextureStream};
use crate::texture;

/// One decode running on the task pool: its request and its future.
type Decode = (TextureRequest, Task<Option<(Image, bool)>>);

/// The texture loader's queue and the decodes in flight.
#[derive(Resource)]
pub struct TextureLoader {
    stream: TextureStream,
    /// The decodes running on the task pool, oldest first.
    decoding: VecDeque<Decode>,
    /// How many decodes may run at once. The original's loader thread
    /// decodes one request at a time; Exarust pipelines up to this many so
    /// that a level's textures are ready sooner, and still uploads them in
    /// request order (only the oldest decode is collected, so the one
    /// decoded image the original's yield rule allows keeps its place).
    pub decodes_in_flight: usize,
    /// How many ready uploads one frame takes. The original's pump drains
    /// every entry that was ready when its pass started; this stays
    /// unbounded unless a caller needs to cap a frame's asset insertions.
    pub upload_budget: usize,
}

impl Default for TextureLoader {
    fn default() -> Self {
        Self {
            stream: TextureStream::new(),
            decoding: VecDeque::new(),
            decodes_in_flight: 4,
            upload_budget: usize::MAX,
        }
    }
}

impl TextureLoader {
    /// The loader's queue, for its state and its priority path.
    pub fn stream(&self) -> &TextureStream {
        &self.stream
    }

    pub fn stream_mut(&mut self) -> &mut TextureStream {
        &mut self.stream
    }

    /// How many decodes are running.
    pub fn decodes(&self) -> usize {
        self.decoding.len()
    }

    /// Whether a decode is running.
    pub fn is_decoding(&self) -> bool {
        !self.decoding.is_empty()
    }

    /// Whether the loader has nothing queued, decoding or waiting for its
    /// upload.
    pub fn is_idle(&self) -> bool {
        self.decoding.is_empty() && self.stream.is_idle()
    }
}

/// Runs the texture loader: hands the manager's queued requests to the
/// queue, starts decodes on the task pool, collects their images in request
/// order and pumps the ready uploads into the texture records' placeholder
/// handles.
pub struct TextureStreamPlugin;

impl Plugin for TextureStreamPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TextureLoader>()
            .add_systems(Update, run_texture_loader);
    }
}

fn run_texture_loader(
    mut cache: ResMut<ModelCache>,
    mut loader: ResMut<TextureLoader>,
    game: Option<Res<Game>>,
    support: Option<Res<CompressedImageFormatSupport>>,
    mut images: ResMut<Assets<Image>>,
) {
    // The manager's streaming path queues a load when it creates a record.
    for request in cache.textures.take_requests() {
        loader.stream.request(request);
    }
    let settings = *cache.textures.settings();
    let bc = model::bc_supported(support.as_deref());

    // Collect the oldest decode only, so the uploads stay in request order;
    // a finished decode behind it waits its turn.
    while let Some((_, task)) = loader.decoding.front_mut() {
        let Some(decoded) = block_on(poll_once(task)) else {
            break;
        };
        let (request, _) = loader.decoding.pop_front().expect("a front task");
        loader.stream.finish_decode(request, decoded);
    }

    // The loader's pass: one request at a time per free slot, and none
    // while the upload list is waiting on the main thread (the original's
    // yield rule).
    while loader.decoding.len() < loader.decodes_in_flight.max(1) && loader.stream.can_decode() {
        let Some(request) = loader.stream.take_next() else {
            break;
        };
        match (game.as_ref(), AsyncComputeTaskPool::try_get()) {
            (Some(game), Some(pool)) => {
                let data = game.0.clone();
                let source = request.source.clone();
                let (space, flags) = (request.space, request.flags);
                let task = pool.spawn(async move {
                    let bytes = data.texture(&source)?;
                    texture::decode_uploaded(bytes, space, flags, &settings, bc)
                });
                loader.decoding.push_back((request, task));
            }
            // No installation (a tool or a test without the game) or no pool:
            // the request fails and uploads the original's placeholder.
            (Some(game), None) => {
                let decoded = game.0.texture(&request.source).and_then(|b| {
                    texture::decode_uploaded(b, request.space, request.flags, &settings, bc)
                });
                loader.stream.finish_decode(request, decoded);
            }
            (None, _) => {
                loader.stream.finish_decode(request, None);
            }
        }
    }

    let ModelCache { textures, .. } = &mut *cache;
    let budget = loader.upload_budget;
    loader.stream.pump_uploads(budget, |upload| {
        let Some(record) = textures.get(&upload.key) else {
            return;
        };
        let Some(handle) = record.image.clone() else {
            return;
        };
        // The record's handle is the placeholder the manager handed out; the
        // decoded image takes its place, so every material that bound it
        // draws the real image from the next frame on.
        let _ = images.insert(handle.id(), upload.image);
        textures.finish(&upload.key, handle, upload.alpha);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::AssetApp;

    /// The streaming path decodes a shipped image on the task pool and
    /// replaces the placeholder under the record's handle (skipped without
    /// the game).
    #[test]
    fn a_streamed_texture_replaces_its_placeholder() {
        let Some(dir) = crate::locate_install(None) else {
            return;
        };
        let game = Game(std::sync::Arc::new(crate::GameData::open(dir).unwrap()));
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Image>()
            .init_resource::<ModelCache>()
            .init_resource::<TextureLoader>()
            .insert_resource(game)
            .add_systems(Update, run_texture_loader);
        app.world_mut()
            .resource_mut::<ModelCache>()
            .textures
            .set_streaming(true);
        let key = crate::TextureKey::new("spatter01", texture::ColorSpace::Srgb);
        let game = app.world().resource::<Game>().0.clone();
        let handle = app
            .world_mut()
            .resource_scope(
                |world: &mut World, mut cache: bevy::ecs::change_detection::Mut<ModelCache>| {
                    let mut images = world.resource_mut::<Assets<Image>>();
                    let mut meshes = Assets::<Mesh>::default();
                    let mut materials = Assets::<exa_render::material::ExaMaterial>::default();
                    let mut loader = crate::ModelLoader {
                        game: &game,
                        cache: &mut cache,
                        meshes: &mut meshes,
                        materials: &mut materials,
                        images: &mut images,
                        bc_supported: true,
                    };
                    loader
                        .texture(key.name.as_str(), texture::ColorSpace::Srgb)
                        .map(|(handle, _)| handle)
                },
            )
            .expect("the texture is shipped");
        let size_of = |app: &mut App, handle: &Handle<Image>| {
            app.world()
                .resource::<Assets<Image>>()
                .get(handle)
                .map(|image| image.texture_descriptor.size)
        };
        // Before the loader runs, the record draws the 1x1 placeholder.
        assert_eq!(size_of(&mut app, &handle).unwrap().width, 1);
        for _ in 0..600 {
            app.update();
            if size_of(&mut app, &handle).unwrap().width > 1 {
                break;
            }
        }
        let size = size_of(&mut app, &handle).unwrap();
        assert!(size.width > 1 && size.height > 1, "{size:?}");
        let loader = app.world().resource::<TextureLoader>();
        assert_eq!(loader.stream().queued(), 0);
        assert!(!loader.is_decoding());
        assert!(
            app.world()
                .resource::<ModelCache>()
                .textures
                .get(&key)
                .is_some()
        );
    }
}
