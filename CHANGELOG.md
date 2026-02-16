# Changelog

All notable changes to KoruDelta will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [3.0.0] - 2026-02-16

### Overview
Complete LCA (Local Causal Agent) Architecture implementation with full WASM platform parity and production-ready cross-platform support.

### Added

#### LCA Architecture v3.0.0
- **20 Specialized Agents** - Complete agent ecosystem with causal perspectives:
  - Storage, Temperature, Chronicle, Archive, Essence, Sleep, Evolution
  - Lineage, Perspective, Identity, Network, Pulse, Workspace, Vector
  - Lifecycle, Session, Subscription, Process, Reconciliation, Orchestrator
- **Native: 19 actions** - Full agent ecosystem (2 gated: Lifecycle, Subscription)
- **WASM: 17 actions** - Full feature parity minus native-only modules
- **Unified Field Pattern** - All agents share a single DistinctionEngine instance
- **Causal Synthesis** - Every operation: `ΔNew = ΔLocal_Root ⊕ ΔAction_Data`
- **Canonical Root Types** - 20 canonical roots in `src/roots.rs`

#### Full WASM Platform Parity
- **Node.js Support** - Complete API via `wasm-pack build --target nodejs`
- **Browser Support** - Web-ready via `wasm-pack build --target web`
- **17/17 WASM Actions** - Full feature parity for browser/Node.js:
  - Core: put, get, delete, history, getAt, query
  - Workspaces: create, list, get, use namespace
  - Vector: putSimilar, findSimilar, embed, embedSearch
- **IndexedDB Integration** - Browser persistence with auto-save/load
- **WorkspaceHandle** - Proper workspace isolation in WASM

#### Cross-Platform API Consistency
- **Rust (Native)** - Full 20-agent LCA architecture with networking
- **Python** - 19/19 features + cluster support via PyO3 bindings  
- **JavaScript/WASM** - 17/17 actions via wasm-bindgen (all WASM-available features)
- **CLI** - 7 commands + cluster mode: get, set, delete, log, diff, view, query, start, peers

#### Distributed Cluster (Production Ready)
- **Live Replication** - Fixed broadcast_write to use actual bound addresses (not port 0)
- **Multi-Node Support** - Full TCP-based clustering for Rust and Python
- **Gossip Protocol** - Automatic peer discovery and cluster membership
- **Causal Consistency** - Vector clock-based conflict resolution
- **WASM Note** - Browser/Node.js WASM is single-node (WebSocket cluster planned for future)

#### Production Hardening
- **Zero Warnings** - Clean build across all platforms
- **463 Tests Passing** - Comprehensive test coverage
- **Feature Gating** - Proper `#[cfg(not(target_arch = "wasm32"))]` gates
  - SubscriptionAction, LifecycleAction (native only)
  - std::time::Instant usage
  - Synchronous identity mining

### Changed

#### API Improvements
- **Workspace Namespacing** - `workspaces.use_await(