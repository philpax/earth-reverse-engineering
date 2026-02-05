//! Async data loading for Google Earth mesh data.
//!
//! Uses Bevy's `AsyncComputeTaskPool` for background data loading.

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, futures_lite::future};
use std::sync::Arc;

use rocktree::{BulkMetadata, BulkRequest, Client, MemoryCache, Node, NodeRequest, Planetoid};

use crate::mesh::{RocktreeMeshMarker, convert_mesh, convert_texture, matrix_to_transform};

/// Plugin for loading Google Earth data.
pub struct DataLoaderPlugin;

impl Plugin for DataLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoaderState>()
            .add_systems(Startup, start_initial_load)
            .add_systems(
                Update,
                (poll_planetoid_task, poll_bulk_task, poll_node_task),
            );
    }
}

/// State for the data loader.
#[derive(Resource)]
pub struct LoaderState {
    /// The HTTP client for fetching data.
    client: Arc<Client<MemoryCache>>,
    /// Planetoid metadata (once loaded).
    pub planetoid: Option<Planetoid>,
    /// Root bulk metadata (once loaded).
    pub root_bulk: Option<BulkMetadata>,
}

impl Default for LoaderState {
    fn default() -> Self {
        Self {
            client: Arc::new(Client::with_cache(MemoryCache::new())),
            planetoid: None,
            root_bulk: None,
        }
    }
}

/// Component for tracking async planetoid load task.
#[derive(Component)]
struct PlanetoidTask(Task<Result<Planetoid, rocktree::Error>>);

/// Component for tracking async bulk load task.
#[derive(Component)]
struct BulkTask {
    task: Task<Result<BulkMetadata, rocktree::Error>>,
    request: BulkRequest,
}

/// Component for tracking async node load task.
#[derive(Component)]
struct NodeTask {
    task: Task<Result<Node, rocktree::Error>>,
    #[allow(dead_code)]
    request: NodeRequest,
}

/// Start loading the initial planetoid data.
#[allow(clippy::needless_pass_by_value)]
fn start_initial_load(mut commands: Commands, state: Res<LoaderState>) {
    let client = Arc::clone(&state.client);
    let task_pool = AsyncComputeTaskPool::get();

    let task = task_pool.spawn(async move { client.fetch_planetoid().await });

    commands.spawn(PlanetoidTask(task));

    tracing::info!("Started loading planetoid metadata");
}

/// Poll the planetoid loading task.
#[allow(clippy::needless_pass_by_value)]
fn poll_planetoid_task(
    mut commands: Commands,
    mut state: ResMut<LoaderState>,
    mut query: Query<(Entity, &mut PlanetoidTask)>,
) {
    for (entity, mut task) in &mut query {
        if let Some(result) = block_on(future::poll_once(&mut task.0)) {
            commands.entity(entity).despawn();

            match result {
                Ok(planetoid) => {
                    tracing::info!(
                        "Loaded planetoid: radius={:.0}m, root_epoch={}",
                        planetoid.radius,
                        planetoid.root_epoch
                    );

                    // Start loading root bulk.
                    let client = Arc::clone(&state.client);
                    let epoch = planetoid.root_epoch;
                    let request = BulkRequest::root(epoch);
                    let req = request.clone();

                    let task_pool = AsyncComputeTaskPool::get();
                    let task = task_pool.spawn(async move { client.fetch_bulk(&req).await });

                    commands.spawn(BulkTask { task, request });

                    state.planetoid = Some(planetoid);
                }
                Err(e) => {
                    tracing::error!("Failed to load planetoid: {}", e);
                }
            }
        }
    }
}

/// Poll bulk loading tasks.
#[allow(clippy::needless_pass_by_value)]
fn poll_bulk_task(
    mut commands: Commands,
    mut state: ResMut<LoaderState>,
    mut query: Query<(Entity, &mut BulkTask)>,
) {
    for (entity, mut task) in &mut query {
        if let Some(result) = block_on(future::poll_once(&mut task.task)) {
            commands.entity(entity).despawn();

            match result {
                Ok(bulk) => {
                    tracing::info!(
                        "Loaded bulk '{}': {} nodes, {} child bulks",
                        bulk.path,
                        bulk.nodes.len(),
                        bulk.child_bulk_paths.len()
                    );

                    // Queue loading first few nodes with data.
                    let task_pool = AsyncComputeTaskPool::get();
                    let mut loaded = 0;
                    for node_meta in &bulk.nodes {
                        if !node_meta.has_data {
                            continue;
                        }
                        if loaded >= 3 {
                            // Limit initial load for testing.
                            break;
                        }

                        let request = NodeRequest::new(
                            node_meta.path.clone(),
                            node_meta.epoch,
                            node_meta.texture_format,
                            node_meta.imagery_epoch,
                        );

                        let client = Arc::clone(&state.client);
                        let req = request.clone();
                        let task = task_pool.spawn(async move { client.fetch_node(&req).await });

                        commands.spawn(NodeTask { task, request });
                        loaded += 1;
                    }

                    // Store root bulk.
                    if task.request.path.is_empty() {
                        state.root_bulk = Some(bulk);
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to load bulk '{}': {}", task.request.path, e);
                }
            }
        }
    }
}

/// Poll node loading tasks and spawn meshes.
#[allow(clippy::needless_pass_by_value)]
fn poll_node_task(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut query: Query<(Entity, &mut NodeTask)>,
) {
    for (entity, mut task) in &mut query {
        if let Some(result) = block_on(future::poll_once(&mut task.task)) {
            commands.entity(entity).despawn();

            match result {
                Ok(node) => {
                    tracing::info!(
                        "Loaded node '{}': {} meshes, meters_per_texel={:.2}",
                        node.path,
                        node.meshes.len(),
                        node.meters_per_texel
                    );

                    // Spawn mesh entities.
                    for rocktree_mesh in &node.meshes {
                        let mesh = convert_mesh(rocktree_mesh);
                        let texture = convert_texture(rocktree_mesh);

                        let mesh_handle = meshes.add(mesh);
                        let texture_handle = images.add(texture);

                        let material = materials.add(StandardMaterial {
                            base_color_texture: Some(texture_handle),
                            unlit: true, // Use unlit for now since we don't have proper normals.
                            ..Default::default()
                        });

                        let transform = matrix_to_transform(&node.matrix_globe_from_mesh);

                        commands.spawn((
                            Mesh3d(mesh_handle),
                            MeshMaterial3d(material),
                            transform,
                            RocktreeMeshMarker {
                                path: node.path.clone(),
                                meters_per_texel: node.meters_per_texel,
                            },
                        ));
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to load node: {}", e);
                }
            }
        }
    }
}
