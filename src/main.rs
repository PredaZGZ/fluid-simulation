use nannou::prelude::*;

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
        .size(900, 900)
        .min_size(1000, 900)
        .max_size(1000, 900)
        .view(view) // Connects the view function
        .build()
        .unwrap();

    app.set_loop_mode(LoopMode::rate_fps(60.0));

    let mut particles = Vec::new();
    let spacing = 20.0;
    let num_x = 20;
    let num_y = 10;
    // Total: 400 particles

    let start_x = -200.0;
    let start_y = 200.0;

    for i in 0..num_x {
        for j in 0..num_y {
            let x = start_x + (i as f32 * spacing);
            let y = start_y - (j as f32 * spacing);

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
    // Delta time between frames
    let dt = update.since_last.as_secs_f32();

    let gravity = vec2(0.0, -981.0); // 9.81 m/s^2 * 10 px/m = 98.1 px/s^2

    let win = app.window_rect();

    for particle in model.particle.iter_mut() {
        // Force integration
        particle.velocity += gravity * dt;

        // Velocity integration
        particle.position += particle.velocity * dt;

        // Damping
        let damping = -0.85; // reflection coefficient
        let radius = 5.0;

        // Collision detection
        if particle.position.x > win.right() - radius {
            particle.position.x = win.right() - radius;
            particle.velocity.x *= damping;
        }
        if particle.position.x < win.left() + radius {
            particle.position.x = win.left() + radius;
            particle.velocity.x *= damping;
        }
        if particle.position.y > win.top() - radius {
            particle.position.y = win.top() - radius;
            particle.velocity.y *= damping;
        }
        if particle.position.y < win.bottom() + radius {
            particle.position.y = win.bottom() + radius;
            particle.velocity.y *= damping;
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
            .w_h(10.0, 10.0)
            .xy(particle.position);
    }

    let fps_text = format!("FPS: {:.0}", app.fps());
    draw.text(&fps_text)
        .font_size(16)
        .color(WHITE)
        .x_y(win.left() + 40.0, win.top() - 20.0);
    draw.to_frame(app, &frame).unwrap();
}
