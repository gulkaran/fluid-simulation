use bevy::{prelude::*, window::PrimaryWindow};
use rand::Rng;

fn main() {
  App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(ParticlePlugin)
    .run();
}

const PARTICLE_SIZE: f32 = 5.0;
const NUM_PARTICLES: i32 = 100;
const GRAVITY_FACTOR: f32 = 500.0;
const COLLISION_DAMPENING: f32 = 0.5; // [0,1]
pub struct ParticlePlugin;

impl Plugin for ParticlePlugin {
  fn build(&self, app: &mut App) {
    app
      .add_systems(Startup, setup)
      .add_systems(Update, gravity);
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

    let particle = Particle {
      position: Vec3::new(x, y, 0.0),
      velocity: Vec3::ZERO
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
      Text::new("Fluid Simulation"),
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
  let window_width = window.width() / 2.0 - PARTICLE_SIZE;
  let window_height = window.height() / 2.0 - PARTICLE_SIZE;
  
  if particle.position.y.abs() > window_height {
    particle.position.y = window_height * particle.position.y.signum();
    particle.velocity *= Vec3::NEG_Y * COLLISION_DAMPENING;
  }

  if particle.position.x.abs() > window_width {
    particle.position.x = window_height * particle.position.x.signum();
    particle.velocity *= Vec3::NEG_X * COLLISION_DAMPENING;
  }
}

#[derive(Component)]
pub struct Particle {
  pub position: Vec3,
  pub velocity: Vec3
}