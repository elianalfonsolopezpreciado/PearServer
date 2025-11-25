// AI Security Module
// ML-powered anomaly detection and threat identification

pub mod anomaly;
pub mod ddos;
pub mod path_monitor;
pub mod performance_baseline;

use anyhow::Result;
use std::sync::Arc;
use tracing::{info, debug, warn, instrument};

/// AI module configuration
#[derive(Debug, Clone)]
pub struct AiConfig {
    /// Enable anomaly detection
    pub enable_anomaly_detection: bool,
    
    /// Anomaly detection threshold (0.0-1.0)
    pub anomaly_threshold: f64,
    
    /// Sample rate for traffic analysis (0.0-1.0, 1.0 = analyze all requests)
    pub sample_rate: f64,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enable_anomaly_detection: true,
            anomaly_threshold: 0.8,  // 80% confidence threshold
            sample_rate: 0.1,         // Analyze 10% of traffic
        }
    }
}

/// AI-powered security module
pub struct AiSecurityModule {
    config: AiConfig,
    anomaly_detector: Arc<anomaly::AnomalyDetector>,
    threats_detected: Arc<std::sync::atomic::AtomicU64>,
}

impl AiSecurityModule {
    /// Create a new AI security module
    pub fn new(config: AiConfig) -> Result<Self> {
        info!("Initializing AI Security Module");
        
        let anomaly_detector = Arc::new(anomaly::AnomalyDetector::new()?);
        
        Ok(Self {
            config,
            anomaly_detector,
            threats_detected: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        })
    }

    /// Analyze a request for anomalies
    #[instrument(skip(self, request_features))]
    pub async fn analyze_request(&self, request_features: RequestFeatures) -> AnalysisResult {
        if !self.config.enable_anomaly_detection {
            return AnalysisResult::safe();
        }

        // Sample requests based on configured rate
        if !self.should_sample() {
            return AnalysisResult::safe();
        }

        debug!("Analyzing request for anomalies");

        // Perform anomaly detection
        match self.anomaly_detector.detect(&request_features).await {
            Ok(score) => {
                if score > self.config.anomaly_threshold {
                    warn!(
                        score = score,
                        threshold = self.config.anomaly_threshold,
                        "Anomaly detected in request"
                    );
                    
                    self.threats_detected.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    
                    AnalysisResult {
                        is_safe: false,
                        confidence: score,
                        threat_type: Some(ThreatType::Anomalous),
                        details: Some(format!("Anomaly score: {:.2}", score)),
                    }
                } else {
                    AnalysisResult::safe()
                }
            }
            Err(e) => {
                warn!(error = %e, "Anomaly detection failed");
                AnalysisResult::safe() // Fail open for availability
            }
        }
    }

    /// Check if request should be sampled
    fn should_sample(&self) -> bool {
        rand::random::<f64>() < self.config.sample_rate
    }

    /// Get statistics
    pub fn stats(&self) -> AiStats {
        AiStats {
            threats_detected: self.threats_detected.load(std::sync::atomic::Ordering::Relaxed),
            anomaly_detection_enabled: self.config.enable_anomaly_detection,
        }
    }
}

/// Request features for analysis
#[derive(Debug, Clone)]
pub struct RequestFeatures {
    pub method: String,
    pub path: String,
    pub query_params: Vec<(String, String)>,
    pub headers: Vec<(String, String)>,
    pub body_size: usize,
    pub source_ip: String,
}

impl RequestFeatures {
    pub fn to_feature_vector(&self) -> Vec<f64> {
        // Extract numerical features for ML model
        vec![
            self.path.len() as f64,
            self.query_params.len() as f64,
            self.headers.len() as f64,
            self.body_size as f64,
            // Add more sophisticated features in production
        ]
    }
}

/// Analysis result
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub is_safe: bool,
    pub confidence: f64,
    pub threat_type: Option<ThreatType>,
    pub details: Option<String>,
}

impl AnalysisResult {
    pub fn safe() -> Self {
        Self {
            is_safe: true,
            confidence: 1.0,
            threat_type: None,
            details: None,
        }
    }
}

/// Threat classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreatType {
    /// Traffic pattern anomaly
    Anomalous,
    
    /// SQL injection attempt
    SqlInjection,
    
    /// XSS attempt
    Xss,
    
    /// DDoS pattern
    DdosPattern,
    
    /// Suspicious bot activity
    BotActivity,
}

/// AI statistics
#[derive(Debug, Clone)]
pub struct AiStats {
    pub threats_detected: u64,
    pub anomaly_detection_enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ai_module_creation() {
        let config = AiConfig::default();
        let module = AiSecurityModule::new(config);
        assert!(module.is_ok());
    }

    #[test]
    fn test_request_features() {
        let features = RequestFeatures {
            method: "GET".to_string(),
            path: "/api/users".to_string(),
            query_params: vec![],
            headers: vec![],
            body_size: 0,
            source_ip: "127.0.0.1".to_string(),
        };
        
        let vector = features.to_feature_vector();
        assert!(!vector.is_empty());
    }

    #[test]
    fn test_analysis_result() {
        let result = AnalysisResult::safe();
        assert!(result.is_safe);
        assert_eq!(result.confidence, 1.0);
    }
}
