use std::time::{SystemTime, UNIX_EPOCH};
use rand::{RngExt, SeedableRng, prelude::StdRng};
use tui::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::Color,
};

use super::Animation;

const BASE_GRAVITY: f32 = 0.05;
const DEFAULT_SPARK_CHARS: [char; 6] = ['█', '▓', '▒', '░', '.', ' '];
const ABSOLUTE_MAX_PARTICLES: usize = 20_000;

#[derive(Debug, Clone, Copy, PartialEq)]
enum ParticleType {
    Rocket,
    Spark,
}

#[derive(Debug, Clone, Copy)]
struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: f32,
    max_life: f32,
    target_y: f32,

    payload_colors: [Color; 4],
    color: Color,
    p_type: ParticleType,
    payload_count: u8,
}

impl Default for Particle {
    fn default() -> Self {
        Self {
            x: 0.0, y: 0.0, vx: 0.0, vy: 0.0,
            life: 0.0, max_life: 1.0, target_y: 0.0,
            payload_colors: [Color::Reset; 4],
            color: Color::Reset,
            p_type: ParticleType::Rocket,
            payload_count: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Options {
    pub max_particles: usize,
    pub gravity: f32,
    pub launch_freq: f32,
    pub height_scale: f32,
    pub size_scale: f32,
    pub climb_speed: f32,
    pub decay_speed: f32,
    pub explosion_speed: f32,
    pub spark_drag: f32,
    pub spark_chars: Vec<char>,
    pub palettes: Vec<(u32, Vec<Color>)>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            max_particles: 2000,
            gravity: 1.0,
            launch_freq: 1.0,
            height_scale: 1.0,
            size_scale: 1.0,
            climb_speed: 1.0,
            decay_speed: 1.0,
            explosion_speed: 1.0,
            spark_drag: 0.96,
            spark_chars: DEFAULT_SPARK_CHARS.to_vec(),
            palettes: vec![
                (15, vec![Color::LightYellow, Color::LightRed, Color::Red, Color::Yellow]),
                (12, vec![Color::LightMagenta, Color::LightRed, Color::Red, Color::Magenta]),
                (10, vec![Color::White, Color::LightRed, Color::Red, Color::Gray]),
                (10, vec![Color::LightRed, Color::Red, Color::Yellow, Color::Gray]),
                (9, vec![Color::LightMagenta, Color::Magenta, Color::LightRed, Color::Red]),
                (8, vec![Color::LightYellow, Color::Yellow, Color::Red, Color::Gray]),
                (5, vec![Color::LightMagenta, Color::Magenta, Color::LightBlue, Color::Blue]),
                (5, vec![Color::LightCyan, Color::LightBlue, Color::Blue, Color::Gray]),
                (4, vec![Color::LightGreen, Color::Green, Color::LightYellow, Color::Yellow]),
                (3, vec![Color::LightRed, Color::Red, Color::Gray, Color::White]),
                (3, vec![Color::LightMagenta, Color::Magenta, Color::Gray, Color::White]),
                (2, vec![Color::White, Color::LightBlue, Color::Blue, Color::Gray]),
                (1, vec![Color::White, Color::LightYellow, Color::Gray, Color::White]),
                (1, vec![Color::Yellow, Color::White, Color::Gray, Color::White]),
            ],
        }
    }
}

pub struct Fireworks {
    width: u16,
    height: u16,
    particles: Vec<Particle>,
    active_count: usize,
    current_max_particles: usize,
    opts: Options,
    rng: StdRng,
    explosions_buffer: Vec<(f32, f32, [Color; 4], u8)>,
    cumulative_palettes: Vec<(u32, Vec<Color>)>,
    total_palette_weight: u32,
    base_min_vy: f32,
    base_max_vy: f32,
    trig_lut: [(f32, f32); 512],
    glyph_lut: [char; 256],
}

impl Fireworks {
    pub fn new(opts: Options) -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);

        let mut safe_opts = opts;
        if safe_opts.spark_chars.is_empty() {
            safe_opts.spark_chars = DEFAULT_SPARK_CHARS.to_vec();
        }

        let safe_max_particles = safe_opts.max_particles.clamp(1, ABSOLUTE_MAX_PARTICLES);

        let mut cumulative_palettes = Vec::with_capacity(safe_opts.palettes.len());
        let mut total_weight = 0;
        for (w, p) in &safe_opts.palettes {
            cumulative_palettes.push((total_weight, p.clone()));
            total_weight += w;
        }

        let mut trig_lut = [(0.0, 0.0); 512];
        for i in 0..512 {
            let angle = (i as f32 / 512.0) * std::f32::consts::TAU;
            trig_lut[i] = (angle.cos(), angle.sin());
        }

        let mut glyph_lut = [' '; 256];
        let len_f = (safe_opts.spark_chars.len().saturating_sub(1)) as f32;
        for i in 0..=255 {
            let ratio = i as f32 / 255.0;
            let idx = ((1.0 - ratio) * len_f).round() as usize;
            glyph_lut[i] = safe_opts.spark_chars[idx.clamp(0, safe_opts.spark_chars.len() - 1)];
        }

        Self {
            width: 0,
            height: 0,
            particles: vec![Particle::default(); safe_max_particles],
            active_count: 0,
            current_max_particles: safe_max_particles,
            opts: safe_opts,
            rng: StdRng::seed_from_u64(seed),
            explosions_buffer: Vec::with_capacity(256),

            cumulative_palettes,
            total_palette_weight: total_weight,
            base_min_vy: 0.0,
            base_max_vy: 0.0,
            trig_lut,
            glyph_lut,
        }
    }

    fn spawn_particle(&mut self, p: Particle) {
        if self.active_count < self.current_max_particles {
            self.particles[self.active_count] = p;
            self.active_count += 1;
        }
    }

    fn explode(&mut self, x: f32, y: f32, payload_colors: [Color; 4], payload_count: u8) {
        if payload_count == 0 { return; }

        let effective_size_scale = self.opts.size_scale * 5.0;
        let count = (80.0 * effective_size_scale) as usize;

        for _ in 0..count {
            if self.active_count >= self.current_max_particles {
                break;
            }

            let angle_idx = self.rng.random_range(0..512);
            let (cos_a, sin_a) = self.trig_lut[angle_idx];

            let layer = self.rng.random_range(0..payload_count) as usize;
            let min_speed = 0.2 + (layer as f32 * 0.4);
            let max_speed = min_speed + 0.4;
            let speed: f32 = self.rng.random_range(min_speed..max_speed) * (1.5 * self.opts.explosion_speed);

            let base_life = self.rng.random_range(20..35) + (layer as u8 * 5);
            let life = (base_life as f32) * (2.0 * effective_size_scale);

            self.spawn_particle(Particle {
                p_type: ParticleType::Spark,
                x,
                y,
                vx: cos_a * speed,
                vy: sin_a * speed * 0.5,
                life,
                max_life: life,
                color: payload_colors[layer],
                payload_colors: [Color::Reset; 4],
                payload_count: 0,
                target_y: 0.0,
            });
        }
    }
}

impl Animation for Fireworks {
    fn resize(&mut self, area: Rect) {
        if self.width == area.width && self.height == area.height {
            return;
        }
        self.width = area.width;
        self.height = area.height;
        self.active_count = 0;

        let total_cells = (self.width as usize).saturating_mul(self.height as usize);
        self.current_max_particles = self.opts.max_particles
            .min(total_cells)
            .min(self.particles.len());

        let h = self.height as f32;
        let world_gravity = self.opts.gravity * BASE_GRAVITY;
        self.base_min_vy = (h * 0.4 * 2.0 * world_gravity).sqrt();
        self.base_max_vy = (h * 0.9 * 2.0 * world_gravity).sqrt();
    }

    fn step(&mut self) {
        if self.width <= 4 || self.height <= 4 {
            return;
        }

        let w = self.width as f32;
        let h = self.height as f32;
        let world_gravity = self.opts.gravity * BASE_GRAVITY;

        if self.rng.random::<f32>() < (0.15 * self.opts.launch_freq) {
            let safe_w = (w - 2.0).max(2.0);
            let x = self.rng.random_range(2.0..safe_w);

            let min_target_h = h * 0.4 * self.opts.height_scale;
            let max_target_h = h * 0.9 * self.opts.height_scale;
            let target_h = self.rng.random_range(min_target_h..max_target_h);
            let target_y = (h - target_h).max(2.0);

            let rand_vx = self.rng.random_range(-0.2..0.2);
            let rand_vy = -self.rng.random_range(self.base_min_vy..self.base_max_vy) * (1.2 * self.opts.climb_speed);

            let mut payload = [Color::Reset; 4];
            let mut num_colors = 0;

            if self.total_palette_weight > 0 {
                let roll = self.rng.random_range(0..self.total_palette_weight);
                let idx = self.cumulative_palettes.partition_point(|&(cw, _)| cw <= roll).saturating_sub(1);
                let current_palette = &self.cumulative_palettes[idx].1;

                let roll_colors = self.rng.random_range(0..100);
                num_colors = if roll_colors < 20 { 1 } else if roll_colors < 50 { 2 } else if roll_colors < 90 { 3 } else { 4 };

                let p_len = current_palette.len();
                if p_len > 0 {
                    for i in 0..num_colors {
                        let color_idx = self.rng.random_range(0..p_len);
                        payload[i as usize] = current_palette[color_idx];
                    }
                }
            }

            self.spawn_particle(Particle {
                p_type: ParticleType::Rocket,
                x,
                y: h - 2.0,
                vx: rand_vx,
                vy: rand_vy,
                life: 100.0,
                max_life: 100.0,
                color: payload[0],
                payload_colors: payload,
                payload_count: num_colors as u8,
                target_y,
            });
        }

        self.explosions_buffer.clear();

        for i in (0..self.active_count).rev() {
            let p = &mut self.particles[i];
            p.x += p.vx;
            p.y += p.vy;
            let mut is_dead = false;
            match p.p_type {
                ParticleType::Rocket => {
                    p.vy += world_gravity;

                    if p.vy >= 0.0 || p.y <= p.target_y || p.y <= 1.0 {
                        is_dead = true;
                        let safe_y_max = (h - 2.0).max(1.0);
                        let explode_y = p.y.max(1.0).min(safe_y_max);

                        if p.x >= 1.0 && p.x < w - 1.0 {
                            self.explosions_buffer.push((p.x, explode_y, p.payload_colors, p.payload_count));
                        }
                    }
                }
                ParticleType::Spark => {
                    p.vx *= self.opts.spark_drag;
                    p.vy *= self.opts.spark_drag;
                    p.vy += world_gravity * 0.4;
                    p.life -= 1.2 * (self.opts.decay_speed * 3.0);

                    if p.life <= 0.0 || p.x < 1.0 || p.x >= w - 1.0 || p.y < 1.0 || p.y >= h - 1.0 {
                        is_dead = true;
                    }
                }
            }

            if is_dead {
                self.particles.swap(i, self.active_count - 1);
                self.active_count -= 1;
            }
        }

        for i in 0..self.explosions_buffer.len() {
            let (ex, ey, p_colors, p_count) = self.explosions_buffer[i];
            self.explode(ex, ey, p_colors, p_count);
        }
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        if self.width <= 4 || self.height <= 4 {
            return;
        }

        for i in 0..self.active_count {
            let p = &self.particles[i];

            let px = p.x.round() as i32;
            let py = p.y.round() as i32;

            if px >= 1 && px < (self.width as i32 - 1) && py >= 1 && py < (self.height as i32 - 1) {
                let x = area.x + px as u16;
                let y = area.y + py as u16;

                if let Some(cell) = buf.cell_mut(Position { x, y }) {
                    let sym = cell.symbol();
                    if sym.is_empty() { continue; }
                    let is_space = sym == " ";

                    match p.p_type {
                        ParticleType::Rocket => {
                            if is_space { cell.set_char('|'); }
                            cell.set_fg(p.color);
                        }
                        ParticleType::Spark => {
                            let ratio = (p.life / p.max_life).clamp(0.0, 1.0);
                            if is_space {
                                let lut_idx = (ratio * 255.0) as usize;
                                cell.set_char(self.glyph_lut[lut_idx.min(255)]);
                            }

                            if ratio > 0.3 {
                                cell.set_fg(p.color);
                            } else {
                                cell.set_fg(Color::DarkGray);
                            }
                        }
                    }
                }
            }
        }
    }
}
