use std::f32::consts::PI;
use bevy::{prelude::*, window::PrimaryWindow, diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin}};
use rand::Rng;

fn main() {
  App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(ParticlePlugin)
    .add_plugins((FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin::default()))
    .run();
}

const PARTICLE_SIZE: f32 = 3.0;
const NUM_PARTICLES: i32 = 1000;
const GRAVITY_FACTOR: f32 = 0.0;
const COLLISION_DAMPENING: f32 = 0.5; // [0,1]
const RESTITUTION: f32 = 1.0; // [0,1]
const SMOOTHING_RADIUS: f32 = 200.0;
const MASS: f32 = 1.0;
const TARGET_DENSITY: f32 = 2.5;
const PRESSURE_MULTIPLIER: f32 = 200.0;


#[derive(Resource)]
pub struct SimulationState {
  densities: Vec<f32>,
}

pub struct ParticlePlugin;

impl Plugin for ParticlePlugin {
  fn build(&self, app: &mut App) {
    app
      .insert_resource(SimulationState {
          densities: vec![0.0; NUM_PARTICLES as usize],
      })
      .add_systems(Startup, setup)
      .add_systems(Update, (gravity, detect_collisions, (update_density, apply_pressure_force).chain()));
  }
}

#[derive(Component)]
pub struct Particle {
  pub position: Vec3,
  pub velocity: Vec3,
  pub mass: f32
}

pub fn setup(
  mut commands: Commands,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<ColorMaterial>>,
  window_query: Query<&Window, With<PrimaryWindow>>
) {
  commands.spawn(Camera2d);

  let window = window_query.get_single().unwrap();
  let window_width = window.width();
  let window_height = window.height();

  for _ in 0..NUM_PARTICLES {
    
    let x = rand::thread_rng().gen_range(- window_width / 2.0 .. window_width / 2.0);
    let y = rand::thread_rng().gen_range(- window_height / 2.0 .. window_height / 2.0);

    let particle = Particle {
      position: Vec3::new(x, y, 0.0),
      velocity: Vec3::ZERO,
      mass: PARTICLE_SIZE
    };

    let shape = meshes.add(Circle::new(PARTICLE_SIZE));
    let color = Color::hsl(360. * rand::thread_rng().gen_range(0.0..1.0), 0.95, 0.7);
    
    commands.spawn((
      particle,
      Mesh2d(shape),
      MeshMaterial2d(materials.add(color)),
      Transform::from_xyz(x, y,0.0)
    ));
    
    #[cfg(not(target_arch = "wasm32"))]
    commands.spawn((
      Text::new("Particle Simulation"),
      Node {
        position_type: PositionType::Absolute,
        top: Val::Px(12.0),
        left: Val::Px(12.0),
        ..default()
      },
    ));
  }
}

pub fn gravity(
  mut particle_query: Query<(&mut Transform, &mut Particle)>,
  window_query: Query<&Window, With<PrimaryWindow>>,
  time: Res<Time>
) {
  for (mut transform, mut particle) in &mut particle_query {
    particle.velocity += Vec3::NEG_Y * GRAVITY_FACTOR * time.delta_secs();

    let velocity = particle.velocity;
    particle.position += velocity * time.delta_secs();
    transform.translation = particle.position;

    detect_boundaries(&mut particle, &window_query);
  }
}

fn detect_boundaries(
  particle: &mut Particle, 
  window_query: &Query<&Window, With<PrimaryWindow>>
) {

  let window = window_query.get_single().unwrap();
  let window_width = window.width() / 2.0 - particle.mass;
  let window_height = window.height() / 2.0 - particle.mass;
  
  if particle.position.y.abs() > window_height {
    particle.position.y = window_height * particle.position.y.signum();
    particle.velocity.y *= -COLLISION_DAMPENING;
  }

  if particle.position.x.abs() > window_width {
    particle.position.x = window_width * particle.position.x.signum();
    particle.velocity.x *= -COLLISION_DAMPENING;
  }
}

pub fn detect_collisions(
  mut particle_query: Query<(Entity, &Transform, &mut Particle)>,
) {
  let entities: Vec<(Entity, Vec3, Vec3, f32)> = particle_query
    .iter()
    .map(|(entity, transform, particle)| {
        (entity, transform.translation, particle.velocity, particle.mass)
    })
    .collect();

  let mut collisions = Vec::new();

  for i in 0..entities.len() {
    for j in (i + 1)..entities.len() {
      let (e1, pos1, vel1, mass1) = entities[i];
      let (e2, pos2, vel2, mass2) = entities[j];

      let delta = pos1 - pos2;
      let dist = delta.length();

      // Check for collision
      if dist < (mass1 + mass2) {
        collisions.push((e1, e2, pos1, pos2, vel1, vel2, mass1, mass2));
      }
    }
  }

  for (e1, e2, pos1, pos2, vel1, vel2, mass1, mass2) in collisions {
    let (new_vel1, new_vel2) = elastic_collision(
      mass1, mass2,
      vel1, vel2,
      pos1, pos2
    );

    if let Ok((_, _, mut particle)) = particle_query.get_mut(e1) {
      particle.velocity = new_vel1;
    }
    if let Ok((_, _, mut particle)) = particle_query.get_mut(e2) {
      particle.velocity = new_vel2;
    }
  }
}

fn elastic_collision(
  m1: f32, m2: f32,
  v1: Vec3, v2: Vec3,
  r1: Vec3, r2: Vec3
) -> (Vec3, Vec3) {

  let n = (r1 - r2).normalize();
  
  let v_rel = (v1 - v2).dot(n);
  
  if v_rel > 0.0 {
    return (v1, v2);
  }

  // Calculate impulse scalar
  let j = -(1.0 + RESTITUTION) * v_rel / (1.0/m1 + 1.0/m2);
  
  // Apply impulse to get final velocities
  let v1f = v1 + (j / m1) * n;
  let v2f = v2 - (j / m2) * n;

  (v1f, v2f)
}


pub fn apply_pressure_force(
  mut particle_query: Query<(&Transform, &mut Particle)>,
  time: Res<Time>,
  state: Res<SimulationState>,
) {

  // collect positions first to avoid conflicts
  let particle_data: Vec<(Vec3, usize)> = particle_query
    .iter()
    .enumerate()
    .map(|(i, (transform, _))| (transform.translation, i))
    .collect();

  for (i, (_, mut particle)) in particle_query.iter_mut().enumerate() {
    let pressure_force = calculate_pressure_force(&particle_data, &particle, &state, i);
    let pressure_acceleration = pressure_force / state.densities[i];
    particle.velocity += pressure_acceleration * time.delta_secs();
  }
}


fn smoothing_kernel(radius: f32, dist: f32) -> f32 {
  let volume = (PI * radius.powf(4.0)) / 6.0;
  (0.0 as f32).max(radius - dist).powf(2.0) / volume
}

fn smoothing_kernel_dx(radius: f32, dist: f32) -> f32 {

  if dist >= radius {
    return 0.0;
  }

  let scale = 12.0 / (radius.powf(4.0) * PI);
  (radius - dist) * scale
}

fn calculate_density(
  particle_query: &Query<(&Transform, &Particle)>,
  sample_particle: &Particle, 
) -> f32 {
  let mut density: f32 = 0.0;
  
  for (_, particle) in particle_query {
    let dist = particle.position.distance(sample_particle.position);
    let influence = smoothing_kernel(SMOOTHING_RADIUS, dist);
    
    density += MASS * influence;
  }

  density
}

fn update_density(
  particle_query: Query<(&Transform, &Particle)>,
  mut state: ResMut<SimulationState>,
) {
  for (i, (_, sample_particle)) in particle_query.iter().enumerate() {
      state.densities[i] = calculate_density(&particle_query, sample_particle);
  }
}


fn calculate_pressure_force(
  particle_data: &[(Vec3, usize)],
  sample_particle: &Particle,
  state: &SimulationState,
  sample_index: usize,
) -> Vec3 {
  let mut pressure_force = Vec3::ZERO;

  for &(position, i) in particle_data {
    if i != sample_index {
      let dist = position.distance(sample_particle.position);

      if dist > 0.0 {
        let dir = (position - sample_particle.position) / dist;
        let slope = smoothing_kernel_dx(SMOOTHING_RADIUS, dist);
        let density = state.densities[i];
        let pressure = shared_pressure(density, state.densities[sample_index]);
        
        pressure_force += pressure * dir * slope * MASS / density;
      }
    }
  }
  pressure_force
}


fn density_to_pressure(density: f32) -> f32 {
  let density_err = density - TARGET_DENSITY;  
  let pressure = density_err * PRESSURE_MULTIPLIER;
  pressure
}

fn shared_pressure(density: f32, other_density: f32) -> f32 {
  let p1 = density_to_pressure(density);
  let p2 = density_to_pressure(other_density);
  (p1 + p2) / 2.0
}