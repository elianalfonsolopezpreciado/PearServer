// Anomaly detection using machine learning
// Implements Isolation Forest for detecting abnormal traffic patterns

use anyhow::Result;
use ndarray::{Array1, Array2};
use tracing::{debug, info};
use linfa::prelude::*;
use linfa_clustering::IsolationForest;

/// Anomaly detector using Isolation Forest
pub struct AnomalyDetector {
    /// Trained model (optional, starts None and trains on-the-fly)
    model: Option<IsolationForest<f64>>,
    
    /// Training data buffer
    training_buffer: Vec<Vec<f64>>,
    
    /// Buffer size before training
    buffer_size: usize,
}

impl AnomalyDetector {
    /// Create a new anomaly detector
    pub fn new() -> Result<Self> {
        info!("Creating anomaly detector");
        
        Ok(Self {
            model: None,
            training_buffer: Vec::new(),
            buffer_size: 100,  // Train after 100 samples
        })
    }

    /// Detect anomaly in request features
    pub async fn detect(&self, features: &super::RequestFeatures) -> Result<f64> {
        let feature_vector = features.to_feature_vector();
        
        // If model is trained, use it
        if let Some(ref model) = self.model {
            let score = self.score_sample(&feature_vector, model);
            debug!(score = score, "Anomaly score calculated");
            Ok(score)
        } else {
            // Model not yet trained, return neutral score
            Ok(0.5)
        }
    }

    /// Score a single sample (simplified for Phase 2)
    fn score_sample(&self, _features: &[f64], _model: &IsolationForest<f64>) -> f64 {
        // In production, this would use the model to score
        // For Phase 2, return a random score weighted toward normal
        let base_score = rand::random::<f64>();
        
        // Weight toward normal (low scores)
        if base_score < 0.95 {
            base_score * 0.5  // Most traffic is normal
        } else {
            0.8 + rand::random::<f64>() * 0.2  // Occasionally flag as suspicious
        }
    }

    /// Train model on buffered data
    pub async fn train(&mut self) -> Result<()> {
        if self.training_buffer.len() < self.buffer_size {
            return Ok(());
        }

        info!(samples = self.training_buffer.len(), "Training anomaly detection model");

        // Convert buffer to ndarray
        let n_samples = self.training_buffer.len();
        let n_features = self.training_buffer[0].len();
        
        let mut data = Vec::new();
        for sample in &self.training_buffer {
            data.extend_from_slice(sample);
        }
        
        let array = Array2::<f64>::from_shape_vec((n_samples, n_features), data)?;
        let dataset = DatasetBase::from(array);

        // Train Isolation Forest
        let model = IsolationForest::params()
            .n_trees(100)
            .sample_size(20)
            .fit(&dataset)?;

        self.model = Some(model);
        
        // Clear buffer
        self.training_buffer.clear();
        
        info!("Model training complete");
        Ok(())
    }

    /// Add sample to training buffer
    pub fn add_training_sample(&mut self, features: Vec<f64>) {
        self.training_buffer.push(features);
        
        // Auto-train when buffer is full
        if self.training_buffer.len() >= self.buffer_size {
            // Spawn training task
            let detector = self.clone_for_training();
            tokio::spawn(async move {
                let mut det = detector;
                if let Err(e) = det.train().await {
                    tracing::error!(error = %e, "Model training failed");
                }
            });
        }
    }

    /// Clone for training task
    fn clone_for_training(&self) -> Self {
        Self {
            model: self.model.clone(),
            training_buffer: self.training_buffer.clone(),
            buffer_size: self.buffer_size,
        }
    }
}

/// Simple statistical anomaly detector (fallback)
/// Uses z-score for simple anomaly detection
pub struct StatisticalDetector {
    mean: f64,
    std_dev: f64,
    samples: Vec<f64>,
}

impl StatisticalDetector {
    pub fn new() -> Self {
        Self {
            mean: 0.0,
            std_dev: 1.0,
            samples: Vec::new(),
        }
    }

    pub fn add_sample(&mut self, value: f64) {
        self.samples.push(value);
        self.update_statistics();
    }

    fn update_statistics(&mut self) {
        if self.samples.is_empty() {
            return;
        }

        // Calculate mean
        self.mean = self.samples.iter().sum::<f64>() / self.samples.len() as f64;
        
        // Calculate standard deviation
        let variance: f64 = self.samples.iter()
            .map(|x| (x - self.mean).powi(2))
            .sum::<f64>() / self.samples.len() as f64;
        
        self.std_dev = variance.sqrt();
    }

    pub fn is_anomaly(&self, value: f64, threshold: f64) -> bool {
        if self.std_dev == 0.0 {
            return false;
        }
        
        let z_score = (value - self.mean).abs() / self.std_dev;
        z_score > threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_detector_creation() {
        let detector = AnomalyDetector::new();
        assert!(detector.is_ok());
    }

    #[test]
    fn test_statistical_detector() {
        let mut detector = StatisticalDetector::new();
        
        // Add normal samples
        for i in 0..100 {
            detector.add_sample(50.0 + (i as f64 % 10.0));
        }
        
        // Test normal value
        assert!(!detector.is_anomaly(52.0, 3.0));
        
        // Test anomalous value
        assert!(detector.is_anomaly(500.0, 3.0));
    }
}
