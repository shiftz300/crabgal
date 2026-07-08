// Dissolve / transition easing functions.
// Ported from Raven's dissolve module (Ren'Py-style curves).

/// Ren'Py dissolve easing — slow start, fast middle, slow end.
/// Maps progress t ∈ [0,1] to eased progress.
pub fn renpy_dissolve(t: f32) -> f32 {
    if t < 0.1 {
        // First 10% very slow
        let p = t / 0.1;
        0.05 * ease_out_expo(p)
    } else if t < 0.9 {
        // Middle 80% smooth transition
        let p = (t - 0.1) / 0.8;
        0.05 + 0.9 * ease_in_out_cubic(p)
    } else {
        // Last 10% slow settle
        let p = (t - 0.9) / 0.1;
        0.95 + 0.05 * ease_in_expo(p)
    }
}

/// Smooth fade suitable for bg transitions.
pub fn smooth_fade(t: f32) -> f32 {
    ease_in_out_cubic(t)
}

// ── Easing primitives ──

fn ease_out_expo(t: f32) -> f32 {
    if t >= 1.0 {
        1.0
    } else {
        1.0 - 2.0_f32.powf(-10.0 * t)
    }
}

fn ease_in_expo(t: f32) -> f32 {
    if t <= 0.0 {
        0.0
    } else {
        2.0_f32.powf(10.0 * (t - 1.0))
    }
}

fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoints() {
        assert!((renpy_dissolve(0.0) - 0.0).abs() < 0.01);
        assert!((renpy_dissolve(1.0) - 1.0).abs() < 0.01);
        assert!((smooth_fade(0.0) - 0.0).abs() < 0.01);
        assert!((smooth_fade(1.0) - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_monotonic() {
        let mut prev = 0.0;
        for i in 0..=100 {
            let t = i as f32 / 100.0;
            let v = renpy_dissolve(t);
            assert!(v >= prev, "not monotonic at t={t}: {v} < {prev}");
            prev = v;
        }
    }
}
