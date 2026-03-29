mod sph;

use nannou::prelude::*;
use sph::{Interaction, InteractionMode, SphConfig, SphSimulation};

const WINDOW_WIDTH: u32 = 900;
const WINDOW_HEIGHT: u32 = 900;
const FIXED_TIME_STEP: f32 = 1.0 / 240.0;
const MAX_STEPS_PER_FRAME: usize = 6;
const PIXELS_PER_METER: f32 = 110.0;
const MAX_FRAME_DELTA: f32 = 1.0 / 30.0;
const INITIAL_PARTICLES_X: usize = 36;
const INITIAL_PARTICLES_Y: usize = 48;
const PARTICLE_DRAW_RESOLUTION: f32 = 6.0;
const INTERACTION_DRAW_RESOLUTION: f32 = 24.0;

struct Model {
    _window: window::Id,
    simulation: SphSimulation,
    accumulator: f32,
    steps_last_frame: usize,
}

fn main() {
    nannou::app(model).update(update).run();
}

fn model(app: &App) -> Model {
    let _window = app
        .new_window()
        .title("WCSPH Fluid Simulation")
        .size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .min_size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .max_size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .view(view)
        .build()
        .unwrap();

    app.set_loop_mode(LoopMode::rate_fps(60.0));

    let bounds = simulation_bounds();
    let mut simulation = SphSimulation::new(SphConfig::new(bounds));
    let config = *simulation.config();
    let origin = vec2(
        config.bounds.left() + config.bounds.w() * 0.14,
        config.bounds.bottom() + config.bounds.h() * 0.12,
    );

    simulation.seed_block(INITIAL_PARTICLES_X, INITIAL_PARTICLES_Y, origin);

    Model {
        _window,
        simulation,
        accumulator: 0.0,
        steps_last_frame: 0,
    }
}

fn update(app: &App, model: &mut Model, update: Update) {
    model.accumulator += update.since_last.as_secs_f32().min(MAX_FRAME_DELTA);

    let interaction = active_interaction(app, *model.simulation.config());
    let mut steps = 0;

    while model.accumulator >= FIXED_TIME_STEP && steps < MAX_STEPS_PER_FRAME {
        model.simulation.step(FIXED_TIME_STEP, interaction);
        model.accumulator -= FIXED_TIME_STEP;
        steps += 1;
    }

    if steps == MAX_STEPS_PER_FRAME {
        model.accumulator = 0.0;
    }

    model.steps_last_frame = steps;
    if steps > 0 {
        model.simulation.refresh_stats();
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    let win = app.window_rect();
    let config = *model.simulation.config();

    draw.background().color(srgba(0.03, 0.04, 0.07, 1.0));

    draw.rect()
        .xy(world_to_screen(config.bounds.xy()))
        .w_h(
            config.bounds.w() * PIXELS_PER_METER,
            config.bounds.h() * PIXELS_PER_METER,
        )
        .no_fill()
        .stroke(srgba(0.65, 0.78, 0.95, 0.65))
        .stroke_weight(2.0);

    for ((position, velocity), density) in model
        .simulation
        .positions()
        .iter()
        .zip(model.simulation.velocities())
        .zip(model.simulation.densities())
    {
        let speed = velocity.length();
        let density_ratio = (*density / config.rest_density).clamp(0.85, 1.35);
        let hue = map_range(speed, 0.0, 8.0, 0.56, 0.03).clamp(0.03, 0.56);
        let lightness = map_range(density_ratio, 0.85, 1.35, 0.44, 0.68).clamp(0.40, 0.72);

        draw.ellipse()
            .xy(world_to_screen(*position))
            .radius(config.particle_radius * PIXELS_PER_METER)
            .resolution(PARTICLE_DRAW_RESOLUTION)
            .hsla(hue, 0.78, lightness, 0.95);
    }

    if let Some(interaction) = active_interaction(app, config) {
        let ring_color = match interaction.mode {
            InteractionMode::Attract => srgba(0.35, 0.82, 0.95, 0.35),
            InteractionMode::Repel => srgba(0.95, 0.45, 0.35, 0.35),
        };

        draw.ellipse()
            .xy(world_to_screen(interaction.position))
            .radius(interaction.radius * PIXELS_PER_METER)
            .resolution(INTERACTION_DRAW_RESOLUTION)
            .no_fill()
            .stroke(ring_color)
            .stroke_weight(2.0);
    }

    let stats = model.simulation.stats();
    let hud = format!(
        "WCSPH + rayon\nparticles: {}  threads: {}  fps: {:.0}\nmax speed: {:.2} m/s  density: {:.2} rho0  steps/frame: {}\nmouse: left attracts, right repels",
        stats.particle_count,
        stats.threads,
        app.fps(),
        stats.max_speed,
        stats.max_density_ratio,
        model.steps_last_frame,
    );

    draw.text(&hud)
        .left_justify()
        .color(WHITE)
        .font_size(16)
        .w_h(470.0, 110.0)
        .x_y(win.left() + 240.0, win.top() - 48.0);

    draw.to_frame(app, &frame).unwrap();
}

fn simulation_bounds() -> Rect {
    Rect::from_w_h(
        WINDOW_WIDTH as f32 / PIXELS_PER_METER,
        WINDOW_HEIGHT as f32 / PIXELS_PER_METER,
    )
}

fn active_interaction(app: &App, config: SphConfig) -> Option<Interaction> {
    let mouse_position = screen_to_world(app.mouse.position());

    if app.mouse.buttons.left().is_down() {
        Some(Interaction {
            position: mouse_position,
            radius: config.interaction_radius,
            strength: config.interaction_strength,
            mode: InteractionMode::Attract,
        })
    } else if app.mouse.buttons.right().is_down() {
        Some(Interaction {
            position: mouse_position,
            radius: config.interaction_radius,
            strength: config.interaction_strength,
            mode: InteractionMode::Repel,
        })
    } else {
        None
    }
}

fn screen_to_world(position: Vec2) -> Vec2 {
    position / PIXELS_PER_METER
}

fn world_to_screen(position: Vec2) -> Vec2 {
    position * PIXELS_PER_METER
}
