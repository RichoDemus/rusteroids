use ggez::{Context, ContextBuilder, GameResult, graphics, timer};
use ggez::conf::{FullscreenType, WindowMode, WindowSetup};
use ggez::event::{self, EventHandler};
use ggez::nalgebra::Point2;
use legion::prelude::*;
use nalgebra::Vector2;
use rand::Rng;

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
struct Mass(f32);

#[derive(Clone, Debug, PartialEq)]
struct Data {
    name: String
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Model(usize);

#[derive(Clone, Copy, Debug, PartialEq)]
struct Static;

const WIDTH: f32 = 800.0;
const HEIGHT: f32 = 600.0;
const NUM_BODIES: i32 = 100;
const GRAVITATIONAL_CONSTANT: f32 = 1.0f32;

fn main() {

    // {
    //     let vec = Vector2::new(0., 1.);
    //     let vec2 = Vector2::new(1.,0.);
    //     let vec3 = vec + vec2;
    //     let mut vec4 = vec3.clone();
    //     vec4.set_magnitude(1.);
    //
    //     println!("vector: {:?}", vec);
    //     println!("vector2: {:?}", vec2);
    //     println!("vector3: {:?}", vec3);
    //     println!("vector4: {:?}", vec4);
    // }


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
            (0..NUM_BODIES).map(|i| {
                let x = rng.gen_range(0., WIDTH);
                let y = rng.gen_range(0., HEIGHT);

                let x_velocity = rng.gen_range(-1., 1.);
                let y_velocity = rng.gen_range(-1., 1.);

                (
                    Data { name: i.to_string() },
                    Position { vector: Vector2::new(x, y) },
                    Velocity { vector: Vector2::new(x_velocity, y_velocity) },
                    Mass(1.)
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
            (Point2::new(0.0, 0.0), graphics::BLACK),
        )
    }
}

impl EventHandler for MyGame {
    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
        let query = <(Read<Position>, Read<Data>)>::query();
        let bodies = query.iter(&self.world)
            .map(|(pos, data)| (*pos, data.as_ref().clone()))
            .collect::<Vec<_>>();

        let query = <(Read<Position>, Write<Velocity>, Read<Data>)>::query();
        for (position, mut velocity, data) in query.iter_mut(&mut self.world) {
            for (other_position, other_data) in &bodies {
                let data: &Data = &data;
                let other_data: &Data = other_data;
                if data == other_data {
                    continue;
                }
                let position: &Position = &position;
                // println!("\t\tComparing body {:?}: {:?} with {:?}: {:?}", data, position, other_data, other_position);


                let difference: Vector2<f32> = &other_position.vector - &position.vector;
                let distance = difference.magnitude();
                let gravity_direction: Vector2<f32> = difference.normalize();
                let gravity: f32 = GRAVITATIONAL_CONSTANT * (1. * 1.) / (distance * distance);

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

        let query = <(Write<Position>, Read<Velocity>)>::query();
        for (mut position, velocity) in query.iter_mut(&mut self.world) {
            let current_position: Vector2<f32> = position.vector.clone_owned();
            let velocity: Vector2<f32> = velocity.vector.clone_owned();

            let new_position: Vector2<f32> = current_position + velocity;

            position.vector = new_position;
        }

        // collision detection
        let query = <(Read<Position>, Read<Data>)>::query();
        let bodies = query.iter(&self.world)
            .map(|(pos, data)| (*pos, data.as_ref().clone()))
            .collect::<Vec<_>>();

        let query = <(Read<Position>, Write<Velocity>, Read<Data>)>::query();
        let mut entities_to_delete = vec![];
        for (entity, (position, _velocity, data)) in query.iter_entities_mut(&mut self.world) {
            for (other_position, other_data) in &bodies {
                let data: &Data = &data;
                let other_data: &Data = other_data;
                if data == other_data {
                    continue;
                }

                let difference: Vector2<f32> = &other_position.vector - &position.vector;
                let distance = difference.magnitude();
                if distance < 0.99 {
                    entities_to_delete.push(entity);
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

        let query = <Read<Position>>::query();
        for pos in query.iter(&self.world) {
            let circle = graphics::Mesh::new_circle(
                ctx,
                graphics::DrawMode::fill(),
                Point2::new(0.0, 0.0),
                1.0,
                2.0,
                graphics::WHITE,
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
}
