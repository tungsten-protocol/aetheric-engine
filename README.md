# Aetheric Engine

A multithreaded game engine written in Rust, designed for deterministic simulation and clean separation between platform-specific code and core game logic.

## Features

- **Dual-thread architecture**: Platform thread for windowing/input, core thread for deterministic game logic at fixed TPS
- **Message-passing coordination**: Thread-safe communication via crossbeam channels and MessageBus
- **Type-safe message bus**: Multi-consumer message queue for inter-system communication
- **Generic action mapping**: High-level input system with customizable action bindings and input contexts
- **Stack-based scene management**: MessageBus-based pattern for deterministic scene transitions
- **Platform abstraction**: Clean separation between OS-specific (Winit) and core game logic
- **Type-safe APIs**: Scene and action systems fully generic over user-defined types

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         AETHERIC ENGINE ARCHITECTURE                    │
└─────────────────────────────────────────────────────────────────────────┘

    PLATFORM THREAD (main)                 CORE THREAD (fixed TPS)  
    ══════════════════════              ═════════════════════════════

    ┌──────────────┐
    │ Winit Event  │
    │     Loop     │
    └──────┬───────┘
           │ OS Events
           ▼
    ┌──────────────┐
    │    Input     │
    │  Processor   │
    └──────┬───────┘
           │ InputEvents
           ▼
    ┌──────────────┐
    │    Input     │
    │    Buffer    │
    └──────┬───────┘
           │ Frame Batch
           ▼
    [ MPSC Channel ]═════════════════════════════════╗
      (crossbeam)                                    ║
                                                     ║
                                        ┌─────────── ║ ────────┐
                                        │            ▼         │                                        
                                        │   EventCollector     │
                                        │  (batch receiver)    │
                                        └────────────┬─────────┘
                                                     │
                                        ╔═══════════ │ ════════════╗
                                        ║            ▼             ║
                                        ║    GlobalContext         ║
                                        ║  ┌────────────────────┐  ║
                                        ║  │   StateTracker     │  ║
                                        ║  │  (keys, mouse)     │  ║
                                        ║  └────────────────────┘  ║
                                        ║  ┌────────────────────┐  ║
                                        ║  │    MessageBus      │  ║
                                        ║  │ (type-safe queue)  │  ║
                                        ║  └────────────────────┘  ║
                                        ╚════════════╤═════════════╝
                                                     │
                                        ╔═══════════ │ ════════════╗
                                        ║            ▼             ║
                                        ║   GlobalSystems<S,A>     ║
                                        ║  ┌────────────────────┐  ║
                                        ║  │   InputSystem<A>   │────┐
                                        ║  │ (action mapping)   │  ║ │
                                        ║  └────────────────────┘  ║ │
                                        ║  ┌────────────────────┐  ║ │ Publish
                                        ║  │  SceneManager<S>   │  ║ │ Actions
                                        ║  │ (stack lifecycle)  │  ║ │
                                        ║  └────────────────────┘  ║ │
                                        ╚════════════╤═════════════╝ │
                                                     │               │
                                                     │               ▼
                                            ┌─────── ▼ ───────────────────┐
                                            │      MessageBus Queue       │
                                            │  • Actions (A)              │
                                            │  • SceneTransition<S>       │
                                            │  • Custom Messages          │
                                            └────────┬────────────────────┘
                                                     │
                                                     │ Multi-Consumer
                                                     ▼
                                            ┌─────────────────┐
                                            │  Active Scenes  │
                                            │  (read & push)  │
                                            └─────────────────┘

KEY:
  ═══  High-level container
  ───  Component boundary
  ─ ►  Data flow
  ═ ►  Cross-thread message passing
```

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
aetheric_engine = { git = "https://github.com/tungsten-protocol/aetheric-engine" }
```

Define your scene and action types:

```rust
use aetheric_engine::prelude::*;

// Define scene keys
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum MyScene {
    MainMenu,
    Gameplay,
}
impl SceneKey for MyScene {}

// Define game actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum GameAction {
    Jump,
    Shoot,
    Pause,
}
impl Action for GameAction {}

// Implement scene behavior
struct GameplayScene;

impl Scene<MyScene> for GameplayScene {
    fn update(&mut self, context: &GlobalContext) {
        // Access raw input state
        if context.input_state.is_key_pressed(KeyCode::Space) {
            println!("Jump!");
        }

        // Read actions from MessageBus
        for action in context.message_bus.read::<GameAction>() {
            match action {
                GameAction::Jump => println!("Jump action!"),
                GameAction::Shoot => println!("Shoot action!"),
                GameAction::Pause => {
                    // Queue scene transition via MessageBus
                    context.message_bus.push(
                        SceneTransition::Push(MyScene::MainMenu)
                    );
                }
            }
        }

        // Or queue transitions directly
        if context.input_state.is_key_pressed(KeyCode::Escape) {
            context.message_bus.push(
                SceneTransition::Push(MyScene::MainMenu)
            );
        }
    }

    fn on_enter(&mut self, _context: &GlobalContext) {
        println!("Gameplay scene started");
    }

    fn on_exit(&mut self, _context: &GlobalContext) {
        println!("Gameplay scene ended");
    }
}

fn main() {
    EngineBuilder::<MyScene, GameAction>::new()
        .build()
        .init(|systems| {
            // Configure input bindings
            systems.input.bind_key(
                KeyCode::Space,
                GameAction::Jump,
                InputContext::Primary
            );

            // Register scenes
            systems.scene_manager.register_scene(
                MyScene::Gameplay,
                Box::new(GameplayScene)
            );
        })
        .run();
}
```

## Input System

The engine provides three tiers of input queries:

### 1. Actions (high-level, via `MessageBus`)
```rust
// In scene update - read actions published by InputSystem
for action in context.message_bus.read::<GameAction>() {
    match action {
        GameAction::Jump => player.jump(),
        GameAction::Shoot => player.shoot(),
    }
}
```

### 2. Raw State (mid-level, via `GlobalContext.input_state`)
```rust
if context.input_state.is_key_down(KeyCode::KeyW) {
    player.move_forward();
}
if context.input_state.is_key_pressed(KeyCode::Space) {
    player.jump(); // Only on first frame
}
```

### 3. Mouse Queries (low-level, via `GlobalContext.input_state`)
```rust
let (x, y) = context.input_state.mouse_position();
let (dx, dy) = context.input_state.mouse_delta();
if context.input_state.is_mouse_button_down(MouseButton::Left) {
    player.shoot();
}
```

**Note**: Actions are published to MessageBus each frame by InputSystem, allowing multiple scenes and systems to read them.

## Scene Management

Stack-based scenes with MessageBus-based transitions:

```rust
// Push new scene on top
context.message_bus.push(SceneTransition::Push(MyScene::Pause));

// Remove specific scene
context.message_bus.push(SceneTransition::Remove(MyScene::Pause));

// Replace one scene with another
context.message_bus.push(
    SceneTransition::Replace(MyScene::MainMenu, MyScene::Gameplay)
);

// Clear all scenes
context.message_bus.push(SceneTransition::Clear);
```

Transitions are queued to the MessageBus during scene updates and processed at tick boundaries for deterministic behavior.

**Simplified API**: Scene trait is now `Scene<S>` (no longer generic over Action type). Scenes receive non-generic `&GlobalContext`.

## Configuration

Customize engine settings via builder pattern:

```rust
EngineBuilder::<MyScene, GameAction>::new()
    .with_tps(120.0)                // Fixed simulation rate (default: 60)
    .with_channel_capacity(256)     // Event channel size (default: 128)
    .build()
    .init(|systems| { /* ... */ })
    .run();
```

## Dependencies

- `winit` 0.30 - Cross-platform windowing
- `crossbeam-channel` 0.5 - MPSC thread communication
- `log` - Logging infrastructure

## Development Status

- ✅ Multithreaded engine architecture
- ✅ Input action mapping and event pipeline
- ✅ Platform bridge and event collection
- ✅ State tracker for keys/mouse with delta support
- ✅ Stack-based scene management with MessageBus transitions
- ✅ Type-safe MessageBus for inter-system communication
- ✅ Simplified Scene trait 
- ✅ Prelude module for convenient imports
- ⏳ Graphics rendering

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
