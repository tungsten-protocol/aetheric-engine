//=========================================================================
// Platform Bridge
//=========================================================================
//
// Bridges platform layer (winit/SDL/etc.) with core systems.
//
// This module defines the contract between platform implementations and
// core logic, enabling platform backends to be swapped without changing
// core code (Dependency Inversion Principle).
//
// Components:
// - `interface`: Event types and error definitions (the contract)
// - `event_collector`: Core-side event collection and buffering
//
//=========================================================================

//=== Module Declarations =================================================

pub(crate) mod event_collector;
pub(crate) mod interface;

//=== Internal API ========================================================

pub(crate) use event_collector::{EventCollector, TickControl};
pub(crate) use interface::{PlatformError, PlatformEvent};