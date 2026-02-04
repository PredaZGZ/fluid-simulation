use nannou::prelude::*;

struct Particle {
    position: Vec2,
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
    };
    Model {
        _window,
        particle: initial_particle,
    }
}

fn update(_app: &App, _model: &mut Model, _update: Update) {}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();

    draw.background().color(BLACK);

    draw.ellipse()
        .color(DARKGREY)
        .w_h(20.0, 20.0)
        .xy(model.particle.position);

    draw.to_frame(app, &frame).unwrap();
}
