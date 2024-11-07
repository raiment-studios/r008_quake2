use bevy::{
    app::App,
    asset::{self, io::Reader, AssetLoader, AsyncReadExt, LoadContext},
    prelude::{default, *},
    reflect::TypePath,
    render::{render_asset::RenderAssetUsages, render_resource::PrimitiveTopology},
    DefaultPlugins,
};
use bevy_mod_raycast::prelude::{Raycast, RaycastSettings};
use thiserror::Error;
use wasm_bindgen::prelude::*;

use crate::{bsp38::BSP38, render::RenderPlugin};

#[derive(Resource, Default)]
struct State {
    ready: bool,
    handle: Handle<BSP38Asset>,
    count: usize,
}

#[wasm_bindgen]
pub fn start(canvas_id: &str) {
    let id = format!("#{}", canvas_id);

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                canvas: Some(id.into()),
                ..default()
            }),
            ..default()
        }))
        .init_asset::<BSP38Asset>()
        .init_asset_loader::<BSP38AssetLoader>()
        .add_plugins(RenderPlugin)
        .init_resource::<State>()
        .add_systems(
            Startup,
            (
                setup_window, //
                setup_camera,
                setup_assets.after(setup_camera),
            ),
        )
        .add_systems(
            Update,
            (
                update_camera, //
                update_assets.after(update_camera),
                update_raycast.after(update_assets),
            ),
        )
        .run();
}

fn setup_window(mut windows: Query<&mut Window>) {
    let mut window = windows.single_mut();
    let canvas_id = window.canvas.as_ref().unwrap().trim_start_matches("#");
    let (width, height) = {
        use wasm_bindgen::JsCast;
        use web_sys::window;

        let window = window().unwrap();
        let document = window.document().unwrap();
        let canvas = document.get_element_by_id(canvas_id).unwrap();

        let el = canvas.dyn_into::<web_sys::HtmlCanvasElement>().unwrap();
        (el.width() as f32, el.height() as f32)
    };

    window.resolution.set(width, height);
    window.resizable = false;
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera3dBundle {
        projection: Projection::Perspective(PerspectiveProjection {
            near: 0.1,     // Set the near clipping plane
            far: 10_000.0, // Set the far clipping plane
            ..default()
        }),
        transform: Transform::from_xyz(-1275.0, 1300.0, 1250.0).looking_at(Vec3::ZERO, Vec3::Z),
        ..default()
    });
}

fn setup_assets(mut state: ResMut<State>, asset_server: Res<AssetServer>) {
    state.handle = asset_server.load("q2dm1.bsp");
}

#[derive(Asset, TypePath, Debug)]
pub struct BSP38Asset {
    pub bsp: BSP38,
}

#[non_exhaustive]
#[derive(Debug, Error)]
enum BSP38AssetLoaderError {
    /// An [IO](std::io) Error
    #[error("Could not load asset: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Default)]
struct BSP38AssetLoader;

impl AssetLoader for BSP38AssetLoader {
    type Asset = BSP38Asset;
    type Settings = ();
    type Error = BSP38AssetLoaderError;

    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _settings: &'a (),
        _load_context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        match reader.read_to_end(&mut bytes).await {
            Ok(_) => {}
            Err(e) => return Err(BSP38AssetLoaderError::from(e)),
        };
        let custom_asset = BSP38Asset {
            bsp: BSP38::from_bytes(bytes),
        };
        Ok(custom_asset)
    }

    fn extensions(&self) -> &[&str] {
        &["bsp"]
    }
}

fn update_camera(
    mut query: Query<&mut Transform, With<Camera>>, //
    time: Res<Time>,
) {
    let radius = 2250.0; // Distance from the origin
    let speed = 0.25; // Speed of rotation

    for mut transform in query.iter_mut() {
        let angle = time.elapsed_seconds() * speed;

        // Calculate new position
        transform.translation.x = radius * angle.cos();
        transform.translation.y = radius * angle.sin();
        transform.translation.z = (radius * 0.5) + 400.0 * angle.cos();
        transform.look_at(Vec3::new(0.0, 0.0, 500.0 + 100.0 * angle.sin()), Vec3::Z);
    }
}

fn update_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut state: ResMut<State>,
    bsp38_assets: Res<Assets<BSP38Asset>>,
) {
    if state.ready {
        return;
    }

    let asset = bsp38_assets.get(&state.handle);
    match asset {
        Some(asset) => {
            info!("Asset loaded: {:#?}", asset);
            state.ready = true;

            commands.spawn(PbrBundle {
                mesh: meshes.add(Circle::new(2000.0)),
                material: materials.add(Color::WHITE),
                transform: Transform::from_rotation(Quat::from_rotation_x(
                    0.0, //-std::f32::consts::FRAC_PI_2,
                )),
                ..default()
            });

            let vertices = asset.bsp.read_vertices();
            let bounds = asset.bsp.bounds();
            let faces = asset.bsp.read_faces();

            // Center point of bounds
            let center = [
                (bounds.min[0] + bounds.max[0]) / 2.0,
                (bounds.min[1] + bounds.max[1]) / 2.0,
                (bounds.min[2] + bounds.max[2]) / 2.0,
            ];

            let light_direction = Vec3::new(-1.0, -1.0, -1.0).normalize();
            commands.spawn(DirectionalLightBundle {
                directional_light: DirectionalLight {
                    illuminance: 100000.0,
                    shadows_enabled: false,
                    ..default()
                },
                transform: Transform::from_rotation(Quat::from_rotation_arc(
                    Vec3::NEG_Z,
                    light_direction,
                )),
                ..default()
            });

            if false {
                commands.insert_resource(AmbientLight {
                    color: Color::WHITE,
                    ..default()
                });
            }

            // Create a grid of point lights from -1000 to 1000 in x and y
            /*use rand::{thread_rng, Rng};
            let mut rng = thread_rng();
            for x in (-2000..2000).step_by(250) {
                for y in (-2000..2000).step_by(250) {
                    commands.spawn(PointLightBundle {
                        point_light: PointLight {
                            color: Color::hsl(rng.gen_range(0.0..360.0), 1.0, 0.5),
                            range: 3000.0,
                            ..default()
                        },
                        transform: Transform::from_xyz(x as f32, y as f32, 600.0),
                        ..default()
                    });
                }
            }*/

            // Create a new mesh using faces points and normals
            let mut mesh = Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::default(),
            );

            // Collect faces.points into a new array of [f32; 3] where each element is
            // three elements of the original array.
            let vertices2: Vec<[f32; 3]> =
                faces.points.chunks(3).map(|v| [v[0], v[1], v[2]]).collect();
            let normals2: Vec<[f32; 3]> = faces
                .normals
                .chunks(3)
                .map(|v| [v[0], v[1], v[2]])
                .collect();

            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices2);
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals2);

            commands.spawn(PbrBundle {
                mesh: meshes.add(mesh),
                material: materials.add(StandardMaterial {
                    base_color: Color::srgb(0.8, 0.3, 0.85),
                    ..default()
                }),
                transform: Transform::from_xyz(-center[0], -center[1], 0.0),
                ..default()
            });

            let mesh = meshes.add(Cuboid::new(10.0, 10.0, 10.0));
            let material = materials.add(Color::srgb(1.0, 0.15, 0.15));

            for v in vertices.chunks(3) {
                commands.spawn(PbrBundle {
                    mesh: mesh.clone(),
                    material: material.clone(),
                    transform: Transform::from_xyz(v[0] - center[0], v[1] - center[1], v[2]),
                    ..default()
                });
            }
        }
        None => {}
    }
}

// Write a function that selects the main mesh and cast a
// random ray in the -5000 to 5000 world space and adds
// a cube at each hit point
fn update_raycast(
    mut state: ResMut<State>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut raycast: Raycast,
) {
    use rand::{thread_rng, Rng};

    let mut rng = thread_rng();
    let p1 = Vec3::new(
        rng.gen_range(-5000.0..5000.0),
        rng.gen_range(-5000.0..5000.0),
        rng.gen_range(-5000.0..5000.0),
    )
    .normalize()
        * 5000.0;
    let p2 = Vec3::new(
        rng.gen_range(-5000.0..5000.0),
        rng.gen_range(-5000.0..5000.0),
        rng.gen_range(-5000.0..5000.0),
    )
    .normalize()
        * 5000.0;

    let ray = Ray3d::new(p1, p2 - p1);
    let hits = raycast.cast_ray(ray, &RaycastSettings::default());

    if (state.count < 5) {
        for (ent, isect) in hits {
            info!("Hit: {:?}", isect);
            state.count += 1;

            let mesh = meshes.add(Cuboid::new(10.0, 10.0, 10.0));
            let material = materials.add(Color::srgb(1.0, 0.15, 0.15));

            let pos = isect.position();

            commands.spawn(PbrBundle {
                mesh: mesh.clone(),
                material: material.clone(),
                transform: Transform::from_xyz(pos[0], pos[1], pos[2]),
                ..default()
            });

            //commands.spawn(PbrBundle {
            //    mesh: cube.clone(),
            //    material: material.clone(),
            //    transform: Transform::from_xyz(hit.x, hit.y, hit.z),
            //    ..default()
            //});
        }
    }
}
