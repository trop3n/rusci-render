//! Lua script parser — evaluates Lua scripts to produce per-sample points.
//!
//! Lua scripts return `{x, y}` or `{x, y, z}` tables each call.
//! The runtime exposes global variables: `step`, `phase`, `sampleRate`,
//! `frequency`, `cycle`, and 26 slider variables (`slider_a` through `slider_z`).

#[cfg(feature = "lua")]
mod inner {
    use mlua::{Lua, Function, Result as LuaResult, Value, MultiValue};
    use osci_core::Point;

    const NUM_SLIDERS: usize = 26;
    const MAX_INSTRUCTIONS: u32 = 5_000_000;

    const SLIDER_NAMES: [&str; NUM_SLIDERS] = [
        "slider_a", "slider_b", "slider_c", "slider_d", "slider_e", "slider_f",
        "slider_g", "slider_h", "slider_i", "slider_j", "slider_k", "slider_l",
        "slider_m", "slider_n", "slider_o", "slider_p", "slider_q", "slider_r",
        "slider_s", "slider_t", "slider_u", "slider_v", "slider_w", "slider_x",
        "slider_y", "slider_z",
    ];

    /// Variables exposed to Lua scripts.
    pub struct LuaVariables {
        pub sliders: [f64; NUM_SLIDERS],
        pub step: f64,
        pub phase: f64,
        pub sample_rate: f64,
        pub frequency: f64,
        pub cycle: f64,
        /// For effect mode: input point coordinates
        pub is_effect: bool,
        pub x: f64,
        pub y: f64,
        pub z: f64,
        pub ext_x: f64,
        pub ext_y: f64,
    }

    impl Default for LuaVariables {
        fn default() -> Self {
            Self {
                sliders: [0.0; NUM_SLIDERS],
                step: 1.0,
                phase: 0.0,
                sample_rate: 44100.0,
                frequency: 440.0,
                cycle: 1.0,
                is_effect: false,
                x: 0.0,
                y: 0.0,
                z: 0.0,
                ext_x: 0.0,
                ext_y: 0.0,
            }
        }
    }

    impl LuaVariables {
        /// Advance state after one sample.
        pub fn increment(&mut self) {
            self.step += 1.0;
            if self.sample_rate > 0.0 {
                self.phase += std::f64::consts::TAU * self.frequency / self.sample_rate;
                if self.phase >= std::f64::consts::TAU {
                    self.phase -= std::f64::consts::TAU;
                    self.cycle += 1.0;
                }
            }
        }
    }

    /// A compiled Lua script ready for per-sample execution.
    pub struct LuaParser {
        lua: Lua,
        script: String,
        fallback_script: String,
        using_fallback: bool,
    }

    impl LuaParser {
        /// Create a new Lua parser with the given script.
        pub fn new(script: &str) -> Result<Self, String> {
            let fallback = "return { 0.0, 0.0 }".to_string();
            let lua = Lua::new();

            // Register built-in shape functions
            Self::register_builtins(&lua).map_err(|e| format!("lua init error: {e}"))?;

            // Try to load the script
            let actual_script = script.to_string();
            if let Err(e) = lua.load(&actual_script).exec() {
                log::warn!("Lua script error, using fallback: {e}");
                lua.load(&fallback)
                    .exec()
                    .map_err(|e| format!("fallback script error: {e}"))?;
                return Ok(Self {
                    lua,
                    script: fallback.clone(),
                    fallback_script: fallback,
                    using_fallback: true,
                });
            }

            Ok(Self {
                lua,
                script: actual_script,
                fallback_script: fallback,
                using_fallback: false,
            })
        }

        /// Run the script once with the given variables, returning a point.
        pub fn run(&self, vars: &mut LuaVariables) -> Point {
            let result = self.run_inner(vars);
            vars.increment();
            result.unwrap_or(Point::ZERO)
        }

        fn run_inner(&self, vars: &LuaVariables) -> Result<Point, mlua::Error> {
            let globals = self.lua.globals();

            // Set global variables
            globals.set("step", vars.step)?;
            globals.set("phase", vars.phase)?;
            globals.set("sampleRate", vars.sample_rate)?;
            globals.set("frequency", vars.frequency)?;
            globals.set("cycle", vars.cycle)?;

            for (i, name) in SLIDER_NAMES.iter().enumerate() {
                globals.set(*name, vars.sliders[i])?;
            }

            if vars.is_effect {
                globals.set("x", vars.x)?;
                globals.set("y", vars.y)?;
                globals.set("z", vars.z)?;
                globals.set("ext_x", vars.ext_x)?;
                globals.set("ext_y", vars.ext_y)?;
            }

            // Execute the chunk — it should return a table
            let chunk = self.lua.load(&self.script);
            let value: Value = chunk.eval()?;

            // Parse the return value as a table of floats
            match value {
                Value::Table(table) => {
                    let x: f64 = table.get(1).unwrap_or(0.0);
                    let y: f64 = table.get(2).unwrap_or(0.0);
                    let z: f64 = table.get(3).unwrap_or(0.0);
                    Ok(Point::new(x as f32, y as f32, z as f32))
                }
                _ => Ok(Point::ZERO),
            }
        }

        fn register_builtins(lua: &Lua) -> LuaResult<()> {
            let globals = lua.globals();

            // osci_line(phase, point1, point2) -> {x, y, z}
            let line_fn = lua.create_function(|_, (phase, p1, p2): (f64, Vec<f64>, Vec<f64>)| {
                let t = phase / std::f64::consts::TAU;
                let x1 = p1.first().copied().unwrap_or(-1.0);
                let y1 = p1.get(1).copied().unwrap_or(-1.0);
                let x2 = p2.first().copied().unwrap_or(1.0);
                let y2 = p2.get(1).copied().unwrap_or(1.0);
                let x = x1 + (x2 - x1) * t;
                let y = y1 + (y2 - y1) * t;
                Ok(vec![x, y])
            })?;
            globals.set("osci_line", line_fn)?;

            // osci_circle(phase, radius) -> {x, y}
            let circle_fn = lua.create_function(|_, (phase, radius): (f64, Option<f64>)| {
                let r = radius.unwrap_or(1.0);
                let x = r * phase.cos();
                let y = r * phase.sin();
                Ok(vec![x, y])
            })?;
            globals.set("osci_circle", circle_fn)?;

            // osci_polygon(phase, sides) -> {x, y}
            let polygon_fn = lua.create_function(|_, (phase, sides): (f64, f64)| {
                let n = sides.max(3.0);
                let t = phase / std::f64::consts::TAU;
                let segment = (t * n).floor();
                let local_t = t * n - segment;
                let angle1 = std::f64::consts::TAU * segment / n;
                let angle2 = std::f64::consts::TAU * (segment + 1.0) / n;
                let x = angle1.cos() + (angle2.cos() - angle1.cos()) * local_t;
                let y = angle1.sin() + (angle2.sin() - angle1.sin()) * local_t;
                Ok(vec![x, y])
            })?;
            globals.set("osci_polygon", polygon_fn)?;

            Ok(())
        }

        /// Get the script source.
        pub fn script(&self) -> &str {
            &self.script
        }

        /// Whether the parser fell back to the default script.
        pub fn is_using_fallback(&self) -> bool {
            self.using_fallback
        }
    }
}

#[cfg(feature = "lua")]
pub use inner::*;

/// Stub types when lua feature is not enabled.
#[cfg(not(feature = "lua"))]
pub mod stub {
    use osci_core::Point;

    pub struct LuaVariables;

    impl Default for LuaVariables {
        fn default() -> Self { Self }
    }

    impl LuaVariables {
        pub fn increment(&mut self) {}
    }

    pub struct LuaParser;

    impl LuaParser {
        pub fn new(_script: &str) -> Result<Self, String> {
            Err("Lua support not enabled (compile with --features lua)".to_string())
        }

        pub fn run(&self, _vars: &mut LuaVariables) -> Point {
            Point::ZERO
        }
    }
}

#[cfg(not(feature = "lua"))]
pub use stub::*;
