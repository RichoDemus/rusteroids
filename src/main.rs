use ggez::conf::{FullscreenType, WindowMode, WindowSetup};
use ggez::event::{self, EventHandler};
use ggez::graphics::{Color, WHITE};
use ggez::nalgebra::Point2;
use ggez::{graphics, timer, Context, ContextBuilder, GameResult};

use crate::core::Core;

mod core;

pub(crate) const WIDTH: f32 = 1600.0;
pub(crate) const HEIGHT: f32 = 1200.0;
pub(crate) const NUM_BODIES: i32 = 100;
pub(crate) const BODY_INITIAL_MASS_MAX: f32 = 50.;
pub(crate) const INITIAL_SPEED: i32 = 50;
pub(crate) const SUN_SIZE: f32 = 1000.;
pub(crate) const GRAVITATIONAL_CONSTANT: f32 = 0.5;
pub(crate) const YELLOW: Color = Color::new(1., 1.0, 0., 1.0);
pub(crate) const GREEN: Color = Color::new(0., 1.0, 0., 1.0);

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
        Err(e) => println!("Error occured: {}", e),
    }
}

struct MyGame {
    core: Core,
}

impl MyGame {
    pub fn new(_ctx: &mut Context) -> MyGame {
        let mut core = Core::new();
        core.init();
        MyGame { core }
    }

    pub fn draw_fps_counter(&self, ctx: &mut Context) -> GameResult<()> {
        let fps = timer::fps(ctx);
        let stats_display = graphics::Text::new(format!("FPS: {:.0}", fps));
        graphics::draw(ctx, &stats_display, (Point2::new(0.0, 0.0), GREEN))
    }

    pub fn draw_body_counter(&self, ctx: &mut Context, bodies: usize) -> GameResult<()> {
        let stats_display = graphics::Text::new(format!("Bodies: {:.0}", bodies));
        graphics::draw(ctx, &stats_display, (Point2::new(0.0, 20.0), GREEN))
    }
}

impl EventHandler for MyGame {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        let dt = timer::delta(ctx).as_secs_f32();

        self.core.tick(dt);

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx, [0.1, 0.2, 0.3, 1.0].into());
        self.draw_fps_counter(ctx)?;

        let drawables = self.core.draw();
        let num_drawables = drawables.len();
        for drawable in drawables {
            let circle = graphics::Mesh::new_circle(
                ctx,
                graphics::DrawMode::fill(),
                Point2::new(0.0, 0.0),
                drawable.radius,
                2.0,
                match drawable.sun {
                    true => YELLOW,
                    false => WHITE,
                },
            )?;
            graphics::draw(
                ctx,
                &circle,
                (Point2::new(drawable.position.x, drawable.position.y),),
            )?;
        }

        self.draw_body_counter(ctx, num_drawables)?;

        graphics::present(ctx)
    }
}
