use crate::brain::linalg::{Vector, Matrix};
use crate::brain::model::EMBEDDING_DIM;

#[allow(dead_code)]
pub struct Projector {
    pub projection_matrix: Matrix,
    pub mean_vector: Vector,
    pub scales: (f32, f32, f32),
}

impl Projector {
    #[allow(dead_code)]
    pub fn new() -> Self {
        let mut mat = Matrix::zeros(3, EMBEDDING_DIM);
        
        for i in 0..EMBEDDING_DIM {
            mat.rows[0].data[i] = (i as f32 * 0.1).sin();
            mat.rows[1].data[i] = (i as f32 * 0.15).cos();
            mat.rows[2].data[i] = (i as f32 * 0.2).sin() * (i as f32 * 0.05).cos();
        }
        
        mat.rows[0].normalize();
        mat.rows[1].normalize();
        mat.rows[2].normalize();

        Self { 
            projection_matrix: mat,
            mean_vector: Vector::zeros(EMBEDDING_DIM),
            scales: (1.0, 1.0, 1.0),
        }
    }

    #[allow(dead_code)]
    pub fn project(&self, v: &Vector) -> (f32, f32, f32) {
        let centered = v.sub(&self.mean_vector);
        let p = self.projection_matrix.multiply_vec(&centered);
        
        (
            p.data[0] * self.scales.0, 
            p.data[1] * self.scales.1, 
            p.data[2] * self.scales.2
        )
    }

    #[allow(dead_code)]
    pub fn fit(&mut self, embeddings: &[Vector]) {
        if embeddings.is_empty() { return; }

        let mut mean = Vector::zeros(EMBEDDING_DIM);
        for v in embeddings {
            mean.add(v);
        }
        mean.scale(1.0 / embeddings.len() as f32);
        self.mean_vector = mean;

        for axis_idx in 0..3 {
            let mut current_axis = Vector::zeros(EMBEDDING_DIM);
            for i in 0..EMBEDDING_DIM {
                current_axis.data[i] = ((axis_idx + i + 7) as f32 * 0.123).sin() + ((axis_idx * i) as f32 * 0.456).cos();
            }
            current_axis.normalize();

            for _ in 0..30 {
                let mut new_axis = Vector::zeros(EMBEDDING_DIM);
                
                for v in embeddings {
                    let centered_v = v.sub(&self.mean_vector);
                    let score = centered_v.dot(&current_axis);
                    new_axis.add_scaled(&centered_v, score);
                }

                for prev_idx in 0..axis_idx {
                    let prev_axis = &self.projection_matrix.rows[prev_idx];
                    let overlap = new_axis.dot(prev_axis);
                    new_axis.add_scaled(prev_axis, -overlap);
                }
                
                if new_axis.length() > 1e-9 {
                    new_axis.normalize();
                    current_axis = new_axis;
                }
            }
            self.projection_matrix.rows[axis_idx] = current_axis;
        }

        let mut vars = [0.0f32; 3];
        for v in embeddings {
            let centered = v.sub(&self.mean_vector);
            let p = self.projection_matrix.multiply_vec(&centered);
            for i in 0..3 {
                vars[i] += p.data[i] * p.data[i];
            }
        }
        
        for i in 0..3 {
            let std_dev = (vars[i] / embeddings.len() as f32).sqrt();
            if std_dev > 1e-6 {
                if i == 0 { self.scales.0 = 1.5 / std_dev; }
                else if i == 1 { self.scales.1 = 1.5 / std_dev; }
                else { self.scales.2 = 1.5 / std_dev; }
            }
        }
    }
}
