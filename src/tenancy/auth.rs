// Multi-Tenancy Authentication and Authorization
// JWT-based tenant authentication with role-based access control

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use anyhow::{Result, Context};
use tracing::{info, warn};

/// User role in the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Role {
    /// Root administrator with global access
    RootAdmin,
    
    /// Tenant administrator with tenant-specific access
    TenantAdmin,
}

/// Authentication token claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    /// User ID
    pub user_id: Uuid,
    
    /// User role
    pub role: Role,
    
    /// Tenant ID (None for RootAdmin)
    pub tenant_id: Option<Uuid>,
    
    /// Issued at (Unix timestamp)
    pub iat: i64,
    
    /// Expires at (Unix timestamp)
    pub exp: i64,
}

/// Authentication manager
pub struct AuthManager {
    /// Root admin token (for demo)
    root_admin_token: String,
}

impl AuthManager {
    /// Create a new auth manager
    pub fn new() -> Self {
        // In production, this would use proper JWT signing
        let root_admin_token = "root_admin_secret_token".to_string();
        
        info!("Authentication manager initialized");
        
        Self {
            root_admin_token,
        }
    }

    /// Validate token and extract claims
    pub fn validate_token(&self, token: &str) -> Result<TokenClaims> {
        // Simplified validation for Phase 4
        // In production, use proper JWT library (jsonwebtoken crate)
        
        if token == self.root_admin_token {
            Ok(TokenClaims {
                user_id: Uuid::nil(),
                role: Role::RootAdmin,
                tenant_id: None,
                iat: chrono::Utc::now().timestamp(),
                exp: chrono::Utc::now().timestamp() + 3600 * 24, // 24 hours
            })
        } else if token.starts_with("tenant_") {
            // Parse tenant token: "tenant_{tenant_id}"
            let tenant_id = token.strip_prefix("tenant_")
                .and_then(|id| Uuid::parse_str(id).ok())
                .context("Invalid tenant token")?;
            
            Ok(TokenClaims {
                user_id: Uuid::new_v4(),
                role: Role::TenantAdmin,
                tenant_id: Some(tenant_id),
                iat: chrono::Utc::now().timestamp(),
                exp: chrono::Utc::now().timestamp() + 3600, // 1 hour
            })
        } else {
            anyhow::bail!("Invalid token")
        }
    }

    /// Generate tenant token
    pub fn generate_tenant_token(&self, tenant_id: Uuid) -> String {
        // Simplified token generation
        // In production, use proper JWT signing
        format!("tenant_{}", tenant_id)
    }

    /// Check if user has access to tenant
    pub fn check_tenant_access(&self, claims: &TokenClaims, tenant_id: Uuid) -> bool {
        match claims.role {
            Role::RootAdmin => true, // Root admin has access to all tenants
            Role::TenantAdmin => claims.tenant_id == Some(tenant_id),
        }
    }

    /// Check if user is root admin
    pub fn is_root_admin(&self, claims: &TokenClaims) -> bool {
        claims.role == Role::RootAdmin
    }
}

impl Default for AuthManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Authorization middleware result
#[derive(Debug)]
pub enum AuthResult {
    Authorized(TokenClaims),
    Unauthorized,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_manager_creation() {
        let auth = AuthManager::new();
        assert!(!auth.root_admin_token.is_empty());
    }

    #[test]
    fn test_root_admin_token() {
        let auth = AuthManager::new();
        let claims = auth.validate_token("root_admin_secret_token").unwrap();
        
        assert_eq!(claims.role, Role::RootAdmin);
        assert!(auth.is_root_admin(&claims));
    }

    #[test]
    fn test_tenant_token() {
        let auth = AuthManager::new();
        let tenant_id = Uuid::new_v4();
        
        let token = auth.generate_tenant_token(tenant_id);
        let claims = auth.validate_token(&token).unwrap();
        
        assert_eq!(claims.role, Role::TenantAdmin);
        assert_eq!(claims.tenant_id, Some(tenant_id));
    }

    #[test]
    fn test_tenant_access() {
        let auth = AuthManager::new();
        let tenant_id = Uuid::new_v4();
        let other_tenant_id = Uuid::new_v4();
        
        let token = auth.generate_tenant_token(tenant_id);
        let claims = auth.validate_token(&token).unwrap();
        
        // Should have access to own tenant
        assert!(auth.check_tenant_access(&claims, tenant_id));
        
        // Should not have access to other tenant
        assert!(!auth.check_tenant_access(&claims, other_tenant_id));
    }

    #[test]
    fn test_root_admin_access() {
        let auth = AuthManager::new();
        let claims = auth.validate_token("root_admin_secret_token").unwrap();
        
        let any_tenant = Uuid::new_v4();
        
        // Root admin has access to all tenants
        assert!(auth.check_tenant_access(&claims, any_tenant));
    }
}
