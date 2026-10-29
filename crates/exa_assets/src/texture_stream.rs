//! The texture loader's thread, on Bevy's task pool.
//!
//! The original decodes one queued image at a time on a loader thread and
//! hands the decoded image to the main thread through a pending-upload list.
//! When it has decoded an image while earlier uploads are still waiting
//! (and no material promoted it), it suspends itself until the main thread's
//! upload pass wakes it. [`crate::stream::TextureStream`] holds those rules;
//! this module runs them: a request is decoded by a task on Bevy's async compute pool, the
//! main thread collects the result, and the image is put under the
//! placeholder handle the texture manager handed out when the record was
//! created. The upload pass drains every ready entry, as the original's
//! does; a frame's cost is the decodes in the background and the image
//! insertions, not the decode itself. Each frame collects the finished
//! decodes, uploads them, and then starts the next decodes.
//!
//! A replaced placeholder is a new image under the same handle: materials
//! and shadow cuts that bound the placeholder are touched or rebuilt so they
//! draw the real one; a caller that needs the real size at once (parallax
//! depth, terrain and particle materials) decodes on its own thread with
//! `ModelLoader::texture_sync`.
//!
//! The manager's streaming path is off unless something turns it on
//! ([`crate::texture_cache::TextureCache::set_streaming`]); the running game
//! turns it on in [`crate::AssetsPlugin`]. Off, a loader decodes on the
//! calling thread.

use bevy::image::CompressedImageFormatSupport;
use bevy::prelude::*;
use std::collections::{HashSet, VecDeque};

use exa_render::material::ExaMaterial;

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

    /// Waits for the decodes in flight and collects them, oldest first, so
    /// their uploads are queued in request order: the original's wait for
    /// the loader before a caller needs a texture. The caller drains the
    /// uploads after it ([`TextureStream::pump_uploads`]).
    pub fn wait_decodes(&mut self) {
        while let Some((request, task)) = self.decoding.pop_front() {
            let decoded = block_on(task);
            self.stream.finish_decode(request, decoded);
        }
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
            .add_systems(Update, (run_texture_loader, rebind_replaced_images).chain());
    }
}

/// Touches the materials that bind an image whose contents were replaced.
///
/// A material's bind group keeps the texture view it was prepared with, so a
/// material built while its texture was the 1x1 placeholder would go on
/// drawing the placeholder after the loader's upload replaced the image
/// under the same handle. Touching the material (taking it for writing) makes Bevy
/// prepare its bind group again with the new image. The scan runs only on a
/// frame in which some image was modified, and looks at each material's
/// handles once.
fn rebind_replaced_images(
    mut events: MessageReader<AssetEvent<Image>>,
    materials: Option<ResMut<Assets<ExaMaterial>>>,
    mut flames: Option<ResMut<Assets<exa_render::flame::FlameMaterial>>>,
) {
    let replaced: HashSet<AssetId<Image>> = events
        .read()
        .filter_map(|event| match event {
            AssetEvent::Modified { id } => Some(*id),
            _ => None,
        })
        .collect();
    if replaced.is_empty() {
        return;
    }
    if let Some(flames) = flames.as_mut() {
        // The flame material binds its particle texture directly, so a
        // streamed fire texture rebuilds its bind group the same way.
        let touched: Vec<AssetId<exa_render::flame::FlameMaterial>> = flames
            .iter()
            .filter(|(_, material)| replaced.contains(&material.texture.id()))
            .map(|(id, _)| id)
            .collect();
        for id in touched {
            let _ = flames.get_mut(id).map(bevy::asset::AssetMut::into_inner);
        }
    }
    let Some(mut materials) = materials else {
        return;
    };
    let touched: Vec<AssetId<ExaMaterial>> = materials
        .iter()
        .filter(|(_, material)| binds_any(material, &replaced))
        .map(|(id, _)| id)
        .collect();
    for id in touched {
        // Taking the asset for writing is what queues its modified event.
        let _ = materials.get_mut(id).map(bevy::asset::AssetMut::into_inner);
    }
}

/// Whether the material binds one of the images.
fn binds_any(material: &ExaMaterial, images: &HashSet<AssetId<Image>>) -> bool {
    [
        &material.diffuse_map,
        &material.normal_map,
        &material.specular_map,
        &material.emissive_map,
        &material.opacity_map,
        &material.dirt_map,
        &material.gloss_map,
        &material.height_map,
    ]
    .into_iter()
    .flatten()
    .any(|handle| images.contains(&handle.id()))
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

    pump_uploads(&mut loader, &mut cache, &mut images);

    // The loader's pass: one request at a time per free slot, and none
    // while it waits for an upload pass (it decoded a request behind
    // waiting uploads). The pump above has just run that pass, so the
    // loader only waits when a capped upload budget left entries behind.
    while loader.decoding.len() < loader.decodes_in_flight.max(1) && loader.stream.can_decode() {
        let Some(request) = loader.stream.take_next() else {
            break;
        };
        match (game.as_ref(), AsyncComputeTaskPool::try_get()) {
            (Some(game), Some(pool)) => {
                let data = game.0.clone();
                let source = request.source.clone();
                let pack = request.pack;
                let (space, flags) = (request.space, request.flags);
                let task = pool.spawn(async move {
                    // The load keeps the pack its record was found in, as the
                    // original keeps the provider index.
                    let bytes = match pack {
                        Some(kind) => data.pack(kind).and_then(|p| p.get(&source)),
                        None => data.texture(&source),
                    }?;
                    texture::decode_entry(&source, bytes, space, flags, &settings, bc)
                });
                loader.decoding.push_back((request, task));
            }
            // No installation (a tool or a test without the game) or no pool:
            // the request fails and uploads the original's placeholder.
            (Some(game), None) => {
                let bytes = match request.pack {
                    Some(kind) => game.0.pack(kind).and_then(|p| p.get(&request.source)),
                    None => game.0.texture(&request.source),
                };
                let decoded = bytes.and_then(|b| {
                    texture::decode_entry(
                        &request.source,
                        b,
                        request.space,
                        request.flags,
                        &settings,
                        bc,
                    )
                });
                loader.stream.finish_decode(request, decoded);
            }
            (None, _) => {
                loader.stream.finish_decode(request, None);
            }
        }
    }

    // Requests that failed on the spot are ready now.
    pump_uploads(&mut loader, &mut cache, &mut images);
}

/// Uploads the decoded images that are ready into the texture records'
/// placeholder handles.
fn pump_uploads(loader: &mut TextureLoader, cache: &mut ModelCache, images: &mut Assets<Image>) {
    let ModelCache { textures, .. } = cache;
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

    /// An app with the loader on the streaming path, and its installation
    /// (skipped without the game).
    fn streaming_app() -> Option<(App, std::sync::Arc<crate::GameData>)> {
        let dir = crate::locate_install(None)?;
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
        let data = app.world().resource::<Game>().0.clone();
        Some((app, data))
    }

    /// Runs `f` with a loader over the app's assets and cache.
    fn with_loader<T>(
        app: &mut App,
        data: &crate::GameData,
        f: impl FnOnce(&mut crate::ModelLoader<'_>) -> T,
    ) -> T {
        app.world_mut().resource_scope(
            |world: &mut World, mut cache: bevy::ecs::change_detection::Mut<ModelCache>| {
                let mut images = world.resource_mut::<Assets<Image>>();
                let mut meshes = Assets::<Mesh>::default();
                let mut materials = Assets::<exa_render::material::ExaMaterial>::default();
                let mut loader = crate::ModelLoader {
                    game: data,
                    cache: &mut cache,
                    meshes: &mut meshes,
                    materials: &mut materials,
                    images: &mut images,
                    bc_supported: true,
                };
                f(&mut loader)
            },
        )
    }

    /// Requests `name`; with `now`, through the synchronous priority path.
    fn request(app: &mut App, data: &crate::GameData, name: &str, now: bool) -> Handle<Image> {
        with_loader(app, data, |loader| {
            let texture = if now {
                loader.texture_now(name, texture::ColorSpace::Srgb, texture::default_flags())
            } else {
                loader.texture(name, texture::ColorSpace::Srgb)
            };
            texture.map(|(handle, _)| handle)
        })
        .unwrap_or_else(|| panic!("texture {name} is shipped"))
    }

    fn width(app: &App, handle: &Handle<Image>) -> u32 {
        app.world()
            .resource::<Assets<Image>>()
            .get(handle)
            .map(|image| image.texture_descriptor.size.width)
            .unwrap_or(0)
    }

    /// The terrain path looks the record up first and otherwise queues a
    /// load behind the placeholder even with the manager's general streaming
    /// switch off; a second call reuses the live record without queueing.
    #[test]
    fn the_terrain_path_queues_once_and_then_reuses_the_record() {
        let Some((mut app, data)) = streaming_app() else {
            return;
        };
        app.world_mut()
            .resource_mut::<ModelCache>()
            .textures
            .set_streaming(false);
        let first = with_loader(&mut app, &data, |loader| {
            loader.texture_stream_first(
                "spatter01",
                texture::ColorSpace::Srgb,
                texture::default_flags(),
            )
        })
        .expect("the image is shipped")
        .0;
        assert_eq!(width(&app, &first), 1, "the placeholder first");
        assert_eq!(
            app.world()
                .resource::<ModelCache>()
                .textures
                .queued_loads()
                .len(),
            1
        );
        // A live record (even the placeholder) wins without another load.
        let second = with_loader(&mut app, &data, |loader| {
            loader.texture_existing("spatter01", texture::ColorSpace::Srgb)
        })
        .expect("the record is live");
        assert_eq!(second.0, first);
        let third = with_loader(&mut app, &data, |loader| {
            loader.texture_stream_first(
                "spatter01",
                texture::ColorSpace::Srgb,
                texture::default_flags(),
            )
        })
        .expect("the record is live");
        assert_eq!(third.0, first);
        assert_eq!(
            app.world()
                .resource::<ModelCache>()
                .textures
                .queued_loads()
                .len(),
            1
        );
        // A name that is not live and not shipped leaves no handle.
        assert!(
            with_loader(&mut app, &data, |loader| loader
                .texture_existing("no_such_image_name", texture::ColorSpace::Srgb))
            .is_none()
        );
        assert!(
            with_loader(&mut app, &data, |loader| loader.texture_stream_first(
                "no_such_image_name",
                texture::ColorSpace::Srgb,
                texture::default_flags()
            ))
            .is_none()
        );
    }

    /// A record still holding its placeholder is finished on the caller by
    /// the priority wait: the real image goes under the same handle, the
    /// queued load is dropped and no reference is added.
    #[test]
    fn a_waiting_record_finishes_now_under_its_handle() {
        let Some((mut app, data)) = streaming_app() else {
            return;
        };
        let space = texture::ColorSpace::Srgb;
        let key = crate::TextureKey::new("spatter01", space);
        let first = with_loader(&mut app, &data, |loader| {
            loader.texture_stream_first("spatter01", space, texture::default_flags())
        })
        .expect("the image is shipped")
        .0;
        assert!(with_loader(&mut app, &data, |loader| loader
            .texture_waiting("spatter01", space)));
        let now = with_loader(&mut app, &data, |loader| {
            loader.texture_finish_now("spatter01", space)
        })
        .expect("the record is live")
        .0;
        assert_eq!(now, first);
        assert!(width(&app, &first) > 1);
        assert!(!with_loader(&mut app, &data, |loader| loader
            .texture_waiting("spatter01", space)));
        let cache = app.world().resource::<ModelCache>();
        assert!(cache.textures.queued_loads().is_empty());
        assert_eq!(cache.textures.get(&key).unwrap().references, 1);
        // No record: nothing waits and nothing is finished.
        assert!(
            with_loader(&mut app, &data, |loader| loader
                .texture_finish_now("no_such_image_name", space))
            .is_none()
        );
    }

    /// Under the manager's fast-load flag the material path queues its load
    /// as the loader's priority item (status 3); the terrain path queues
    /// with status 1 whatever the flag says, and the flag is off by default.
    #[test]
    fn the_fast_load_flag_promotes_material_loads_only() {
        for (fast, terrain, promoted) in [
            (false, false, false),
            (true, false, true),
            (true, true, false),
        ] {
            let Some((mut app, data)) = streaming_app() else {
                return;
            };
            {
                let mut cache = app.world_mut().resource_mut::<ModelCache>();
                assert!(!cache.textures.fast_load());
                cache.textures.set_fast_load(fast);
            }
            with_loader(&mut app, &data, |loader| {
                let space = texture::ColorSpace::Srgb;
                if terrain {
                    loader.texture_stream_first("spatter01", space, texture::default_flags())
                } else {
                    loader.texture("spatter01", space)
                }
            })
            .expect("the image is shipped");
            let cache = app.world().resource::<ModelCache>();
            let queued = cache.textures.queued_loads();
            assert_eq!(queued.len(), 1);
            assert_eq!(
                queued[0].promoted, promoted,
                "fast {fast} terrain {terrain}"
            );
        }
    }

    /// The streaming path decodes a shipped image on the task pool and
    /// replaces the placeholder under the record's handle.
    #[test]
    fn a_streamed_texture_replaces_its_placeholder() {
        let Some((mut app, data)) = streaming_app() else {
            return;
        };
        let key = crate::TextureKey::new("spatter01", texture::ColorSpace::Srgb);
        let handle = request(&mut app, &data, "spatter01", false);
        // Before the loader runs, the record draws the 1x1 placeholder.
        assert_eq!(width(&app, &handle), 1);
        for _ in 0..5000 {
            app.update();
            if width(&app, &handle) > 1 {
                break;
            }
        }
        assert!(width(&app, &handle) > 1);
        let loader = app.world().resource::<TextureLoader>();
        assert_eq!(loader.stream().queued(), 0);
        assert!(!loader.is_decoding());
        let cache = app.world().resource::<ModelCache>();
        assert!(cache.textures.get(&key).is_some());
        assert!(
            !cache.textures.get(&key).unwrap().pending,
            "the upload finished"
        );
    }

    /// A batch of shipped images streams through the loader and finishes in
    /// request order, each at the size its header asks for.
    #[test]
    fn a_batch_finishes_in_request_order() {
        let Some((mut app, data)) = streaming_app() else {
            return;
        };
        let mut names = Vec::new();
        for (_, pack) in data.packs() {
            for entry in pack.entries() {
                if names.len() >= 16 {
                    break;
                }
                if exa_formats::FileKind::detect(pack.bytes(entry)) == exa_formats::FileKind::Image
                {
                    names.push(crate::TextureName::new(&entry.name).as_str().to_string());
                }
            }
        }
        assert!(names.len() >= 8, "shipped images to stream");
        let handles: Vec<Handle<Image>> = names
            .iter()
            .map(|name| request(&mut app, &data, name, false))
            .collect();
        for handle in &handles {
            assert_eq!(width(&app, handle), 1, "placeholders first");
        }
        let mut finished = Vec::new();
        for _ in 0..5000 {
            app.update();
            finished.extend(
                app.world_mut()
                    .resource_mut::<TextureLoader>()
                    .stream_mut()
                    .take_finished(),
            );
            let cache = app.world().resource::<ModelCache>();
            if names.iter().all(|name| {
                let key = crate::TextureKey::new(name, texture::ColorSpace::Srgb);
                cache
                    .textures
                    .get(&key)
                    .is_some_and(|record| !record.pending)
            }) {
                break;
            }
        }
        let requested: Vec<String> = names
            .iter()
            .map(|name| crate::TextureName::new(name).as_str().to_string())
            .collect();
        let done: Vec<String> = finished
            .iter()
            .map(|key| key.name.as_str().to_string())
            .collect();
        assert_eq!(done, requested, "uploads keep request order");
        // Each finished image has the base size its detail class leaves.
        let settings = *app.world().resource::<ModelCache>().textures.settings();
        for (name, handle) in names.iter().zip(&handles) {
            let bytes = data.texture(name).expect("the image is shipped");
            let rfi = exa_formats::rfi::Rfi::parse(bytes).unwrap();
            let first = rfi
                .header
                .first_mip_level(settings.texture_detail, settings.max_texture_size);
            let expected = (rfi.header.width >> first).max(1);
            assert_eq!(width(&app, handle), expected, "{name}");
        }
    }

    /// The synchronous priority path replaces a placeholder at once and
    /// drops its queued load.
    #[test]
    fn a_priority_load_replaces_the_placeholder_at_once() {
        let Some((mut app, data)) = streaming_app() else {
            return;
        };
        let key = crate::TextureKey::new("spatter01", texture::ColorSpace::Srgb);
        let first = request(&mut app, &data, "spatter01", false);
        assert_eq!(width(&app, &first), 1);
        assert_eq!(
            app.world()
                .resource::<ModelCache>()
                .textures
                .queued_loads()
                .len(),
            1
        );
        let second = request(&mut app, &data, "spatter01", true);
        assert!(width(&app, &second) > 1);
        // The queued load was dropped and the record is finished.
        let cache = app.world().resource::<ModelCache>();
        assert!(cache.textures.queued_loads().is_empty());
        assert!(!cache.textures.get(&key).unwrap().pending);
    }

    /// A material that binds an image is touched when the image is replaced
    /// under its handle, and one that does not is left alone; a flame
    /// material, which binds its particle texture directly, is touched too.
    #[test]
    fn replacing_an_image_touches_the_materials_that_bind_it() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Image>()
            .init_asset::<ExaMaterial>()
            .init_asset::<exa_render::flame::FlameMaterial>()
            .add_systems(Update, rebind_replaced_images);
        let (image, other) = {
            let mut images = app.world_mut().resource_mut::<Assets<Image>>();
            (
                images.add(texture::placeholder_image(
                    texture::ColorSpace::Srgb,
                    texture::default_flags(),
                    &texture::TextureSettings::default(),
                )),
                images.add(Image::default()),
            )
        };
        let (bound, unbound) = {
            let mut materials = app.world_mut().resource_mut::<Assets<ExaMaterial>>();
            let bound = materials.add(ExaMaterial {
                gloss_map: Some(image.clone()),
                ..default()
            });
            let unbound = materials.add(ExaMaterial {
                diffuse_map: Some(other),
                ..default()
            });
            (bound, unbound)
        };
        let flame = app
            .world_mut()
            .resource_mut::<Assets<exa_render::flame::FlameMaterial>>()
            .add(exa_render::flame::FlameMaterial {
                texture: image.clone(),
            });
        // Settle the creation events, then replace the image.
        app.update();
        app.update();
        let _ = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .insert(image.id(), Image::default());
        app.update();
        app.update();
        let mut modified = Vec::new();
        let events = app.world().resource::<Messages<AssetEvent<ExaMaterial>>>();
        let mut cursor = events.get_cursor();
        for event in cursor.read(events) {
            if let AssetEvent::Modified { id } = event {
                modified.push(*id);
            }
        }
        assert!(modified.contains(&bound.id()));
        assert!(!modified.contains(&unbound.id()));
        let events = app
            .world()
            .resource::<Messages<AssetEvent<exa_render::flame::FlameMaterial>>>();
        let mut cursor = events.get_cursor();
        assert!(
            cursor
                .read(events)
                .any(|event| matches!(event, AssetEvent::Modified { id } if *id == flame.id())),
            "the flame material is rebound"
        );
    }
}
