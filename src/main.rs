use nannou::prelude::*;

const WINDOW_WIDTH: u32 = 900;
const WINDOW_HEIGHT: u32 = 900;
const MIN_WINDOW_WIDTH: u32 = 1000;
const MIN_WINDOW_HEIGHT: u32 = 900;

const PARTICLE_SPACING: f32 = 20.0;
const NUM_PARTICLES_X: i32 = 20;
const NUM_PARTICLES_Y: i32 = 10;
const START_X: f32 = -200.0;
const START_Y: f32 = 200.0;

const GRAVITY_Y: f32 = -981.0; // m/s^2 * 100 px/m = 981 px/s^2
const DAMPING: f32 = -0.85; // Inelastic collision
const PARTICLE_RADIUS: f32 = 5.0;
const PARTICLE_DIAMETER: f32 = PARTICLE_RADIUS * 2.0;

struct Particle {
    position: Vec2,
    velocity: Vec2,
}

struct Model {
    _window: window::Id,
    particle: Vec<Particle>,
}

fn main() {
    nannou::app(model).update(update).run();
}

fn model(app: &App) -> Model {
    let _window = app
        .new_window()
        .title("Fluid Simulation")
        .size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .min_size(MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT)
        .max_size(MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT)
        .view(view)
        .build()
        .unwrap();

    app.set_loop_mode(LoopMode::rate_fps(60.0));

    let mut particles = Vec::new();

    for i in 0..NUM_PARTICLES_X {
        for j in 0..NUM_PARTICLES_Y {
            let x = START_X + (i as f32 * PARTICLE_SPACING);
            let y = START_Y - (j as f32 * PARTICLE_SPACING);

            let p = Particle {
                position: vec2(x, y),
                velocity: vec2(random_range(-200.0, 200.0), random_range(-200.0, 200.0)),
            };

            particles.push(p);
        }
    }

    println!("{} particles", particles.len());

    Model {
        _window,
        particle: particles,
    }
}

fn update(app: &App, model: &mut Model, update: Update) {
    let dt = update.since_last.as_secs_f32();
    let win = app.window_rect();

    for particle in model.particle.iter_mut() {
        // Force integration
        particle.velocity += vec2(0.0, GRAVITY_Y) * dt;

        // Velocity integration
        particle.position += particle.velocity * dt;

        // Collision detection
        if particle.position.x > win.right() - PARTICLE_RADIUS {
            particle.position.x = win.right() - PARTICLE_RADIUS;
            particle.velocity.x *= DAMPING;
        }
        if particle.position.x < win.left() + PARTICLE_RADIUS {
            particle.position.x = win.left() + PARTICLE_RADIUS;
            particle.velocity.x *= DAMPING;
        }
        if particle.position.y > win.top() - PARTICLE_RADIUS {
            particle.position.y = win.top() - PARTICLE_RADIUS;
            particle.velocity.y *= DAMPING;
        }
        if particle.position.y < win.bottom() + PARTICLE_RADIUS {
            particle.position.y = win.bottom() + PARTICLE_RADIUS;
            particle.velocity.y *= DAMPING;
        }
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    let win = app.window_rect();

    draw.background().color(BLACK);

    for particle in model.particle.iter() {
        draw.ellipse()
            .color(DARKGREY)
            .w_h(PARTICLE_DIAMETER, PARTICLE_DIAMETER)
            .xy(particle.position);
    }

    let fps_text = format!("FPS: {:.0}", app.fps());
    draw.text(&fps_text)
        .font_size(16)
        .color(WHITE)
        .x_y(win.left() + 40.0, win.top() - 20.0);
    draw.to_frame(app, &frame).unwrap();
}
