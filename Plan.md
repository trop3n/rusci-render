# Rusci-Render: osci-render Rewritten in Rust

## What osci-render Is

osci-render is a **real-time audio synthesizer that generates XY oscilloscope vector graphics**. It takes input files (SVG, OBJ, images, text, Lua scripts, audio) and converts them into audio signals where the left/right stereo channels drive the X/Y axes of an oscilloscope, drawing shapes on screen. It runs as a JUCE-based VST3/AU plugin or standalone app with 28+ audio/geometric effects, a polyphonic MIDI synthesizer, Lua scripting, OpenGL visualization, video recording, and Blender integration.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                     rusci-render                        │
├─────────┬──────────┬───────────┬──────────┬─────────────┤
│   GUI   │  Plugin  │   Audio   │ Parsers  │ Visualizer  │
│  (egui) │  (VST3)  │  Engine   │          │  (wgpu)     │
├─────────┴──────────┴───────────┴──────────┴─────────────┤
│                    Core Library                          │
│  (Point, Shape, Effect, EffectChain, Parameters)        │
├─────────────────────────────────────────────────────────┤
│              Platform / Runtime Layer                    │
│  (cpal audio, winit window, mlua, networking)           │
└─────────────────────────────────────────────────────────┘
```

---

## Crate / Dependency Selection

| Subsystem                    | Rust Crate(s)                     | Replaces (C++/JUCE)          |
| ---------------------------- | --------------------------------- | ----------------------------- |
| **Audio plugin (VST3/CLAP)** | `nih-plug`                        | JUCE AudioProcessor           |
| **GUI framework**            | `nih-plug-egui` + `egui`          | JUCE GUI components           |
| **GPU rendering**            | `wgpu` + `naga`                   | JUCE OpenGL                   |
| **Standalone audio I/O**     | `cpal` (via nih-plug)             | JUCE AudioDeviceManager       |
| **MIDI**                     | `nih-plug` built-in MIDI          | JUCE MidiBuffer               |
| **Lua scripting**            | `mlua` (LuaJIT backend)           | LuaJIT (C bindings)           |
| **OBJ parsing**              | `tobj`                            | tiny_obj_loader               |
| **SVG parsing**              | `resvg` / `usvg` + `lyon`        | JUCE Path + SvgParser         |
| **Image loading**            | `image` crate                     | JUCE ImageFileFormat          |
| **GIF decoding**             | `gif` crate                       | JUCE GIF support              |
| **Video decoding**           | `ffmpeg-next` (FFmpeg bindings)   | FFmpeg (CLI)                  |
| **Audio file decoding**      | `symphonia`                       | JUCE AudioFormatReader        |
| **Text rendering to paths**  | `rusttype` / `cosmic-text` + `lyon` | JUCE GlyphArrangement      |
| **Serialization (presets)**  | `serde` + `serde_json`            | JUCE XML ValueTree            |
| **WebSocket / networking**   | `tokio-tungstenite`               | ixwebsocket                   |
| **Video encoding**           | `ffmpeg-next`                     | FFmpeg (CLI)                  |
| **Shared texture (Spout/Syphon)** | Custom FFI bindings          | juce_sharedtexture            |
| **Math**                     | `glam` (SIMD vectors)             | Manual math                   |
| **Thread-safe queues**       | `crossbeam`                       | BlockingQueue (custom)        |
| **Chinese Postman solver**   | Port or wrap existing C++         | chinese_postman (private)     |
| **Font embedding**           | `include_bytes!` + `cosmic-text`  | JUCE BinaryData               |

---

## Workspace Structure

```
rusci-render/
├── Cargo.toml                    (workspace root)
├── crates/
│   ├── osci-core/                (zero-dependency core types & DSP)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── point.rs          (osci::Point → glam::Vec3)
│   │   │   ├── shape.rs          (line segments, length, interpolation)
│   │   │   ├── frame.rs          (Vec<Shape>, frame producer)
│   │   │   ├── effect.rs         (Effect trait, EffectParameter, LFO)
│   │   │   ├── effect_chain.rs   (ordered, toggleable effect pipeline)
│   │   │   ├── parameter.rs      (FloatParam, BoolParam, automation)
│   │   │   ├── envelope.rs       (ADSR with curves)
│   │   │   └── lfo.rs            (Sine, Saw, Triangle, Square, S&H)
│   │   └── Cargo.toml
│   │
│   ├── osci-effects/             (all 28 effect implementations)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── bitcrush.rs
│   │   │   ├── bulge.rs
│   │   │   ├── vector_cancelling.rs
│   │   │   ├── ripple.rs
│   │   │   ├── rotate.rs
│   │   │   ├── translate.rs
│   │   │   ├── scale.rs
│   │   │   ├── swirl.rs
│   │   │   ├── smooth.rs
│   │   │   ├── delay.rs
│   │   │   ├── dashed_line.rs    (+ trace variant)
│   │   │   ├── wobble.rs
│   │   │   ├── duplicator.rs
│   │   │   ├── multiplex.rs
│   │   │   ├── unfold.rs
│   │   │   ├── bounce.rs
│   │   │   ├── twist.rs
│   │   │   ├── skew.rs
│   │   │   ├── polygonizer.rs
│   │   │   ├── kaleidoscope.rs
│   │   │   ├── vortex.rs
│   │   │   ├── god_ray.rs
│   │   │   ├── spiral_bitcrush.rs
│   │   │   ├── perspective.rs
│   │   │   ├── custom_lua.rs     (Lua scripting effect)
│   │   │   └── registry.rs       (effect factory + metadata)
│   │   └── Cargo.toml
│   │
│   ├── osci-parsers/             (all file format parsers)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── file_parser.rs    (enum dispatch over formats)
│   │   │   ├── svg.rs            (usvg → lyon tessellation → shapes)
│   │   │   ├── obj.rs            (tobj → edge extraction → shapes)
│   │   │   ├── text.rs           (font → glyph outlines → shapes)
│   │   │   ├── lua.rs            (mlua script → per-sample points)
│   │   │   ├── image.rs          (threshold + stride → shapes)
│   │   │   ├── gif.rs            (animated frames)
│   │   │   ├── gpla.rs           (line art binary format)
│   │   │   ├── audio.rs          (symphonia → sample buffer)
│   │   │   ├── video.rs          (ffmpeg → frame extraction)
│   │   │   └── chinese_postman.rs(path optimization)
│   │   └── Cargo.toml
│   │
│   ├── osci-synth/               (synthesizer engine)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── synthesizer.rs    (voice allocator, poly engine)
│   │   │   ├── voice.rs          (ShapeVoice: per-voice rendering)
│   │   │   ├── sound.rs          (ShapeSound: frame queue)
│   │   │   ├── renderer.rs       (shape → point interpolation)
│   │   │   └── frame_producer.rs (async frame generation thread)
│   │   └── Cargo.toml
│   │
│   ├── osci-visualizer/          (GPU oscilloscope renderer)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── renderer.rs       (wgpu render pipeline)
│   │   │   ├── line_renderer.rs  (beam drawing with glow)
│   │   │   ├── bloom.rs          (multi-pass bloom/glow)
│   │   │   ├── persistence.rs    (phosphor decay simulation)
│   │   │   ├── overlays.rs       (graticule, CRT, vector display)
│   │   │   ├── settings.rs       (VisualiserSettings)
│   │   │   └── shaders/          (WGSL shader files)
│   │   │       ├── line.wgsl
│   │   │       ├── bloom.wgsl
│   │   │       ├── composite.wgsl
│   │   │       └── overlay.wgsl
│   │   └── Cargo.toml
│   │
│   ├── osci-gui/                 (egui-based UI components)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── editor.rs         (main plugin editor layout)
│   │   │   ├── effect_panel.rs   (effect list, drag-reorder)
│   │   │   ├── effect_widget.rs  (per-effect slider + LFO)
│   │   │   ├── file_controls.rs  (file navigation bar)
│   │   │   ├── frame_settings.rs (animation/image controls)
│   │   │   ├── midi_panel.rs     (MIDI keyboard + ADSR)
│   │   │   ├── lua_panel.rs      (26 sliders A-Z)
│   │   │   ├── code_editor.rs    (Lua/SVG text editor)
│   │   │   ├── visualizer_panel.rs(embedded wgpu viewport)
│   │   │   ├── visualizer_settings.rs
│   │   │   ├── recording_panel.rs
│   │   │   ├── timeline.rs       (audio/animation scrubber)
│   │   │   ├── menu_bar.rs       (File, Edit, Audio, View, Help)
│   │   │   ├── about.rs
│   │   │   └── theme.rs          (Dracula color scheme, Fira Sans)
│   │   └── Cargo.toml
│   │
│   ├── osci-plugin/              (nih-plug VST3/CLAP plugin)
│   │   ├── src/
│   │   │   ├── lib.rs            (Plugin trait impl, processBlock)
│   │   │   ├── params.rs         (all automatable parameters)
│   │   │   └── state.rs          (preset save/load via serde)
│   │   └── Cargo.toml
│   │
│   ├── osci-standalone/          (standalone application binary)
│   │   ├── src/
│   │   │   ├── main.rs           (winit + cpal standalone runner)
│   │   │   └── audio_io.rs       (device selection, routing)
│   │   └── Cargo.toml
│   │
│   └── osci-net/                 (networking: Blender, Spout/Syphon)
│       ├── src/
│       │   ├── lib.rs
│       │   ├── object_server.rs  (TCP socket for Blender line art)
│       │   ├── websocket.rs      (tokio-tungstenite server)
│       │   └── shared_texture.rs (Spout/Syphon FFI)
│       └── Cargo.toml
│
├── assets/
│   ├── fonts/                    (Fira Sans .ttf)
│   ├── overlays/                 (graticule, CRT textures)
│   └── examples/                 (bundled example files)
│
└── README.md
```

---

## Core Design Decisions

### 1. Point Representation

```rust
// osci-core/src/point.rs
use glam::Vec3;

pub type Point = Vec3;  // x, y, z — SIMD-accelerated via glam
```

Using `glam::Vec3` gives us SIMD-accelerated vector math (add, scale, dot, cross, rotate) for free, which is critical since every sample processes a 3D point through multiple effects.

### 2. Effect Trait System

```rust
// osci-core/src/effect.rs
pub trait EffectApplication: Send + Sync {
    /// Process a single sample in-place
    fn apply(&mut self, point: &mut Point, context: &EffectContext);

    /// Clone for per-voice instances
    fn box_clone(&self) -> Box<dyn EffectApplication>;

    /// Metadata
    fn name(&self) -> &str;
    fn parameters(&self) -> &[EffectParameter];
}

pub struct EffectContext<'a> {
    pub sample_rate: f32,
    pub frequency: f32,
    pub volume: f32,
    pub sample_index: u64,
    pub external_input: Option<&'a [f32; 2]>,
}

pub struct Effect {
    pub application: Box<dyn EffectApplication>,
    pub enabled: bool,
    pub precedence: i32,
    pub parameters: Vec<EffectParameter>,
}
```

### 3. Effect Parameter with LFO

```rust
// osci-core/src/parameter.rs
pub struct EffectParameter {
    pub id: String,
    pub name: String,
    pub value: AtomicF32,           // lock-free audio-thread reads
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub lfo: LfoState,
    pub smooth_speed: f32,
}

pub struct LfoState {
    pub lfo_type: LfoType,         // Static, Sine, Saw, Triangle, Square, S&H
    pub rate: f32,                  // Hz
    pub phase: f32,                 // internal accumulator
    pub start_percent: f32,
    pub end_percent: f32,
}
```

### 4. Shape & Frame

```rust
// osci-core/src/shape.rs
pub struct Shape {
    pub segments: Vec<[Point; 2]>,  // line start/end pairs
    pub length: f32,                // total path length
}

impl Shape {
    /// Interpolate a point along the shape at `progress` ∈ [0, 1]
    pub fn sample(&self, progress: f32) -> Point { ... }
}

pub struct Frame {
    pub shapes: Vec<Shape>,
    pub total_length: f32,
}
```

### 5. Synthesizer Voice

```rust
// osci-synth/src/voice.rs
pub struct ShapeVoice {
    pub note: u8,
    pub velocity: f32,
    pub phase: f64,                    // shape drawing progress
    pub envelope: AdsrEnvelope,
    pub effects: Vec<Effect>,          // cloned per-voice effect chain
    pub lua_state: Option<mlua::Lua>,  // per-voice Lua VM
}
```

### 6. Lock-Free Audio Thread Communication

```rust
// Use crossbeam for frame queue (parser → synth)
use crossbeam::channel::{bounded, Sender, Receiver};

// Frame producer sends parsed frames
let (frame_tx, frame_rx) = bounded::<Frame>(4);

// Parameter changes via atomic f32 (nih-plug provides AtomicF32)
// Effect chain mutations via a swap buffer or arc-swap pattern
```

### 7. Visualizer Pipeline (wgpu)

```
Samples → Line vertex buffer (uploaded each frame)
       → Pass 1: Draw lines with intensity/width → lineTexture
       → Pass 2-5: Gaussian blur cascade (4 levels) → bloomTextures
       → Pass 6: Composite line + bloom + persistence → outputTexture
       → Pass 7: Apply overlay (graticule/CRT) → final framebuffer
```

Each pass is a wgpu render pipeline with WGSL shaders. The persistence effect uses temporal blending between current and previous frame textures.

### 8. GUI Architecture (egui)

`nih-plug-egui` provides the bridge between the plugin and egui. The layout mirrors the original:

- **Left panel**: File controls, effect list (drag-reorderable), frame/animation settings, MIDI controls
- **Right panel**: Visualizer viewport (wgpu texture rendered into egui via `egui::TextureId`)
- **Bottom panel**: Code editor (using `egui_code_editor` or custom), Lua console
- **Menu bar**: File/Edit/Audio/View/Help

### 9. Preset / State Persistence

```rust
#[derive(Serialize, Deserialize)]
pub struct OsciState {
    pub files: Vec<FileEntry>,
    pub effects: Vec<EffectState>,
    pub midi: MidiSettings,
    pub animation: AnimationSettings,
    pub adsr: AdsrSettings,
    pub visualizer: VisualizerSettings,
    pub lua_sliders: [f32; 26],
}
```

nih-plug has built-in state persistence. We serialize to JSON via serde instead of JUCE's XML ValueTree.

---

## Complete Effect Reference

### Free Effects (13)

| #  | Effect             | Parameters                          | DSP Algorithm                                               |
| -- | ------------------ | ----------------------------------- | ----------------------------------------------------------- |
| 1  | BitCrush           | Dry/Wet, Strength                   | `dequant * round(input * quant)` with power-based depth     |
| 2  | Bulge              | Strength (0-1)                      | Radial warp: `scale = r^(1-value) / r`                      |
| 3  | VectorCancelling   | Frequency (0-1)                     | Periodic inversion every N samples                          |
| 4  | Ripple             | Depth, Phase, Amount                | `z += depth * sin(phase + 100*amount*(x²+y²))`              |
| 5  | Rotate             | X, Y, Z (-π to π)                  | 3D rotation matrix                                          |
| 6  | Translate          | X, Y, Z                            | Vector addition                                             |
| 7  | Swirl              | Strength                            | `rotate = 10 * strength * magnitude(input)`                 |
| 8  | Smooth             | Factor (0-1)                        | Low-pass EMA with log-scaled sample rate adaptation         |
| 9  | Delay              | Decay (0-1), Length (seconds)       | Circular buffer, 1.92M samples (10s @ 192kHz)               |
| 10 | DashedLine/Trace   | Count, Offset, Width                | Phase-based time-domain sampling with interpolation          |
| 11 | Wobble             | Amount, Phase                       | `delta = 0.5 * amount * sin(phase + nextPhase(freq, sr))`   |
| 12 | Duplicator         | Copies (1-6), Spread, Angle Offset  | Rotational duplication: `θ = floor(phase*n)/n * 2π + offset` |
| 13 | Scale              | X, Y, Z (-3 to 3)                  | Simple multiplication with lockable X/Y                     |

### Premium Effects (10)

| #  | Effect           | Parameters                                  | DSP Algorithm                                            |
| -- | ---------------- | ------------------------------------------- | -------------------------------------------------------- |
| 14 | Multiplex        | Grid X/Y/Z, Interpolation, Delay            | Grid tessellation with buffered delay sampling            |
| 15 | Unfold           | Segments (fractional), LFO                  | Polar coordinate angular compression/expansion            |
| 16 | Bounce           | Size (0.05-1), Speed, Angle                 | 2D physics sim with edge collision                        |
| 17 | Twist            | Strength                                    | Y-dependent rotation: `θ = 4π * strength * y`            |
| 18 | Skew             | Skew X/Y/Z                                  | Sequential shear transformations                          |
| 19 | Polygonizer      | Strength, Sides, Stripe Size, Rotation, Phase | Polar quantization to polygon/stripe grid               |
| 20 | Kaleidoscope     | Segments, Mirror, Spread, Clip              | Polar segment clipping with plane-normal mirroring        |
| 21 | Vortex           | Strength, Amount, Rotation                  | Complex exponentiation: `z^n` in polar form               |
| 22 | GodRay           | Strength, Position (bias)                   | Per-sample noise with directional bias                    |
| 23 | SpiralBitCrush   | Strength, Density, Twist, Zoom, Rotation    | Log-polar quantization with rotation/zoom                 |

### System Effects (5)

| #  | Effect      | Parameters              | Role                               |
| -- | ----------- | ----------------------- | ---------------------------------- |
| 24 | Perspective | Strength, FOV (5-130°)  | 3D camera projection (always on)   |
| 25 | Custom Lua  | 26 sliders (A-Z)        | User-defined per-sample scripting  |
| 26 | Volume      | Gain (0-3)              | Output gain scaling                |
| 27 | Threshold   | Level (0-1)             | Hard clipping limiter              |
| 28 | Frequency   | Hz (0-4200)             | Shape drawing rate                 |

---

## Supported File Formats

| Format              | Extension(s)              | Parser                     | Output Type     |
| ------------------- | ------------------------- | -------------------------- | --------------- |
| Wavefront OBJ       | `.obj`                    | `tobj` crate               | 3D edge shapes  |
| SVG                 | `.svg`                    | `usvg` + `lyon`            | 2D path shapes  |
| Plain text          | `.txt`                    | `cosmic-text` + `lyon`     | Glyph outlines  |
| Lua script          | `.lua`                    | `mlua` (LuaJIT)            | Per-sample XYZ  |
| GPLA line art       | `.gpla`                   | Custom binary parser        | Animated frames |
| GIF                 | `.gif`                    | `gif` crate                | Animated frames |
| Static image        | `.png`, `.jpg`, `.jpeg`   | `image` crate              | Threshold shapes |
| Audio               | `.wav`, `.aiff`, `.ogg`, `.flac`, `.mp3` | `symphonia` | Sample buffer   |
| Video (premium)     | `.mp4`, `.mov`            | `ffmpeg-next`              | Frame extraction |

---

## Audio DSP Pipeline

```
MIDI Input → MidiBuffer
             ↓
processBlock():
  1. Parse MIDI (note on/off, pitch wheel)
  2. Update ADSR if parameters changed
  3. Pre-animate all effects (LFO tick)
  4. Render audio (via synthesizer or input mode)
  5. Apply per-voice toggleable effects (precedence order)
  6. Apply global permanent effects
  7. Apply Lua effects
  8. Scale by volume & clip to threshold
  9. Output to channels + feed visualizer

Per-Voice Path (Synthesizer):
  ShapeVoice::render_next_block()
  1. Load frame from FileParser
  2. For each sample:
     - Calculate shape position via frequency
     - Generate Point from shape interpolation
     - Fill voice buffers
  3. Apply ADSR envelope
  4. Apply per-voice effects
  5. Mix into output buffer
```

---

## Visualizer Rendering Pipeline

```
Audio Samples (X, Y, [Z])
  → Upload to line vertex buffer
  → Pass 1: Draw lines with intensity/width       → lineTexture
  → Pass 2-5: Gaussian blur cascade (4 levels)    → bloomTextures[0..3]
  → Pass 6: Composite line + bloom + persistence   → outputTexture
  → Pass 7: Apply overlay (graticule/CRT/vector)   → final framebuffer
```

### Visualizer Settings

- **Line**: Hue (0-359), Intensity (0-10), Saturation (0-5)
- **Screen**: Hue, Saturation, Ambient Light (premium)
- **Light Effects**: Persistence (0-6s), Focus (0.3-10), Glow (0-1), Afterglow, Overexposure (premium)
- **Overlays**: Empty, Graticule, Smudged, Real Oscilloscope (premium), Vector Display (premium)
- **Modes**: XY, XYZ (Z=brightness), XYRGB (color)
- **Recording**: Resolution (128-2048), FPS (10-240), Codec (H264/H265/VP9/ProRes), Lossless toggle

---

## Implementation Phases

### Phase 1: Core Engine (Weeks 1-3)

- [ ] `osci-core`: Point, Shape, Frame, EffectParameter, LFO, ADSR envelope
- [ ] `osci-effects`: Port all 28 effects (pure math, no dependencies)
- [ ] `osci-synth`: Polyphonic voice allocator, shape-to-sample renderer
- [ ] Unit tests for every effect against reference output from the C++ version

### Phase 2: File Parsers (Weeks 3-5)

- [ ] `osci-parsers`: SVG (usvg+lyon), OBJ (tobj), text (cosmic-text+lyon), image/GIF, audio (symphonia), Lua (mlua)
- [ ] GPLA binary format parser
- [ ] Chinese Postman path optimizer (reimplement in Rust)
- [ ] Frame producer (background thread feeding frames to synth)

### Phase 3: Plugin Shell (Weeks 5-7)

- [ ] `osci-plugin`: nih-plug VST3/CLAP plugin with all parameters exposed
- [ ] processBlock: MIDI handling → voice rendering → effect chain → output
- [ ] Parameter automation and state save/load
- [ ] Verify in a DAW (Ableton, Reaper) that audio output matches the original

### Phase 4: Basic GUI (Weeks 7-10)

- [ ] `osci-gui`: egui-based editor
- [ ] File controls, effect list with drag reorder, sliders with LFO selectors
- [ ] MIDI keyboard widget, ADSR visual editor
- [ ] Frame/animation settings panel
- [ ] Lua slider panel (A-Z)
- [ ] Code editor panel for Lua/SVG editing

### Phase 5: Visualizer (Weeks 10-13)

- [ ] `osci-visualizer`: wgpu render pipeline
- [ ] Line drawing with variable intensity
- [ ] Multi-pass bloom/glow
- [ ] Phosphor persistence (temporal decay)
- [ ] Screen overlays (graticule, CRT, vector display)
- [ ] Embed wgpu output as egui texture
- [ ] Fullscreen and pop-out window support

### Phase 6: Standalone App (Weeks 13-14)

- [ ] `osci-standalone`: winit window + cpal audio device
- [ ] Audio device selection dialog
- [ ] MIDI device selection
- [ ] Menu bar with keyboard shortcuts (Ctrl+S, Ctrl+O, etc.)

### Phase 7: Networking & Premium Features (Weeks 14-16)

- [ ] `osci-net`: Blender TCP socket server (port 51677)
- [ ] WebSocket server for real-time model streaming
- [ ] Spout (Windows) / Syphon (macOS) shared texture via FFI
- [ ] Video recording via ffmpeg-next
- [ ] Video file (MP4/MOV) decoding
- [ ] Premium visualizer features (afterglow, overexposure, reflections, goniometer)

### Phase 8: Polish & Parity (Weeks 16-18)

- [ ] Preset compatibility (import .osci project files)
- [ ] Bundled example files
- [ ] Look-and-feel parity (Dracula theme, Fira Sans font)
- [ ] Performance profiling and SIMD optimization
- [ ] Cross-platform testing (Windows, macOS, Linux)
- [ ] CI/CD pipeline

---

## Key Technical Challenges & Mitigations

| Challenge                              | Mitigation                                                                                                                                              |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **egui lacks JUCE's rich audio widgets** | Build custom widgets (ADSR curve editor, drag-reorderable list, code editor with syntax highlighting) using egui's painter API                          |
| **wgpu in plugin context**             | nih-plug-egui supports custom rendering; use `egui::PaintCallback` to inject wgpu commands                                                              |
| **Per-voice Lua VMs**                  | `mlua` supports creating multiple Lua states; use `Lua::new()` per voice with shared bytecode                                                           |
| **Lock-free audio thread**             | nih-plug enforces `Send + Sync` on process; use `Arc<AtomicF32>` for parameters, `crossbeam` channels for frames                                       |
| **Chinese Postman (private C++ lib)**  | Reimplement in Rust (minimum-weight perfect matching on odd-degree vertices, then Euler tour)                                                            |
| **Spout/Syphon (platform C libs)**     | Thin FFI wrappers behind a `SharedTexture` trait with platform-specific implementations                                                                  |
| **Matching exact DSP behavior**        | Port effects line-by-line from C++ with identical math; use golden-file tests comparing output sample buffers                                            |

---

## Performance Advantages of Rust

- **SIMD via glam**: All point math (rotate, scale, translate) uses SSE2/AVX automatically
- **Zero-cost abstractions**: Effect trait dispatch can be monomorphized where the compiler sees fit
- **No GC pauses**: Deterministic memory in the audio thread (critical for real-time)
- **Fearless concurrency**: Compiler-enforced thread safety between audio thread and UI thread
- **wgpu vs OpenGL**: Modern GPU API with better Vulkan/Metal/DX12 support and validation

---

## Execution Strategy

### What Claude Code handles well

- Scaffold the entire workspace and crate structure
- Implement all 28 effects — pure math, well-defined from the C++ source, ported line-by-line
- Write the core engine: Shape, Frame, Point, EffectChain, ADSR, LFO, parameter system
- Write all file parsers (SVG, OBJ, text, image, GIF, GPLA, audio, Lua)
- Build the nih-plug VST3/CLAP plugin with full parameter automation
- Write the polyphonic synthesizer and voice allocator
- Build the egui UI — effect panels, drag-reorder lists, sliders, code editor, MIDI keyboard
- Write the wgpu render pipeline and WGSL shaders
- Implement the networking layer, state serialization, standalone app

### What requires human-in-the-loop testing

- **Listening tests**: Verifying effects sound right and drawing frequency maps correctly
- **Visual verification**: Confirming wgpu bloom, persistence, and overlays look correct
- **DAW testing**: Loading VST3 in Reaper/Ableton, confirming parameter automation, MIDI, and presets
- **Platform quirks**: Spout/Syphon FFI, audio device enumeration edge cases, OS-specific build issues

### Workflow

Write a phase → build and test → report issues → fix → repeat. Phase by phase, iteratively, until full feature parity is reached.

---

## Relationship to rusci (sosci Clone)

### What sosci Actually Is

sosci is **not a separate application** — it's a stripped-down build target of the same osci-render codebase. The repo contains two Projucer files:

- `osci-render.jucer` — full application (synthesizer + effects + parsers + visualizer)
- `sosci.jucer` — audio plugin only (audio input → visualizer)

sosci = the osci-render visualizer, packaged as a VST3/AU plugin with no file parsers, no Lua scripting, no shape synthesis. It takes audio in, renders it on a GPU-simulated oscilloscope, and outputs the visual.

The Rust clone of sosci is called **rusci**, planned in a separate directory at `/projects/rusci/`.

### Rendering Heritage

The core rendering technique traces through three projects:

1. **[woscope](https://github.com/m1el/woscope)** (MIT) — m1el's WebGL oscilloscope POC with analytical Gaussian beam rendering
2. **[dood.al/oscilloscope](https://dood.al/oscilloscope/)** — Neil Thapen's expanded web app adapting woscope's line rendering
3. **osci-render / sosci** — James Ball's JUCE/OpenGL rewrite with full production features

The mathematical foundation is [publicly documented](https://m1el.github.io/woscope-how/): each line segment's brightness is computed as the analytical integral of a Gaussian using the error function (`erf`), giving physically-accurate electron beam appearance with mathematically perfect joint intensity.

### Shared Code with rusci-render

rusci reuses the `osci-visualizer` crate from this workspace directly. If rusci-render Phase 5 is complete, building rusci is ~3-4 days of work — just a thin nih-plug wrapper with a simplified egui UI.

### sosci Feature Inventory

**What sosci has:**
- Stereo audio input (L/R → X/Y) + optional Z channel (brightness)
- GPU oscilloscope visualizer with Gaussian beam rendering
- Multi-pass bloom/glow pipeline
- Phosphor persistence and afterglow
- Tone mapping with overexposure
- Screen overlays (graticule, smudged, real oscilloscope, vector display)
- Render modes: XY, XYZ, XYRGB
- Goniometer mode (45° rotation)
- Shutter sync
- Flip vertical/horizontal, scale, offset
- Video recording (H264, H265, VP9, ProRes)
- Syphon/Spout shared texture output
- Vintage oscilloscope presets
- VST3, AU, and standalone builds

**What sosci does NOT have (vs osci-render):**
- No file parsers (SVG, OBJ, text, image, Lua, GPLA)
- No polyphonic synthesizer / voice system
- No 28 audio/geometric effects
- No effect chain / drag-reorder UI
- No MIDI keyboard / ADSR editor
- No code editor / Lua console
- No Blender integration / WebSocket server
- No frame animation system

### GPU Rendering Pipeline (Shared)

The complete shader pipeline that powers both osci-render's visualizer and sosci:

#### Line Rendering (Gaussian Beam)
- **Vertex shader**: Creates 4-vertex quads per line segment with perpendicular offset
- **Fragment shader**: Analytical Gaussian integral via `erf()` approximation
  - Short segments: `gaussian(distance, σ)` where `σ = focus/5.0`
  - Long segments: `[erf(x₁/√2σ) - erf(x₂/√2σ)] × exp(-y²/2σ²) / 2L`
  - `erf` approximation: `1 + (0.278393 + (0.230389 + 0.078108·a²)·a)·a`

#### Blur Pipeline
- **Tight blur**: 17-tap Gaussian (512×512) — immediate glow around beam
- **Wide blur**: 65-tap Gaussian (128×128) — diffuse phosphor scatter

#### Persistence System (Two-Layer)
1. Per-frame exponential fade: `fadeAmount = pow(0.5, persistence) × 0.4 × (60/fps)`
2. Afterglow (premium): hyperbolic tangent curve for non-linear phosphor decay

#### Output Composition
- Bloom: `glow × (tightGlow + scatter × scatterScalar)`
- Tone mapping: `1.0 - exp(-exposure × light)`
- Overexposure: white clipping via `mix(color, white, pow(brightness³) × overexposure)`
- Ambient light tinting, procedural noise, saturation control

#### Texture Pipeline
```
lineTexture (1024×1024)  →  afterglow fade
    → resize to 512×512  →  H blur → V blur → blur1 (tight glow)
    → resize to 128×128  →  wide H blur → wide V blur → blur3 (scatter)
    → [optional glow shader → glowTexture (reflections)]
    → output composition with screen overlay → final framebuffer
```

### Feasibility Assessment for rusci

| Component | Difficulty | Notes |
|---|---|---|
| nih-plug audio passthrough | Easy | Receive buffers, no synthesis |
| wgpu line renderer (Gaussian beam) | Medium | Port GLSL → WGSL, math is documented |
| Bloom/blur pipeline | Easy-Medium | Standard multi-pass Gaussian blur |
| Persistence/afterglow | Easy | Frame blending with exponential decay |
| Tone mapping & composition | Easy | Single fragment shader |
| egui settings panel | Easy | Sliders and dropdowns only |
| Screen overlays | Easy | Procedural graticule + texture loading |
| Video recording (ffmpeg) | Medium | GPU readback + ffmpeg pipe |
| Spout/Syphon | Hard | Platform-specific FFI, no Rust crates |
| wgpu inside nih-plug-egui | Hard | Trickiest integration point |

**Confidence**: ~90% for the rendering math, ~70% for the plugin integration, ~50% for Spout/Syphon FFI.

**Recommendation**: Build rusci-render through Phase 5 first. The `osci-visualizer` crate IS rusci's core — sosci falls out as a second build target in the same workspace, exactly mirroring the original project's structure.

### References

- [woscope — WebGL oscilloscope (MIT)](https://github.com/m1el/woscope)
- [How to draw oscilloscope lines with math and WebGL](https://m1el.github.io/woscope-how/)
- [sosci official page](https://osci-render.com/sosci/)
- [KVR forum thread](https://www.kvraudio.com/forum/viewtopic.php?t=617556)
- [Gearspace announcement](https://gearspace.com/board/new-product-alert-2-older-threads/1443100-introducing-sosci-super-realistic-software-oscilloscope-plugin.html)
