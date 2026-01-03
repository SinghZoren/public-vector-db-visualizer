use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Vector {
    pub data: Vec<f32>,
}

impl Vector {
    pub fn new(data: Vec<f32>) -> Self {
        Self { data }
    }

    #[allow(dead_code)]
    pub fn zeros(n: usize) -> Self {
        Self { data: vec![0.0; n] }
    }

    pub fn dot(&self, other: &Self) -> f32 {
        let mut sum: f64 = 0.0;
        for i in 0..self.data.len() {
            sum += (self.data[i] as f64) * (other.data[i] as f64);
        }
        sum as f32
    }

    #[allow(dead_code)]
    pub fn add(&mut self, other: &Self) {
        for i in 0..self.data.len() {
            self.data[i] += other.data[i];
        }
    }

    #[allow(dead_code)]
    pub fn sub_assign(&mut self, other: &Self) {
        for i in 0..self.data.len() {
            self.data[i] -= other.data[i];
        }
    }

    #[allow(dead_code)]
    pub fn sub(&self, other: &Self) -> Self {
        let mut result = vec![0.0; self.data.len()];
        for i in 0..self.data.len() {
            result[i] = self.data[i] - other.data[i];
        }
        Self::new(result)
    }

    pub fn scale(&mut self, factor: f32) {
        for i in 0..self.data.len() {
            self.data[i] *= factor;
        }
    }

    pub fn add_scaled(&mut self, other: &Self, factor: f32) {
        for i in 0..self.data.len() {
            self.data[i] += other.data[i] * factor;
        }
    }

    pub fn length(&self) -> f32 {
        let mut sum: f64 = 0.0;
        for i in 0..self.data.len() {
            sum += (self.data[i] as f64) * (self.data[i] as f64);
        }
        sum.sqrt() as f32
    }

    pub fn normalize(&mut self) {
        let len = self.length();
        if len > 1e-10 && len.is_finite() {
            self.scale(1.0 / len);
        } else {
        }
    }

    pub fn is_finite(&self) -> bool {
        self.data.iter().all(|x| x.is_finite())
    }
}

#[allow(dead_code)]
pub struct Matrix {
    pub rows: Vec<Vector>,
}

impl Matrix {
    #[allow(dead_code)]
    pub fn zeros(r: usize, c: usize) -> Self {
        Self {
            rows: (0..r).map(|_| Vector::zeros(c)).collect(),
        }
    }

    #[allow(dead_code)]
    pub fn multiply_vec(&self, v: &Vector) -> Vector {
        let mut result = vec![0.0; self.rows.len()];
        for i in 0..self.rows.len() {
            result[i] = self.rows[i].dot(v);
        }
        Vector::new(result)
    }
}
