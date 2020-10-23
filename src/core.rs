use std::collections::HashMap;
use std::f64::consts::PI;
use std::ops::Not;

use itertools::Itertools;
use legion::prelude::*;
use nalgebra::{Isometry2, Point, Point2, Vector2};
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
    point: Point2<f64>,
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
    predicted_orbit: Option<Vec<Point2<f64>>>,
}

impl Core {
    pub(crate) fn new() -> Core {
        let universe = Universe::new();
        let world = universe.create_world();
        Core {
            world,
            paused: false,
            predicted_orbit: None,
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
                    point: Point2::new((WIDTH as f32 / 2.).into(), (HEIGHT as f32 / 2.).into()),
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
                        point: Point2::new(x, y),
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

    pub(crate) fn tick(&mut self, dt: f64, camera_x_axis: f64, camera_y_axis: f64) {
        if self.paused {
            if self.predicted_orbit.is_none() {
                self.predicted_orbit = Some(predict_orbit(dt, &self.world));
            }
            return;
        }

        let bodies = get_bodies(&self.world);

        let updated_bodies = do_one_physics_step(dt, bodies);

        let (bodies_to_delete, bodies_to_update): (Vec<_>, Vec<_>) =
            updated_bodies.into_iter().partition(|body| body.delete);
        let bodies_to_update = bodies_to_update
            .into_iter()
            .map(|body| (body.id, body))
            .collect::<HashMap<_, _>>();

        let ids_to_delete = bodies_to_delete
            .into_iter()
            .map(|body| body.id)
            .collect::<Vec<_>>();
        let mut entities_to_delete = vec![];

        let query = <(
            Write<Position>,
            Write<Velocity>,
            Write<Dimensions>,
            Read<Id>,
        )>::query();
        for (entity, (mut pos, mut velocity, mut dimensions, id)) in
            query.iter_entities_mut(&mut self.world)
        {
            if ids_to_delete.contains(&id.id) {
                entities_to_delete.push(entity)
            } else {
                let updated_version = bodies_to_update
                    .get(&id.id)
                    .expect("updated body should exist");
                pos.point = updated_version.position;
                // camera movement
                pos.point += Vector2::new(camera_x_axis, camera_y_axis);
                velocity.vector = updated_version.velocity;
                dimensions.mass = updated_version.mass; //todo recalculate radius
            }
        }

        for entity in entities_to_delete {
            self.world.delete(entity);
        }
    }

    pub(crate) fn draw(&self) -> (Vec<Drawable>, Vec<Point2<f64>>) {
        let query = <(Read<Position>, Read<Data>, Read<Dimensions>)>::query();
        let mut bodies = query
            .iter(&self.world)
            .map(|(pos, data, dimensions)| {
                let position = *pos;
                let position: Point2<f64> = position.point;
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
                position: position.point,
                sun: false,
                radius: dimensions.radius,
                select_marker: true,
            })
            .collect::<Vec<_>>();

        bodies.append(&mut selection_markers);
        (bodies, self.predicted_orbit.clone().unwrap_or_default())
    }

    pub(crate) fn click(&mut self, click_position: Vector2<f64>) {
        self.predicted_orbit = None;
        let id_of_clicked_body = {
            <(Read<Position>, Read<Dimensions>, Read<Id>)>::query()
                .iter(&self.world)
                .map(|(position, dimensions, id)| {
                    let ball = Ball::new(dimensions.radius);
                    let distance = ball.distance_to_point(
                        &Isometry2::translation(position.point.x, position.point.y),
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
    pub(crate) position: Point2<f64>,
    pub(crate) sun: bool,
    pub(crate) radius: f64,
    pub(crate) select_marker: bool,
}

fn calculate_gravitational_force(
    position: &Point2<f64>,
    mass: &f64,
    other_position: &Point2<f64>,
    other_mass: &f64,
) -> Vector2<f64> {
    let difference: Vector2<f64> = other_position - position;
    let distance = difference.magnitude();
    let gravity_direction: Vector2<f64> = difference.normalize();
    let gravity: f64 = GRAVITATIONAL_CONSTANT * (mass * other_mass) / (distance * distance);

    gravity_direction * gravity
}

fn are_colliding(
    position: Point2<f64>,
    radius: f64,
    other_position: Point2<f64>,
    other_radius: f64,
) -> bool {
    let shape = Ball::new(radius);
    let position = Isometry2::new(position.coords, nalgebra::zero());
    let other_shape = Ball::new(other_radius);
    let other_position = Isometry2::new(other_position.coords, nalgebra::zero());

    let proximity = query::proximity(&position, &shape, &other_position, &other_shape, 0.);
    if let Proximity::Intersecting = proximity {
        true
    } else {
        false
    }
}

fn get_bodies(world: &World) -> Vec<Body> {
    <(
        Read<Position>,
        Read<Velocity>,
        Read<Dimensions>,
        Read<MetaInfo>,
        Read<Id>,
        Read<Data>,
    )>::query()
    .iter(world)
    .map(|(pos, velocity, dimensions, meta_info, id, data)| Body {
        position: pos.point,
        velocity: velocity.vector,
        radius: dimensions.radius,
        mass: dimensions.mass,
        selected: meta_info.selected,
        id: id.id,
        sun: data.sun,
        delete: false,
    })
    .collect::<Vec<_>>()
}

fn predict_orbit(time_step: f64, world: &World) -> Vec<Point2<f64>> {
    let mut bodies = get_bodies(world);

    let mut predicted_positions = vec![];
    for i in 0..10000 {
        bodies = do_one_physics_step(time_step, bodies);
        bodies = bodies
            .into_iter()
            .filter(|body| !body.delete)
            .collect::<Vec<_>>();
        if i % 100 == 0 {
            let maybe_selected = bodies.iter().find(|body| body.selected);
            if let Some(body) = maybe_selected {
                predicted_positions.push(body.position);
            }
        }
    }
    predicted_positions
}

// intermediare struct to pass a body around
#[derive(Clone, Debug)]
struct Body {
    position: Point2<f64>,
    velocity: Vector2<f64>,
    radius: f64,
    mass: f64,
    selected: bool,
    id: i32,
    sun: bool,
    delete: bool,
}

fn do_one_physics_step(time_step: f64, mut bodies: Vec<Body>) -> Vec<Body> {
    // calculate new velocities
    let clones = bodies.clone();
    bodies = bodies
        .into_iter()
        .map(|mut body| {
            for clone in &clones {
                if body.id == clone.id || body.sun {
                    continue;
                }
                let gravitational_force = calculate_gravitational_force(
                    &body.position,
                    &body.mass,
                    &clone.position,
                    &clone.mass,
                );
                body.velocity += gravitational_force * time_step;
            }
            body
        })
        .collect::<Vec<_>>();
    // move bodies
    bodies = bodies
        .into_iter()
        .map(|mut body| {
            body.position += body.velocity * time_step;
            body
        })
        .collect::<Vec<_>>();

    // collision detection
    let clones = bodies.clone();
    bodies = bodies
        .into_iter()
        .map(|mut body| {
            for clone in &clones {
                if body.id == clone.id || body.sun {
                    continue;
                }
                if are_colliding(body.position, body.radius, clone.position, clone.radius) {
                    // the bigger body swallows the smaller one
                    // this will happen twice for each collision, with this and other swapped, lets utilize this
                    if body.mass > clone.mass {
                        // when this is the bigger one, enlarge it
                        let mass_ratio = clone.mass / body.mass;
                        body.velocity += clone.velocity * mass_ratio;
                        body.mass += clone.mass;
                    } else {
                        // when it's the smaller one, schedule it for deletion
                        body.delete = true;
                    }
                }
            }
            body
        })
        .collect::<Vec<_>>();

    bodies
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
