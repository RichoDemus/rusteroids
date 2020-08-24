use std::f32::consts::PI;

use legion::prelude::*;
use nalgebra::{Vector2, Isometry2};
use rand::Rng;
use ncollide2d::query::{self, Proximity};

use crate::{
    BODY_INITIAL_MASS_MAX, GRAVITATIONAL_CONSTANT, HEIGHT, INITIAL_SPEED, NUM_BODIES, SUN_SIZE,
    WIDTH,
};
use ncollide2d::shape::Ball;

// Define our entity data types
#[derive(Clone, Copy, Debug, PartialEq)]
struct Position {
    vector: Vector2<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct MyVector2 {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Velocity {
    vector: Vector2<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Dimensions {
    radius: f32,
    mass: f32,
}

impl Dimensions {
    fn from_mass(mass: f32) -> Dimensions {
        let radius: f32 = mass / (4. / 3. * PI);
        let radius = radius.cbrt();
        Dimensions { mass, radius }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Data {
    name: String,
    sun: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Model(usize);

#[derive(Clone, Copy, Debug, PartialEq)]
struct Static;

pub(crate) struct Core {
    world: World,
}

impl Core {
    pub(crate) fn new() -> Core {
        let universe = Universe::new();
        let world = universe.create_world();
        Core { world }
    }

    pub(crate) fn init(&mut self) {
        let mut rng = rand::thread_rng();
        self.world.insert(
            (),
            vec![(
                Data {
                    name: "sun".to_string(),
                    sun: true,
                },
                Position {
                    vector: Vector2::new(WIDTH / 2., HEIGHT / 2.),
                },
                Velocity {
                    vector: Vector2::new(0., 0.),
                },
                Dimensions::from_mass(SUN_SIZE),
            )],
        );
        self.world.insert(
            (),
            (0..NUM_BODIES).map(|i| {
                let x = rng.gen_range(0., WIDTH);
                let y = rng.gen_range(0., HEIGHT);

                let x_velocity = match INITIAL_SPEED {
                    0 => 0.,
                    speed => rng.gen_range(-speed as f32, speed as f32),
                };
                let y_velocity = match INITIAL_SPEED {
                    0 => 0.,
                    speed => rng.gen_range(-speed as f32, speed as f32),
                };

                let mass = rng.gen_range(1., BODY_INITIAL_MASS_MAX);
                (
                    Data {
                        name: i.to_string(),
                        sun: false,
                    },
                    Position {
                        vector: Vector2::new(x, y),
                    },
                    Velocity {
                        vector: Vector2::new(x_velocity, y_velocity),
                    },
                    Dimensions::from_mass(mass),
                )
            }),
        );
    }

    pub(crate) fn tick(&mut self, dt: f32) {
        let query = <(Read<Position>, Read<Dimensions>, Read<Data>)>::query();
        let bodies = query
            .iter(&self.world)
            .map(|(pos, dimensions, data)| (*pos, dimensions.mass, data.as_ref().clone()))
            .collect::<Vec<_>>();

        let query = <(
            Read<Position>,
            Read<Dimensions>,
            Write<Velocity>,
            Read<Data>,
        )>::query();
        for (position, dimensions, mut velocity, data) in query.iter_mut(&mut self.world) {
            for (other_position, other_mass, other_data) in &bodies {
                let data: &Data = &data;
                let other_data: &Data = other_data;
                if data == other_data || data.sun {
                    continue;
                }

                velocity.vector += calculate_gravitational_force(
                    position.vector,
                    dimensions.mass,
                    other_position.vector,
                    other_mass,
                );
            }
        }

        // update positions
        let query = <(Write<Position>, Read<Velocity>)>::query();
        for (mut position, velocity) in query.iter_mut(&mut self.world) {
            let current_position: Vector2<f32> = position.vector.clone_owned();
            let velocity: Vector2<f32> = velocity.vector.clone_owned() * dt;

            position.vector = current_position + velocity;
        }

        // collision detection
        let query = <(Read<Position>, Read<Velocity>, Read<Dimensions>, Read<Data>)>::query();
        let bodies = query
            .iter(&self.world)
            .map(|(pos, velocity, dimensions, data)| {
                (
                    *pos,
                    *velocity.as_ref(),
                    dimensions.mass,
                    dimensions.radius,
                    data.as_ref().clone(),
                )
            })
            .collect::<Vec<_>>();

        let query = <(
            Read<Position>,
            Write<Velocity>,
            Write<Dimensions>,
            Read<Data>,
        )>::query();
        let mut entities_to_delete = vec![];
        for (entity, (position, mut velocity, mut dimensions, data)) in
            query.iter_entities_mut(&mut self.world)
        {
            for (other_position, other_velocity, other_mass, other_radius, other_data) in &bodies {
                let data: &Data = &data;
                let other_data: &Data = other_data;
                if data == other_data || data.sun {
                    continue;
                }

                if are_colliding(position.vector, dimensions.radius, other_position.vector, *other_radius) {
                    // the bigger body swallows the smaller one
                    // this will one twice for each collision, with this and other swapped, lets utilize this
                    if dimensions.mass > *other_mass {
                        // when this is the bigger one, enlarge it
                        // todo scale vector based on size difference
                        let mass_ratio = *other_mass / dimensions.mass;
                        velocity.vector += other_velocity.vector * mass_ratio;
                        // velocity.vector = Vector2::new(0.,0.);
                        dimensions.mass += *other_mass;
                    } else {
                        // when it's the smaller one, schedule it for deletion
                        entities_to_delete.push(entity);
                    }
                }
            }
        }

        for entity in entities_to_delete {
            self.world.delete(entity);
        }
    }

    pub(crate) fn draw(&self) -> Vec<Drawable> {
        let query = <(Read<Position>, Read<Data>, Read<Dimensions>)>::query();
        query
            .iter(&self.world)
            .map(|(pos, data, dimensions)| {
                let position = *pos;
                let position: Vector2<f32> = position.vector;
                Drawable {
                    position,
                    sun: data.sun,
                    radius: dimensions.radius,
                }
            })
            .collect::<Vec<_>>()
    }
}

pub(crate) struct Drawable {
    pub(crate) position: Vector2<f32>,
    pub(crate) sun: bool,
    pub(crate) radius: f32,
}

fn calculate_gravitational_force(
    position: Vector2<f32>,
    mass: f32,
    other_position: Vector2<f32>,
    other_mass: &f32,
) -> Vector2<f32> {
    let difference: Vector2<f32> = other_position - position;
    let distance = difference.magnitude();
    let gravity_direction: Vector2<f32> = difference.normalize();
    let gravity: f32 = GRAVITATIONAL_CONSTANT * (mass * other_mass) / (distance * distance);

    gravity_direction * gravity
}

fn are_colliding(position: Vector2<f32>, radius: f32, other_position: Vector2<f32>, other_radius:f32) -> bool {
    let shape = Ball::new(radius);
    let position = Isometry2::new(position, nalgebra::zero());
    let other_shape = Ball::new(other_radius);
    let other_position = Isometry2::new(other_position, nalgebra::zero());

    let proximity = query::proximity(&position, &shape, &other_position, &other_shape, 0.);
    if let Proximity::Intersecting = proximity {
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let vector: Vector2<f32> = Vector2::new(11., 11.);
        let vector1 = Vector2::new(10., 10.);

        let result: Vector2<f32> = vector1 - vector;

        let result = result.magnitude();

        print!("{:?}", result)
    }
}
