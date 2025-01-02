# **Fluid/Particle Simulation in Rust**

This is a fluid and particle simulation made in Rust using the Bevy game engine.
The theory, math, and step-by-step paper is (soon to be) uploaded
for other's to gain the intuition with molecular dynamics they'll need
to create a fluid simulation for themselves!

## **Installation**

To run the bevy project,

1. Install [Rust](https://rustup.rs/)
2. Clone the repository: **`git clone https://github.com/gulkaran/fluid-simulation.git`**
3. Build the project: **`cargo run --release`**

## **Showcase**

**Current Update** - Self-correcting constant density to simulate incompressible properties
of fluids (bugs with boundaries for now).

![Example](imgs/eg2.gif)

**Previous Update** - 10,000 particle collision optimized using uniform grids (gif
capped to 30fps)

![Example](imgs/eg1.gif)

## **Note**

There are different branches that correspond to the different stages of
the project. For instance, I diverted from the fluid simulation portion of
the project early on to gain more insight on how optimized particle simulations work.

The branch `collisions-optimized` shows off the simulation in a completely different light,
more focused on optimizing elastic particle collisions in 2D using a uniform grid approach
rather than emulating a fluid. This journey was one of experimenting and learning rather
than trying to build a polished product.
