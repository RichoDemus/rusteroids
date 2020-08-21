use legion::prelude::*;
use rand::Rng;
use ggez::{graphics, Context, ContextBuilder, GameResult, timer};
use ggez::event::{self, EventHandler};
use ggez::nalgebra::Point2;
use ggez::conf::{WindowMode, FullscreenType, WindowSetup};
use nalgebra::Vector2;

// Define our entity data types
#[derive(Clone, Copy, Debug, PartialEq)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct MyVector2 {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Velocity {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Mass(f32);

#[derive(Clone, Copy, Debug, PartialEq)]
struct Model(usize);

#[derive(Clone, Copy, Debug, PartialEq)]
struct Static;

const WIDTH: f32 = 800.0;
const HEIGHT: f32 = 600.0;
const NUM_BODIES: i32 = 100;

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
            resizable: false
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
    universe: Universe,
    world: World,
}

impl MyGame {
    pub fn new(_ctx: &mut Context) -> MyGame {
        let universe = Universe::new();
        let mut world = universe.create_world();

        let mut rng = rand::thread_rng();
        world.insert(
            (),
            (0..NUM_BODIES).map(|_| (
                Position { x: rng.gen_range(0., WIDTH), y: rng.gen_range(0., HEIGHT) },
                Velocity { x: 0.0, y: 0.0 },
                Mass(1.)
            )),
        );

        MyGame {
            universe,
            world,
        }
    }

    pub fn draw_fps_counter(&self, ctx: &mut Context) -> GameResult<()> {
        let fps = timer::fps(ctx);
        let delta = timer::delta(ctx);
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
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {

        let mut query = <(Write<Position>, Write<Velocity>, Read<Mass>)>::query();

// Iterate through all entities that match the query in the world
        let dt = timer::delta(ctx).as_secs_f32();
        for (mut pos, mut vel, mass) in query.iter_mut(&mut self.world) {
            let force = MyVector2 { x: 0.0, y: mass.0 * 9.81 };
            let acceleration = MyVector2 { x: force.x / mass.0, y: force.y / mass.0 };
            vel.x += acceleration.x * dt;
            vel.y += acceleration.y * dt;
            pos.x += vel.x * dt;
            pos.y += vel.y * dt;
        }

        let mut query = <(Read<Position>)>::query();

        let entities_to_delete = query.iter_entities(&mut self.world)
            .filter(|(_, pos)| pos.y > HEIGHT)
            .map(|(entity,_)|entity)
            .collect::<Vec<_>>();

        entities_to_delete.into_iter().for_each(|entity| {
            self.world.delete(entity);
        });

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx, [0.1, 0.2, 0.3, 1.0].into());
        self.draw_fps_counter(ctx)?;

        let query = <(Read<Position>)>::query();
        for pos in query.iter((&self.world)) {
            let circle = graphics::Mesh::new_circle(
                ctx,
                graphics::DrawMode::fill(),
                Point2::new(0.0, 0.0),
                1.0,
                2.0,
                graphics::WHITE,
            )?;
            graphics::draw(ctx, &circle, (Point2::new(pos.x, pos.y),))?;
        }

        graphics::present(ctx)
    }
}
