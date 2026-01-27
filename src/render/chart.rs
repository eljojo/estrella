//! Chart rendering for thermal receipt printers.
//!
//! Renders data charts as raster images suitable for 1-bit dithered thermal
//! printing. Supports line, area, bar, and dot styles with axes, grid lines,
//! and bitmap font labels.

use crate::document::types::{Chart, ChartStyle};
use crate::render::dither::{self, DitheringAlgorithm};
use spleen_font::{PSF2Font, FONT_6X12, FONT_12X24};

// ============================================================================
// CONSTANTS
// ============================================================================

// Small font (6x12) for X-axis labels
const FONT_SM_W: usize = 6;

// Large font (12x24) for Y-axis labels, title
const FONT_LG_W: usize = 12;
const FONT_LG_H: usize = 24;

const TITLE_H: usize = 28; // FONT_LG_H + 4px padding
const TOP_PAD: usize = 4;
const BOTTOM_PAD: usize = 4;
const X_LABEL_H: usize = 16; // FONT_SM_H + 4px spacing above
const Y_TICK_PAD: usize = 4; // space between y label text and axis
const RIGHT_MARGIN: usize = 4;

const LINE_THICKNESS: f32 = 3.0;
const MARKER_RADIUS: f32 = 4.0;
const AXIS_THICKNESS: usize = 2;
const GRID_DASH_ON: usize = 3;
const GRID_DASH_OFF: usize = 5;
const GRID_INTENSITY: f32 = 0.5;

const AREA_FILL_INTENSITY: f32 = 0.55;
const BAR_FILL_INTENSITY: f32 = 0.85;
const BAR_GAP: usize = 2;

// ============================================================================
// CANVAS
// ============================================================================

/// Grayscale intensity buffer for chart rendering.
/// 0.0 = white (no print), 1.0 = black (print).
struct Canvas {
    buf: Vec<f32>,
    width: usize,
    height: usize,
}

impl Canvas {
    fn new(width: usize, height: usize) -> Self {
        Self {
            buf: vec![0.0; width * height],
            width,
            height,
        }
    }

    #[inline]
    fn in_bounds(&self, x: usize, y: usize) -> bool {
        x < self.width && y < self.height
    }

    /// Set pixel intensity, taking the maximum of existing and new value.
    #[inline]
    fn blend(&mut self, x: usize, y: usize, intensity: f32) {
        if self.in_bounds(x, y) {
            let idx = y * self.width + x;
            self.buf[idx] = self.buf[idx].max(intensity);
        }
    }

}

// ============================================================================
// DRAWING PRIMITIVES
// ============================================================================

fn draw_hline(canvas: &mut Canvas, x1: usize, x2: usize, y: usize, thickness: usize, intensity: f32) {
    let half = thickness / 2;
    for dy in 0..thickness {
        let py = (y + dy).saturating_sub(half);
        for px in x1..=x2.min(canvas.width.saturating_sub(1)) {
            canvas.blend(px, py, intensity);
        }
    }
}

fn draw_vline(canvas: &mut Canvas, x: usize, y1: usize, y2: usize, thickness: usize, intensity: f32) {
    let half = thickness / 2;
    for dx in 0..thickness {
        let px = (x + dx).saturating_sub(half);
        for py in y1..=y2.min(canvas.height.saturating_sub(1)) {
            canvas.blend(px, py, intensity);
        }
    }
}

fn draw_hline_dashed(
    canvas: &mut Canvas,
    x1: usize,
    x2: usize,
    y: usize,
    thickness: usize,
    intensity: f32,
) {
    let mut x = x1;
    let mut drawing = true;
    let mut count = 0;
    while x <= x2.min(canvas.width.saturating_sub(1)) {
        if drawing {
            let half = thickness / 2;
            for dy in 0..thickness {
                let py = (y + dy).saturating_sub(half);
                canvas.blend(x, py, intensity);
            }
        }
        count += 1;
        let period = if drawing { GRID_DASH_ON } else { GRID_DASH_OFF };
        if count >= period {
            drawing = !drawing;
            count = 0;
        }
        x += 1;
    }
}

fn draw_filled_rect(canvas: &mut Canvas, x1: usize, y1: usize, x2: usize, y2: usize, intensity: f32) {
    for y in y1..y2.min(canvas.height) {
        for x in x1..x2.min(canvas.width) {
            canvas.blend(x, y, intensity);
        }
    }
}

fn draw_filled_circle(canvas: &mut Canvas, cx: f32, cy: f32, radius: f32, intensity: f32) {
    let r_ceil = radius.ceil() as i32 + 1;
    let cxi = cx as i32;
    let cyi = cy as i32;

    for dy in -r_ceil..=r_ceil {
        for dx in -r_ceil..=r_ceil {
            let px = cxi + dx;
            let py = cyi + dy;
            if px < 0 || py < 0 {
                continue;
            }
            let dist = ((dx as f32 - (cx - cxi as f32)).powi(2)
                + (dy as f32 - (cy - cyi as f32)).powi(2))
            .sqrt();
            if dist <= radius {
                canvas.blend(px as usize, py as usize, intensity);
            } else if dist <= radius + 1.0 {
                // Anti-alias edge
                let aa = 1.0 - (dist - radius);
                canvas.blend(px as usize, py as usize, intensity * aa);
            }
        }
    }
}

fn draw_line_thick(canvas: &mut Canvas, x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, intensity: f32) {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 0.001 {
        draw_filled_circle(canvas, x1, y1, thickness / 2.0, intensity);
        return;
    }

    let half_t = thickness / 2.0;

    // Bounding box
    let min_x = (x1.min(x2) - half_t - 1.0).max(0.0) as usize;
    let max_x = ((x1.max(x2) + half_t + 1.0) as usize).min(canvas.width.saturating_sub(1));
    let min_y = (y1.min(y2) - half_t - 1.0).max(0.0) as usize;
    let max_y = ((y1.max(y2) + half_t + 1.0) as usize).min(canvas.height.saturating_sub(1));

    for py in min_y..=max_y {
        for px in min_x..=max_x {
            let pxf = px as f32;
            let pyf = py as f32;

            // Project point onto line segment
            let t = ((pxf - x1) * dx + (pyf - y1) * dy) / (len * len);
            let t_clamped = t.clamp(0.0, 1.0);
            let closest_x = x1 + t_clamped * dx;
            let closest_y = y1 + t_clamped * dy;
            let dist = ((pxf - closest_x).powi(2) + (pyf - closest_y).powi(2)).sqrt();

            if dist <= half_t {
                canvas.blend(px, py, intensity);
            } else if dist <= half_t + 1.0 {
                let aa = 1.0 - (dist - half_t);
                canvas.blend(px, py, intensity * aa);
            }
        }
    }
}

fn fill_below_polyline(canvas: &mut Canvas, points: &[(f32, f32)], baseline_y: usize, intensity: f32) {
    if points.len() < 2 {
        return;
    }

    for i in 0..points.len() - 1 {
        let (x1, y1) = points[i];
        let (x2, y2) = points[i + 1];

        let px_start = x1.floor() as usize;
        let px_end = x2.ceil() as usize;

        for px in px_start..px_end.min(canvas.width) {
            // Interpolate y at this x position
            let t = if (x2 - x1).abs() < 0.001 {
                0.0
            } else {
                (px as f32 - x1) / (x2 - x1)
            };
            let t = t.clamp(0.0, 1.0);
            let y_at_x = y1 + t * (y2 - y1);
            let top = y_at_x.round() as usize;

            for py in top..baseline_y.min(canvas.height) {
                canvas.blend(px, py, intensity);
            }
        }
    }
}

// ============================================================================
// TEXT RENDERING
// ============================================================================

/// Font size for text rendering.
#[derive(Clone, Copy)]
enum FontSize {
    /// 6x12 — compact, for X-axis labels
    Small,
    /// 12x24 — readable, for Y-axis labels and title
    Large,
}

impl FontSize {
    fn char_width(self) -> usize {
        match self {
            FontSize::Small => FONT_SM_W,
            FontSize::Large => FONT_LG_W,
        }
    }

    fn font_data(self) -> &'static [u8] {
        match self {
            FontSize::Small => FONT_6X12,
            FontSize::Large => FONT_12X24,
        }
    }
}

fn draw_text_sized(canvas: &mut Canvas, text: &str, x: usize, y: usize, intensity: f32, size: FontSize) {
    let mut font = PSF2Font::new(size.font_data()).unwrap();
    let char_w = size.char_width();
    let mut cursor_x = x;
    for ch in text.chars() {
        let utf8 = ch.to_string();
        if let Some(glyph) = font.glyph_for_utf8(utf8.as_bytes()) {
            for (row_y, row) in glyph.enumerate() {
                for (col_x, on) in row.enumerate() {
                    if on {
                        canvas.blend(cursor_x + col_x, y + row_y, intensity);
                    }
                }
            }
        }
        cursor_x += char_w;
    }
}

fn text_width_sized(text: &str, size: FontSize) -> usize {
    text.chars().count() * size.char_width()
}

fn draw_text_right_sized(canvas: &mut Canvas, text: &str, right_x: usize, y: usize, intensity: f32, size: FontSize) {
    let w = text_width_sized(text, size);
    let x = right_x.saturating_sub(w);
    draw_text_sized(canvas, text, x, y, intensity, size);
}

fn draw_text_centered_sized(canvas: &mut Canvas, text: &str, center_x: usize, y: usize, intensity: f32, size: FontSize) {
    let w = text_width_sized(text, size);
    let x = center_x.saturating_sub(w / 2);
    draw_text_sized(canvas, text, x, y, intensity, size);
}

// ============================================================================
// NICE TICK GENERATION
// ============================================================================

fn nice_step(rough: f64) -> f64 {
    let exponent = rough.abs().log10().floor();
    let fraction = rough / 10.0f64.powf(exponent);
    let nice = if fraction <= 1.0 {
        1.0
    } else if fraction <= 2.0 {
        2.0
    } else if fraction <= 5.0 {
        5.0
    } else {
        10.0
    };
    nice * 10.0f64.powf(exponent)
}

fn compute_nice_ticks(min: f64, max: f64, target_count: usize) -> Vec<f64> {
    if (max - min).abs() < 1e-10 {
        return vec![min];
    }

    let range = max - min;
    let rough_step = range / target_count as f64;
    let step = nice_step(rough_step);

    let tick_min = (min / step).floor() * step;
    let tick_max = (max / step).ceil() * step;

    let mut ticks = Vec::new();
    let mut v = tick_min;
    while v <= tick_max + step * 0.01 {
        ticks.push(v);
        v += step;
    }

    // Ensure at least min and max are represented
    if ticks.is_empty() {
        ticks.push(min);
        ticks.push(max);
    }

    ticks
}

fn format_number(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{}", v as i64)
    } else {
        format!("{:.1}", v)
    }
}

fn format_y_label(v: f64, prefix: &str, suffix: &str) -> String {
    format!("{}{}{}", prefix, format_number(v), suffix)
}

// ============================================================================
// LAYOUT
// ============================================================================

struct Layout {
    /// Left edge of data area (after y-axis labels).
    data_left: usize,
    /// Right edge of data area.
    data_right: usize,
    /// Top edge of data area.
    data_top: usize,
    /// Bottom edge of data area.
    data_bottom: usize,
}

impl Layout {
    fn data_width(&self) -> usize {
        self.data_right.saturating_sub(self.data_left)
    }

    fn data_height(&self) -> usize {
        self.data_bottom.saturating_sub(self.data_top)
    }
}

fn compute_layout(
    total_width: usize,
    total_height: usize,
    y_label_max_chars: usize,
    has_title: bool,
    has_x_labels: bool,
) -> Layout {
    let left_gutter = y_label_max_chars * FONT_LG_W + Y_TICK_PAD;
    let title_height = if has_title { TITLE_H } else { 0 };
    let x_label_height = if has_x_labels { X_LABEL_H } else { 0 };

    Layout {
        data_left: left_gutter,
        data_right: total_width.saturating_sub(RIGHT_MARGIN),
        data_top: title_height + TOP_PAD,
        data_bottom: total_height.saturating_sub(x_label_height + BOTTOM_PAD),
    }
}

// ============================================================================
// DATA MAPPING
// ============================================================================

fn map_data_points(
    values: &[f64],
    y_min: f64,
    y_max: f64,
    layout: &Layout,
) -> Vec<(f32, f32)> {
    let n = values.len();
    if n == 0 {
        return Vec::new();
    }

    let y_range = y_max - y_min;
    let data_w = layout.data_width() as f32;
    let data_h = layout.data_height() as f32;

    values
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let px_x = if n == 1 {
                layout.data_left as f32 + data_w / 2.0
            } else {
                layout.data_left as f32 + (i as f32 / (n - 1) as f32) * data_w
            };
            let normalized = if y_range.abs() < 1e-10 {
                0.5
            } else {
                (v - y_min) / y_range
            };
            let px_y = layout.data_bottom as f32 - normalized as f32 * data_h;
            (px_x, px_y)
        })
        .collect()
}

// ============================================================================
// CHART RENDERING
// ============================================================================

/// Render a chart to raster data.
///
/// Returns `(raster_data, width, height)` where raster_data is packed 1-bit
/// data suitable for `Op::Raster`.
pub fn render(chart: &Chart, width: usize, dithering: DitheringAlgorithm) -> (Vec<u8>, u16, u16) {
    let total_height = chart.height.unwrap_or(200);
    let n = chart.values.len();

    if n == 0 {
        return (Vec::new(), 0, 0);
    }

    let prefix = chart.y_prefix.as_deref().unwrap_or("");
    let suffix = chart.y_suffix.as_deref().unwrap_or("");

    // Compute Y range with padding
    let v_min = chart.values.iter().cloned().fold(f64::INFINITY, f64::min);
    let v_max = chart.values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let ticks = compute_nice_ticks(v_min, v_max, 4);
    let y_min = ticks.first().copied().unwrap_or(v_min);
    let y_max = ticks.last().copied().unwrap_or(v_max);

    // Compute Y-axis label widths
    let y_labels: Vec<String> = ticks.iter().map(|&v| format_y_label(v, prefix, suffix)).collect();
    let y_label_max_chars = y_labels.iter().map(|l| l.chars().count()).max().unwrap_or(0);

    let has_title = chart.title.is_some();
    let has_x_labels = !chart.labels.is_empty();
    let layout = compute_layout(width, total_height, y_label_max_chars, has_title, has_x_labels);

    let mut canvas = Canvas::new(width, total_height);

    // Title (large font)
    if let Some(ref title) = chart.title {
        draw_text_centered_sized(&mut canvas, title, width / 2, 2, 1.0, FontSize::Large);
    }

    // Y-axis grid lines and labels (large font)
    let y_range = y_max - y_min;
    for (tick, label) in ticks.iter().zip(y_labels.iter()) {
        let normalized = if y_range.abs() < 1e-10 {
            0.5
        } else {
            (tick - y_min) / y_range
        };
        let py = layout.data_bottom as f32 - normalized as f32 * layout.data_height() as f32;
        let py_usize = py.round() as usize;

        // Dashed grid line
        draw_hline_dashed(
            &mut canvas,
            layout.data_left,
            layout.data_right,
            py_usize,
            1,
            GRID_INTENSITY,
        );

        // Y-axis label (right-aligned in left gutter, large font)
        let label_y = py_usize.saturating_sub(FONT_LG_H / 2);
        draw_text_right_sized(
            &mut canvas,
            label,
            layout.data_left.saturating_sub(Y_TICK_PAD / 2),
            label_y,
            1.0,
            FontSize::Large,
        );
    }

    // Axes
    draw_vline(&mut canvas, layout.data_left, layout.data_top, layout.data_bottom, AXIS_THICKNESS, 1.0);
    draw_hline(&mut canvas, layout.data_left, layout.data_right, layout.data_bottom, AXIS_THICKNESS, 1.0);

    // X-axis labels (small font)
    if has_x_labels {
        let label_y = layout.data_bottom + 4;
        let label_count = chart.labels.len().min(n);

        // Determine how many labels we can fit without overlap
        let max_label_chars = chart.labels.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        let label_pixel_width = max_label_chars * FONT_SM_W + FONT_SM_W; // +1 char spacing
        let available_width = layout.data_width();
        let max_labels = if label_pixel_width > 0 {
            (available_width / label_pixel_width).max(1)
        } else {
            label_count
        };
        let step = if label_count > max_labels {
            (label_count + max_labels - 1) / max_labels
        } else {
            1
        };

        for i in (0..label_count).step_by(step) {
            let px_x = if n == 1 {
                layout.data_left + layout.data_width() / 2
            } else {
                layout.data_left + i * layout.data_width() / (n - 1)
            };
            // Clamp so label text doesn't overflow canvas edges
            let label_half_w = text_width_sized(&chart.labels[i], FontSize::Small) / 2;
            let px_x = px_x.max(label_half_w).min(width.saturating_sub(label_half_w));
            draw_text_centered_sized(&mut canvas, &chart.labels[i], px_x, label_y, 1.0, FontSize::Small);
        }
    }

    // Map data to pixel coordinates
    let points = map_data_points(&chart.values, y_min, y_max, &layout);

    // Draw data according to style
    match chart.style {
        ChartStyle::Line => draw_line_style(&mut canvas, &points),
        ChartStyle::Area => draw_area_style(&mut canvas, &points, &layout),
        ChartStyle::Bar => draw_bar_style(&mut canvas, &chart.values, y_min, y_max, &layout),
        ChartStyle::Dot => draw_dot_style(&mut canvas, &points),
    }

    // Dither to 1-bit raster
    let raster = dither::generate_raster(width, total_height, |x, y, _w, _h| canvas.buf[y * width + x], dithering);

    (raster, width as u16, total_height as u16)
}

// ============================================================================
// STYLE RENDERERS
// ============================================================================

fn draw_line_style(canvas: &mut Canvas, points: &[(f32, f32)]) {
    // Lines
    for pair in points.windows(2) {
        draw_line_thick(canvas, pair[0].0, pair[0].1, pair[1].0, pair[1].1, LINE_THICKNESS, 1.0);
    }
    // Markers
    for &(x, y) in points {
        draw_filled_circle(canvas, x, y, MARKER_RADIUS, 1.0);
    }
}

fn draw_area_style(canvas: &mut Canvas, points: &[(f32, f32)], layout: &Layout) {
    // Fill below the polyline
    fill_below_polyline(canvas, points, layout.data_bottom, AREA_FILL_INTENSITY);
    // Line on top
    for pair in points.windows(2) {
        draw_line_thick(canvas, pair[0].0, pair[0].1, pair[1].0, pair[1].1, 2.0, 1.0);
    }
}

fn draw_bar_style(canvas: &mut Canvas, values: &[f64], y_min: f64, y_max: f64, layout: &Layout) {
    let n = values.len();
    if n == 0 {
        return;
    }

    let data_w = layout.data_width();
    let total_gaps = if n > 1 { (n - 1) * BAR_GAP } else { 0 };
    let bar_width = if data_w > total_gaps {
        (data_w - total_gaps) / n
    } else {
        1
    };

    let y_range = y_max - y_min;
    let data_h = layout.data_height() as f32;
    let min_bar_h: usize = 3; // minimum visible bar height

    for (i, &v) in values.iter().enumerate() {
        let bar_left = layout.data_left + i * (bar_width + BAR_GAP);
        let bar_right = bar_left + bar_width;

        let normalized = if y_range.abs() < 1e-10 {
            0.5
        } else {
            (v - y_min) / y_range
        };
        let raw_top = (layout.data_bottom as f32 - normalized as f32 * data_h).round() as usize;
        let bar_top = raw_top.min(layout.data_bottom.saturating_sub(min_bar_h));

        // Filled bar
        draw_filled_rect(canvas, bar_left, bar_top, bar_right, layout.data_bottom, BAR_FILL_INTENSITY);

        // Black outline
        draw_vline(canvas, bar_left, bar_top, layout.data_bottom, 1, 1.0);
        if bar_right > 0 {
            draw_vline(canvas, bar_right.saturating_sub(1), bar_top, layout.data_bottom, 1, 1.0);
        }
        draw_hline(canvas, bar_left, bar_right.saturating_sub(1), bar_top, 1, 1.0);
    }
}

fn draw_dot_style(canvas: &mut Canvas, points: &[(f32, f32)]) {
    for &(x, y) in points {
        draw_filled_circle(canvas, x, y, MARKER_RADIUS + 1.0, 1.0);
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nice_ticks_negative_range() {
        let ticks = compute_nice_ticks(-16.0, -11.0, 4);
        assert!(ticks.len() >= 2);
        assert!(*ticks.first().unwrap() <= -16.0);
        assert!(*ticks.last().unwrap() >= -11.0);
        // Ticks should be on round numbers
        for t in &ticks {
            assert!((t - t.round()).abs() < 1e-9, "Tick {} not round", t);
        }
    }

    #[test]
    fn test_nice_ticks_positive_range() {
        let ticks = compute_nice_ticks(0.0, 100.0, 4);
        assert!(ticks.len() >= 2);
        assert!(*ticks.first().unwrap() <= 0.0);
        assert!(*ticks.last().unwrap() >= 100.0);
    }

    #[test]
    fn test_nice_ticks_small_range() {
        let ticks = compute_nice_ticks(0.0, 1.0, 4);
        assert!(ticks.len() >= 2);
    }

    #[test]
    fn test_nice_ticks_equal_values() {
        let ticks = compute_nice_ticks(5.0, 5.0, 4);
        assert!(!ticks.is_empty());
    }

    #[test]
    fn test_format_number_integer() {
        assert_eq!(format_number(-16.0), "-16");
        assert_eq!(format_number(0.0), "0");
        assert_eq!(format_number(100.0), "100");
    }

    #[test]
    fn test_format_number_decimal() {
        assert_eq!(format_number(-16.5), "-16.5");
        assert_eq!(format_number(0.5), "0.5");
    }

    #[test]
    fn test_format_y_label() {
        assert_eq!(format_y_label(-16.0, "", "°C"), "-16°C");
        assert_eq!(format_y_label(4.5, "$", ""), "$4.5");
    }

    #[test]
    fn test_canvas_blend() {
        let mut canvas = Canvas::new(10, 10);
        canvas.blend(5, 5, 0.5);
        assert_eq!(canvas.buf[5 * 10 + 5], 0.5);
        canvas.blend(5, 5, 0.3);
        assert_eq!(canvas.buf[5 * 10 + 5], 0.5); // max of 0.5 and 0.3
        canvas.blend(5, 5, 0.8);
        assert_eq!(canvas.buf[5 * 10 + 5], 0.8);
    }

    #[test]
    fn test_canvas_out_of_bounds() {
        let mut canvas = Canvas::new(10, 10);
        // Should not panic
        canvas.blend(100, 100, 1.0);
        canvas.blend(0, 100, 1.0);
    }

    #[test]
    fn test_render_chart_dimensions() {
        let chart = Chart {
            values: vec![1.0, 2.0, 3.0],
            height: Some(200),
            ..Default::default()
        };
        let (data, w, h) = render(&chart, 576, DitheringAlgorithm::Bayer);
        assert_eq!(w, 576);
        assert_eq!(h, 200);
        assert_eq!(data.len(), 72 * 200); // 576/8 = 72 bytes per row
    }

    #[test]
    fn test_render_chart_not_all_white() {
        let chart = Chart {
            values: vec![1.0, 5.0, 3.0, 7.0],
            labels: vec!["A".into(), "B".into(), "C".into(), "D".into()],
            height: Some(100),
            ..Default::default()
        };
        let (data, _, _) = render(&chart, 576, DitheringAlgorithm::Bayer);
        // Should have some black pixels
        assert!(data.iter().any(|&b| b != 0), "Chart should not be all white");
    }

    #[test]
    fn test_render_empty_values() {
        let chart = Chart {
            values: vec![],
            ..Default::default()
        };
        let (data, w, h) = render(&chart, 576, DitheringAlgorithm::Bayer);
        assert!(data.is_empty());
        assert_eq!(w, 0);
        assert_eq!(h, 0);
    }

    #[test]
    fn test_render_single_value() {
        let chart = Chart {
            values: vec![42.0],
            height: Some(100),
            ..Default::default()
        };
        let (data, w, h) = render(&chart, 576, DitheringAlgorithm::Bayer);
        assert_eq!(w, 576);
        assert_eq!(h, 100);
        assert!(!data.is_empty());
    }

    #[test]
    fn test_render_all_styles() {
        for style in [ChartStyle::Line, ChartStyle::Area, ChartStyle::Bar, ChartStyle::Dot] {
            let chart = Chart {
                style,
                values: vec![1.0, 3.0, 2.0, 5.0, 4.0],
                labels: vec!["A".into(), "B".into(), "C".into(), "D".into(), "E".into()],
                height: Some(150),
                y_suffix: Some("°C".into()),
                ..Default::default()
            };
            let (data, w, h) = render(&chart, 576, DitheringAlgorithm::Bayer);
            assert_eq!(w, 576);
            assert_eq!(h, 150);
            assert!(data.iter().any(|&b| b != 0), "Style {:?} should not be all white", style);
        }
    }

    #[test]
    fn test_render_with_title() {
        let chart = Chart {
            values: vec![1.0, 2.0, 3.0],
            title: Some("Test Chart".into()),
            height: Some(100),
            ..Default::default()
        };
        let (data, _, _) = render(&chart, 576, DitheringAlgorithm::Bayer);
        assert!(!data.is_empty());
    }
}
