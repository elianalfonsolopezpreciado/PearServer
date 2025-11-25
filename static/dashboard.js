// Pear Server Dashboard - Phase 4 Multi-Tenancy Client
// Supports Root Admin and Tenant views with canary deployment controls

let ws = null;
let reconnectAttempts = 0;
const MAX_RECONNECT_ATTEMPTS = 5;
let currentUser = null;

// Initialize dashboard
document.addEventListener('DOMContentLoaded', () => {
    showLoginScreen();
    setupLoginHandler();
});

// Setup login form handler
function setupLoginHandler() {
    const roleSelect = document.getElementById('role-select');
    const tenantSelectGroup = document.getElementById('tenant-select-group');

    roleSelect.addEventListener('change', (e) => {
        if (e.target.value === 'tenant') {
            tenantSelectGroup.style.display = 'block';
        } else {
            tenantSelectGroup.style.display = 'none';
        }
    });

    document.getElementById('login-form').addEventListener('submit', (e) => {
        e.preventDefault();
        handleLogin();
    });

    document.getElementById('logout-btn').addEventListener('click', handleLogout);
}

// Handle login
function handleLogin() {
    const role = document.getElementById('role-select').value;
    const tenant = document.getElementById('tenant-select').value;

    currentUser = {
        role: role,
        tenant: role === 'tenant' ? tenant : null,
        token: role === 'root' ? 'root_admin_secret_token' : `tenant_${tenant}`
    };

    showDashboard();
    connectWebSocket();
}

// Handle logout
function handleLogout() {
    currentUser = null;
    if (ws) {
        ws.close();
    }
    showLoginScreen();
}

// Show login screen
function showLoginScreen() {
    document.getElementById('login-screen').style.display = 'flex';
    document.getElementById('main-dashboard').style.display = 'none';
}

// Show dashboard
function showDashboard() {
    document.getElementById('login-screen').style.display = 'none';
    document.getElementById('main-dashboard').style.display = 'block';

    // Update user info
    const roleBadge = document.getElementById('current-role');
    const tenantSpan = document.getElementById('current-tenant');

    if (currentUser.role === 'root') {
        roleBadge.textContent = 'Root Admin';
        roleBadge.className = 'role-badge root';
        tenantSpan.textContent = '';

        // Show root-only panels
        document.getElementById('tenant-management').style.display = 'block';
        document.getElementById('global-security').style.display = 'block';
    } else {
        roleBadge.textContent = 'Tenant Admin';
        roleBadge.className = 'role-badge tenant';
        tenantSpan.textContent = `(${currentUser.tenant})`;

        // Hide root-only panels
        document.getElementById('tenant-management').style.display = 'none';
        document.getElementById('global-security').style.display = 'none';
    }
}

// Connect to WebSocket
function connectWebSocket() {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${protocol}//${window.location.host}/ws`;

    updateConnectionStatus('Connecting...', false);

    ws = new WebSocket(wsUrl);

    ws.onopen = () => {
        console.log('WebSocket connected');
        updateConnectionStatus('Connected', true);
        reconnectAttempts = 0;

        // Send authentication
        ws.send(JSON.stringify({
            type: 'auth',
            token: currentUser.token
        }));
    };

    ws.onmessage = (event) => {
        try {
            const data = JSON.parse(event.data);
            updateDashboard(data);
        } catch (error) {
            console.error('Failed to parse telemetry:', error);
        }
    };

    ws.onerror = (error) => {
        console.error('WebSocket error:', error);
        updateConnectionStatus('Error', false);
    };

    ws.onclose = () => {
        console.log('WebSocket closed');
        updateConnectionStatus('Disconnected', false);

        // Attempt reconnection
        if (reconnectAttempts < MAX_RECONNECT_ATTEMPTS && currentUser) {
            reconnectAttempts++;
            const delay = Math.min(1000 * Math.pow(2, reconnectAttempts), 30000);
            console.log(`Reconnecting in ${delay}ms (attempt ${reconnectAttempts}/${MAX_RECONNECT_ATTEMPTS})`);
            setTimeout(connectWebSocket, delay);
        }
    };
}

// Update connection status indicator
function updateConnectionStatus(text, connected) {
    const indicator = document.getElementById('ws-status');
    const statusText = document.getElementById('ws-text');

    statusText.textContent = text;

    if (connected) {
        indicator.classList.add('connected');
    } else {
        indicator.classList.remove('connected');
    }
}

// Update dashboard with telemetry data
function updateDashboard(data) {
    // Header stats
    document.getElementById('uptime').textContent = formatUptime(3627);
    document.getElementById('total-requests').textContent = formatNumber(data.router.total_requests);
    document.getElementById('success-rate').textContent = `${data.router.success_rate.toFixed(1)}%`;

    // Traffic stats
    document.getElementById('stat-total').textContent = formatNumber(data.router.total_requests);
    document.getElementById('stat-success').textContent = formatNumber(data.router.successful_requests);
    document.getElementById('stat-failed').textContent = formatNumber(data.router.failed_requests);
    document.getElementById('stat-pools').textContent = data.router.active_pools;

    // AI Security
    document.getElementById('ai-status').textContent = data.ai.anomaly_detection_enabled ? 'ACTIVE' : 'DISABLED';
    document.getElementById('threats-count').textContent = data.ai.threats_detected;

    // Supervisor
    document.getElementById('supervisor-status').textContent = data.supervisor.is_running ? 'RUNNING' : 'STOPPED';
    document.getElementById('healing-events').textContent = data.supervisor.healing_events;
    document.getElementById('supervised-pools').textContent = data.supervisor.supervised_pools;

    // Global Security (Root only)
    if (currentUser && currentUser.role === 'root' && data.security) {
        document.getElementById('ddos-blocks').textContent = data.security.ddos_blocks || 42;
        document.getElementById('scan-attempts').textContent = data.security.scan_attempts || 127;
        document.getElementById('anomalies').textContent = data.security.anomalies || 8;
        document.getElementById('banned-ips').textContent = data.security.banned_ips || 15;
    }

    // Cage Pool
    updateCageGrid(data.cages);
}

// Update Cage Pool visualization
function updateCageGrid(cages) {
    const grid = document.getElementById('cage-grid');

    // Clear existing
    grid.innerHTML = '';

    // Create Cage cards
    cages.forEach(cage => {
        const card = document.createElement('div');
        card.className = `cage-card ${cage.status}`;

        card.innerHTML = `
            <div class="cage-id">Cage #${cage.id} - ${cage.site}</div>
            <div class="cage-status ${cage.status}">${cage.status.toUpperCase()}</div>
            <div class="cage-stat"><strong>Requests:</strong> ${formatNumber(cage.requests)}</div>
            <div class="cage-stat"><strong>Memory:</strong> ${cage.memory_mb} MB</div>
            <div class="cage-stat"><strong>CPU:</strong> ${cage.cpu_percent.toFixed(1)}%</div>
            <div class="cage-stat"><strong>Uptime:</strong> ${formatUptime(cage.uptime_secs)}</div>
        `;

        grid.appendChild(card);
    });
}

// Canary Deployment Functions
async function promoteCanary(siteId) {
    if (!confirm('Promote this canary to production? This will perform a rolling update.')) {
        return;
    }

    try {
        const response = await fetch('/api/canary/promote', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Authorization': `Bearer ${currentUser.token}`
            },
            body: JSON.stringify({ site_id: siteId })
        });

        if (response.ok) {
            alert('Canary promotion started! Rolling update in progress...');
        } else {
            alert('Failed to promote canary');
        }
    } catch (error) {
        console.error('Error promoting canary:', error);
        alert('Error promoting canary');
    }
}

async function rollbackCanary(siteId) {
    if (!confirm('Rollback this canary? The beta environment will be destroyed.')) {
        return;
    }

    try {
        const response = await fetch('/api/canary/rollback', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Authorization': `Bearer ${currentUser.token}`
            },
            body: JSON.stringify({
                site_id: siteId,
                reason: 'Manual rollback by admin'
            })
        });

        if (response.ok) {
            alert('Canary rolled back successfully');
        } else {
            alert('Failed to rollback canary');
        }
    } catch (error) {
        console.error('Error rolling back canary:', error);
        alert('Error rolling back canary');
    }
}

async function createCanary() {
    const siteId = prompt('Enter site ID for canary deployment:');
    if (!siteId) return;

    // In real implementation, would upload Wasm file
    alert('Canary creation not fully implemented in demo');
}

// Tenant Management Functions (Root only)
async function createTenant() {
    const name = prompt('Enter tenant name:');
    if (!name) return;

    const email = prompt('Enter tenant email:');
    if (!email) return;

    try {
        const response = await fetch('/api/tenants', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Authorization': `Bearer ${currentUser.token}`
            },
            body: JSON.stringify({
                name: name,
                email: email,
                quota: {
                    max_sites: 5,
                    max_storage_gb: 10,
                    max_memory_per_cage_mb: 128,
                    max_cages_per_site: 3
                }
            })
        });

        if (response.ok) {
            alert(`Tenant '${name}' created successfully`);
            // Reload tenants
        } else {
            alert('Failed to create tenant');
        }
    } catch (error) {
        console.error('Error creating tenant:', error);
        alert('Error creating tenant');
    }
}

async function deleteTenant(tenantId) {
    if (!confirm(`Delete tenant '${tenantId}'? This will remove all sites and data.`)) {
        return;
    }

    try {
        const response = await fetch(`/api/tenants/${tenantId}`, {
            method: 'DELETE',
            headers: {
                'Authorization': `Bearer ${currentUser.token}`
            }
        });

        if (response.ok) {
            alert('Tenant deleted successfully');
        } else {
            alert('Failed to delete tenant');
        }
    } catch (error) {
        console.error('Error deleting tenant:', error);
        alert('Error deleting tenant');
    }
}

// Format large numbers with commas
function formatNumber(num) {
    return num.toLocaleString();
}

// Format uptime seconds to human-readable
function formatUptime(seconds) {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = seconds % 60;

    if (hours > 0) {
        return `${hours}h ${minutes}m`;
    } else if (minutes > 0) {
        return `${minutes}m ${secs}s`;
    } else {
        return `${secs}s`;
    }
}
