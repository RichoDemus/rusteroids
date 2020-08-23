use ggez::{Context, ContextBuilder, GameResult, graphics, timer};
use ggez::conf::{FullscreenType, WindowMode, WindowSetup};
use ggez::event::{self, EventHandler};
use ggez::graphics::{Color, WHITE};
use ggez::nalgebra::Point2;
use legion::prelude::*;
use nalgebra::Vector2;
use rand::Rng;
use std::f32::consts::PI;

// Define our entity data types
#[derive(Clone, Copy, Debug, PartialEq)]
struct Position {
    vector: Vector2<f32>
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct MyVector2 {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Velocity {
    vector: Vector2<f32>
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
        Dimensions {
            mass,
            radius
        }
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

const WIDTH: f32 = 1600.0;
const HEIGHT: f32 = 1200.0;
const NUM_BODIES: i32 = 100;
const BODY_INITIAL_MASS_MAX: f32 = 50.;
const INITIAL_SPEED: i32 = 50;
const SUN_SIZE: f32 = 1000.;
const GRAVITATIONAL_CONSTANT: f32 = 0.5;
const YELLOW: Color = Color::new(1., 1.0, 0., 1.0);
const GREEN: Color = Color::new(0., 1.0, 0., 1.0);

fn main() {
    let mut window_setup = WindowSetup::default();
    window_setup.vsync = false;
    let (mut ctx, mut event_loop) = ContextBuilder::new("my_game", "Cool Game Author")
        .window_mode(WindowMode {
            width: WIDTH,
            height: HEIGHT,
            maximized: false,
            fullscreen_type: FullscreenType::Windowed,
            borderless: false,
            min_width: 0.0,
            min_height: 0.0,
            max_width: 0.0,
            max_height: 0.0,
            resizable: false,
        })
        .window_setup(window_setup)
        .build()
        .expect("aieee, could not create ggez context!");

    // Create an instance of your event handler.
    // Usually, you should provide it with the Context object to
    // use when setting your game up.
    let mut my_game = MyGame::new(&mut ctx);

    // Run!
    match event::run(&mut ctx, &mut event_loop, &mut my_game) {
        Ok(_) => println!("Exited cleanly."),
        Err(e) => println!("Error occured: {}", e)
    }
}

struct MyGame {
    // universe: Universe,
    world: World,
}

impl MyGame {
    pub fn new(_ctx: &mut Context) -> MyGame {
        let universe = Universe::new();
        let mut world = universe.create_world();

        let mut rng = rand::thread_rng();
        world.insert(
            (),
            vec![(
                Data { name: "sun".to_string(), sun: true },
                Position { vector: Vector2::new(WIDTH / 2., HEIGHT / 2.) },
                Velocity { vector: Vector2::new(0., 0.) },
                Dimensions::from_mass(SUN_SIZE)
            )],
        );
        world.insert(
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
                    speed=> rng.gen_range(-speed as f32, speed as f32),
                };

                let mass = rng.gen_range(1., BODY_INITIAL_MASS_MAX);
                (
                    Data { name: i.to_string(), sun: false },
                    Position { vector: Vector2::new(x, y) },
                    Velocity { vector: Vector2::new(x_velocity, y_velocity) },
                    Dimensions::from_mass(mass)
                )
            }),
        );

        MyGame {
            // universe,
            world,
        }
    }

    pub fn draw_fps_counter(&self, ctx: &mut Context) -> GameResult<()> {
        let fps = timer::fps(ctx);
        // let delta = timer::delta(ctx);
        // let stats_display = graphics::Text::new(format!("FPS: {:.0}, delta: {:?}", fps, delta));
        let stats_display = graphics::Text::new(format!("FPS: {:.0}", fps));
        // println!(
        //     "[draw] ticks: {}\tfps: {}\tdelta: {:?}",
        //     timer::ticks(ctx),
        //     fps,
        //     delta,
        // );
        graphics::draw(
            ctx,
            &stats_display,
            (Point2::new(0.0, 0.0), GREEN),
        )
    }

    pub fn draw_body_counter(&self, ctx: &mut Context, bodies: usize) -> GameResult<()> {
        // let fps = timer::fps(ctx);
        // let delta = timer::delta(ctx);
        // let stats_display = graphics::Text::new(format!("FPS: {:.0}, delta: {:?}", fps, delta));
        let stats_display = graphics::Text::new(format!("Bodies: {:.0}", bodies));
        // println!(
        //     "[draw] ticks: {}\tfps: {}\tdelta: {:?}",
        //     timer::ticks(ctx),
        //     fps,
        //     delta,
        // );
        graphics::draw(
            ctx,
            &stats_display,
            (Point2::new(0.0, 20.0), GREEN),
        )
    }
}

impl EventHandler for MyGame {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        let dt = timer::delta(ctx).as_secs_f32();
        let query = <(Read<Position>, Read<Dimensions>, Read<Data>)>::query();
        let bodies = query.iter(&self.world)
            .map(|(pos, dimensions, data)| (*pos, dimensions.mass, dimensions.radius, data.as_ref().clone()))
            .collect::<Vec<_>>();

        let query = <(Read<Position>, Read<Dimensions>, Write<Velocity>, Read<Data>)>::query();
        for (position, dimensions, mut velocity, data) in query.iter_mut(&mut self.world) {
            for (other_position, other_mass, _other_radius, other_data) in &bodies {
                let data: &Data = &data;
                let other_data: &Data = other_data;
                if data == other_data || data.sun {
                    continue;
                }
                let position: &Position = &position;
                // println!("\t\tComparing body {:?}: {:?} with {:?}: {:?}", data, position, other_data, other_position);


                let difference: Vector2<f32> = &other_position.vector - &position.vector;
                let distance = difference.magnitude();
                let gravity_direction: Vector2<f32> = difference.normalize();
                let gravity: f32 = GRAVITATIONAL_CONSTANT * (dimensions.mass * other_mass) / (distance * distance);

                // *velocity.vector = *velocity.vector + *velocity.vector;

                let force: Vector2<f32> = gravity_direction * gravity;
                // let force: Vector2<f32> = force.into();
                let original_vector: Vector2<f32> = velocity.vector.clone_owned();
                let new_vector: Vector2<f32> = original_vector + force;
                // println!("\tgravitational force: {:?}", force);
                // let vel:Vector2<f32> = *velocity.vector + force;
                // velocity.vector = vel;
                velocity.vector = new_vector;
                // println!("\ttotal force: {:?}", *velocity);
            }
        }

        // update positions
        let query = <(Write<Position>, Read<Velocity>)>::query();
        for (mut position, velocity) in query.iter_mut(&mut self.world) {
            let current_position: Vector2<f32> = position.vector.clone_owned();
            let velocity: Vector2<f32> = velocity.vector.clone_owned() * dt;

            let new_position: Vector2<f32> = current_position + velocity;

            position.vector = new_position;
        }

        // collision detection
        let query = <(Read<Position>, Read<Velocity>, Read<Dimensions>, Read<Data>)>::query();
        let bodies = query.iter(&self.world)
            .map(|(pos, velocity, dimensions, data)| (*pos, velocity.as_ref().clone(), dimensions.mass, dimensions.radius, data.as_ref().clone()))
            .collect::<Vec<_>>();

        let query = <(Read<Position>, Write<Velocity>, Write<Dimensions>, Read<Data>)>::query();
        let mut entities_to_delete = vec![];
        for (entity, (position, mut velocity, mut dimensions, data)) in query.iter_entities_mut(&mut self.world) {
            for (other_position, other_velocity, other_mass, other_radius, other_data) in &bodies {
                let data: &Data = &data;
                let other_data: &Data = other_data;
                if data == other_data || data.sun {
                    continue;
                }

                let difference: Vector2<f32> = &other_position.vector - &position.vector;
                let distance = difference.magnitude();

                // collision
                if dimensions.radius + other_radius > distance {
                    // the bigger body swallows the smaller one
                    // this will one twice for each collision, with this and other swapped, lets utilize this
                    if dimensions.mass > *other_mass {
                        // when this is the bigger one, enlarge it
                        // todo scale vector based on size difference
                        let mass_ratio =  *other_mass / dimensions.mass;
                        velocity.vector += &other_velocity.vector * mass_ratio;
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

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx, [0.1, 0.2, 0.3, 1.0].into());
        self.draw_fps_counter(ctx)?;

        let bodies = <Read<Data>>::query().iter(&self.world).count();
        self.draw_body_counter(ctx, bodies)?;


        let query = <(Read<Position>, Read<Data>, Read<Dimensions>)>::query();
        for (pos, data, dimensions) in query.iter(&self.world) {
            let circle = graphics::Mesh::new_circle(
                ctx,
                graphics::DrawMode::fill(),
                Point2::new(0.0, 0.0),
                dimensions.radius,
                2.0,
                match data.sun {
                    true => YELLOW,
                    false => WHITE,
                },
            )?;
            graphics::draw(ctx, &circle, (Point2::new(pos.vector.x, pos.vector.y), ))?;
        }

        graphics::present(ctx)
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

    #[test]
    fn test_dimensions_from_volume() {
        let result = Dimensions::from_mass(113.);
        assert_eq!(3., result.radius)
    }
}
