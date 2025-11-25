// CRDT State Synchronization Module
// Shared eventually consistent state across Cage instances

pub mod sync;
pub mod session;

use automerge::{Automerge, transaction::Transactable, ObjType, ScalarValue};
use anyhow::{Result, Context};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, warn, instrument};

/// CRDT-based shared state manager
/// Provides eventually consistent state across all Cages in a pool
pub struct CrdtStateManager {
    /// Automerge document for state
    document: Arc<RwLock<Automerge>>,
    
    /// Site identifier for this state manager
    site_id: String,
}

impl CrdtStateManager {
    /// Create a new CRDT state manager
    pub fn new(site_id: String) -> Self {
        info!(site_id = %site_id, "Initializing CRDT state manager");
        
        let document = Automerge::new();
        
        Self {
            document: Arc::new(RwLock::new(document)),
            site_id,
        }
    }

    /// Set a value in the shared state
    #[instrument(skip(self, value))]
    pub async fn set(&self, key: &str, value: serde_json::Value) -> Result<()> {
        let mut doc = self.document.write().await;
        
        let mut tx = doc.transaction();
        
        // Convert JSON value to Automerge scalar
        let scalar = match value {
            serde_json::Value::String(s) => ScalarValue::Str(s.into()),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    ScalarValue::Int(i)
                } else if let Some(f) = n.as_f64() {
                    ScalarValue::F64(f)
                } else {
                    ScalarValue::Str(n.to_string().into())
                }
            }
            serde_json::Value::Bool(b) => ScalarValue::Boolean(b),
            serde_json::Value::Null => ScalarValue::Null,
            _ => {
                // For complex types, serialize to string
                ScalarValue::Str(value.to_string().into())
            }
        };

        tx.put(automerge::ROOT, key, scalar)
            .context("Failed to set value in CRDT")?;
        
        tx.commit();

        debug!(key = %key, "CRDT value set");
        Ok(())
    }

    /// Get a value from the shared state
    #[instrument(skip(self))]
    pub async fn get(&self, key: &str) -> Result<Option<serde_json::Value>> {
        let doc = self.document.read().await;
        
        if let Some((value, _)) = doc.get(automerge::ROOT, key)? {
            let json_value = match value {
                automerge::Value::Scalar(s) => match s.as_ref() {
                    ScalarValue::Str(s) => serde_json::Value::String(s.to_string()),
                    ScalarValue::Int(i) => serde_json::Value::Number((*i).into()),
                    ScalarValue::F64(f) => {
                        serde_json::Number::from_f64(*f)
                            .map(serde_json::Value::Number)
                            .unwrap_or(serde_json::Value::Null)
                    }
                    ScalarValue::Boolean(b) => serde_json::Value::Bool(*b),
                    ScalarValue::Null => serde_json::Value::Null,
                    _ => serde_json::Value::Null,
                },
                _ => serde_json::Value::Null,
            };
            
            debug!(key = %key, "CRDT value retrieved");
            Ok(Some(json_value))
        } else {
            Ok(None)
        }
    }

    /// Get the changeset for synchronization
    pub async fn get_changes(&self) -> Result<Vec<u8>> {
        let doc = self.document.read().await;
        Ok(doc.save())
    }

    /// Apply changes from another instance
    #[instrument(skip(self, changes))]
    pub async fn apply_changes(&self, changes: &[u8]) -> Result<()> {
        let mut doc = self.document.write().await;
        
        doc.load_incremental(changes)
            .context("Failed to apply CRDT changes")?;
        
        info!("CRDT changes applied successfully");
        Ok(())
    }

    /// Merge with another document
    pub async fn merge(&self, other_doc: &Automerge) -> Result<()> {
        let mut doc = self.document.write().await;
        
        doc.merge(other_doc)
            .context("Failed to merge CRDT documents")?;
        
        info!("CRDT documents merged successfully");
        Ok(())
    }

    /// Get current document size
    pub async fn size(&self) -> usize {
        let doc = self.document.read().await;
        doc.save().len()
    }
}

/// Shared state handle for Cages
/// Each Cage gets a handle to interact with shared state
#[derive(Clone)]
pub struct StateHandle {
    manager: Arc<CrdtStateManager>,
}

impl StateHandle {
    pub fn new(manager: Arc<CrdtStateManager>) -> Self {
        Self { manager }
    }

    pub async fn set(&self, key: &str, value: serde_json::Value) -> Result<()> {
        self.manager.set(key, value).await
    }

    pub async fn get(&self, key: &str) -> Result<Option<serde_json::Value>> {
        self.manager.get(key).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_crdt_creation() {
        let manager = CrdtStateManager::new("test-site".to_string());
        assert!(manager.size().await > 0);
    }

    #[tokio::test]
    async fn test_crdt_set_get() {
        let manager = CrdtStateManager::new("test-site".to_string());
        
        manager.set("test_key", serde_json::json!("test_value")).await.unwrap();
        
        let value = manager.get("test_key").await.unwrap();
        assert!(value.is_some());
        assert_eq!(value.unwrap(), serde_json::json!("test_value"));
    }

    #[tokio::test]
    async fn test_crdt_synchronization() {
        let manager1 = CrdtStateManager::new("site1".to_string());
        let manager2 = CrdtStateManager::new("site2".to_string());
        
        // Set value in manager1
        manager1.set("key1", serde_json::json!(42)).await.unwrap();
        
        // Get changes from manager1
        let changes = manager1.get_changes().await.unwrap();
        
        // Apply changes to manager2
        manager2.apply_changes(&changes).await.unwrap();
        
        // Verify value is now in manager2
        let value = manager2.get("key1").await.unwrap();
        assert_eq!(value, Some(serde_json::json!(42)));
    }
}
