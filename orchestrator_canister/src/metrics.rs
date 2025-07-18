use crate::http_types::{HttpRequest, HttpResponse, HttpResponseBuilder};
use ic_metrics_encoder::MetricsEncoder;
use crate::state::read_state;
use crate::types::*;

pub fn http_request(req: HttpRequest) -> HttpResponse {
    match req.path() {
        "/metrics" => handle_metrics_request(),
        _ => HttpResponseBuilder::not_found().build(),
    }
}

fn handle_metrics_request() -> HttpResponse {
    let mut writer = MetricsEncoder::new(vec![], ic_cdk::api::time() as i64 / 1_000_000);
    
    match encode_orchestrator_metrics(&mut writer) {
        Ok(()) => HttpResponseBuilder::ok()
            .header("Content-Type", "text/plain; version=0.0.4")
            .with_body_and_content_length(writer.into_inner())
            .build(),
        Err(err) => HttpResponseBuilder::server_error(format!("Failed to encode metrics: {}", err))
            .build(),
    }
}

fn encode_orchestrator_metrics(writer: &mut MetricsEncoder<Vec<u8>>) -> std::io::Result<()> {
    const WASM_PAGE_SIZE_IN_BYTES: f64 = 65536.0;
    
    // Basic canister metrics
    writer.encode_gauge(
        "stable_memory_bytes",
        ic_cdk::api::stable::stable_size() as f64 * WASM_PAGE_SIZE_IN_BYTES,
        "Size of the stable memory allocated by this canister.",
    )?;
    
    writer.encode_gauge(
        "heap_memory_bytes",
        heap_memory_size_bytes() as f64,
        "Size of the heap memory allocated by this canister.",
    )?;
    
    writer.gauge_vec("cycle_balance", "Cycle balance of this canister.")?
        .value(
            &[("canister", "orchestrator")],
            ic_cdk::api::canister_balance128() as f64,
        )?;
    
    // Orchestrator-specific metrics
    read_state(|state| {
        // Chain service metrics
        let total_chain_services = state.chain_services.len() as f64;
        let active_chain_services = state.chain_services.values()
            .filter(|info| matches!(info.status, ChainServiceStatus::Active))
            .count() as f64;
        let paused_chain_services = state.chain_services.values()
            .filter(|info| matches!(info.status, ChainServiceStatus::Paused))
            .count() as f64;
        let failed_chain_services = state.chain_services.values()
            .filter(|info| matches!(info.status, ChainServiceStatus::Failed))
            .count() as f64;
        
        writer.encode_gauge(
            "orchestrator_total_chain_services",
            total_chain_services,
            "Total number of chain services managed by the orchestrator.",
        )?;
        
        writer.encode_gauge(
            "orchestrator_active_chain_services",
            active_chain_services,
            "Number of active chain services.",
        )?;
        
        writer.encode_gauge(
            "orchestrator_paused_chain_services",
            paused_chain_services,
            "Number of paused chain services.",
        )?;
        
        writer.encode_gauge(
            "orchestrator_failed_chain_services",
            failed_chain_services,
            "Number of failed chain services.",
        )?;
        
        // User metrics
        writer.encode_gauge(
            "orchestrator_total_users",
            state.user_registry.len() as f64,
            "Total number of registered users.",
        )?;
        
        // WASM management metrics
        let has_wasm = if state.chain_service_wasm.is_some() { 1.0 } else { 0.0 };
        writer.encode_gauge(
            "orchestrator_has_chain_service_wasm",
            has_wasm,
            "Whether the orchestrator has a ChainService WASM module loaded.",
        )?;
        
        writer.encode_gauge(
            "orchestrator_previous_wasms_count",
            state.previous_wasms.len() as f64,
            "Number of previous WASM versions stored for rollback.",
        )?;
        
        // Chain service status by chain ID
        for (chain_id, info) in &state.chain_services {
            let status_value = match info.status {
                ChainServiceStatus::Active => 1.0,
                ChainServiceStatus::Paused => 2.0,
                ChainServiceStatus::Upgrading => 3.0,
                ChainServiceStatus::Failed => 4.0,
            };
            
            writer.gauge_vec("orchestrator_chain_service_status", "Status of each chain service (1=Active, 2=Paused, 3=Upgrading, 4=Failed).")?
                .value(
                    &[("chain_id", &chain_id.to_string()), ("chain_name", &info.chain_name)],
                    status_value,
                )?;
        }
        
        Ok::<(), std::io::Error>(())
    })?;
    
    Ok(())
}

/// Returns the amount of heap memory in bytes that has been allocated.
#[cfg(target_arch = "wasm32")]
pub fn heap_memory_size_bytes() -> usize {
    const WASM_PAGE_SIZE_BYTES: usize = 65536;
    core::arch::wasm32::memory_size(0) * WASM_PAGE_SIZE_BYTES
}

#[cfg(not(any(target_arch = "wasm32")))]
pub fn heap_memory_size_bytes() -> usize {
    0
}

pub fn get_orchestrator_metrics() -> OrchestratorMetrics {
    read_state(|state| {
        let total_chain_services = state.chain_services.len() as u32;
        let active_chain_services = state.chain_services.values()
            .filter(|info| matches!(info.status, ChainServiceStatus::Active))
            .count() as u32;
        let paused_chain_services = state.chain_services.values()
            .filter(|info| matches!(info.status, ChainServiceStatus::Paused))
            .count() as u32;
        let failed_chain_services = state.chain_services.values()
            .filter(|info| matches!(info.status, ChainServiceStatus::Failed))
            .count() as u32;
        
        OrchestratorMetrics {
            total_chain_services,
            active_chain_services,
            paused_chain_services,
            failed_chain_services,
            total_subscriptions: 0, // Would need to query all chain services
            total_users: state.user_registry.len() as u64,
            cycle_balance: ic_cdk::api::canister_balance128(),
            successful_upgrades: 0, // TODO: Track this in state
            failed_upgrades: 0,     // TODO: Track this in state
            current_wasm_version: state.version.clone(),
        }
    })
}
