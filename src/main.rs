use quicksilver::geom::Circle;
use quicksilver::graphics::VectorFont;
use quicksilver::{
    geom::Vector, graphics::Color, run, Graphics, Input, Result, Settings, Timer, Window,
};

use crate::core::Core;

mod core;

pub(crate) const WIDTH: f32 = 1600.0;
pub(crate) const HEIGHT: f32 = 1200.0;
#[cfg(debug_assertions)]
pub(crate) const NUM_BODIES: i32 = 5;
#[cfg(not(debug_assertions))]
pub(crate) const NUM_BODIES: i32 = 100;
pub(crate) const BODY_INITIAL_MASS_MAX: f32 = 50.;
pub(crate) const INITIAL_SPEED: i32 = 50;
pub(crate) const SUN_SIZE: f32 = 1000.;
pub(crate) const GRAVITATIONAL_CONSTANT: f32 = 0.5;

fn main() {
    run(
        Settings {
            title: "Rusteroids",
            size: Vector {
                x: WIDTH,
                y: HEIGHT,
            },
            ..Settings::default()
        },
        app,
    );
}

async fn app(window: Window, mut gfx: Graphics, mut input: Input) -> Result<()> {
    let mut core = Core::new();
    core.init();

    // Here we make 2 kinds of timers.
    // One to provide an consistant update time, so our example updates 30 times per second
    // the other informs us when to draw the next frame, this causes our example to draw 60 times per second
    let mut update_timer = Timer::time_per_second(60.0);
    let mut draw_timer = Timer::time_per_second(60.0);

    let ttf = VectorFont::load("BebasNeue-Regular.ttf").await?;
    let mut font = ttf.to_renderer(&gfx, 72.0)?;

    loop {
        while input.next_event().await.is_some() {}

        let dt = update_timer.elapsed().as_secs_f32();
        // We use a while loop rather than an if so that we can try to catch up in the event of having a slow down.
        while update_timer.tick() {
            core.tick(dt);
        }

        // Unlike the update cycle drawing doesn't change our state
        // Because of this there is no point in trying to catch up if we are ever 2 frames late
        // Instead it is better to drop/skip the lost frames
        if draw_timer.exhaust().is_some() {
            gfx.clear(Color::BLACK);

            let drawables = core.draw();
            let num_bodies = drawables.len();
            for drawable in drawables {
                let circle = Circle::new(
                    Vector::new(drawable.position.x, drawable.position.y),
                    drawable.radius,
                );
                gfx.fill_circle(
                    &circle,
                    match drawable.sun {
                        true => Color::YELLOW,
                        false => Color::WHITE,
                    },
                );
            }

            font.draw(
                &mut gfx,
                format!("Bodies: {}", num_bodies).as_str(),
                Color::GREEN,
                Vector::new(50.0, 50.0),
            )?;

            gfx.present(&window)?;
        }
    }
}
