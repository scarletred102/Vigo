// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! CSS Animation & Transition Engine.
//!
//! Drives CSS animations (`@keyframes`) and transitions by computing
//! interpolated property values at each frame.
//!
//! The engine is tick-based: call `tick(dt)` each frame with the elapsed
//! time, and it updates all running animations/transitions.

use std::collections::HashMap;

use vex_core::VexId;

use crate::values::animation::{
    AnimationDirection, AnimationFillMode, AnimationIterationCount, AnimationPlayState,
    KeyframeRule, TimingFunction, TransitionProperty,
};

// ── AnimationState ───────────────────────────────────────────────────────────

/// Tracks the runtime state of a single CSS animation on an element.
#[derive(Debug, Clone)]
pub struct AnimationInstance {
    /// The name of the @keyframes rule.
    pub name: String,
    /// Total duration in seconds.
    pub duration: f32,
    /// Delay before start in seconds.
    pub delay: f32,
    /// Easing function.
    pub timing_function: TimingFunction,
    /// How many times to iterate.
    pub iteration_count: AnimationIterationCount,
    /// Play direction.
    pub direction: AnimationDirection,
    /// Fill mode.
    pub fill_mode: AnimationFillMode,
    /// Current play state.
    pub play_state: AnimationPlayState,
    /// Elapsed time in seconds (including delay).
    pub elapsed: f32,
    /// Current iteration index (0-based).
    pub current_iteration: u32,
    /// Whether the animation has finished.
    pub finished: bool,
}

impl AnimationInstance {
    /// Create a new animation instance.
    pub fn new(
        name: &str,
        duration: f32,
        delay: f32,
        timing_function: TimingFunction,
        iteration_count: AnimationIterationCount,
        direction: AnimationDirection,
        fill_mode: AnimationFillMode,
    ) -> Self {
        Self {
            name: name.to_string(),
            duration,
            delay,
            timing_function,
            iteration_count,
            direction,
            fill_mode,
            play_state: AnimationPlayState::Running,
            elapsed: 0.0,
            current_iteration: 0,
            finished: false,
        }
    }

    /// Advance the animation by `dt` seconds. Returns the current progress [0, 1].
    pub fn tick(&mut self, dt: f32) -> f32 {
        if self.finished || self.play_state == AnimationPlayState::Paused {
            return self.current_progress();
        }

        self.elapsed += dt;

        // Still in delay phase
        if self.elapsed < self.delay {
            return match self.fill_mode {
                AnimationFillMode::Backwards | AnimationFillMode::Both => 0.0,
                _ => f32::NAN, // No value to apply
            };
        }

        let active_time = self.elapsed - self.delay;

        if self.duration <= 0.0 {
            self.finished = true;
            return 1.0;
        }

        let raw_iteration = active_time / self.duration;

        // Check if we've exceeded iteration count
        let max_iterations = match self.iteration_count {
            AnimationIterationCount::Number(n) => n,
            AnimationIterationCount::Infinite => f32::INFINITY,
        };

        if raw_iteration >= max_iterations {
            self.finished = true;
            self.current_iteration = max_iterations.ceil() as u32;
            return match self.fill_mode {
                AnimationFillMode::Forwards | AnimationFillMode::Both => {
                    self.direction_adjusted_progress(1.0, max_iterations.ceil() as u32 - 1)
                }
                _ => f32::NAN,
            };
        }

        let iteration_index = raw_iteration.floor() as u32;
        self.current_iteration = iteration_index;
        let local_progress = raw_iteration - raw_iteration.floor();

        self.direction_adjusted_progress(local_progress, iteration_index)
    }

    /// Get the current progress without advancing time.
    pub fn current_progress(&self) -> f32 {
        if self.duration <= 0.0 {
            return 1.0;
        }
        let active_time = (self.elapsed - self.delay).max(0.0);
        let raw_iteration = active_time / self.duration;
        let local_progress = raw_iteration - raw_iteration.floor();
        self.direction_adjusted_progress(local_progress, self.current_iteration)
    }

    /// Apply direction to progress.
    fn direction_adjusted_progress(&self, progress: f32, iteration: u32) -> f32 {
        let reversed = match self.direction {
            AnimationDirection::Normal => false,
            AnimationDirection::Reverse => true,
            AnimationDirection::Alternate => iteration % 2 == 1,
            AnimationDirection::AlternateReverse => iteration % 2 == 0,
        };

        let directed = if reversed { 1.0 - progress } else { progress };

        self.timing_function.evaluate(directed)
    }
}

// ── TransitionInstance ───────────────────────────────────────────────────────

/// Tracks the runtime state of a single CSS transition.
#[derive(Debug, Clone)]
pub struct TransitionInstance {
    /// The property being transitioned.
    pub property: String,
    /// Starting value (as f32 for numeric props).
    pub start_value: f32,
    /// Target value.
    pub end_value: f32,
    /// Duration in seconds.
    pub duration: f32,
    /// Delay before start in seconds.
    pub delay: f32,
    /// Easing function.
    pub timing_function: TimingFunction,
    /// Elapsed time.
    pub elapsed: f32,
    /// Whether the transition has completed.
    pub finished: bool,
}

impl TransitionInstance {
    pub fn new(
        property: &str,
        start_value: f32,
        end_value: f32,
        duration: f32,
        delay: f32,
        timing_function: TimingFunction,
    ) -> Self {
        Self {
            property: property.to_string(),
            start_value,
            end_value,
            duration,
            delay,
            timing_function,
            elapsed: 0.0,
            finished: false,
        }
    }

    /// Advance the transition by `dt` seconds. Returns the current interpolated value.
    pub fn tick(&mut self, dt: f32) -> f32 {
        if self.finished {
            return self.end_value;
        }

        self.elapsed += dt;

        if self.elapsed < self.delay {
            return self.start_value;
        }

        let active_time = self.elapsed - self.delay;

        if self.duration <= 0.0 || active_time >= self.duration {
            self.finished = true;
            return self.end_value;
        }

        let raw_progress = active_time / self.duration;
        let eased = self.timing_function.evaluate(raw_progress);

        self.start_value + (self.end_value - self.start_value) * eased
    }

    /// Current interpolated value without advancing.
    pub fn current_value(&self) -> f32 {
        if self.finished || self.duration <= 0.0 {
            return self.end_value;
        }
        let active_time = (self.elapsed - self.delay).max(0.0);
        if active_time >= self.duration {
            return self.end_value;
        }
        let raw_progress = active_time / self.duration;
        let eased = self.timing_function.evaluate(raw_progress);
        self.start_value + (self.end_value - self.start_value) * eased
    }
}

// ── AnimationEngine ──────────────────────────────────────────────────────────

/// The CSS animation engine. Manages all running animations and transitions.
#[derive(Debug, Default)]
pub struct AnimationEngine {
    /// @keyframes rules indexed by name.
    pub keyframe_rules: HashMap<String, KeyframeRule>,
    /// Active animations, keyed by (element_id, animation_name).
    pub animations: HashMap<(VexId, String), AnimationInstance>,
    /// Active transitions, keyed by (element_id, property_name).
    pub transitions: HashMap<(VexId, String), TransitionInstance>,
}

impl AnimationEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a @keyframes rule.
    pub fn add_keyframes(&mut self, rule: KeyframeRule) {
        self.keyframe_rules.insert(rule.name.clone(), rule);
    }

    /// Start an animation on an element.
    pub fn start_animation(&mut self, element_id: VexId, instance: AnimationInstance) {
        let key = (element_id, instance.name.clone());
        self.animations.insert(key, instance);
    }

    /// Start a transition on an element.
    pub fn start_transition(&mut self, element_id: VexId, instance: TransitionInstance) {
        let key = (element_id, instance.property.clone());
        self.transitions.insert(key, instance);
    }

    /// Check whether a property should be transitioned on the given element.
    pub fn should_transition(
        &self,
        _element_id: VexId,
        property: &str,
        transition_prop: &TransitionProperty,
    ) -> bool {
        match transition_prop {
            TransitionProperty::All => is_animatable(property),
            TransitionProperty::None => false,
            TransitionProperty::Property(name) => name == property,
        }
    }

    /// Tick all animations and transitions by `dt` seconds.
    /// Returns a set of element IDs that were updated.
    pub fn tick(&mut self, dt: f32) -> Vec<VexId> {
        let mut updated = Vec::new();

        for ((element_id, _), anim) in &mut self.animations {
            if !anim.finished {
                anim.tick(dt);
                if !updated.contains(element_id) {
                    updated.push(*element_id);
                }
            }
        }

        for ((element_id, _), trans) in &mut self.transitions {
            if !trans.finished {
                trans.tick(dt);
                if !updated.contains(element_id) {
                    updated.push(*element_id);
                }
            }
        }

        // Remove finished transitions
        self.transitions.retain(|_, t| !t.finished);

        updated
    }

    /// Get the current animation progress for an element's animation.
    pub fn animation_progress(&self, element_id: VexId, name: &str) -> Option<f32> {
        self.animations
            .get(&(element_id, name.to_string()))
            .map(|a| a.current_progress())
    }

    /// Get the current transition value for an element's property.
    pub fn transition_value(&self, element_id: VexId, property: &str) -> Option<f32> {
        self.transitions
            .get(&(element_id, property.to_string()))
            .map(|t| t.current_value())
    }

    /// Get the keyframes rule for a given animation name.
    pub fn get_keyframes(&self, name: &str) -> Option<&KeyframeRule> {
        self.keyframe_rules.get(name)
    }

    /// Remove all animations and transitions for an element.
    pub fn remove_element(&mut self, element_id: VexId) {
        self.animations.retain(|(eid, _), _| *eid != element_id);
        self.transitions.retain(|(eid, _), _| *eid != element_id);
    }

    /// Number of active animations.
    pub fn active_animation_count(&self) -> usize {
        self.animations.values().filter(|a| !a.finished).count()
    }

    /// Number of active transitions.
    pub fn active_transition_count(&self) -> usize {
        self.transitions.len()
    }

    /// Interpolate a numeric value between two keyframes.
    pub fn interpolate_f32(from: f32, to: f32, progress: f32) -> f32 {
        from + (to - from) * progress
    }
}

/// Check whether a CSS property is animatable.
pub fn is_animatable(property: &str) -> bool {
    matches!(
        property,
        "opacity"
            | "width"
            | "height"
            | "margin-top"
            | "margin-right"
            | "margin-bottom"
            | "margin-left"
            | "padding-top"
            | "padding-right"
            | "padding-bottom"
            | "padding-left"
            | "border-top-width"
            | "border-right-width"
            | "border-bottom-width"
            | "border-left-width"
            | "top"
            | "right"
            | "bottom"
            | "left"
            | "font-size"
            | "line-height"
            | "flex-grow"
            | "flex-shrink"
            | "color"
            | "background-color"
            | "border-top-color"
            | "border-right-color"
            | "border-bottom-color"
            | "border-left-color"
            | "transform"
            | "z-index"
            | "column-gap"
            | "row-gap"
    )
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_id(n: u32) -> VexId {
        VexId::new(n)
    }

    // ── AnimationInstance ────────────────────────────────────────

    #[test]
    fn animation_basic_progress() {
        let mut anim = AnimationInstance::new(
            "fade",
            1.0, // 1 second
            0.0, // no delay
            TimingFunction::Linear,
            AnimationIterationCount::Number(1.0),
            AnimationDirection::Normal,
            AnimationFillMode::None,
        );
        let p = anim.tick(0.5);
        assert!((p - 0.5).abs() < 0.01);
    }

    #[test]
    fn animation_with_delay() {
        let mut anim = AnimationInstance::new(
            "fade",
            1.0,
            0.5, // 0.5s delay
            TimingFunction::Linear,
            AnimationIterationCount::Number(1.0),
            AnimationDirection::Normal,
            AnimationFillMode::None,
        );
        // During delay
        let p = anim.tick(0.3);
        assert!(p.is_nan()); // No fill mode

        // After delay starts
        let p = anim.tick(0.7); // elapsed = 1.0, active = 0.5
        assert!((p - 0.5).abs() < 0.01);
    }

    #[test]
    fn animation_with_backwards_fill() {
        let mut anim = AnimationInstance::new(
            "fade",
            1.0,
            0.5,
            TimingFunction::Linear,
            AnimationIterationCount::Number(1.0),
            AnimationDirection::Normal,
            AnimationFillMode::Backwards,
        );
        let p = anim.tick(0.2);
        assert!((p - 0.0).abs() < 0.01); // Backwards fill = 0
    }

    #[test]
    fn animation_finishes() {
        let mut anim = AnimationInstance::new(
            "fade",
            1.0,
            0.0,
            TimingFunction::Linear,
            AnimationIterationCount::Number(1.0),
            AnimationDirection::Normal,
            AnimationFillMode::Forwards,
        );
        anim.tick(0.5);
        anim.tick(0.6); // elapsed = 1.1 > duration
        assert!(anim.finished);
    }

    #[test]
    fn animation_infinite() {
        let mut anim = AnimationInstance::new(
            "spin",
            1.0,
            0.0,
            TimingFunction::Linear,
            AnimationIterationCount::Infinite,
            AnimationDirection::Normal,
            AnimationFillMode::None,
        );
        anim.tick(2.5);
        assert!(!anim.finished);
        assert_eq!(anim.current_iteration, 2);
    }

    #[test]
    fn animation_alternate() {
        let mut anim = AnimationInstance::new(
            "bounce",
            1.0,
            0.0,
            TimingFunction::Linear,
            AnimationIterationCount::Number(4.0),
            AnimationDirection::Alternate,
            AnimationFillMode::None,
        );
        // First iteration: normal (0→1)
        let p = anim.tick(0.5);
        assert!((p - 0.5).abs() < 0.01);

        // Second iteration: reversed (1→0)
        let p = anim.tick(1.0); // elapsed = 1.5, iteration 1
        assert!((p - 0.5).abs() < 0.01);
    }

    #[test]
    fn animation_reverse() {
        let mut anim = AnimationInstance::new(
            "slide",
            1.0,
            0.0,
            TimingFunction::Linear,
            AnimationIterationCount::Number(1.0),
            AnimationDirection::Reverse,
            AnimationFillMode::None,
        );
        let p = anim.tick(0.25);
        assert!((p - 0.75).abs() < 0.01);
    }

    #[test]
    fn animation_paused() {
        let mut anim = AnimationInstance::new(
            "fade",
            1.0,
            0.0,
            TimingFunction::Linear,
            AnimationIterationCount::Number(1.0),
            AnimationDirection::Normal,
            AnimationFillMode::None,
        );
        anim.tick(0.3);
        anim.play_state = AnimationPlayState::Paused;
        let p1 = anim.current_progress();
        anim.tick(0.5); // Should not advance
        let p2 = anim.current_progress();
        assert!((p1 - p2).abs() < 0.001);
    }

    // ── TransitionInstance ───────────────────────────────────────

    #[test]
    fn transition_basic() {
        let mut trans =
            TransitionInstance::new("opacity", 0.0, 1.0, 1.0, 0.0, TimingFunction::Linear);
        let v = trans.tick(0.5);
        assert!((v - 0.5).abs() < 0.01);
    }

    #[test]
    fn transition_with_delay() {
        let mut trans =
            TransitionInstance::new("opacity", 0.0, 1.0, 1.0, 0.5, TimingFunction::Linear);
        let v = trans.tick(0.3);
        assert!((v - 0.0).abs() < 0.01); // Still in delay

        let v = trans.tick(0.7); // elapsed = 1.0, active = 0.5
        assert!((v - 0.5).abs() < 0.01);
    }

    #[test]
    fn transition_finishes() {
        let mut trans =
            TransitionInstance::new("width", 100.0, 200.0, 0.5, 0.0, TimingFunction::Linear);
        let v = trans.tick(0.6);
        assert!((v - 200.0).abs() < 0.01);
        assert!(trans.finished);
    }

    #[test]
    fn transition_ease_in() {
        let mut trans =
            TransitionInstance::new("opacity", 0.0, 1.0, 1.0, 0.0, TimingFunction::EaseIn);
        let v = trans.tick(0.5);
        // Ease-in is slow at start, so at 50% time, should be < 50% value
        assert!(v < 0.5);
    }

    // ── AnimationEngine ─────────────────────────────────────────

    #[test]
    fn engine_tick_animations() {
        let mut engine = AnimationEngine::new();
        let anim = AnimationInstance::new(
            "fade",
            1.0,
            0.0,
            TimingFunction::Linear,
            AnimationIterationCount::Number(1.0),
            AnimationDirection::Normal,
            AnimationFillMode::None,
        );
        engine.start_animation(test_id(1), anim);
        let updated = engine.tick(0.5);
        assert!(updated.contains(&test_id(1)));
        assert_eq!(engine.active_animation_count(), 1);
    }

    #[test]
    fn engine_tick_transitions() {
        let mut engine = AnimationEngine::new();
        let trans = TransitionInstance::new("opacity", 0.0, 1.0, 0.3, 0.0, TimingFunction::Linear);
        engine.start_transition(test_id(2), trans);
        assert_eq!(engine.active_transition_count(), 1);

        // Tick past completion
        engine.tick(0.5);
        assert_eq!(engine.active_transition_count(), 0); // Finished, removed
    }

    #[test]
    fn engine_remove_element() {
        let mut engine = AnimationEngine::new();
        let anim = AnimationInstance::new(
            "fade",
            1.0,
            0.0,
            TimingFunction::Linear,
            AnimationIterationCount::Number(1.0),
            AnimationDirection::Normal,
            AnimationFillMode::None,
        );
        engine.start_animation(test_id(3), anim);
        engine.remove_element(test_id(3));
        assert_eq!(engine.active_animation_count(), 0);
    }

    #[test]
    fn engine_keyframes() {
        let mut engine = AnimationEngine::new();
        let mut rule = KeyframeRule::new("fadeIn");
        rule.add_keyframe(0.0, vec![]);
        rule.add_keyframe(1.0, vec![]);
        engine.add_keyframes(rule);
        assert!(engine.get_keyframes("fadeIn").is_some());
        assert!(engine.get_keyframes("missing").is_none());
    }

    #[test]
    fn engine_animation_progress() {
        let mut engine = AnimationEngine::new();
        let anim = AnimationInstance::new(
            "slide",
            2.0,
            0.0,
            TimingFunction::Linear,
            AnimationIterationCount::Number(1.0),
            AnimationDirection::Normal,
            AnimationFillMode::None,
        );
        engine.start_animation(test_id(5), anim);
        engine.tick(1.0);
        let progress = engine.animation_progress(test_id(5), "slide");
        assert!(progress.is_some());
        assert!((progress.unwrap() - 0.5).abs() < 0.01);
    }

    #[test]
    fn engine_transition_value() {
        let mut engine = AnimationEngine::new();
        let trans = TransitionInstance::new("opacity", 0.0, 1.0, 2.0, 0.0, TimingFunction::Linear);
        engine.start_transition(test_id(6), trans);
        engine.tick(1.0);
        let val = engine.transition_value(test_id(6), "opacity");
        assert!(val.is_some());
        assert!((val.unwrap() - 0.5).abs() < 0.01);
    }

    #[test]
    fn engine_interpolate() {
        assert!((AnimationEngine::interpolate_f32(0.0, 100.0, 0.5) - 50.0).abs() < 0.01);
        assert!((AnimationEngine::interpolate_f32(10.0, 20.0, 0.25) - 12.5).abs() < 0.01);
    }

    #[test]
    fn is_animatable_check() {
        assert!(is_animatable("opacity"));
        assert!(is_animatable("width"));
        assert!(is_animatable("transform"));
        assert!(!is_animatable("display"));
        assert!(!is_animatable("position"));
    }

    #[test]
    fn engine_should_transition() {
        let engine = AnimationEngine::new();
        assert!(engine.should_transition(test_id(1), "opacity", &TransitionProperty::All));
        assert!(!engine.should_transition(test_id(1), "display", &TransitionProperty::All));
        assert!(!engine.should_transition(test_id(1), "opacity", &TransitionProperty::None));
        assert!(engine.should_transition(
            test_id(1),
            "opacity",
            &TransitionProperty::Property("opacity".to_string())
        ));
    }
}
