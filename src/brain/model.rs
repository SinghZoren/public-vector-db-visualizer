use std::collections::HashMap;
use crate::brain::linalg::Vector;
use serde::{Deserialize, Serialize};

pub const EMBEDDING_DIM: usize = 128;

#[allow(dead_code)]
#[derive(Deserialize, Serialize, Debug)]
pub struct JsonWordData {
    #[serde(rename = "MEANINGS")]
    pub meanings: HashMap<String, serde_json::Value>,
    #[serde(rename = "ANTONYMS")]
    pub antonyms: Vec<String>,
    #[serde(rename = "SYNONYMS")]
    pub synonyms: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct SemanticBrain {
    pub vocabulary: HashMap<String, usize>,
    pub embeddings: Vec<Vector>,
    pub context_embeddings: Vec<Vector>,
}

impl SemanticBrain {
    pub fn new() -> Self {
        Self {
            vocabulary: HashMap::new(),
            embeddings: Vec::new(),
            context_embeddings: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        bincode::deserialize(bytes).map_err(|e| format!("Model Load Error: {}", e))
    }

    #[allow(dead_code)]
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        bincode::serialize(self).map_err(|e| format!("Model Save Error: {}", e))
    }

    #[allow(dead_code)]
    pub fn get_embedding(&self, word: &str) -> Option<&Vector> {
        self.vocabulary.get(&word.to_uppercase()).map(|&idx| &self.embeddings[idx])
    }

    #[allow(dead_code)]
    pub fn extract_context(&self, json_str: &str) -> Result<Vec<(String, Vec<String>, Vec<String>)>, String> {
        let data: HashMap<String, JsonWordData> = serde_json::from_str(json_str)
            .map_err(|e| format!("JSON Parse Error: {}", e))?;

        let stop_words = [
            "THE", "AND", "FOR", "ANY", "NOT", "BUT", "HAD", "WAS", "ARE", "WITH", 
            "THAT", "THIS", "FROM", "THEIR", "WHICH", "ALSO", "BEEN", "HAVE", "WERE",
            "THEY", "YOU", "YOUR", "THEM", "THESE", "THOSE", "WHEN", "WHERE", "WHO",
            "HOW", "WHY", "CAN", "WILL", "SOME", "MORE", "MOST", "OTHER", "INTO"
        ];

        let mut results = Vec::new();
        for (word, info) in data {
            let mut pos_context = info.synonyms.clone();
            let neg_context = info.antonyms.clone();

            for meaning in info.meanings.values() {
                if let Some(arr) = meaning.as_array() {
                    if let Some(def) = arr.get(1).and_then(|v| v.as_str()) {
                        for part in def.split_whitespace() {
                            let clean = part.trim_matches(|c: char| !c.is_alphabetic()).to_uppercase();
                            if clean.len() > 3 && !stop_words.contains(&clean.as_str()) && clean != word.to_uppercase() {
                                pos_context.push(clean);
                            }
                        }
                    }
                }
            }
            results.push((word.to_uppercase(), pos_context, neg_context));
        }
        Ok(results)
    }

    pub fn train_step(&mut self, word: &str, pos_context: &[String], neg_context: &[String], learning_rate: f32, negative_samples: usize) {
        let word_upper = word.to_uppercase();
        
        if self.should_skip(&word_upper) {
            return;
        }

        let word_idx = self.ensure_word(&word_upper);
        
        if !self.embeddings[word_idx].is_finite() {
            let word = word_upper.clone();
            self.embeddings[word_idx] = Self::generate_initial_vector_static(&word, false);
        }
        
        let sigmoid = |x: f32| 1.0 / (1.0 + (-x).exp());

        for context in pos_context {
            let ctx_upper = context.to_uppercase();
            if self.should_skip(&ctx_upper) { continue; }
            
            let context_idx = self.ensure_word(&ctx_upper);
            if word_idx == context_idx { continue; }

            if !self.context_embeddings[context_idx].is_finite() {
                self.context_embeddings[context_idx] = Self::generate_initial_vector_static(&ctx_upper, true);
            }

            let w = self.embeddings[word_idx].clone();
            let c = self.context_embeddings[context_idx].clone();
            
            let dot = w.dot(&c);
            let p = sigmoid(dot);
            let error = 1.0 - p;

            self.embeddings[word_idx].add_scaled(&c, learning_rate * error);
            self.context_embeddings[context_idx].add_scaled(&w, learning_rate * error);
        }

        for context in neg_context {
            let ctx_upper = context.to_uppercase();
            let context_idx = self.ensure_word(&ctx_upper);
            if word_idx == context_idx { continue; }

            let w = self.embeddings[word_idx].clone();
            let n = self.context_embeddings[context_idx].clone();
            
            let dot = w.dot(&n);
            let p = sigmoid(dot);
            let error = 0.0 - p;

            self.embeddings[word_idx].add_scaled(&n, learning_rate * error * 2.5);
            self.context_embeddings[context_idx].add_scaled(&w, learning_rate * error * 2.5);
        }

        if negative_samples > 0 && self.embeddings.len() > 10 {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            
            for i in 0..negative_samples {
                let mut hasher = DefaultHasher::new();
                word_upper.hash(&mut hasher);
                i.hash(&mut hasher);
                let rand_idx = (hasher.finish() as usize) % self.embeddings.len();
                
                if rand_idx == word_idx { continue; }
                
                let w = self.embeddings[word_idx].clone();
                let n = self.context_embeddings[rand_idx].clone();
                
                let dot = w.dot(&n);
                let p = sigmoid(dot);
                let error = 0.0 - p;

                self.embeddings[word_idx].add_scaled(&n, learning_rate * error * 0.75);
                self.context_embeddings[rand_idx].add_scaled(&w, learning_rate * error * 0.75);
            }
        }
        
        self.embeddings[word_idx].normalize();
    }

    pub fn find_most_similar(&self, word: &str, top_n: usize) -> Vec<(String, f32)> {
        let word_upper = word.to_uppercase();
        let target_idx = match self.vocabulary.get(&word_upper) {
            Some(&idx) => idx,
            None => return vec![],
        };

        let target_vec = &self.embeddings[target_idx];
        
        if !target_vec.is_finite() {
            return vec![("ERROR: Vector for this word is corrupt (NaN). Run /train/wiki/sanitize".to_string(), 0.0)];
        }

        let mut similarities = Vec::new();
        let mut nan_count = 0;
        let mut infinite_count = 0;
        let mut valid_count = 0;

        for (other_word, &idx) in &self.vocabulary {
            if idx == target_idx { continue; }
            let other_vec = &self.embeddings[idx];
            let sim = target_vec.dot(other_vec);
            if sim.is_nan() {
                nan_count += 1;
            } else if sim.is_infinite() {
                infinite_count += 1;
            } else {
                similarities.push((other_word.clone(), sim));
                valid_count += 1;
            }
        }

        println!("> Similarity test for {}: {} valid, {} NaNs, {} Infinities", word_upper, valid_count, nan_count, infinite_count);

        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        similarities.truncate(top_n);
        similarities
    }

    pub fn balance_vectors(&mut self) {
        if self.embeddings.is_empty() { return; }
        
        let mut idx_to_word = vec![String::new(); self.embeddings.len()];
        for (word, &idx) in &self.vocabulary {
            if idx < idx_to_word.len() {
                idx_to_word[idx] = word.clone();
            }
        }

        let mut mean = Vector::zeros(EMBEDDING_DIM);
        let mut count = 0;

        let mut healed_count = 0;
        for idx in 0..self.embeddings.len() {
            let v_len = self.embeddings[idx].length();
            if !self.embeddings[idx].is_finite() || v_len < 1e-4 || v_len > 10.0 {
                let word = &idx_to_word[idx];
                self.embeddings[idx] = Self::generate_initial_vector_static(word, false);
                healed_count += 1;
            }
            mean.add(&self.embeddings[idx]);
            count += 1;
        }
        println!("> Healed {} target embeddings.", healed_count);
        
        let mut healed_ctx_count = 0;
        for idx in 0..self.context_embeddings.len() {
            let v_len = self.context_embeddings[idx].length();
            if !self.context_embeddings[idx].is_finite() || v_len < 1e-4 || v_len > 10.0 {
                let word = &idx_to_word[idx];
                self.context_embeddings[idx] = Self::generate_initial_vector_static(word, true);
                healed_ctx_count += 1;
            }
        }
        println!("> Healed {} context embeddings.", healed_ctx_count);

        if count > 0 {
            mean.scale(1.0 / count as f32);
            println!("> Global mean length: {}", mean.length());
            
            if mean.length() < 0.9 {
                for v in &mut self.embeddings {
                    v.sub_assign(&mean);
                    v.normalize();
                }
            } else {
                println!("> Mean is too dominant, skipping centering to avoid collapse. Just normalizing.");
                for v in &mut self.embeddings {
                    v.normalize();
                }
            }
        }
    }

    pub fn calculate_analogy(&self, a: &str, b: &str, c: &str, top_n: usize) -> Vec<(String, f32)> {
        let vec_a = match self.get_embedding(a) { Some(v) => v, None => return vec![] };
        let vec_b = match self.get_embedding(b) { Some(v) => v, None => return vec![] };
        let vec_c = match self.get_embedding(c) { Some(v) => v, None => return vec![] };

        let mut target = vec_b.clone();
        target.sub_assign(vec_a);
        target.add(vec_c);
        target.normalize();

        let mut similarities = Vec::new();
        let skip_list = [a.to_uppercase(), b.to_uppercase(), c.to_uppercase()];

        for (word, &idx) in &self.vocabulary {
            if skip_list.contains(&word) { continue; }
            let other_vec = &self.embeddings[idx];
            let sim = target.dot(other_vec);
            if sim.is_finite() {
                similarities.push((word.clone(), sim));
            }
        }

        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        similarities.truncate(top_n);
        similarities
    }

    pub fn calculate_attention(&self, target_word: &str, context_words: &[String]) -> Vec<(String, f32)> {
        let target_vec = match self.get_embedding(target_word) {
            Some(v) => v,
            None => return vec![],
        };

        let mut attention_scores = Vec::new();
        let mut total_score = 0.0;

        for word in context_words {
            if let Some(ctx_vec) = self.get_embedding(word) {
                let score = (target_vec.dot(ctx_vec) / (EMBEDDING_DIM as f32).sqrt()).exp();
                attention_scores.push((word.clone(), score));
                total_score += score;
            }
        }

        if total_score > 0.0 {
            for (_, score) in &mut attention_scores {
                *score /= total_score;
            }
        }

        attention_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        attention_scores
    }

    pub fn compare_relationships(&self, a1: &str, b1: &str, a2: &str, b2: &str) -> f32 {
        let v_a1 = match self.get_embedding(a1) { Some(v) => v, None => return 0.0 };
        let v_b1 = match self.get_embedding(b1) { Some(v) => v, None => return 0.0 };
        let v_a2 = match self.get_embedding(a2) { Some(v) => v, None => return 0.0 };
        let v_b2 = match self.get_embedding(b2) { Some(v) => v, None => return 0.0 };

        let mut rel1 = v_b1.sub(v_a1);
        let mut rel2 = v_b2.sub(v_a2);
        
        let l1 = rel1.length();
        let l2 = rel2.length();
        
        if l1 < 1e-6 || l2 < 1e-6 { return 0.0; }
        
        rel1.scale(1.0 / l1);
        rel2.scale(1.0 / l2);
        
        rel1.dot(&rel2)
    }

    fn should_skip(&self, word: &str) -> bool {
        let common_words = ["THE", "AND", "FOR", "THAT", "THIS", "WITH", "FROM", "WAS", "WERE"];
        if common_words.contains(&word) {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut h = DefaultHasher::new();
            word.hash(&mut h);
            (h.finish() % 100) < 80
        } else {
            false
        }
    }

    fn generate_initial_vector_static(word: &str, is_context: bool) -> Vector {
        let mut data = vec![0.0; EMBEDDING_DIM];
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        for i in (0..EMBEDDING_DIM).step_by(2) {
            let mut h1 = DefaultHasher::new();
            if is_context { "ctx".hash(&mut h1); }
            word.hash(&mut h1);
            i.hash(&mut h1);
            let u1 = ((h1.finish() % 1000) as f32 / 1000.0).max(1e-6);

            let mut h2 = DefaultHasher::new();
            if is_context { "ctx".hash(&mut h2); }
            word.hash(&mut h2);
            (i + 1).hash(&mut h2);
            let u2 = ((h2.finish() % 1000) as f32 / 1000.0).max(1e-6);

            let r = (-2.0 * u1.ln()).sqrt();
            let theta = 2.0 * std::f32::consts::PI * u2;
            
            data[i] = r * theta.cos() * 0.1;
            if i + 1 < EMBEDDING_DIM {
                data[i+1] = r * theta.sin() * 0.1;
            }
        }
        let mut v = Vector::new(data);
        v.normalize();
        v
    }

    fn ensure_word(&mut self, word: &str) -> usize {
        if let Some(&idx) = self.vocabulary.get(word) {
            return idx;
        }

        let idx = self.embeddings.len();
        self.vocabulary.insert(word.to_string(), idx);
        
        self.embeddings.push(Self::generate_initial_vector_static(word, false));
        self.context_embeddings.push(Self::generate_initial_vector_static(word, true));
        
        idx
    }
}
