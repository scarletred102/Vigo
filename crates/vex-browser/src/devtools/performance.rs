// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Performance panel — frame timing, layout/paint/script profiling.
//!
//! Records per-frame timing for each pipeline stage: parse, style
//! computation, layout, paint, and JavaScript execution. Provides a
//! stacked bar chart model and highlights slow frames (>16ms budget).

use std::time::Duration;

use vex_core::geometry::Rect;

/// The 16ms frame budget at 60 fps.
const FRAME_BUDGET: Duration = Duration::from_millis(16);

/// Maximum number of frame records kept.
const MAX_FRAME_RECORDS: usize = 600;

/// Pipeline stage measured per frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PipelineStage {
    /// HTML parsing.
    Parse,
    /// CSS cascade + style computation.
    Style,
    /// Layout (block, inline, flex, positioned).
    Layout,
    /// Paint (display list build + GPU submission).
    Paint,
    /// JavaScript execution.
    Script,
    /// Everything else (event handling, compositor, etc.).
    Other,
}

impl PipelineStage {
    /// Display label for the stage.
    pub fn label(self) -> &'static str {
        match self {
            Self::Parse => "Parse",
            Self::Style => "Style",
            Self::Layout => "Layout",
            Self::Paint => "Paint",
            Self::Script => "Script",
            Self::Other => "Other",
        }
    }

    /// All stages in display order (for stacked bars).
    pub const ALL: &'static [PipelineStage] = &[
        Self::Parse,
        Self::Style,
        Self::Layout,
        Self::Paint,
        Self::Script,
        Self::Other,
    ];
}

/// Timing data for a single frame.
#[derive(Debug, Clone, Copy)]
pub struct FrameRecord {
    /// Frame sequence number.
    pub frame_number: u64,
    /// Time spent in HTML parsing.
    pub parse: Duration,
    /// Time spent in style computation.
    pub style: Duration,
    /// Time spent in layout.
    pub layout: Duration,
    /// Time spent in paint.
    pub paint: Duration,
    /// Time spent in JavaScript execution.
    pub script: Duration,
    /// Time spent in other activities.
    pub other: Duration,
}

impl FrameRecord {
    /// Total frame time (sum of all stages).
    pub fn total(&self) -> Duration {
        self.parse + self.style + self.layout + self.paint + self.script + self.other
    }

    /// Whether this frame exceeded the 16ms budget.
    pub fn is_slow(&self) -> bool {
        self.total() > FRAME_BUDGET
    }

    /// Duration of a specific pipeline stage.
    pub fn stage_duration(&self, stage: PipelineStage) -> Duration {
        match stage {
            PipelineStage::Parse => self.parse,
            PipelineStage::Style => self.style,
            PipelineStage::Layout => self.layout,
            PipelineStage::Paint => self.paint,
            PipelineStage::Script => self.script,
            PipelineStage::Other => self.other,
        }
    }
}

/// A bar segment in the stacked bar chart.
#[derive(Debug, Clone)]
pub struct BarSegment {
    /// Pipeline stage this segment represents.
    pub stage: PipelineStage,
    /// Normalized height within the bar (0.0–1.0).
    pub height: f32,
    /// Raw duration for tooltip.
    pub duration: Duration,
}

/// Build stacked bar segments for a frame record.
///
/// The bar is normalized to the given `max_duration` so that
/// bars across frames are comparable.
pub fn build_bar_segments(frame: &FrameRecord, max_duration: Duration) -> Vec<BarSegment> {
    let max_ms = max_duration.as_secs_f64() * 1000.0;
    if max_ms <= 0.0 {
        return Vec::new();
    }

    PipelineStage::ALL
        .iter()
        .filter_map(|&stage| {
            let dur = frame.stage_duration(stage);
            let ms = dur.as_secs_f64() * 1000.0;
            if ms > 0.0 {
                Some(BarSegment {
                    stage,
                    height: (ms / max_ms) as f32,
                    duration: dur,
                })
            } else {
                None
            }
        })
        .collect()
}

/// Aggregated statistics over a range of frames.
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    /// Number of frames analyzed.
    pub frame_count: usize,
    /// Average total frame time.
    pub avg_frame_time: Duration,
    /// Maximum total frame time.
    pub max_frame_time: Duration,
    /// Number of slow frames (>16ms).
    pub slow_frame_count: usize,
    /// Average FPS (1000 / avg_ms).
    pub avg_fps: f64,
    /// Per-stage average times.
    pub avg_by_stage: [(PipelineStage, Duration); 6],
}

impl PerformanceStats {
    /// Format a human-readable summary.
    pub fn summary(&self) -> String {
        format!(
            "{:.1} FPS  |  avg {:.1}ms  |  max {:.1}ms  |  {} slow frames",
            self.avg_fps,
            self.avg_frame_time.as_secs_f64() * 1000.0,
            self.max_frame_time.as_secs_f64() * 1000.0,
            self.slow_frame_count,
        )
    }
}

/// State for the Performance panel.
#[derive(Debug, Clone)]
pub struct PerformanceState {
    /// Recorded frames in chronological order.
    frames: Vec<FrameRecord>,
    /// Whether recording is active.
    recording: bool,
    /// Currently selected frame index (for detail view).
    selected_frame: Option<usize>,
    /// Scroll offset (in frames from the right).
    scroll_offset: usize,
    /// Next frame number.
    next_frame: u64,
}

impl Default for PerformanceState {
    fn default() -> Self {
        Self {
            frames: Vec::new(),
            recording: false,
            selected_frame: None,
            scroll_offset: 0,
            next_frame: 1,
        }
    }
}

impl PerformanceState {
    /// Create a new empty performance state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Start recording frame timings.
    pub fn start_recording(&mut self) {
        self.recording = true;
    }

    /// Stop recording.
    pub fn stop_recording(&mut self) {
        self.recording = false;
    }

    /// Whether recording is active.
    pub fn is_recording(&self) -> bool {
        self.recording
    }

    /// Record a single frame's timing data.
    pub fn record_frame(&mut self, record: FrameRecord) {
        if !self.recording {
            return;
        }

        self.frames.push(record);

        // Trim old frames.
        while self.frames.len() > MAX_FRAME_RECORDS {
            self.frames.remove(0);
        }
    }

    /// Record a frame using individual stage durations.
    pub fn record_frame_timings(
        &mut self,
        parse: Duration,
        style: Duration,
        layout: Duration,
        paint: Duration,
        script: Duration,
        other: Duration,
    ) {
        let record = FrameRecord {
            frame_number: self.next_frame,
            parse,
            style,
            layout,
            paint,
            script,
            other,
        };
        self.next_frame += 1;
        self.record_frame(record);
    }

    /// All recorded frames.
    pub fn frames(&self) -> &[FrameRecord] {
        &self.frames
    }

    /// Number of recorded frames.
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Clear all recorded frames.
    pub fn clear(&mut self) {
        self.frames.clear();
        self.selected_frame = None;
        self.next_frame = 1;
    }

    /// Select a frame for detail view.
    pub fn select_frame(&mut self, index: usize) {
        if index < self.frames.len() {
            self.selected_frame = Some(index);
        }
    }

    /// Deselect the current frame.
    pub fn deselect_frame(&mut self) {
        self.selected_frame = None;
    }

    /// Currently selected frame.
    pub fn selected_frame(&self) -> Option<&FrameRecord> {
        self.selected_frame.and_then(|i| self.frames.get(i))
    }

    /// Maximum total frame time across all recorded frames.
    pub fn max_frame_time(&self) -> Duration {
        self.frames
            .iter()
            .map(|f| f.total())
            .max()
            .unwrap_or_default()
    }

    /// Compute aggregate statistics.
    pub fn stats(&self) -> PerformanceStats {
        let frame_count = self.frames.len();
        if frame_count == 0 {
            return PerformanceStats {
                frame_count: 0,
                avg_frame_time: Duration::ZERO,
                max_frame_time: Duration::ZERO,
                slow_frame_count: 0,
                avg_fps: 0.0,
                avg_by_stage: PipelineStage::ALL
                    .iter()
                    .map(|&s| (s, Duration::ZERO))
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap_or([
                        (PipelineStage::Parse, Duration::ZERO),
                        (PipelineStage::Style, Duration::ZERO),
                        (PipelineStage::Layout, Duration::ZERO),
                        (PipelineStage::Paint, Duration::ZERO),
                        (PipelineStage::Script, Duration::ZERO),
                        (PipelineStage::Other, Duration::ZERO),
                    ]),
            };
        }

        let total_time: Duration = self.frames.iter().map(|f| f.total()).sum();
        let avg_frame_time = total_time / frame_count as u32;
        let max_frame_time = self.max_frame_time();
        let slow_frame_count = self.frames.iter().filter(|f| f.is_slow()).count();
        let avg_ms = avg_frame_time.as_secs_f64() * 1000.0;
        let avg_fps = if avg_ms > 0.0 { 1000.0 / avg_ms } else { 0.0 };

        let n = frame_count as u32;
        let avg_by_stage = [
            (
                PipelineStage::Parse,
                self.frames.iter().map(|f| f.parse).sum::<Duration>() / n,
            ),
            (
                PipelineStage::Style,
                self.frames.iter().map(|f| f.style).sum::<Duration>() / n,
            ),
            (
                PipelineStage::Layout,
                self.frames.iter().map(|f| f.layout).sum::<Duration>() / n,
            ),
            (
                PipelineStage::Paint,
                self.frames.iter().map(|f| f.paint).sum::<Duration>() / n,
            ),
            (
                PipelineStage::Script,
                self.frames.iter().map(|f| f.script).sum::<Duration>() / n,
            ),
            (
                PipelineStage::Other,
                self.frames.iter().map(|f| f.other).sum::<Duration>() / n,
            ),
        ];

        PerformanceStats {
            frame_count,
            avg_frame_time,
            max_frame_time,
            slow_frame_count,
            avg_fps,
            avg_by_stage,
        }
    }

    /// Scroll offset.
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Set scroll offset.
    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_offset = offset;
    }
}

/// Layout regions for the performance panel.
#[derive(Debug, Clone)]
pub struct PerformanceLayout {
    /// Toolbar with record/stop/clear buttons.
    pub toolbar: Rect,
    /// The stacked bar chart area.
    pub chart: Rect,
    /// Summary stats bar.
    pub stats_bar: Rect,
    /// Detail panel for selected frame (if any).
    pub detail: Option<Rect>,
}

/// Toolbar height.
const TOOLBAR_HEIGHT: f32 = 32.0;
/// Stats bar height.
const STATS_BAR_HEIGHT: f32 = 24.0;
/// Detail panel height.
const DETAIL_HEIGHT: f32 = 100.0;

/// Compute the performance panel layout.
pub fn compute_performance_layout(body: Rect, has_selection: bool) -> PerformanceLayout {
    let toolbar = Rect::new(
        body.origin.x,
        body.origin.y,
        body.size.width,
        TOOLBAR_HEIGHT,
    );
    let stats_bar = Rect::new(
        body.origin.x,
        body.origin.y + body.size.height - STATS_BAR_HEIGHT,
        body.size.width,
        STATS_BAR_HEIGHT,
    );

    let chart_top = toolbar.origin.y + TOOLBAR_HEIGHT;
    let available_h = (stats_bar.origin.y - chart_top).max(0.0);

    let (chart, detail) = if has_selection {
        let detail_h = DETAIL_HEIGHT.min(available_h * 0.4);
        let chart_h = (available_h - detail_h).max(0.0);
        let chart = Rect::new(body.origin.x, chart_top, body.size.width, chart_h);
        let detail_rect = Rect::new(
            body.origin.x,
            chart_top + chart_h,
            body.size.width,
            detail_h,
        );
        (chart, Some(detail_rect))
    } else {
        let chart = Rect::new(body.origin.x, chart_top, body.size.width, available_h);
        (chart, None)
    };

    PerformanceLayout {
        toolbar,
        chart,
        stats_bar,
        detail,
    }
}

/// Width of each frame bar in the chart.
pub const BAR_WIDTH: f32 = 4.0;
/// Gap between bars.
pub const BAR_GAP: f32 = 1.0;

/// Number of bars that fit in a given chart width.
pub fn visible_bar_count(chart_width: f32) -> usize {
    if BAR_WIDTH + BAR_GAP <= 0.0 {
        return 0;
    }
    (chart_width / (BAR_WIDTH + BAR_GAP)).floor() as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_frame(ms: u64) -> FrameRecord {
        let per_stage = Duration::from_micros(ms * 1000 / 5);
        FrameRecord {
            frame_number: 1,
            parse: per_stage,
            style: per_stage,
            layout: per_stage,
            paint: per_stage,
            script: per_stage,
            other: Duration::ZERO,
        }
    }

    // ── FrameRecord ──

    #[test]
    fn frame_total() {
        let frame = FrameRecord {
            frame_number: 1,
            parse: Duration::from_millis(2),
            style: Duration::from_millis(3),
            layout: Duration::from_millis(4),
            paint: Duration::from_millis(5),
            script: Duration::from_millis(1),
            other: Duration::from_millis(1),
        };
        assert_eq!(frame.total(), Duration::from_millis(16));
    }

    #[test]
    fn frame_slow_detection() {
        let fast = sample_frame(10);
        assert!(!fast.is_slow());

        let slow = sample_frame(25);
        assert!(slow.is_slow());
    }

    #[test]
    fn stage_duration_lookup() {
        let frame = FrameRecord {
            frame_number: 1,
            parse: Duration::from_millis(1),
            style: Duration::from_millis(2),
            layout: Duration::from_millis(3),
            paint: Duration::from_millis(4),
            script: Duration::from_millis(5),
            other: Duration::from_millis(6),
        };
        assert_eq!(
            frame.stage_duration(PipelineStage::Parse),
            Duration::from_millis(1)
        );
        assert_eq!(
            frame.stage_duration(PipelineStage::Script),
            Duration::from_millis(5)
        );
    }

    // ── Bar segments ──

    #[test]
    fn bar_segments_normalized() {
        let frame = FrameRecord {
            frame_number: 1,
            parse: Duration::from_millis(5),
            style: Duration::from_millis(5),
            layout: Duration::from_millis(0),
            paint: Duration::from_millis(5),
            script: Duration::from_millis(5),
            other: Duration::from_millis(0),
        };
        let segments = build_bar_segments(&frame, Duration::from_millis(20));
        assert_eq!(segments.len(), 4); // Two zero-duration stages excluded.
        let total_height: f32 = segments.iter().map(|s| s.height).sum();
        assert!((total_height - 1.0).abs() < 0.01);
    }

    #[test]
    fn bar_segments_empty_for_zero() {
        let frame = sample_frame(0);
        let segments = build_bar_segments(&frame, Duration::ZERO);
        assert!(segments.is_empty());
    }

    // ── PerformanceState ──

    #[test]
    fn recording_lifecycle() {
        let mut state = PerformanceState::new();
        assert!(!state.is_recording());

        state.start_recording();
        assert!(state.is_recording());

        state.record_frame(sample_frame(10));
        assert_eq!(state.frame_count(), 1);

        state.stop_recording();
        state.record_frame(sample_frame(10)); // Ignored.
        assert_eq!(state.frame_count(), 1);
    }

    #[test]
    fn record_frame_timings() {
        let mut state = PerformanceState::new();
        state.start_recording();
        state.record_frame_timings(
            Duration::from_millis(1),
            Duration::from_millis(2),
            Duration::from_millis(3),
            Duration::from_millis(4),
            Duration::from_millis(5),
            Duration::from_millis(1),
        );
        assert_eq!(state.frame_count(), 1);
        assert_eq!(state.frames()[0].frame_number, 1);
    }

    #[test]
    fn clear_frames() {
        let mut state = PerformanceState::new();
        state.start_recording();
        state.record_frame(sample_frame(10));
        state.clear();
        assert_eq!(state.frame_count(), 0);
    }

    #[test]
    fn max_frames_trimmed() {
        let mut state = PerformanceState::new();
        state.start_recording();
        for _ in 0..700 {
            state.record_frame(sample_frame(10));
        }
        assert!(state.frame_count() <= MAX_FRAME_RECORDS);
    }

    #[test]
    fn select_frame() {
        let mut state = PerformanceState::new();
        state.start_recording();
        state.record_frame(sample_frame(10));
        state.record_frame(sample_frame(20));

        state.select_frame(1);
        assert!(state.selected_frame().is_some());
        assert!(state.selected_frame().unwrap().is_slow());

        state.deselect_frame();
        assert!(state.selected_frame().is_none());
    }

    #[test]
    fn stats_computation() {
        let mut state = PerformanceState::new();
        state.start_recording();
        // 10ms frames.
        for _ in 0..10 {
            state.record_frame(sample_frame(10));
        }
        // 1 slow frame (20ms).
        state.record_frame(sample_frame(20));

        let stats = state.stats();
        assert_eq!(stats.frame_count, 11);
        assert_eq!(stats.slow_frame_count, 1);
        assert!(stats.avg_fps > 0.0);
        assert!(stats.max_frame_time >= Duration::from_millis(20));
    }

    #[test]
    fn stats_empty() {
        let state = PerformanceState::new();
        let stats = state.stats();
        assert_eq!(stats.frame_count, 0);
        assert_eq!(stats.avg_fps, 0.0);
    }

    #[test]
    fn stats_summary_format() {
        let mut state = PerformanceState::new();
        state.start_recording();
        for _ in 0..5 {
            state.record_frame(sample_frame(10));
        }
        let summary = state.stats().summary();
        assert!(summary.contains("FPS"));
        assert!(summary.contains("avg"));
        assert!(summary.contains("slow frames"));
    }

    // ── Pipeline Stage ──

    #[test]
    fn pipeline_stage_labels() {
        assert_eq!(PipelineStage::Parse.label(), "Parse");
        assert_eq!(PipelineStage::Paint.label(), "Paint");
        assert_eq!(PipelineStage::Script.label(), "Script");
    }

    // ── Layout ──

    #[test]
    fn performance_layout_without_selection() {
        let body = Rect::new(0.0, 0.0, 800.0, 400.0);
        let layout = compute_performance_layout(body, false);
        assert!(layout.detail.is_none());
        assert!(layout.chart.size.height > 0.0);
    }

    #[test]
    fn performance_layout_with_selection() {
        let body = Rect::new(0.0, 0.0, 800.0, 400.0);
        let layout = compute_performance_layout(body, true);
        assert!(layout.detail.is_some());
    }

    #[test]
    fn visible_bar_count_calc() {
        // 800px / (4 + 1) = 160 bars.
        assert_eq!(visible_bar_count(800.0), 160);
    }
}
