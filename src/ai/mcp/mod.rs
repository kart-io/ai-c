//! Model Context Protocol (MCP) Implementation
//!
//! This module provides a complete implementation of the Model Context Protocol (MCP)
//! for communication with AI services and external tools.
//!
//! Key components:
//! - Protocol core with JSON-RPC 2.0 message handling
//! - Multiple transport layers (WebSocket, HTTP, stdio)
//! - Resource and tool management
//! - Client and server implementations

pub mod protocol;
pub mod transport;
pub mod resources;
pub mod tools;
pub mod client;
pub mod server;
pub mod errors;

pub use protocol::{
    MCPMessage, MCPRequest, MCPResponse, MCPNotification, MCPError,
    ProtocolVersion, MessageId, MethodName,
};
pub use transport::{
    MCPTransport, TransportType, WebSocketTransport, HttpTransport, StdioTransport,
    TransportConfig, ConnectionStatus, TransportStats
};
pub use resources::{
    ResourceManager, Resource, ResourceType, ResourceUri, ResourceContent,
    ResourceRegistry, ResourcePermissions
};
pub use tools::{
    ToolManager, Tool, ToolCall, ToolResult, ToolParameter, ToolRegistry,
    ToolExecutor, ToolPermissions
};
pub use client::{MCPClient, MCPClientConfig, ClientStatus};
pub use server::{MCPServer, MCPServerConfig, ServerStatus};
pub use errors::{MCPError as Error, MCPResult as Result};