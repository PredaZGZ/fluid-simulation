use nannou::prelude::*;
use rayon::prelude::*;
use std::f32::consts::PI;

const EPSILON: f32 = 1.0e-6;
const MIN_PAR_CHUNK: usize = 64;

#[derive(Clone, Copy, Debug)]
pub enum InteractionMode {
    Attract,
    Repel,
}

#[derive(Clone, Copy, Debug)]
pub struct Interaction {
    pub position: Vec2,
    pub radius: f32,
    pub strength: f32,
    pub mode: InteractionMode,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SimulationStats {
    pub particle_count: usize,
    pub max_speed: f32,
    pub max_density_ratio: f32,
    pub threads: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct SphConfig {
    pub bounds: Rect,
    pub particle_spacing: f32,
    pub particle_radius: f32,
    pub particle_mass: f32,
    pub smoothing_radius: f32,
    pub rest_density: f32,
    pub pressure_stiffness: f32,
    pub gamma: f32,
    pub gravity: Vec2,
    pub viscosity: f32,
    pub surface_tension: f32,
    pub surface_threshold: f32,
    pub xsph_factor: f32,
    pub interaction_radius: f32,
    pub interaction_strength: f32,
    pub interaction_max_acceleration: f32,
    pub boundary_stiffness: f32,
    pub boundary_damping: f32,
    pub boundary_restitution: f32,
    pub boundary_margin: f32,
    pub max_density_ratio: f32,
    pub max_acceleration: f32,
    pub max_velocity: f32,
    pub xsph_max_velocity: f32,
    pub velocity_damping: f32,
}

impl SphConfig {
    pub fn new(bounds: Rect) -> Self {
        let particle_spacing = 0.055;
        let smoothing_radius = particle_spacing * 2.1;
        let rest_density = 1000.0;
        let gamma = 7.0;
        let sound_speed = 8.5;

        Self {
            bounds,
            particle_spacing,
            particle_radius: particle_spacing * 0.42,
            particle_mass: rest_density * particle_spacing * particle_spacing,
            smoothing_radius,
            rest_density,
            pressure_stiffness: rest_density * sound_speed * sound_speed / gamma,
            gamma,
            gravity: vec2(0.0, -9.81),
            viscosity: 0.14,
            surface_tension: 0.18,
            surface_threshold: 3.0,
            xsph_factor: 0.04,
            interaction_radius: 0.65,
            interaction_strength: 700.0,
            interaction_max_acceleration: 180.0,
            boundary_stiffness: 240.0,
            boundary_damping: 18.0,
            boundary_restitution: 0.15,
            boundary_margin: particle_spacing * 0.5,
            max_density_ratio: 2.4,
            max_acceleration: 320.0,
            max_velocity: 14.0,
            xsph_max_velocity: 1.2,
            velocity_damping: 0.9992,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct KernelSet {
    support_radius: f32,
    support_sq: f32,
    poly6_coeff: f32,
    spiky_grad_coeff: f32,
    viscosity_laplacian_coeff: f32,
}

impl KernelSet {
    #[inline]
    fn new(smoothing_radius: f32) -> Self {
        Self {
            support_radius: smoothing_radius,
            support_sq: smoothing_radius * smoothing_radius,
            poly6_coeff: 4.0 / (PI * smoothing_radius.powi(8)),
            spiky_grad_coeff: -30.0 / (PI * smoothing_radius.powi(5)),
            viscosity_laplacian_coeff: 40.0 / (PI * smoothing_radius.powi(5)),
        }
    }

    #[inline]
    fn poly6(&self, distance_sq: f32) -> f32 {
        if distance_sq >= self.support_sq {
            return 0.0;
        }

        let delta = self.support_sq - distance_sq;
        self.poly6_coeff * delta * delta * delta
    }

    #[inline]
    fn spiky_gradient(&self, delta: Vec2, distance: f32) -> Vec2 {
        if distance <= EPSILON || distance >= self.support_radius {
            return Vec2::ZERO;
        }

        let scale = self.spiky_grad_coeff * (self.support_radius - distance).powi(2) / distance;
        delta * scale
    }

    #[inline]
    fn viscosity_laplacian(&self, distance: f32) -> f32 {
        if distance >= self.support_radius {
            return 0.0;
        }

        self.viscosity_laplacian_coeff * (self.support_radius - distance)
    }
}

#[derive(Debug)]
struct UniformGrid {
    cell_counts: Vec<usize>,
    cell_offsets: Vec<usize>,
    cell_cursor: Vec<usize>,
    sorted_particles: Vec<usize>,
    particle_cells: Vec<usize>,
    neighbor_cells: Vec<[usize; 9]>,
    neighbor_counts: Vec<u8>,
    cols: usize,
    inv_cell_size: f32,
    bounds_left: f32,
    bounds_bottom: f32,
}

impl UniformGrid {
    fn new(bounds: Rect, cell_size: f32) -> Self {
        let cols = (bounds.w() / cell_size).ceil().max(1.0) as usize + 1;
        let rows = (bounds.h() / cell_size).ceil().max(1.0) as usize + 1;
        let cell_total = cols * rows;
        let mut neighbor_cells = vec![[0; 9]; cell_total];
        let mut neighbor_counts = vec![0; cell_total];

        for row in 0..rows {
            for col in 0..cols {
                let cell_index = row * cols + col;
                let min_row = row.saturating_sub(1);
                let max_row = (row + 1).min(rows - 1);
                let min_col = col.saturating_sub(1);
                let max_col = (col + 1).min(cols - 1);
                let mut count = 0;

                for current_row in min_row..=max_row {
                    for current_col in min_col..=max_col {
                        neighbor_cells[cell_index][count] = current_row * cols + current_col;
                        count += 1;
                    }
                }

                neighbor_counts[cell_index] = count as u8;
            }
        }

        Self {
            cell_counts: vec![0; cell_total],
            cell_offsets: vec![0; cell_total + 1],
            cell_cursor: vec![0; cell_total],
            sorted_particles: Vec::new(),
            particle_cells: Vec::new(),
            neighbor_cells,
            neighbor_counts,
            cols,
            inv_cell_size: cell_size.recip(),
            bounds_left: bounds.left(),
            bounds_bottom: bounds.bottom(),
        }
    }

    fn rebuild(&mut self, positions: &[Vec2]) {
        self.cell_counts.fill(0);

        if self.sorted_particles.len() != positions.len() {
            self.sorted_particles.resize(positions.len(), 0);
            self.particle_cells.resize(positions.len(), 0);
        }

        for (index, position) in positions.iter().copied().enumerate() {
            let cell = self.cell_index(position);
            self.particle_cells[index] = cell;
            self.cell_counts[cell] += 1;
        }

        let mut offset = 0;
        for cell in 0..self.cell_counts.len() {
            self.cell_offsets[cell] = offset;
            self.cell_cursor[cell] = offset;
            offset += self.cell_counts[cell];
        }
        self.cell_offsets[self.cell_counts.len()] = offset;

        for particle_index in 0..positions.len() {
            let cell = self.particle_cells[particle_index];
            let slot = self.cell_cursor[cell];
            self.sorted_particles[slot] = particle_index;
            self.cell_cursor[cell] = slot + 1;
        }
    }

    #[inline]
    fn for_each_neighbor<F>(&self, particle_cell: usize, mut visit: F)
    where
        F: FnMut(usize),
    {
        let cells = &self.neighbor_cells[particle_cell];
        let count = self.neighbor_counts[particle_cell] as usize;

        for offset in 0..count {
            let cell = cells[offset];
            let start = self.cell_offsets[cell];
            let end = self.cell_offsets[cell + 1];

            for slot in start..end {
                visit(self.sorted_particles[slot]);
            }
        }
    }

    #[inline]
    fn cell_index(&self, position: Vec2) -> usize {
        let col = ((position.x - self.bounds_left) * self.inv_cell_size).floor() as isize;
        let row = ((position.y - self.bounds_bottom) * self.inv_cell_size).floor() as isize;
        let col = col.clamp(0, self.cols as isize - 1) as usize;
        let row = row.clamp(0, self.cell_counts.len() as isize / self.cols as isize - 1) as usize;

        row * self.cols + col
    }
}

pub struct SphSimulation {
    config: SphConfig,
    kernels: KernelSet,
    positions: Vec<Vec2>,
    velocities: Vec<Vec2>,
    densities: Vec<f32>,
    inv_densities: Vec<f32>,
    pressures: Vec<f32>,
    pressure_terms: Vec<f32>,
    accelerations: Vec<Vec2>,
    xsph_corrections: Vec<Vec2>,
    grid: UniformGrid,
    stats: SimulationStats,
    stats_dirty: bool,
}

impl SphSimulation {
    pub fn new(config: SphConfig) -> Self {
        let kernels = KernelSet::new(config.smoothing_radius);
        let grid = UniformGrid::new(config.bounds, config.smoothing_radius);

        Self {
            config,
            kernels,
            positions: Vec::new(),
            velocities: Vec::new(),
            densities: Vec::new(),
            inv_densities: Vec::new(),
            pressures: Vec::new(),
            pressure_terms: Vec::new(),
            accelerations: Vec::new(),
            xsph_corrections: Vec::new(),
            grid,
            stats: SimulationStats {
                threads: rayon::current_num_threads(),
                ..SimulationStats::default()
            },
            stats_dirty: true,
        }
    }

    pub fn seed_block(&mut self, cols: usize, rows: usize, origin: Vec2) {
        let additional = cols * rows;
        self.positions.reserve(additional);
        self.velocities.reserve(additional);
        self.densities.reserve(additional);
        self.inv_densities.reserve(additional);
        self.pressures.reserve(additional);
        self.pressure_terms.reserve(additional);
        self.accelerations.reserve(additional);
        self.xsph_corrections.reserve(additional);

        for row in 0..rows {
            for col in 0..cols {
                let position = origin
                    + vec2(
                        col as f32 * self.config.particle_spacing,
                        row as f32 * self.config.particle_spacing,
                    );

                self.positions.push(position);
                self.velocities.push(Vec2::ZERO);
                self.densities.push(self.config.rest_density);
                self.inv_densities.push(self.config.rest_density.recip());
                self.pressures.push(0.0);
                self.pressure_terms.push(0.0);
                self.accelerations.push(Vec2::ZERO);
                self.xsph_corrections.push(Vec2::ZERO);
            }
        }

        self.stats_dirty = true;
        self.refresh_stats();
    }

    pub fn step(&mut self, dt: f32, interaction: Option<Interaction>) {
        if self.positions.is_empty() {
            return;
        }

        self.grid.rebuild(&self.positions);
        self.compute_densities();
        self.compute_pressures();
        self.compute_accelerations(interaction);
        self.integrate(dt);
        self.stats_dirty = true;
    }

    pub fn config(&self) -> &SphConfig {
        &self.config
    }

    pub fn positions(&self) -> &[Vec2] {
        &self.positions
    }

    pub fn velocities(&self) -> &[Vec2] {
        &self.velocities
    }

    pub fn densities(&self) -> &[f32] {
        &self.densities
    }

    pub fn stats(&self) -> SimulationStats {
        self.stats
    }

    pub fn refresh_stats(&mut self) {
        if !self.stats_dirty {
            return;
        }

        self.stats = SimulationStats {
            particle_count: self.positions.len(),
            max_speed: self
                .velocities
                .par_iter()
                .with_min_len(MIN_PAR_CHUNK)
                .map(|velocity| velocity.length())
                .reduce(|| 0.0, f32::max),
            max_density_ratio: self
                .densities
                .par_iter()
                .with_min_len(MIN_PAR_CHUNK)
                .map(|density| density / self.config.rest_density)
                .reduce(|| 0.0, f32::max),
            threads: rayon::current_num_threads(),
        };
        self.stats_dirty = false;
    }

    fn compute_densities(&mut self) {
        let positions = &self.positions;
        let particle_cells = &self.grid.particle_cells;
        let grid = &self.grid;
        let kernels = self.kernels;
        let particle_mass = self.config.particle_mass;
        let minimum_density = self.config.rest_density * 0.5;

        self.densities
            .par_iter_mut()
            .enumerate()
            .with_min_len(MIN_PAR_CHUNK)
            .for_each(|(index, density)| {
                let position_i = positions[index];
                let mut value = 0.0;

                grid.for_each_neighbor(particle_cells[index], |neighbor| {
                    let delta = position_i - positions[neighbor];
                    value += particle_mass * kernels.poly6(delta.length_squared());
                });

                *density = value.max(minimum_density);
            });
    }

    fn compute_pressures(&mut self) {
        let rest_density = self.config.rest_density;
        let pressure_stiffness = self.config.pressure_stiffness;
        let gamma = self.config.gamma;
        let max_density_ratio = self.config.max_density_ratio;

        self.pressures
            .par_iter_mut()
            .zip(self.inv_densities.par_iter_mut())
            .zip(self.pressure_terms.par_iter_mut())
            .zip(self.densities.par_iter().copied())
            .with_min_len(MIN_PAR_CHUNK)
            .for_each(|(((pressure, inv_density), pressure_term), density)| {
                let clamped_density = density.max(EPSILON);
                let density_inv = clamped_density.recip();
                let ratio = (clamped_density / rest_density).clamp(1.0, max_density_ratio);
                let value = pressure_stiffness * (ratio.powf(gamma) - 1.0);

                *pressure = value;
                *inv_density = density_inv;
                *pressure_term = value * density_inv * density_inv;
            });
    }

    fn compute_accelerations(&mut self, interaction: Option<Interaction>) {
        let config = self.config;
        let kernels = self.kernels;
        let positions = &self.positions;
        let velocities = &self.velocities;
        let inv_densities = &self.inv_densities;
        let pressure_terms = &self.pressure_terms;
        let particle_cells = &self.grid.particle_cells;
        let grid = &self.grid;
        let surface_threshold_sq = config.surface_threshold * config.surface_threshold;

        self.accelerations
            .par_iter_mut()
            .zip(self.xsph_corrections.par_iter_mut())
            .enumerate()
            .with_min_len(MIN_PAR_CHUNK)
            .for_each(|(index, (acceleration, xsph))| {
                let position_i = positions[index];
                let velocity_i = velocities[index];
                let density_inv_i = inv_densities[index];
                let pressure_term_i = pressure_terms[index];

                let mut pressure_force = Vec2::ZERO;
                let mut viscosity_force = Vec2::ZERO;
                let mut surface_normal = Vec2::ZERO;
                let mut color_laplacian = 0.0;
                let mut velocity_blend = Vec2::ZERO;

                grid.for_each_neighbor(particle_cells[index], |neighbor| {
                    if index == neighbor {
                        return;
                    }

                    let delta = position_i - positions[neighbor];
                    let distance_sq = delta.length_squared();

                    if distance_sq >= kernels.support_sq || distance_sq <= EPSILON {
                        return;
                    }

                    let distance = distance_sq.sqrt();
                    let density_inv_j = inv_densities[neighbor];
                    let pressure_term_j = pressure_terms[neighbor];
                    let velocity_j = velocities[neighbor];
                    let gradient = kernels.spiky_gradient(delta, distance);
                    let laplacian = kernels.viscosity_laplacian(distance);
                    let mass_density_j = config.particle_mass * density_inv_j;

                    pressure_force -=
                        config.particle_mass * (pressure_term_i + pressure_term_j) * gradient;
                    viscosity_force +=
                        config.viscosity * mass_density_j * (velocity_j - velocity_i) * laplacian;
                    surface_normal += mass_density_j * gradient;
                    color_laplacian += mass_density_j * laplacian;

                    let inv_average_density = 2.0 * density_inv_i * density_inv_j
                        / (density_inv_i + density_inv_j).max(EPSILON);
                    velocity_blend += config.xsph_factor
                        * config.particle_mass
                        * (velocity_j - velocity_i)
                        * kernels.poly6(distance_sq)
                        * inv_average_density;
                });

                let mut total_acceleration = config.gravity
                    + pressure_force
                    + viscosity_force
                    + Self::boundary_acceleration(config, position_i, velocity_i);

                let normal_sq = surface_normal.length_squared();
                if normal_sq > surface_threshold_sq {
                    total_acceleration +=
                        -config.surface_tension * color_laplacian * surface_normal
                            / normal_sq.sqrt();
                }

                if let Some(current_interaction) = interaction {
                    let density_feedback = (config.rest_density * density_inv_i).clamp(0.35, 1.0);
                    total_acceleration +=
                        Self::interaction_acceleration(config, position_i, current_interaction)
                            * density_feedback;
                }

                *acceleration = Self::clamp_magnitude(total_acceleration, config.max_acceleration);
                *xsph = velocity_blend;
            });
    }

    fn integrate(&mut self, dt: f32) {
        let config = self.config;

        self.positions
            .par_iter_mut()
            .zip(self.velocities.par_iter_mut())
            .zip(self.accelerations.par_iter().copied())
            .zip(self.xsph_corrections.par_iter().copied())
            .with_min_len(MIN_PAR_CHUNK)
            .for_each(|(((position, velocity), acceleration), xsph)| {
                let xsph = Self::clamp_magnitude(xsph, config.xsph_max_velocity);
                *velocity += acceleration * dt;
                *velocity = Self::clamp_magnitude(*velocity, config.max_velocity);
                *velocity *= config.velocity_damping;
                *position += (*velocity + xsph) * dt;

                if !position.x.is_finite() || !position.y.is_finite() {
                    *position = config.bounds.xy();
                }
                if !velocity.x.is_finite() || !velocity.y.is_finite() {
                    *velocity = Vec2::ZERO;
                }

                Self::resolve_boundaries(config, position, velocity);
            });
    }

    fn interaction_acceleration(
        config: SphConfig,
        position: Vec2,
        interaction: Interaction,
    ) -> Vec2 {
        let delta = interaction.position - position;
        let distance_sq = delta.length_squared();
        let radius_sq = interaction.radius * interaction.radius;

        if distance_sq <= EPSILON || distance_sq >= radius_sq {
            return Vec2::ZERO;
        }

        let distance = distance_sq.sqrt();
        let direction = delta / distance;
        let falloff = (1.0 - distance / interaction.radius).sqrt();
        let acceleration = match interaction.mode {
            InteractionMode::Attract => direction * interaction.strength * falloff,
            InteractionMode::Repel => -direction * interaction.strength * falloff,
        };

        Self::clamp_magnitude(acceleration, config.interaction_max_acceleration)
    }

    fn boundary_acceleration(config: SphConfig, position: Vec2, velocity: Vec2) -> Vec2 {
        let mut acceleration = Vec2::ZERO;
        let inv_support = config.smoothing_radius.recip();

        let left_distance = position.x - config.bounds.left();
        if left_distance < config.smoothing_radius {
            let weight = 1.0 - left_distance * inv_support;
            acceleration.x += config.boundary_stiffness * weight * weight;
            if velocity.x < 0.0 {
                acceleration.x += -config.boundary_damping * velocity.x;
            }
        }

        let right_distance = config.bounds.right() - position.x;
        if right_distance < config.smoothing_radius {
            let weight = 1.0 - right_distance * inv_support;
            acceleration.x -= config.boundary_stiffness * weight * weight;
            if velocity.x > 0.0 {
                acceleration.x -= config.boundary_damping * velocity.x;
            }
        }

        let bottom_distance = position.y - config.bounds.bottom();
        if bottom_distance < config.smoothing_radius {
            let weight = 1.0 - bottom_distance * inv_support;
            acceleration.y += config.boundary_stiffness * weight * weight;
            if velocity.y < 0.0 {
                acceleration.y += -config.boundary_damping * velocity.y;
            }
        }

        let top_distance = config.bounds.top() - position.y;
        if top_distance < config.smoothing_radius {
            let weight = 1.0 - top_distance * inv_support;
            acceleration.y -= config.boundary_stiffness * weight * weight;
            if velocity.y > 0.0 {
                acceleration.y -= config.boundary_damping * velocity.y;
            }
        }

        acceleration
    }

    fn resolve_boundaries(config: SphConfig, position: &mut Vec2, velocity: &mut Vec2) {
        let min_x = config.bounds.left() + config.boundary_margin;
        let max_x = config.bounds.right() - config.boundary_margin;
        let min_y = config.bounds.bottom() + config.boundary_margin;
        let max_y = config.bounds.top() - config.boundary_margin;

        if position.x < min_x {
            position.x = min_x;
            if velocity.x < 0.0 {
                velocity.x *= -config.boundary_restitution;
            }
        } else if position.x > max_x {
            position.x = max_x;
            if velocity.x > 0.0 {
                velocity.x *= -config.boundary_restitution;
            }
        }

        if position.y < min_y {
            position.y = min_y;
            if velocity.y < 0.0 {
                velocity.y *= -config.boundary_restitution;
            }
        } else if position.y > max_y {
            position.y = max_y;
            if velocity.y > 0.0 {
                velocity.y *= -config.boundary_restitution;
            }
        }
    }

    #[inline]
    fn clamp_magnitude(vector: Vec2, max_length: f32) -> Vec2 {
        let length_sq = vector.length_squared();
        let max_sq = max_length * max_length;

        if length_sq > max_sq && length_sq > EPSILON {
            vector * (max_length / length_sq.sqrt())
        } else {
            vector
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poly6_is_zero_outside_support() {
        let kernels = KernelSet::new(0.1);

        assert_eq!(kernels.poly6(0.11f32.powi(2)), 0.0);
    }

    #[test]
    fn grid_queries_only_local_cells() {
        let bounds = Rect::from_w_h(1.0, 1.0);
        let mut grid = UniformGrid::new(bounds, 0.1);
        let positions = vec![vec2(-0.25, -0.25), vec2(-0.21, -0.21), vec2(0.35, 0.35)];

        grid.rebuild(&positions);

        let mut hits = Vec::new();
        grid.for_each_neighbor(grid.particle_cells[0], |index| hits.push(index));

        assert!(hits.contains(&0));
        assert!(hits.contains(&1));
        assert!(!hits.contains(&2));
    }
}
