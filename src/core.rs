use std::f64::consts::PI;
use std::ops::Not;

use itertools::Itertools;
use legion::prelude::*;
use nalgebra::{Isometry2, Point, Vector2};
use ncollide2d::query::{self, PointQuery, Proximity};
use ncollide2d::shape::Ball;
use rand::Rng;

use crate::{
    BODY_INITIAL_MASS_MAX, GRAVITATIONAL_CONSTANT, HEIGHT, INITIAL_SPEED, NUM_BODIES, SUN_SIZE,
    WIDTH,
};

// Define our entity data types
#[derive(Clone, Copy, Debug, PartialEq)]
struct Position {
    vector: Vector2<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct MyVector2 {
    x: f64,
    y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Velocity {
    vector: Vector2<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Dimensions {
    radius: f64,
    mass: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
struct MetaInfo {
    selected: bool,
}

impl Dimensions {
    fn from_mass(mass: f64) -> Dimensions {
        let radius: f64 = mass / (4. / 3. * PI);
        let radius = radius.cbrt();
        Dimensions { mass, radius }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Data {
    name: String,
    sun: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct Id {
    id: i32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Model(usize);

#[derive(Clone, Copy, Debug, PartialEq)]
struct Static;

pub(crate) struct Core {
    world: World,
    paused: bool,
}

impl Core {
    pub(crate) fn new() -> Core {
        let universe = Universe::new();
        let world = universe.create_world();
        Core {
            world,
            paused: false,
        }
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
                    vector: Vector2::new((WIDTH as f32 / 2.).into(), (HEIGHT as f32 / 2.).into()),
                },
                Velocity {
                    vector: Vector2::new(0., 0.),
                },
                Dimensions::from_mass(SUN_SIZE),
                MetaInfo::default(),
                Id { id: -1 },
            )],
        );
        self.world.insert(
            (),
            (0..NUM_BODIES).map(|i| {
                let x = rng.gen_range(0., WIDTH as f64);
                let y = rng.gen_range(0., HEIGHT as f64);

                let x_velocity = match INITIAL_SPEED {
                    0 => 0.,
                    speed => rng.gen_range(-speed as f64, speed as f64),
                };
                let y_velocity = match INITIAL_SPEED {
                    0 => 0.,
                    speed => rng.gen_range(-speed as f64, speed as f64),
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
                    MetaInfo::default(),
                    Id { id: i },
                )
            }),
        );
    }

    pub(crate) fn tick(&mut self, dt: f64) {
        if self.paused {
            return;
        }
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

                let gravitational_force = calculate_gravitational_force(
                    position.vector,
                    dimensions.mass,
                    other_position.vector,
                    other_mass,
                );
                velocity.vector += gravitational_force * dt;
            }
        }

        // update positions
        let query = <(Write<Position>, Read<Velocity>)>::query();
        for (mut position, velocity) in query.iter_mut(&mut self.world) {
            let current_position: Vector2<f64> = position.vector.clone_owned();
            let velocity: Vector2<f64> = velocity.vector.clone_owned();

            position.vector = current_position + velocity * dt;
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

                if are_colliding(
                    position.vector,
                    dimensions.radius,
                    other_position.vector,
                    *other_radius,
                ) {
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
        let mut bodies = query
            .iter(&self.world)
            .map(|(pos, data, dimensions)| {
                let position = *pos;
                let position: Vector2<f64> = position.vector;
                Drawable {
                    position,
                    sun: data.sun,
                    radius: dimensions.radius,
                    select_marker: false,
                }
            })
            .collect::<Vec<_>>();

        let query = <(Read<Position>, Read<Dimensions>, Read<MetaInfo>)>::query();
        let mut selection_markers = query
            .iter(&self.world)
            .filter(|(_, _, meta_info)| meta_info.selected)
            .map(|(position, dimensions, _)| Drawable {
                position: position.vector,
                sun: false,
                radius: dimensions.radius,
                select_marker: true,
            })
            .collect::<Vec<_>>();
        bodies.append(&mut selection_markers);
        bodies
    }

    pub(crate) fn click(&mut self, click_position: Vector2<f64>) {
        let id_of_clicked_body = {
            <(Read<Position>, Read<Dimensions>, Read<Id>)>::query()
                .iter(&self.world)
                .map(|(position, dimensions, id)| {
                    let ball = Ball::new(dimensions.radius);
                    let distance = ball.distance_to_point(
                        &Isometry2::translation(position.vector.x, position.vector.y),
                        &Point {
                            coords: click_position,
                        },
                        true,
                    );
                    (distance, id)
                })
                .filter(|(distance, _)| distance < &5f64)
                .sorted_by(|(left_distance, _), (right_distance, _)| {
                    left_distance
                        .partial_cmp(right_distance)
                        .expect("couldn't unwrap ordering")
                })
                .next()
                .map(|(_, id)| Id { id: id.id })
        };

        if let Some(clicked_id) = id_of_clicked_body {
            // we clicked something, clear selected
            <(Read<Id>, Write<MetaInfo>)>::query().for_each_mut(
                &mut self.world,
                |(id, mut meta_info)| {
                    if &clicked_id == id.as_ref() {
                        meta_info.selected = true;
                    } else {
                        meta_info.selected = false;
                    }
                },
            );
        } else {
            <Write<MetaInfo>>::query().for_each_mut(&mut self.world, |mut meta_info| {
                meta_info.selected = false;
            });
        }
    }

    pub(crate) fn pause(&mut self) {
        self.paused = self.paused.not();
    }
}

pub(crate) struct Drawable {
    pub(crate) position: Vector2<f64>,
    pub(crate) sun: bool,
    pub(crate) radius: f64,
    pub(crate) select_marker: bool,
}

fn calculate_gravitational_force(
    position: Vector2<f64>,
    mass: f64,
    other_position: Vector2<f64>,
    other_mass: &f64,
) -> Vector2<f64> {
    let difference: Vector2<f64> = other_position - position;
    let distance = difference.magnitude();
    let gravity_direction: Vector2<f64> = difference.normalize();
    let gravity: f64 = GRAVITATIONAL_CONSTANT * (mass * other_mass) / (distance * distance);

    gravity_direction * gravity
}

fn are_colliding(
    position: Vector2<f64>,
    radius: f64,
    other_position: Vector2<f64>,
    other_radius: f64,
) -> bool {
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
    use nalgebra::{Isometry2, Point2, Vector2};
    use ncollide2d::query::PointQuery;

    use super::*;

    #[test]
    fn it_works() {
        let vector: Vector2<f64> = Vector2::new(11., 11.);
        let vector1 = Vector2::new(10., 10.);

        let result: Vector2<f64> = vector1 - vector;

        let result = result.magnitude();

        print!("{:?}", result)
    }

    #[test]
    fn test_click_inside() {
        let cuboid = Ball::new(1.);
        let click_pos = Point2::from(Vector2::new(11., 20.));

        let cuboid_pos = Isometry2::translation(10., 20.);

        // Solid projection.
        assert_eq!(cuboid.distance_to_point(&cuboid_pos, &click_pos, true), 0.0);
    }
}
