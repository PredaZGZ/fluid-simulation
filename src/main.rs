use nannou::prelude::*;

struct Particle {
    position: Vec2,
    velocity: Vec2,
}

struct Model {
    _window: window::Id,
    particle: Particle,
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

    let initial_particle = Particle {
        position: vec2(0.0, 0.0), // (0,0) is center of screen
        velocity: vec2(5.0, 2.5),
    };
    Model {
        _window,
        particle: initial_particle,
    }
}

fn update(app: &App, model: &mut Model, _update: Update) {
    model.particle.position += model.particle.velocity;

    let win = app.window_rect();

    if model.particle.position.x > win.right() || model.particle.position.x < win.left() {
        model.particle.velocity.x *= -1.0; // Reflect the particle
    }

    if model.particle.position.y > win.top() || model.particle.position.y < win.bottom() {
        model.particle.velocity.y *= -1.0; // Reflect the particle
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();

    draw.background().color(BLACK);

    draw.ellipse()
        .color(DARKGREY)
        .w_h(20.0, 20.0)
        .xy(model.particle.position);

    draw.to_frame(app, &frame).unwrap();
}
