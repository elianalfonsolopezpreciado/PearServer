// Load balancing strategies for traffic distribution

use tracing::debug;

/// Strategy trait for load balancing
pub trait LoadBalancingStrategy: Send + Sync {
    fn name(&self) -> &'static str;
}

/// Round-robin load balancing
/// Distributes requests evenly across all healthy Cages
pub struct RoundRobinStrategy {
    counter: std::sync::atomic::AtomicUsize,
}

impl RoundRobinStrategy {
    pub fn new() -> Self {
        Self {
            counter: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    pub fn next_index(&self, pool_size: usize) -> usize {
        if pool_size == 0 {
            return 0;
        }
        
        let index = self.counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let selected = index % pool_size;
        
        debug!(
            index = index,
            pool_size = pool_size,
            selected = selected,
            "Round-robin selection"
        );
        
        selected
    }
}

impl Default for RoundRobinStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl LoadBalancingStrategy for RoundRobinStrategy {
    fn name(&self) -> &'static str {
        "round-robin"
    }
}

/// Least-connected load balancing
/// Routes requests to the Cage with the fewest active connections
pub struct LeastConnectedStrategy;

impl LeastConnectedStrategy {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LeastConnectedStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl LoadBalancingStrategy for LeastConnectedStrategy {
    fn name(&self) -> &'static str {
        "least-connected"
    }
}

/// Weighted load balancing (for future use)
/// Distributes traffic based on Cage capacity and health scores
pub struct WeightedStrategy {
    weights: std::sync::Arc<std::sync::RwLock<Vec<f64>>>,
}

impl WeightedStrategy {
    pub fn new() -> Self {
        Self {
            weights: std::sync::Arc::new(std::sync::RwLock::new(Vec::new())),
        }
    }

    pub fn update_weights(&self, weights: Vec<f64>) {
        let mut w = self.weights.write().unwrap();
        *w = weights;
    }

    pub fn select_index(&self, pool_size: usize) -> usize {
        let weights = self.weights.read().unwrap();
        
        if weights.is_empty() || weights.len() != pool_size {
            // Fallback to round-robin if weights not configured
            return rand::random::<usize>() % pool_size.max(1);
        }

        // Weighted random selection
        let total: f64 = weights.iter().sum();
        let mut rng = rand::random::<f64>() * total;
        
        for (i, &weight) in weights.iter().enumerate() {
            rng -= weight;
            if rng <= 0.0 {
                return i;
            }
        }
        
        pool_size.saturating_sub(1)
    }
}

impl Default for WeightedStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl LoadBalancingStrategy for WeightedStrategy {
    fn name(&self) -> &'static str {
        "weighted"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_robin() {
        let strategy = RoundRobinStrategy::new();
        
        let idx1 = strategy.next_index(3);
        let idx2 = strategy.next_index(3);
        let idx3 = strategy.next_index(3);
        
        assert!(idx1 < 3);
        assert!(idx2 < 3);
        assert!(idx3 < 3);
    }

    #[test]
    fn test_least_connected_creation() {
        let strategy = LeastConnectedStrategy::new();
        assert_eq!(strategy.name(), "least-connected");
    }

    #[test]
    fn test_weighted_strategy() {
        let strategy = WeightedStrategy::new();
        strategy.update_weights(vec![1.0, 2.0, 3.0]);
        
        let idx = strategy.select_index(3);
        assert!(idx < 3);
    }
}
