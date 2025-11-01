use bevy::math::bounding::{BoundingCircle, IntersectsVolume};
use bevy::prelude::*;
use bevy::window::WindowResolution;
use rand::Rng;

const NUM_BODIES: usize = 100;
const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const GRAVITATIONAL_CONSTANT: f64 = 100.0;
const SOFTENING_FACTOR: f32 = 10.0;
const DENSITY: f32 = 1.0;
const INITIAL_VELOCITY_MAX: f32 = 50.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(WIDTH, HEIGHT).with_scale_factor_override(1.0),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, apply_gravity)
        .add_systems(FixedUpdate, apply_velocity)
        .add_systems(FixedUpdate, detect_collisions)
        .add_observer(handle_collisions)
        .run();
}
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d);

    // create a happy little sun
    let radius = 30.;
    commands.spawn((
        Mesh2d(meshes.add(Circle::new(radius))),
        MeshMaterial2d(materials.add(Color::srgba(1.0, 0.5, 0.0, 1.0))),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Density(DENSITY),
        Velocity(Vec2::ZERO),
        Sun,
        Radius(radius),
    ));

    // create some happy little celestial bodies
    for _ in 0..NUM_BODIES {
        let radius = rand::rng().random_range(1.0..5.0);
        commands.spawn((
            Mesh2d(meshes.add(Circle::new(radius))),
            MeshMaterial2d(materials.add(Color::WHITE)),
            Transform::from_xyz(
                rand::rng().random_range(-(WIDTH as f32) / 2.0..WIDTH as f32 / 2.0),
                rand::rng().random_range(-(HEIGHT as f32) / 2.0..HEIGHT as f32 / 2.0),
                0.0,
            ),
            Velocity(Vec2::new(
                rand::rng().random_range(-INITIAL_VELOCITY_MAX..INITIAL_VELOCITY_MAX),
                rand::rng().random_range(-INITIAL_VELOCITY_MAX..INITIAL_VELOCITY_MAX),
            )),
            Density(DENSITY),
            Radius(radius),
        ));
    }
}

#[derive(Component, Deref, DerefMut)]
struct Radius(f32);

#[derive(Component, Deref, DerefMut)]
struct Density(f32);

#[derive(Event)]
struct CollisionEvent(Entity, Entity);

fn detect_collisions(mut commands: Commands, query: Query<(Entity, &Transform, &Radius)>) {
    for [
        (left_entity, left_transform, left_radius),
        (right_entity, right_transform, right_radius),
    ] in query.iter_combinations()
    {
        let left = BoundingCircle::new(left_transform.translation.truncate(), left_radius.0);
        let right = BoundingCircle::new(right_transform.translation.truncate(), right_radius.0);

        if left.intersects(&right) {
            commands.trigger(CollisionEvent(left_entity, right_entity))
        }
    }
}

fn handle_collisions(
    event: On<CollisionEvent>,
    mut query: Query<(&mut Radius, &mut Velocity, &Density, &mut Mesh2d)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut commands: Commands,
) {
    let (left_entity, right_entity) = (event.event().0, event.event().1);

    // Use get_many_mut to get mutable access to both entities at the same time.
    // This is the idiomatic Bevy way to avoid borrow-checker issues.
    if let Ok(
        [
            (mut left_radius, mut left_velocity, left_density, mut left_mesh),
            (mut right_radius, mut right_velocity, right_density, mut right_mesh),
        ],
    ) = query.get_many_mut([left_entity, right_entity])
    {
        let left_mass = left_density.0 * std::f32::consts::PI * left_radius.0 * left_radius.0;
        let right_mass = right_density.0 * std::f32::consts::PI * right_radius.0 * right_radius.0;

        // The more massive body absorbs the smaller one.
        if left_mass > right_mass {
            // Conserve momentum: v1_new = (m1*v1 + m2*v2) / (m1 + m2)
            // We can simplify this to an inelastic collision calculation.
            let combined_mass = left_mass + right_mass;
            left_velocity.0 =
                (left_mass * left_velocity.0 + right_mass * right_velocity.0) / combined_mass;

            // New radius is calculated to conserve mass
            left_radius.0 = (left_radius.0.powi(2) + right_radius.0.powi(2)).sqrt();
            *left_mesh = Mesh2d(meshes.add(Circle::new(left_radius.0)));

            // Despawn the smaller entity.
            commands.entity(right_entity).despawn();
        } else {
            let combined_mass = left_mass + right_mass;
            right_velocity.0 =
                (left_mass * left_velocity.0 + right_mass * right_velocity.0) / combined_mass;

            right_radius.0 = (left_radius.0.powi(2) + right_radius.0.powi(2)).sqrt();
            *right_mesh = Mesh2d(meshes.add(Circle::new(right_radius.0)));

            commands.entity(left_entity).despawn();
        }
    } else {
        // This can happen if one of the entities was already despawned in the same frame.
    }
}

#[derive(Component)]
struct Sun;

#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec2);

fn apply_velocity(mut query: Query<(&mut Transform, &Velocity), Without<Sun>>, time: Res<Time>) {
    for (mut transform, velocity) in &mut query {
        transform.translation.x += velocity.x * time.delta_secs();
        transform.translation.y += velocity.y * time.delta_secs();
    }
}

fn apply_gravity(
    mut query: Query<(&mut Velocity, &Transform, &Radius, &Density, Option<&Sun>)>,
    time: Res<Time>,
) {
    let mut combinations = query.iter_combinations_mut();
    while let Some(
        [
            (mut vel1, trans1, radius1, density1, sun1),
            (mut vel2, trans2, radius2, density2, sun2),
        ],
    ) = combinations.fetch_next()
    {
        let pos1 = trans1.translation.truncate();
        let pos2 = trans2.translation.truncate();

        let mass1 = density1.0 * std::f32::consts::PI * radius1.0 * radius1.0;
        let mass2 = density2.0 * std::f32::consts::PI * radius2.0 * radius2.0;

        // Calculate acceleration of body 1 due to body 2
        if sun1.is_none() {
            let acc1 = calculate_gravitational_force(pos1, mass1, pos2, mass2);
            vel1.0 += acc1 * time.delta_secs();
        }

        // Calculate acceleration of body 2 due to body 1
        // This is the opposite of the first force, scaled by the mass ratio
        if sun2.is_none() {
            let acc2 = calculate_gravitational_force(pos2, mass2, pos1, mass1);
            vel2.0 += acc2 * time.delta_secs();
        }
    }
}

fn calculate_gravitational_force(
    position: Vec2,
    _mass: f32,
    other_position: Vec2,
    other_mass: f32,
) -> Vec2 {
    let difference = other_position - position;
    let distance_sq = difference.length_squared();

    // a = G * m2 / r^2
    let acceleration_magnitude =
        (GRAVITATIONAL_CONSTANT as f32 * other_mass) / (distance_sq + SOFTENING_FACTOR);

    // Return the acceleration vector
    difference.normalize_or_zero() * acceleration_magnitude
}
