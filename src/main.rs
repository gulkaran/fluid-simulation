use bevy::prelude::*;
use rand::Rng;

fn main() {
  App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(ParticlePlugin)
    .run();
} 


pub struct ParticlePlugin;

impl Plugin for ParticlePlugin {
  fn build(&self, app: &mut App) {
    app
      .add_systems(Startup, setup)
      .add_systems(Update, (positions, compile).chain());
  }
}

pub fn compile() {
  println!("compiled successfully");
}

pub fn setup(
  mut commands: Commands,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<ColorMaterial>>,
) {
  commands.spawn(Camera2d);

  for _ in 1..5 {
    let particle = Particle {
      x: rand::thread_rng().gen_range(0.0..500.0),
      y: rand::thread_rng().gen_range(0.0..500.0),
    };

    let shape = meshes.add(Circle::new(10.0));
    let color = Color::hsl(360. * rand::thread_rng().gen_range(0.0..1.0), 0.95, 0.7);
    
    commands.spawn((
      particle.clone(),
      Mesh2d(shape),
      MeshMaterial2d(materials.add(color)),
      Transform::from_xyz(particle.x, particle.y,0.0)
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

pub fn positions(particle_query: Query<&Particle>) {
  for particle in particle_query.iter() {
    println!("x: {}, y: {}", particle.x, particle.y);
  }
}

#[derive(Component, Copy, Clone)]
pub struct Particle {
  pub x: f32,
  pub y: f32
}