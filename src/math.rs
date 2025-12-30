use bevy::math::Vec3;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

// Simple pseudo-random number generator using hash
struct SimpleRng {
    seed: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { seed }
    }

    fn next_f32(&mut self) -> f32 {
        // Simple LCG (Linear Congruential Generator)
        self.seed = self.seed.wrapping_mul(1103515245).wrapping_add(12345);
        (self.seed % 1000000) as f32 / 1000000.0
    }
}

pub fn project_text_to_3d(text: &str) -> Vec3 {
    // Step 1: Hash the text to get a u64 seed
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    let seed = hasher.finish();

    // Step 2: Generate pseudo-random vector of 384 floats using simple RNG
    let mut rng = SimpleRng::new(seed);
    let random_vector: Vec<f32> = (0..384).map(|_| rng.next_f32()).collect();

    // Step 3: Define 3 hard-coded axis vectors (each of length 384)
    // Using raw trig functions without the +0.5 bias to allow for negative coordinates
    let axis_x: Vec<f32> = (0..384).map(|i| (i as f32 * 0.1).sin()).collect();
    let axis_y: Vec<f32> = (0..384).map(|i| (i as f32 * 0.15).cos()).collect();
    let axis_z: Vec<f32> = (0..384).map(|i| (i as f32 * 0.2).sin() * (i as f32 * 0.05).cos()).collect();

    // Step 4: Dot product with each axis
    let x = dot_product(&random_vector, &axis_x);
    let y = dot_product(&random_vector, &axis_y);
    let z = dot_product(&random_vector, &axis_z);

    // Step 5: Return Vec3 scaled to fill the massive 3D volume
    Vec3::new(x, y, z) * 300.0
}

fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}