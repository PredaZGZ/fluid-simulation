use nannou::prelude::*;

const WINDOW_WIDTH: u32 = 900;
const WINDOW_HEIGHT: u32 = 900;
const MIN_WINDOW_WIDTH: u32 = 1000;
const MIN_WINDOW_HEIGHT: u32 = 900;

const PARTICLE_SPACING: f32 = 15.0;
const NUM_PARTICLES_X: i32 = 40;
const NUM_PARTICLES_Y: i32 = 40;
const START_X: f32 = -200.0;
const START_Y: f32 = 200.0;

const GRAVITY_Y: f32 = -981.0; // m/s^2 * 100 px/m = 981 px/s^2
const DAMPING: f32 = -0.65; // Inelastic collision
const PARTICLE_RADIUS: f32 = 5.0;
const PARTICLE_DIAMETER: f32 = PARTICLE_RADIUS * 2.0;

const INTERACTION_RADIUS: f32 = 500.0;
const FORCE_STRENGTH: f32 = 3000.0;

const SUB_STEPS: usize = 8; // Increases stability but decreases performance
const RESTITUTION: f32 = 0.5; // Coefficient for elastic collision particle-particle

struct Particle {
    position: Vec2,
    velocity: Vec2,
}

struct Model {
    _window: window::Id,
    particle: Vec<Particle>,
    grid: Grid,
}

struct Grid {
    cells: Vec<Vec<usize>>, // index of particles
    cols: usize,
    rows: usize,
    cell_size: f32,
}

impl Grid {
    fn new(width: f32, height: f32, cell_size: f32) -> Self {
        let cols = (width / cell_size).ceil() as usize + 1;
        let rows = (height / cell_size).ceil() as usize + 1;

        let cells = vec![Vec::with_capacity(10); cols * rows];

        Grid {
            cells,
            cols,
            rows,
            cell_size,
        }
    }

    fn clear(&mut self) {
        for cell in self.cells.iter_mut() {
            cell.clear();
        }
    }

    fn add_particle(&mut self, particle_index: usize, position: Vec2, win_rect: Rect) {
        // Convert position from world coordinates to grid coordinates
        let x = position.x - win_rect.left();
        let y = position.y - win_rect.bottom();

        if x < 0.0 || y < 0.0 {
            return;
        }

        let col = (x / self.cell_size).floor() as usize;
        let row = (y / self.cell_size).floor() as usize;

        // Ensure it's within bounds to avoid crash
        let col = col.min(self.cols - 1);
        let row = row.min(self.rows - 1);

        let index = row * self.cols + col;
        self.cells[index].push(particle_index);
    }
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

    let grid = Grid::new(WINDOW_WIDTH as f32, WINDOW_HEIGHT as f32, PARTICLE_DIAMETER);

    Model {
        _window,
        particle: particles,
        grid,
    }
}

fn update(app: &App, model: &mut Model, update: Update) {
    let dt = update.since_last.as_secs_f32() / SUB_STEPS as f32;
    let win = app.window_rect();

    // Mouse interaction
    let mouse_pos = app.mouse.position();
    let is_left_down = app.mouse.buttons.left().is_down();
    let is_right_down = app.mouse.buttons.right().is_down();

    for _ in 0..SUB_STEPS {
        for particle in model.particle.iter_mut() {
            if is_left_down || is_right_down {
                let diff = mouse_pos - particle.position;
                let dist = diff.length();

                if dist < INTERACTION_RADIUS && dist > 0.0 {
                    let dir = diff / dist;
                    // Mouse with cuadratic force
                    let strength = (1.0 - dist / INTERACTION_RADIUS).powi(2) * FORCE_STRENGTH;

                    if is_left_down {
                        particle.velocity -= dir * strength * dt; // Attract
                    } else {
                        particle.velocity += dir * strength * dt; // Repel
                    }
                }
            }
            particle.velocity += vec2(0.0, GRAVITY_Y) * dt;
            particle.position += particle.velocity * dt;
        }

        model.grid.clear();

        for (i, p) in model.particle.iter().enumerate() {
            model.grid.add_particle(i, p.position, win);
        }

        resolve_collisions_with_grid(&mut model.particle, &model.grid);

        resolve_boundaries(model, win);
    }
}

fn resolve_boundaries(model: &mut Model, win: Rect) {
    for particle in model.particle.iter_mut() {
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

fn resolve_collisions_with_grid(particles: &mut Vec<Particle>, grid: &Grid) {
    for y in 0..grid.rows {
        for x in 0..grid.cols {
            let cell_idx = y * grid.cols + x;
            let cell_particles = &grid.cells[cell_idx];

            solve_cell(cell_particles, cell_particles, particles);

            if x + 1 < grid.cols {
                solve_cell(
                    cell_particles,
                    &grid.cells[y * grid.cols + (x + 1)],
                    particles,
                );
            }
            if y + 1 < grid.rows {
                solve_cell(
                    cell_particles,
                    &grid.cells[(y + 1) * grid.cols + x],
                    particles,
                );

                if x + 1 < grid.cols {
                    solve_cell(
                        cell_particles,
                        &grid.cells[(y + 1) * grid.cols + (x + 1)],
                        particles,
                    );
                }
                if x > 0 {
                    solve_cell(
                        cell_particles,
                        &grid.cells[(y + 1) * grid.cols + (x - 1)],
                        particles,
                    );
                }
            }
        }
    }
}

fn solve_cell(idx_list_a: &[usize], idx_list_b: &[usize], particles: &mut Vec<Particle>) {
    for &i in idx_list_a {
        for &j in idx_list_b {
            if i == j {
                continue;
            }

            let p1_pos = particles[i].position;
            let p2_pos = particles[j].position;

            let delta = p1_pos - p2_pos;
            let dist_sq = delta.length_squared();
            let min_dist = PARTICLE_DIAMETER;

            if dist_sq < min_dist * min_dist && dist_sq > 0.0001 {
                let dist = dist_sq.sqrt();
                let overlap = min_dist - dist;
                let direction = delta / dist;

                let correction = direction * overlap * 0.5;

                particles[i].position += correction;
                particles[j].position -= correction;
                let rel_vel = particles[i].velocity - particles[j].velocity;
                let impact = rel_vel.dot(direction) * RESTITUTION;

                particles[i].velocity -= direction * impact;
                particles[j].velocity += direction * impact;
            }
        }
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    let win = app.window_rect();

    draw.background().color(BLACK);

    for particle in model.particle.iter() {
        let speed = particle.velocity.length();

        // 0.4 -> Blue (Low Speed), 0.0 -> Red (High Speed)
        let color = map_range(speed, 0.0, 800.0, 0.4, 0.0);
        draw.ellipse()
            .hsla(color, 0.8, 0.5, 1.0)
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
