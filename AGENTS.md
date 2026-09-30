# AGENTS.md — rusci-render Project Context

**Read this at the start of every session.** This file consolidates project spec, architecture, and implementation status.

---

## Project Overview

**rusci-render** is a from-scratch Rust rewrite of [osci-render](https://github.com/jameshball/osci-render), a JUCE/C++ oscilloscope music synthesizer by James Ball. It takes input files (SVG, OBJ, 3D models, images, text, Lua scripts, audio) and converts them into audio where left/right stereo channels drive the X/Y axes of an oscilloscope, drawing vector graphics in real time.

**Applications in this workspace:**
- **rusci-render** (osci-render clone) — VST3/CLAP plugin + standalone synthesizer
- **rusci** (sosci clone) — Audio-input-only oscilloscope visualizer (no synthesis)

**Reference implementations:**
- osci-render source: `~/dev/projects/osci-render/`
- sosci: https://github.com/jameshball/sosci
- UI screenshot: `osci-render.png` (1891×979, shows target layout)

---

## Build Commands

```bash
# Standalone synthesizer app (primary test target)
cargo run --bin rusci-render

# Release build (required for real audio — debug is too slow)
cargo run --bin rusci-render --release

# Audio visualizer only (no synthesis)
cargo run --bin rusci

# Run all workspace tests
cargo test --workspace

# Build VST3/CLAP plugin DLLs
cargo build --package osci-plugin --release
cargo build --package rusci --release
```

**Linux system dependencies:**
```
libgl-dev libx11-dev libxcursor-dev libxrandr-dev libxinerama-dev
libxi-dev libxkbcommon-dev libx11-xcb-dev
```

---

## Workspace Crate Map

```
crates/
  osci-core/        Zero-dependency core types: Point, Shape, Frame,
                    EffectApplication trait, EffectParameter (LFO/smoothing/
                    sidechain), Env (ADSR with curve), LfoType, LfoState

  osci-effects/     27 effect implementations + registry factory
                    Registry: build_registry(), find_effect(&id)
                    Effects: Bitcrush, Bulge, VectorCancelling, Ripple, Rotate,
                    Translate, Scale, Swirl, Smooth, Delay, DashedLine, Wobble,
                    Duplicator, Multiplex, Unfold, Bounce, Twist, Skew,
                    Polygonizer, Kaleidoscope, Vortex, GodRay, SpiralBitcrush,
                    Perspective, Volume, Threshold, Frequency

  osci-parsers/     File format parsers — all unit tested, all working:
                    SVG (usvg+lyon), OBJ (tobj), text (cosmic-text+lyon),
                    image (threshold scan), GIF (animated), GPLA (binary line art),
                    audio (symphonia: wav/aiff/flac/ogg/mp3), Lua (mlua/Lua 5.4)
                    Entry point: parse_file(path) → Vec<Box<dyn Shape>> or AnimatedFrames

  osci-synth/       Polyphonic synthesizer engine:
                    Synthesizer — 16-voice allocator, MIDI routing, voice stealing
                    ShapeVoice — per-voice shape renderer + ADSR + effect chain
                    ShapeRenderer — walks shapes at frequency, produces Points
                    ShapeSound — crossbeam bounded channel (cap 4) for frame queue
                    VoiceEffect — per-voice effect instance with animation state

  osci-visualizer/  GPU oscilloscope renderer — uses glow (OpenGL 3.3), NOT wgpu
                    OsciRenderer — main render entry point
                    LineRenderer — Gaussian beam (erf-based) line drawing
                    Bloom — tight (17-tap 512px) + wide (65-tap 128px) blur
                    Persistence — temporal frame blending, phosphor decay
                    Compositor — combines line + bloom + persistence → output
                    VisualiserSettings — all display parameters with defaults

  osci-gui/         egui editor UI (uses nih_plug_egui::egui, NOT egui directly)
                    lib.rs — draw_editor() top-level layout
                    state.rs — UiCommand enum, EffectSnapshot, VisBuffer,
                               EditorSharedState, AudioInfo
                    effect_panel.rs — full effect chain UI
                    file_controls.rs — file path input + load button
                    scope.rs — GPU scope widget (glow paint callback)
                    menu_bar.rs — File menu
                    dialogs.rs — About, Audio Info, Keyboard Shortcuts
                    project.rs — ProjectFile, save/load JSON
                    theme.rs — Dracula theme + Fira Code font

  osci-plugin/      nih-plug VST3/CLAP plugin
                    OsciPlugin — Plugin trait impl, process(), editor()
                    OsciParams — FloatParam: volume, frequency, attack, decay,
                                 sustain, release
                    crate-type = ["cdylib", "rlib"] ← do not change

  osci-net/         WebSocket server — receives frames from network clients
                    (Blender integration etc.), feeds them into ShapeSound

  osci-standalone/  Binary: rusci-render
                    main.rs: nih_export_standalone::<OsciPlugin>()

  rusci/            Audio visualizer plugin (no synthesis, mirrors sosci)
                    crate-type = ["cdylib", "lib"]

  rusci-standalone/ Binary: rusci
                    main.rs: nih_export_standalone::<RusciPlugin>()
```

---

## Implementation Status

### COMPLETE ✓

| Component | Status | Notes |
|-----------|--------|-------|
| DSP core | ✓ | All 27 effects, shape rendering, ADSR, LFO — unit tested |
| Parsers | ✓ | All formats parse correctly — SVG, OBJ, text, image, GIF, GPLA, audio, Lua |
| Synthesizer | ✓ | 16-voice poly engine, voice stealing, MIDI routing |
| Effect chain UI | ✓ | Add/remove/reorder/enable effects; per-param sliders, LFO, smoothing, sidechain |
| GPU visualizer | ✓ | Gaussian beam, bloom, persistence, tone mapping — glow/OpenGL |
| Project save/load | ✓ | JSON round-trips effect chain + visualizer settings + synth params |
| Menu bar | ✓ | File → New/Open/Save/Save As with Ctrl+N/O/S |
| Plugin shell | ✓ | Builds as VST3/CLAP, loads in nih-plug standalone runner |
| Drone mode | ✓ | Continuous playback without MIDI — checkbox in synth controls |
| File loading UI | ✓ | Text input + Load button; sends frames to audio thread |

### REMAINING ISSUES

**1. Visualizer settings not accessible**
- `VisualiserSettings` can only change via project file load. No UI sliders.
- Fix: `osci-gui/src/visualizer_settings.rs`

**2. No Lua slider panel**
- `slider_a` through `slider_z` always 0.0. No UI to set them.
- Fix: `osci-gui/src/lua_panel.rs` — 26 sliders

**3. Visualizer is 300px widget, not main view**
- Scope at bottom of scroll area. Should be dominant panel.
- Fix: Restructure to `SidePanel::left` + `CentralPanel`

### NOT STARTED

- Code editor (Lua/SVG text editing in-app)
- ADSR curve visual editor
- Recording panel
- DAW plugin testing
- Chinese Postman path optimizer
- Video file decoding
- Spout/Syphon shared texture output
- Preset compatibility with original .osci files

---

## Locked Architectural Decisions

| Decision | Detail |
|---|---|
| **Visualizer uses glow, not wgpu** | Plan.md says wgpu — ignore that. Implementation uses `glow` via `egui_glow::CallbackFn`. |
| **mlua uses lua54 + vendored** | LuaJIT removed for Windows compatibility. Lua 5.4 compiles from C source. |
| **osci-plugin crate-type** | Must be `["cdylib", "rlib"]`. cdylib = DAW loads it. rlib = osci-standalone imports OsciPlugin. |
| **No egui direct dep in osci-gui** | Use `nih_plug_egui::egui` everywhere. Direct `egui` dep risks version conflicts. |
| **UiCommand channel for effect params** | Effect params use `UiCommand` crossbeam channel. Synth params use nih-plug `FloatParam` for DAW automation. |
| **VisBuffer polling pattern** | Audio thread writes `Arc<Mutex<VisBuffer>>` each block. UI clones it each frame. No callback. |

---

## UI ↔ Audio Thread Communication

```
UI thread                          Audio thread (process())
─────────────────────────────────────────────────────────
UiCommand (crossbeam bounded 256) ──→ drained at top of each process() block
                                        handles: AddEffect, RemoveEffect, MoveEffect,
                                        SetEffectEnabled, SetParamValue, SetLfo,
                                        SetSmoothing, SetSidechain, LoadProject,
                                        ClearProject, StartRecording, StopRecording,
                                        SetDroneEnabled, LoadFile

Arc<Mutex<Vec<EffectSnapshot>>>   ←── written by audio thread after any effect change
Arc<Mutex<VisBuffer>>             ←── written by audio thread every block (last 512 samples)
Arc<Mutex<AudioInfo>>             ←── written once in initialize()
Arc<Mutex<Option<PathBuf>>>       ←── current project path, written by UI on save/open
```
UI thread                          Audio thread (process())
─────────────────────────────────────────────────────────
UiCommand (crossbeam bounded 256) ──→ drained at top of each process() block
                                        handles: AddEffect, RemoveEffect, MoveEffect,
                                        SetEffectEnabled, SetParamValue, SetLfo,
                                        SetSmoothing, SetSidechain, LoadProject,
                                        ClearProject, StartRecording, StopRecording

Arc<Mutex<Vec<EffectSnapshot>>>   ←── written by audio thread after any effect change
Arc<Mutex<VisBuffer>>             ←── written by audio thread every block (last 512 samples)
Arc<Mutex<AudioInfo>>             ←── written once in initialize()
Arc<Mutex<Option<PathBuf>>>       ←── current project path, written by UI on save/open
```

---

## Target UI Layout (from osci-render.png)

```
┌─────────────────────────────────────────────────────────────┐
│ Menu bar (File, Edit, Audio, View, Help)                    │
├─────────────────┬───────────────────────────────────────────┤
│ Left Panel      │ Central Panel                             │
│ ┌─────────────┐ │ ┌───────────────────────────────────────┐ │
│ │ File        │ │ │                                       │ │
│ │ Controls    │ │ │      GPU Oscilloscope Visualizer      │ │
│ └─────────────┘ │ │      (fills available space)          │ │
│ ┌─────────────┐ │ │                                       │ │
│ │ Synth       │ │ │                                       │ │
│ │ Controls    │ │ └───────────────────────────────────────┘ │
│ │ (ADSR, etc) │ │ ┌───────────────────────────────────────┐ │
│ └─────────────┘ │ │ Scope Settings (collapsible)          │ │
│ ┌─────────────┐ │ └───────────────────────────────────────┘ │
│ │ Effect      │ │                                           │
│ │ Chain       │ │ Bottom Panel (optional)                   │
│ │ (draggable) │ │ ┌───────────────────────────────────────┐ │
│ └─────────────┘ │ │ Code Editor / Lua Console             │ │
│ ┌─────────────┐ │ └───────────────────────────────────────┘ │
│ │ Lua Sliders │ │                                           │
│ │ (A-Z)       │ │                                           │
│ └─────────────┘ │                                           │
└─────────────────┴───────────────────────────────────────────┘
```

---

## Complete Effect Reference (27 effects)

### Free Effects (13)
| # | Effect | Parameters |
|---|--------|------------|
| 1 | BitCrush | Dry/Wet, Strength |
| 2 | Bulge | Strength (0-1) |
| 3 | VectorCancelling | Frequency (0-1) |
| 4 | Ripple | Depth, Phase, Amount |
| 5 | Rotate | X, Y, Z (-π to π) |
| 6 | Translate | X, Y, Z |
| 7 | Swirl | Strength |
| 8 | Smooth | Factor (0-1) |
| 9 | Delay | Decay, Length |
| 10 | DashedLine | Count, Offset, Width |
| 11 | Wobble | Amount, Phase |
| 12 | Duplicator | Copies, Spread, Angle Offset |
| 13 | Scale | X, Y, Z (-3 to 3) |

### Premium Effects (10)
| # | Effect | Parameters |
|---|--------|------------|
| 14 | Multiplex | Grid X/Y/Z, Interpolation, Delay |
| 15 | Unfold | Segments, LFO |
| 16 | Bounce | Size, Speed, Angle |
| 17 | Twist | Strength |
| 18 | Skew | Skew X/Y/Z |
| 19 | Polygonizer | Strength, Sides, Stripe Size, Rotation, Phase |
| 20 | Kaleidoscope | Segments, Mirror, Spread, Clip |
| 21 | Vortex | Strength, Amount, Rotation |
| 22 | GodRay | Strength, Position |
| 23 | SpiralBitCrush | Strength, Density, Twist, Zoom, Rotation |

### System Effects (4)
| # | Effect | Parameters |
|---|--------|------------|
| 24 | Perspective | Strength, FOV |
| 25 | Volume | Gain (0-3) |
| 26 | Threshold | Level (0-1) |
| 27 | Frequency | Hz (0-4200) |

---

## Supported File Formats

| Format | Extension(s) | Parser |
|--------|--------------|--------|
| Wavefront OBJ | `.obj` | tobj |
| SVG | `.svg` | usvg + lyon |
| Plain text | `.txt` | cosmic-text + lyon |
| Lua script | `.lua` | mlua (Lua 5.4) |
| GPLA line art | `.gpla` | Custom binary |
| GIF | `.gif` | gif crate |
| Static image | `.png`, `.jpg`, `.jpeg` | image crate |
| Audio | `.wav`, `.aiff`, `.ogg`, `.flac`, `.mp3` | symphonia |

---

## Completion Roadmap

| # | Task | File(s) | Status |
|---|------|---------|--------|
| 9.1 | Drone mode | `osci-synth/src/synthesizer.rs`, `osci-plugin/src/lib.rs`, `osci-gui/src/lib.rs`, `osci-gui/src/state.rs` | ✓ |
| 9.2 | File loading UI | `osci-gui/src/file_controls.rs` | ✓ |
| 9.3 | Visualizer settings panel | `osci-gui/src/visualizer_settings.rs` (new) | ✗ |
| 9.4 | Lua slider panel | `osci-gui/src/lua_panel.rs` (new) | ✗ |
| 9.5 | Layout reflow | `osci-gui/src/lib.rs` | ✗ |
| 9.6 | Code editor | `osci-gui/src/code_editor.rs` (new) | ✗ |
| 9.7 | Recording panel | `osci-gui/src/recording_panel.rs` (new) | ✗ |
| 9.8 | DAW plugin testing | Manual testing | ✗ |
| 9.9 | Chinese Postman | `osci-parsers/src/chinese_postman.rs` | ✗ |
| 9.10 | Preset compatibility | `osci-gui/src/project.rs` | ✗ |
| 9.11 | Spout/Syphon | `osci-net/src/shared_texture.rs` | ✗ |

**Priority: 9.3 → 9.5 → 9.4.** File loading and drone mode are done. Next: visualizer settings, then layout, then Lua sliders.

---

## Common Mistakes to Avoid

- **Do not add wgpu as a dependency** — visualizer is glow-based
- **Do not use `unwrap()` on Mutex locks in audio thread** — use `try_lock()` or handle poison
- **Do not clone `Vec<Box<dyn Shape>>` per sample** — only at frame boundaries
- **`VoiceEffect::clone_voice_effect()` resets animated_values to zeros** — intentional, each voice gets fresh state
- **Effect snapshots publish from audio thread** — UI reads from `Arc<Mutex<Vec<EffectSnapshot>>>`
- **Check `sound.is_empty()` before `sound.update_frame()`/`clone_frame()`**

---

## GPU Rendering Pipeline (Gaussian Beam)

```
Samples → Line vertex buffer (uploaded each frame)
       → Pass 1: Draw lines with intensity/width → lineTexture
       → Pass 2-5: Gaussian blur cascade (4 levels) → bloomTextures
       → Pass 6: Composite line + bloom + persistence → outputTexture
       → Pass 7: Apply overlay (graticule/CRT) → final framebuffer
```

Mathematical foundation from [woscope](https://github.com/m1el/woscope):
- Each line segment's brightness = analytical integral of Gaussian using `erf()`
- `erf` approximation: `1 + (0.278393 + (0.230389 + 0.078108·a²)·a)·a`
- Tight blur: 17-tap Gaussian (512×512)
- Wide blur: 65-tap Gaussian (128×128)
- Persistence: `fadeAmount = pow(0.5, persistence) × 0.4 × (60/fps)`

---

## rusci (sosci Clone) Notes

sosci is NOT a separate app — it's the osci-render visualizer packaged as audio-input-only VST3/AU. It reuses the same GPU pipeline but has no synthesizer, parsers, or effects.

**What sosci has:**
- Stereo audio input (L/R → X/Y) + optional Z (brightness)
- GPU oscilloscope visualizer with Gaussian beam
- Bloom, persistence, afterglow, tone mapping
- Screen overlays (graticule, smudged, real oscilloscope, vector display)
- Render modes: XY, XYZ, XYRGB
- Goniometer mode
- Video recording, Syphon/Spout output

**What sosci does NOT have:**
- No file parsers
- No synthesizer/voice system
- No effects chain
- No MIDI/ADSR
- No Lua scripting

Build rusci AFTER rusci-render Phase 5 is complete — rusci reuses `osci-visualizer` directly.
