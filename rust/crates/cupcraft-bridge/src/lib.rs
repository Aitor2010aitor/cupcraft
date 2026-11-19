//! cupcraft-bridge: Servidor HTTP con axum en 127.0.0.1 con token para agentes de IA.

pub mod server;

pub use server::{
    build_router, generate_token, start_server, ActionRequest, ActionResponse, BridgeState,
    DEFAULT_PORT,
};

#[cfg(test)]
mod tests;
