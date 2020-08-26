use quicksilver::blinds::event::Key::{Escape, Space};
use quicksilver::blinds::event::MouseButton::Left;
use quicksilver::geom::{Circle, Rectangle};
use quicksilver::graphics::VectorFont;
use quicksilver::input::Event;
use quicksilver::{
    geom::Vector, graphics::Color, run, Graphics, Input, Result, Settings, Timer, Window,
};

use crate::core::Core;
use crate::util::convert;

mod core;
mod util;

// use 144 fps for non wasm release, use 60 fps for wasm or debug
#[cfg(any(target_arch = "wasm32", debug_assertions))]
pub(crate) const FPS: f32 = 60.0;
#[cfg(all(not(target_arch = "wasm32"), not(debug_assertions)))]
pub(crate) const FPS: f32 = 144.0;
pub(crate) const UPS: f32 = 200.;

pub(crate) const WIDTH: f32 = 800.0;
pub(crate) const HEIGHT: f32 = 600.0;
#[cfg(debug_assertions)]
pub(crate) const NUM_BODIES: i32 = 5;
#[cfg(not(debug_assertions))]
pub(crate) const NUM_BODIES: i32 = 100;
pub(crate) const BODY_INITIAL_MASS_MAX: f64 = 50.;
pub(crate) const INITIAL_SPEED: i32 = 50;
pub(crate) const SUN_SIZE: f64 = 1000.;
pub(crate) const GRAVITATIONAL_CONSTANT: f64 = 5.;

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
    let mut frames: u32 = 0;
    let mut last_fps: u32 = 0;
    let dt = 1. / (UPS as f64);

    // Here we make 2 kinds of timers.
    // One to provide an consistant update time, so our example updates 30 times per second
    // the other informs us when to draw the next frame, this causes our example to draw 60 times per second
    let mut update_timer = Timer::time_per_second(UPS);
    let mut draw_timer = Timer::time_per_second(FPS);
    let mut fps_timer = Timer::time_per_second(1.);

    let ttf = VectorFont::from_slice(include_bytes!("BebasNeue-Regular.ttf"));
    let mut font = ttf.to_renderer(&gfx, 72.0)?;

    let mut running = true;
    while running {
        while let Some(event) = input.next_event().await {
            if let Event::PointerInput(pointer_input_event) = event {
                if !pointer_input_event.is_down() && pointer_input_event.button() == Left {
                    let mouse_position = input.mouse().location();

                    core.click(convert(mouse_position));
                }
            } else if let Event::KeyboardInput(keyboard_event) = event {
                if keyboard_event.is_down() && keyboard_event.key() == Space {
                    core.pause();
                }
                if keyboard_event.is_down() && keyboard_event.key() == Escape {
                    running = false;
                }
            }
        }

        // We use a while loop rather than an if so that we can try to catch up in the event of having a slow down.
        while update_timer.tick() {
            core.tick(dt);
        }

        // Unlike the update cycle drawing doesn't change our state
        // Because of this there is no point in trying to catch up if we are ever 2 frames late
        // Instead it is better to drop/skip the lost frames
        if draw_timer.exhaust().is_some() {
            gfx.clear(Color::BLACK);

            let (drawables, predicted_orbit) = core.draw();
            let num_bodies = drawables.len();
            for drawable in drawables {
                if drawable.select_marker {
                    let rectangle = Rectangle::new(
                        Vector::new(
                            (drawable.position.x - 10.) as f32,
                            (drawable.position.y - 10.) as f32,
                        ),
                        Vector::new(20., 20.),
                    );
                    gfx.stroke_rect(&rectangle, Color::GREEN)
                } else {
                    let circle = Circle::new(
                        Vector::new(drawable.position.x as f32, drawable.position.y as f32),
                        drawable.radius as f32,
                    );
                    gfx.fill_circle(
                        &circle,
                        match drawable.sun {
                            true => Color::YELLOW,
                            false => Color::WHITE,
                        },
                    );
                }
            }

            for orbit_point in predicted_orbit {
                let circle =
                    Circle::new(Vector::new(orbit_point.x as f32, orbit_point.y as f32), 1.);
                gfx.fill_circle(&circle, Color::YELLOW);
            }

            font.draw(
                &mut gfx,
                format!("Bodies: {}", num_bodies).as_str(),
                Color::GREEN,
                Vector::new(5.0, 100.0),
            )?;

            frames += 1;
            if fps_timer.tick() {
                last_fps = frames;
                frames = 0;
            }
            font.draw(
                &mut gfx,
                format!("FPS: {}", last_fps).as_str(),
                Color::GREEN,
                Vector::new(5.0, 50.0),
            )?;

            gfx.present(&window)?;
        }
    }
    Ok(())
}
