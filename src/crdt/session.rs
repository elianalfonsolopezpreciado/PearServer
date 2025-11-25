// Session data structures for CRDT state
// Manages user sessions, authentication, and application state

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// User session data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    /// Session ID
    pub session_id: String,
    
    /// User ID
    pub user_id: Option<String>,
    
    /// Session creation time (Unix timestamp)
    pub created_at: u64,
    
    /// Last activity time (Unix timestamp)
    pub last_activity: u64,
    
    /// Session data (flexible key-value store)
    pub data: HashMap<String, serde_json::Value>,
    
    /// Authentication token
    pub auth_token: Option<String>,
    
    /// Is authenticated
    pub is_authenticated: bool,
}

impl UserSession {
    pub fn new(session_id: String) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            session_id,
            user_id: None,
            created_at: now,
            last_activity: now,
            data: HashMap::new(),
            auth_token: None,
            is_authenticated: false,
        }
    }

    pub fn authenticate(&mut self, user_id: String, token: String) {
        self.user_id = Some(user_id);
        self.auth_token = Some(token);
        self.is_authenticated = true;
        self.update_activity();
    }

    pub fn update_activity(&mut self) {
        self.last_activity = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    pub fn is_expired(&self, timeout_secs: u64) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        now - self.last_activity > timeout_secs
    }
}

/// Shopping cart (example of shared application state)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShoppingCart {
    pub cart_id: String,
    pub user_id: Option<String>,
    pub items: Vec<CartItem>,
    pub total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CartItem {
    pub product_id: String,
    pub quantity: u32,
    pub price: f64,
}

impl ShoppingCart {
    pub fn new(cart_id: String) -> Self {
        Self {
            cart_id,
            user_id: None,
            items: Vec::new(),
            total: 0.0,
        }
    }

    pub fn add_item(&mut self, product_id: String, quantity: u32, price: f64) {
        self.items.push(CartItem {
            product_id,
            quantity,
            price,
        });
        self.recalculate_total();
    }

    pub fn remove_item(&mut self, product_id: &str) {
        self.items.retain(|item| item.product_id != product_id);
        self.recalculate_total();
    }

    fn recalculate_total(&mut self) {
        self.total = self.items.iter()
            .map(|item| item.price * item.quantity as f64)
            .sum();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_session_creation() {
        let session = UserSession::new("test-session-123".to_string());
        assert_eq!(session.session_id, "test-session-123");
        assert!(!session.is_authenticated);
    }

    #[test]
    fn test_user_session_authentication() {
        let mut session = UserSession::new("test-session".to_string());
        session.authenticate("user123".to_string(), "token456".to_string());
        
        assert!(session.is_authenticated);
        assert_eq!(session.user_id, Some("user123".to_string()));
    }

    #[test]
    fn test_shopping_cart() {
        let mut cart = ShoppingCart::new("cart-1".to_string());
        
        cart.add_item("product-a".to_string(), 2, 10.0);
        cart.add_item("product-b".to_string(), 1, 25.0);
        
        assert_eq!(cart.items.len(), 2);
        assert_eq!(cart.total, 45.0);
        
        cart.remove_item("product-a");
        assert_eq!(cart.items.len(), 1);
        assert_eq!(cart.total, 25.0);
    }
}
