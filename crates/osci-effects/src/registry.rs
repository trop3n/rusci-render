use osci_core::effect::EffectApplication;
use osci_core::parameter::EffectParameter;

/// An entry in the effect registry, containing a constructor and parameter definitions.
pub struct EffectEntry {
    pub id: &'static str,
    pub name: &'static str,
    pub constructor: fn() -> Box<dyn EffectApplication>,
    pub parameters: fn() -> Vec<EffectParameter>,
}

/// Build the complete registry of all available effects.
///
/// Each effect is identified by a unique string ID. The registry is used by
/// the GUI and preset system to enumerate, instantiate, and configure effects.
pub fn build_registry() -> Vec<EffectEntry> {
    vec![
        // ── Free effects ──────────────────────────────────────────
        EffectEntry {
            id: "bitcrush",
            name: "Bit Crush",
            constructor: || Box::new(crate::bitcrush::BitCrush::new()),
            parameters: || vec![
                EffectParameter::new("Bit Crush", "Controls the strength of the bit crush effect.", "bitCrushEffectScale", 1.0, 0.0, 1.0),
                EffectParameter::new("Bit Crush Depth", "Controls the bit depth of the crush.", "bitCrushDepth", 0.6, 0.0, 1.0),
            ],
        },
        EffectEntry {
            id: "bulge",
            name: "Bulge",
            constructor: || Box::new(crate::bulge::Bulge::new()),
            parameters: || vec![
                EffectParameter::new("Bulge", "Controls the radial power-law distortion.", "bulge", 0.0, -1.0, 1.0),
            ],
        },
        EffectEntry {
            id: "vectorCancelling",
            name: "Vector Cancelling",
            constructor: || Box::new(crate::vector_cancelling::VectorCancelling::new()),
            parameters: || vec![
                EffectParameter::new("Vector Cancelling", "Frequency of periodic inversion.", "vectorCancelling", 0.0, 0.0, 1.0),
            ],
        },
        EffectEntry {
            id: "ripple",
            name: "Ripple",
            constructor: || Box::new(crate::ripple::Ripple::new()),
            parameters: || vec![
                EffectParameter::new("Ripple Amplitude", "Height of the ripple wave.", "rippleAmplitude", 0.0, 0.0, 1.0),
                EffectParameter::new("Ripple Phase", "Phase offset of the ripple.", "ripplePhase", 0.0, -1.0, 1.0),
                EffectParameter::new("Ripple Frequency", "Spatial frequency of the ripple.", "rippleFrequency", 0.5, 0.0, 1.0),
            ],
        },
        EffectEntry {
            id: "rotate",
            name: "Rotate",
            constructor: || Box::new(crate::rotate::Rotate::new()),
            parameters: || vec![
                EffectParameter::new("Rotate X", "Rotation around the X axis.", "rotateX", 0.0, -1.0, 1.0),
                EffectParameter::new("Rotate Y", "Rotation around the Y axis.", "rotateY", 0.0, -1.0, 1.0),
                EffectParameter::new("Rotate Z", "Rotation around the Z axis.", "rotateZ", 0.0, -1.0, 1.0),
            ],
        },
        EffectEntry {
            id: "translate",
            name: "Translate",
            constructor: || Box::new(crate::translate::Translate::new()),
            parameters: || vec![
                EffectParameter::new("Translate X", "Offset along the X axis.", "translateX", 0.0, -1.0, 1.0),
                EffectParameter::new("Translate Y", "Offset along the Y axis.", "translateY", 0.0, -1.0, 1.0),
                EffectParameter::new("Translate Z", "Offset along the Z axis.", "translateZ", 0.0, -1.0, 1.0),
            ],
        },
        EffectEntry {
            id: "scale",
            name: "Scale",
            constructor: || Box::new(crate::scale::ScaleEffect::new()),
            parameters: || vec![
                EffectParameter::new("Scale X", "Scale factor for X.", "scaleX", 1.0, -3.0, 3.0),
                EffectParameter::new("Scale Y", "Scale factor for Y.", "scaleY", 1.0, -3.0, 3.0),
                EffectParameter::new("Scale Z", "Scale factor for Z.", "scaleZ", 1.0, -3.0, 3.0),
            ],
        },
        EffectEntry {
            id: "swirl",
            name: "Swirl",
            constructor: || Box::new(crate::swirl::Swirl::new()),
            parameters: || vec![
                EffectParameter::new("Swirl", "Strength of the spiral distortion.", "swirl", 0.0, -1.0, 1.0),
            ],
        },
        EffectEntry {
            id: "smooth",
            name: "Smooth",
            constructor: || Box::new(crate::smooth::SmoothEffect::new()),
            parameters: || vec![
                EffectParameter::new("Smooth", "Amount of smoothing applied.", "smooth", 0.0, 0.0, 1.0),
            ],
        },
        EffectEntry {
            id: "delay",
            name: "Delay",
            constructor: || Box::new(crate::delay::DelayEffect::new()),
            parameters: || vec![
                EffectParameter::new("Decay", "Echo decay factor.", "delayDecay", 0.0, 0.0, 1.0),
                EffectParameter::new("Delay Length", "Length of the delay in seconds.", "delayLength", 0.5, 0.0, 1.0),
            ],
        },
        EffectEntry {
            id: "dashedLine",
            name: "Dashed Line",
            constructor: || Box::new(crate::dashed_line::DashedLineEffect::new()),
            parameters: || vec![
                EffectParameter::new("Dash Count", "Number of dashes.", "dashCount", 4.0, 1.0, 20.0),
                EffectParameter::new("Dash Offset", "Phase offset of dashes.", "dashOffset", 0.0, -1.0, 1.0),
                EffectParameter::new("Dash Coverage", "Fraction of dash visible.", "dashCoverage", 0.5, 0.0, 1.0),
            ],
        },
        EffectEntry {
            id: "wobble",
            name: "Wobble",
            constructor: || Box::new(crate::wobble::WobbleEffect::new()),
            parameters: || vec![
                EffectParameter::new("Wobble Amplitude", "Displacement amount.", "wobbleAmplitude", 0.0, -1.0, 1.0),
                EffectParameter::new("Wobble Phase", "Phase offset.", "wobblePhase", 0.0, -1.0, 1.0),
            ],
        },
        EffectEntry {
            id: "duplicator",
            name: "Duplicator",
            constructor: || Box::new(crate::duplicator::DuplicatorEffect::new()),
            parameters: || vec![
                EffectParameter::new("Copies", "Number of duplicates.", "duplicatorCopies", 1.0, 1.0, 10.0).with_step(1.0),
                EffectParameter::new("Spread", "Radial spread of copies.", "duplicatorSpread", 0.5, 0.0, 1.0),
                EffectParameter::new("Angle Offset", "Angular offset of copies.", "duplicatorAngle", 0.0, -1.0, 1.0),
            ],
        },

        // ── Premium effects ───────────────────────────────────────
        EffectEntry {
            id: "multiplex",
            name: "Multiplex",
            constructor: || Box::new(crate::multiplex::MultiplexEffect::new()),
            parameters: || vec![
                EffectParameter::new("Grid X", "Horizontal grid divisions.", "multiplexGridX", 1.0, 1.0, 10.0).with_step(1.0),
                EffectParameter::new("Grid Y", "Vertical grid divisions.", "multiplexGridY", 1.0, 1.0, 10.0).with_step(1.0),
                EffectParameter::new("Grid Z", "Depth grid divisions.", "multiplexGridZ", 1.0, 1.0, 10.0).with_step(1.0),
                EffectParameter::new("Interpolation", "Smoothness between grid cells.", "multiplexInterp", 0.0, 0.0, 1.0),
                EffectParameter::new("Grid Delay", "Delay between grid cells.", "multiplexDelay", 0.0, 0.0, 1.0),
            ],
        },
        EffectEntry {
            id: "unfold",
            name: "Unfold",
            constructor: || Box::new(crate::unfold::UnfoldEffect::new()),
            parameters: || vec![
                EffectParameter::new("Segments", "Number of angular segments.", "unfoldSegments", 1.0, 1.0, 16.0).with_step(1.0),
            ],
        },
        EffectEntry {
            id: "bounce",
            name: "Bounce",
            constructor: || Box::new(crate::bounce::BounceEffect::new()),
            parameters: || vec![
                EffectParameter::new("Size", "Size of the bouncing shape.", "bounceSize", 0.5, 0.05, 1.0),
                EffectParameter::new("Speed", "Bounce speed.", "bounceSpeed", 1.0, 0.0, 5.0),
                EffectParameter::new("Angle", "Direction of movement.", "bounceAngle", 0.125, 0.0, 1.0),
            ],
        },
        EffectEntry {
            id: "twist",
            name: "Twist",
            constructor: || Box::new(crate::twist::TwistEffect::new()),
            parameters: || vec![
                EffectParameter::new("Twist", "Strength of the Y-axis twist.", "twist", 0.0, -1.0, 1.0),
            ],
        },
        EffectEntry {
            id: "skew",
            name: "Skew",
            constructor: || Box::new(crate::skew::SkewEffect::new()),
            parameters: || vec![
                EffectParameter::new("Skew X", "Horizontal skew.", "skewX", 0.0, -1.0, 1.0),
                EffectParameter::new("Skew Y", "Vertical skew.", "skewY", 0.0, -1.0, 1.0),
                EffectParameter::new("Skew Z", "Depth skew.", "skewZ", 0.0, -1.0, 1.0),
            ],
        },
        EffectEntry {
            id: "polygonizer",
            name: "Polygonizer",
            constructor: || Box::new(crate::polygonizer::PolygonizerEffect::new()),
            parameters: || vec![
                EffectParameter::new("Polygonizer", "Strength of the effect.", "polygonizerScale", 1.0, 0.0, 1.0),
                EffectParameter::new("Sides", "Number of polygon sides.", "polygonizerSides", 4.0, 2.0, 12.0).with_step(1.0),
                EffectParameter::new("Stripe Size", "Size of radial stripes.", "polygonizerStripeSize", 0.5, 0.0, 1.0),
                EffectParameter::new("Rotation", "Rotation of the polygon.", "polygonizerRotation", 0.0, 0.0, 1.0),
                EffectParameter::new("Phase", "Stripe phase offset.", "polygonizerPhase", 0.0, 0.0, 1.0),
            ],
        },
        EffectEntry {
            id: "kaleidoscope",
            name: "Kaleidoscope",
            constructor: || Box::new(crate::kaleidoscope::KaleidoscopeEffect::new()),
            parameters: || vec![
                EffectParameter::new("Segments", "Number of kaleidoscope segments.", "kaleidoscopeSegments", 3.0, 1.0, 16.0).with_step(1.0),
                EffectParameter::new("Mirror", "Enable segment mirroring.", "kaleidoscopeMirror", 1.0, 0.0, 1.0).with_step(1.0),
                EffectParameter::new("Spread", "X-axis projection amount.", "kaleidoscopeSpread", 0.0, 0.0, 1.0),
                EffectParameter::new("Clip", "Clip to segment boundaries.", "kaleidoscopeClip", 1.0, 0.0, 1.0),
            ],
        },
        EffectEntry {
            id: "vortex",
            name: "Vortex",
            constructor: || Box::new(crate::vortex::VortexEffect::new()),
            parameters: || vec![
                EffectParameter::new("Vortex", "Strength of the vortex.", "vortex", 0.0, -1.0, 1.0),
            ],
        },
        EffectEntry {
            id: "godRay",
            name: "God Ray",
            constructor: || Box::new(crate::god_ray::GodRayEffect::new()),
            parameters: || vec![
                EffectParameter::new("God Ray", "Intensity of the god ray.", "godRay", 0.0, 0.0, 1.0),
            ],
        },
        EffectEntry {
            id: "spiralBitcrush",
            name: "Spiral Bitcrush",
            constructor: || Box::new(crate::spiral_bitcrush::SpiralBitcrushEffect::new()),
            parameters: || vec![
                EffectParameter::new("Spiral Bitcrush", "Strength of the effect.", "spiralBitcrushScale", 1.0, 0.0, 1.0),
                EffectParameter::new("Depth", "Bit depth.", "spiralBitcrushDepth", 0.6, 0.0, 1.0),
            ],
        },

        // ── System effects ────────────────────────────────────────
        EffectEntry {
            id: "perspective",
            name: "Perspective",
            constructor: || Box::new(crate::perspective::PerspectiveEffect::new()),
            parameters: || vec![
                EffectParameter::new("Perspective", "Controls the strength of the 3D perspective projection.", "perspectiveStrength", 1.0, 0.0, 1.0),
                EffectParameter::new("Field of View", "Camera field of view in degrees.", "perspectiveFov", 50.0, 5.0, 130.0),
            ],
        },
        EffectEntry {
            id: "volume",
            name: "Volume",
            constructor: || Box::new(crate::volume::VolumeEffect::new()),
            parameters: || vec![
                EffectParameter::new("Volume", "Output gain.", "volume", 1.0, 0.0, 3.0),
            ],
        },
        EffectEntry {
            id: "threshold",
            name: "Threshold",
            constructor: || Box::new(crate::threshold::ThresholdEffect::new()),
            parameters: || vec![
                EffectParameter::new("Threshold", "Clipping level.", "threshold", 1.0, 0.0, 1.0),
            ],
        },
        EffectEntry {
            id: "frequency",
            name: "Frequency",
            constructor: || Box::new(crate::frequency::FrequencyEffect::new()),
            parameters: || vec![
                EffectParameter::new("Frequency", "Shape drawing rate in Hz.", "frequency", 440.0, 0.0, 4200.0),
            ],
        },
    ]
}

/// Look up an effect entry by its ID.
pub fn find_effect(id: &str) -> Option<&'static EffectEntry> {
    // Use a leaked &'static reference for the registry. This is fine because
    // the registry is built once and lives for the entire program duration.
    static REGISTRY: std::sync::OnceLock<Vec<EffectEntry>> = std::sync::OnceLock::new();
    let entries = REGISTRY.get_or_init(build_registry);
    entries.iter().find(|e| e.id == id)
}
