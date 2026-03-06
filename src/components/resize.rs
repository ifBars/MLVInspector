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
