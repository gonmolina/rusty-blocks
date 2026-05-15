# Rusty-Blocks

Rusty-Blocks is a high-performance simulation engine and visual architect for dynamic systems. Built in Rust for speed and safety, it features a web-based GUI for intuitive model design and hierarchical subsystem management.

## Features
- **High-Performance Engine**: $O(N)$ complexity with flat-buffer architecture.
- **Precise Event Detection**: Deterministic sampling and time-event synchronization.
- **Hierarchical Modeling**: Support for nested subsystems and modular design.
- **Vectorized Signals**: Native support for multi-dimensional data flow.
- **Visual Editor**: React-based drag-and-drop interface with real-time charting.

---

## 🚀 Getting Started

### 1. Requirements
- **Rust** (Edition 2024)
- **Node.js** (v18+) & **npm**

### 2. Compilation and Installation

#### Backend & Engine
From the root directory:
```bash
cargo build --release
```

#### Frontend (UI)
```bash
cd ui
npm install
npm run build
```

### 3. Running the Project

#### Web Interface (Recommended)
1. Start the simulation server:
   ```bash
   cargo run --release --bin server
   ```
2. Start the UI development server:
   ```bash
   cd ui
   npm run dev
   ```
3. Open [http://localhost:5173](http://localhost:5173) in your browser.

#### Command Line Interface (CLI)
You can run simulations directly from the terminal using JSON files:
```bash
cargo run --release -- examples/motor_flexible.json examples/sim_rk45.json
```

---

## 🛠 Usage Guide

### Using the UI Editor
- **Add Blocks**: Drag blocks from the left sidebar onto the canvas.
- **Connect**: Drag from an output port (blue) to an input port (black).
- **Configure**: Click a block to edit its parameters (gains, initial conditions, names) in the right panel.
- **Rotate**: Use the "Rotar 90°" button in the properties panel to reorient blocks.
- **Subsystems**: 
  - Drag a "Subsystem" block.
  - **Double-click** to enter and edit its internal logic.
  - Use **InPort** and **OutPort** nodes inside to define the interface.
  - Use the breadcrumbs at the top to navigate back to the parent system.
- **Simulate**: Click the "Simular" button in the sidebar to run the model and view results in the chart.
- **Save/Load**: Use the buttons in the sidebar to export your project as a `.json` file.

### JSON Format
Models are defined in a structured JSON format. A system consists of:
- `name`: String.
- `blocks`: Array of objects with `id`, `type`, and `params`.
- `connections`: Array of objects with `from`, `from_port`, `to`, and `to_port`.

See `docs/JSON_SPEC.md` for the full technical specification.

---

## 🧱 Extending the Simulator

### Creating New Blocks
1. Add the block logic in a new or existing file in `src/blocks/`.
2. Implement the `Block` trait.
3. Define a `build` function for dynamic registration.
4. Register the new block in `src/blocks/mod.rs` inside the `BlockRegistry::std()` method.

### Subsystems and Hierarchies
Subsystems are blocks that contain a nested `SystemConfig`. They aggregate the states of all internal blocks and manage signal propagation recursively through `InPort` and `OutPort` blocks.

---

## 📈 Performance
Rusty-Blocks is optimized for large-scale simulations. Its Version 2.0 architecture utilizes pre-indexed connections and flattened memory buffers, achieving throughputs of up to **400 million block operations per second**.

Detailed performance reports can be found in `docs/EVENT_DETECTION.md`.

---

## 📜 License
MIT License - Developed by Gonzalo Molina.
