/// Panel resize types and helpers.

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ResizeTarget {
    Explorer,
    Findings,
}

#[derive(Clone, Copy, PartialEq)]
pub struct ActiveResize {
    pub target: ResizeTarget,
    pub start_x: f64,
    pub start_width: f64,
}

pub fn clamp_panel_width(target: ResizeTarget, width: f64) -> f64 {
    let (min_width, max_width) = match target {
        ResizeTarget::Explorer => (260.0, 560.0),
        ResizeTarget::Findings => (240.0, 520.0),
    };
    width.clamp(min_width, max_width)
}

#[cfg(test)]
mod tests {
    use super::{clamp_panel_width, ResizeTarget};

    #[test]
    fn clamp_panel_width_enforces_explorer_bounds() {
        assert_eq!(clamp_panel_width(ResizeTarget::Explorer, 200.0), 260.0);
        assert_eq!(clamp_panel_width(ResizeTarget::Explorer, 400.0), 400.0);
        assert_eq!(clamp_panel_width(ResizeTarget::Explorer, 700.0), 560.0);
    }

    #[test]
    fn clamp_panel_width_enforces_findings_bounds() {
        assert_eq!(clamp_panel_width(ResizeTarget::Findings, 200.0), 240.0);
        assert_eq!(clamp_panel_width(ResizeTarget::Findings, 320.0), 320.0);
        assert_eq!(clamp_panel_width(ResizeTarget::Findings, 700.0), 520.0);
    }
}
