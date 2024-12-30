use bevy::{prelude::*, window::PrimaryWindow, diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin}};
use rand::Rng;
use std::collections::HashMap;

fn main() {
  App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(ParticlePlugin)
    .add_plugins((FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin::default()))
    .run();
}

// const PARTICLE_SIZE: f32 = 5.0;
const NUM_PARTICLES: i32 = 10000;
const GRAVITY_FACTOR: f32 = 0.0;
const COLLISION_DAMPENING: f32 = 1.0; // [0,1]
const RESTITUTION: f32 = 1.0; // [0,1]
const CELL_SIZE: f32 = 15.0; // Assuming max particle size is ~10.0


pub struct ParticlePlugin;

impl Plugin for ParticlePlugin {
  fn build(&self, app: &mut App) {
    app
      .add_systems(Startup, setup)
      .add_systems(Update, (gravity, detect_collisions, debug_draw_grid));
  }
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
    // let m = rand::thread_rng().gen_range(4.0 .. 9.0);
    let m = 1.5;

    let particle = Particle {
      position: Vec3::new(x, y, 0.0),
      velocity: Vec3::new(x, y, 0.0),
      mass: m
    };

    let shape = meshes.add(Circle::new(m));
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
    particle.velocity *= Vec3::NEG_Y * COLLISION_DAMPENING;
  }

  if particle.position.x.abs() > window_width {
    particle.position.x = window_width * particle.position.x.signum();
    particle.velocity *= Vec3::NEG_X * COLLISION_DAMPENING;
  }
}

#[derive(Default)]
struct SpatialGrid {
  cells: HashMap<(i32, i32), Vec<(Entity, Vec3, Vec3, f32)>>
}

impl SpatialGrid {
  fn new() -> Self {
    Self {
      cells: HashMap::new()
    }
  }

  fn insert(&mut self, entity: Entity, position: Vec3, velocity: Vec3, mass: f32) {
    let cell = self.get_cell_coords(position);
    self.cells.entry(cell).or_default().push((entity, position, velocity, mass));
  }

  fn get_cell_coords(&self, position: Vec3) -> (i32, i32) {
    (
      (position.x / CELL_SIZE).floor() as i32,
      (position.y / CELL_SIZE).floor() as i32
    )
  }

  fn get_neighboring_cells(&self, cell: (i32, i32)) -> Vec<(i32, i32)> {
    let mut neighbors = Vec::with_capacity(9);
    for dx in -1..=1 {
      for dy in -1..=1 {
        neighbors.push((cell.0 + dx, cell.1 + dy));
      }
    }
    neighbors
  }
}

pub fn detect_collisions(
  mut particle_query: Query<(Entity, &Transform, &mut Particle)>,
) {
  let mut grid = SpatialGrid::new();

  let entities: Vec<(Entity, Vec3, Vec3, f32)> = particle_query
    .iter()
    .map(|(entity, transform, particle)| {
      let data = (entity, transform.translation, particle.velocity, particle.mass);
      grid.insert(entity, transform.translation, particle.velocity, particle.mass);
      data
    })
    .collect();

  let mut collisions = Vec::new();

  // Check for collisions using grid
  for (entity, position, velocity, mass) in &entities {
    let cell = grid.get_cell_coords(*position);
    let neighbor_cells = grid.get_neighboring_cells(cell);

    for neighbor_cell in neighbor_cells {
      if let Some(particles) = grid.cells.get(&neighbor_cell) {
        for (other_entity, other_pos, other_vel, other_mass) in particles {

          if entity == other_entity {
            continue;
          }

          let delta = *position - *other_pos;
          let dist = delta.length();

          if dist < (mass + other_mass) {
            collisions.push((
              *entity,
              *other_entity,
              *position,
              *other_pos,
              *velocity,
              *other_vel,
              *mass,
              *other_mass
            ));
          }
        }
      }
    }
  }

  collisions.sort_by_key(|&(e1, e2, _, _, _, _, _, _)| (e1.min(e2), e1.max(e2)));
  collisions.dedup_by_key(|&mut (e1, e2, _, _, _, _, _, _)| (e1.min(e2), e1.max(e2)));

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

pub fn debug_draw_grid(
  mut gizmos: Gizmos,
  window_query: Query<&Window, With<PrimaryWindow>>,
) {
  let window = window_query.get_single().unwrap();
  let width = window.width();
  let height = window.height();

  let cells_x = (width / CELL_SIZE).ceil() as i32;
  let cells_y = (height / CELL_SIZE).ceil() as i32;

  for x in -cells_x..=cells_x {
    let x_pos = x as f32 * CELL_SIZE;
    gizmos.line_2d(
      Vec2::new(x_pos, -height/2.0),
      Vec2::new(x_pos, height/2.0),
      Color::srgba(0.5, 0.5, 0.5, 0.2)
    );
  }

  for y in -cells_y..=cells_y {
    let y_pos = y as f32 * CELL_SIZE;
    gizmos.line_2d(
      Vec2::new(-width/2.0, y_pos),
      Vec2::new(width/2.0, y_pos),
      Color::srgba(0.5, 0.5, 0.5, 0.2)
    );
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

#[derive(Component)]
pub struct Particle {
  pub position: Vec3,
  pub velocity: Vec3,
  pub mass: f32
}