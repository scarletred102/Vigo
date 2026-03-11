// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Gamepad API implementation.
//!
//! Provides an interface for interacting with game controllers,
//! following the W3C Gamepad specification.

use std::collections::HashMap;

// ── Types ────────────────────────────────────────────────────────────────────

/// Button state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GamepadButton {
    /// Whether the button is pressed.
    pub pressed: bool,
    /// Whether the button is touched (for controllers with touch-sensitive buttons).
    pub touched: bool,
    /// Analog value (0.0 to 1.0).
    pub value: f64,
}

impl GamepadButton {
    pub fn new() -> Self {
        Self {
            pressed: false,
            touched: false,
            value: 0.0,
        }
    }

    pub fn pressed(value: f64) -> Self {
        Self {
            pressed: true,
            touched: true,
            value,
        }
    }
}

impl Default for GamepadButton {
    fn default() -> Self {
        Self::new()
    }
}

/// Standard gamepad mapping (indices match the spec).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StandardButton {
    /// Bottom face button (A / Cross).
    A = 0,
    /// Right face button (B / Circle).
    B = 1,
    /// Left face button (X / Square).
    X = 2,
    /// Top face button (Y / Triangle).
    Y = 3,
    /// Left shoulder.
    LeftBumper = 4,
    /// Right shoulder.
    RightBumper = 5,
    /// Left trigger.
    LeftTrigger = 6,
    /// Right trigger.
    RightTrigger = 7,
    /// Select / Back / Share.
    Select = 8,
    /// Start / Menu.
    Start = 9,
    /// Left stick press.
    LeftStick = 10,
    /// Right stick press.
    RightStick = 11,
    /// D-pad up.
    DPadUp = 12,
    /// D-pad down.
    DPadDown = 13,
    /// D-pad left.
    DPadLeft = 14,
    /// D-pad right.
    DPadRight = 15,
    /// Home / Guide.
    Home = 16,
}

impl StandardButton {
    pub fn index(&self) -> usize {
        *self as usize
    }
}

/// Gamepad mapping type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GamepadMappingType {
    /// Standard mapping (known layout).
    #[default]
    Standard,
    /// No mapping — buttons/axes are in vendor order.
    None,
}

impl GamepadMappingType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::None => "",
        }
    }
}

/// A connected gamepad.
#[derive(Debug, Clone)]
pub struct Gamepad {
    /// Unique index for this gamepad.
    pub index: u32,
    /// Human-readable identifier.
    pub id: String,
    /// Whether the gamepad is connected.
    pub connected: bool,
    /// Mapping type.
    pub mapping: GamepadMappingType,
    /// Button states.
    pub buttons: Vec<GamepadButton>,
    /// Axis values (-1.0 to 1.0).
    pub axes: Vec<f64>,
    /// Timestamp of last update (in ms).
    pub timestamp: f64,
}

impl Gamepad {
    /// Create a new standard gamepad with 17 buttons and 4 axes.
    pub fn new_standard(index: u32, id: &str) -> Self {
        Self {
            index,
            id: id.to_string(),
            connected: true,
            mapping: GamepadMappingType::Standard,
            buttons: vec![GamepadButton::new(); 17],
            axes: vec![0.0; 4],
            timestamp: 0.0,
        }
    }

    /// Update a button state.
    pub fn set_button(&mut self, index: usize, button: GamepadButton) {
        if index < self.buttons.len() {
            self.buttons[index] = button;
        }
    }

    /// Update an axis value.
    pub fn set_axis(&mut self, index: usize, value: f64) {
        if index < self.axes.len() {
            self.axes[index] = value.clamp(-1.0, 1.0);
        }
    }

    /// Get a button by standard mapping.
    pub fn button(&self, button: StandardButton) -> &GamepadButton {
        &self.buttons[button.index()]
    }

    /// Get left stick X axis (axis 0).
    pub fn left_stick_x(&self) -> f64 {
        self.axes.first().copied().unwrap_or(0.0)
    }

    /// Get left stick Y axis (axis 1).
    pub fn left_stick_y(&self) -> f64 {
        self.axes.get(1).copied().unwrap_or(0.0)
    }

    /// Get right stick X axis (axis 2).
    pub fn right_stick_x(&self) -> f64 {
        self.axes.get(2).copied().unwrap_or(0.0)
    }

    /// Get right stick Y axis (axis 3).
    pub fn right_stick_y(&self) -> f64 {
        self.axes.get(3).copied().unwrap_or(0.0)
    }
}

/// Gamepad event type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadEventType {
    Connected,
    Disconnected,
}

/// Manages connected gamepads.
#[derive(Debug, Default)]
pub struct GamepadManager {
    gamepads: HashMap<u32, Gamepad>,
    next_index: u32,
}

impl GamepadManager {
    pub fn new() -> Self {
        Self {
            gamepads: HashMap::new(),
            next_index: 0,
        }
    }

    /// Connect a new gamepad. Returns the gamepad index.
    pub fn connect(&mut self, id: &str) -> u32 {
        let index = self.next_index;
        self.next_index += 1;
        let gamepad = Gamepad::new_standard(index, id);
        self.gamepads.insert(index, gamepad);
        index
    }

    /// Disconnect a gamepad.
    pub fn disconnect(&mut self, index: u32) -> bool {
        if let Some(gp) = self.gamepads.get_mut(&index) {
            gp.connected = false;
            true
        } else {
            false
        }
    }

    /// Get a gamepad by index.
    pub fn get(&self, index: u32) -> Option<&Gamepad> {
        self.gamepads.get(&index).filter(|g| g.connected)
    }

    /// Get a mutable gamepad by index.
    pub fn get_mut(&mut self, index: u32) -> Option<&mut Gamepad> {
        self.gamepads.get_mut(&index).filter(|g| g.connected)
    }

    /// Get all connected gamepads.
    pub fn get_gamepads(&self) -> Vec<Option<&Gamepad>> {
        let max_index = self
            .gamepads
            .keys()
            .max()
            .copied()
            .map(|i| i + 1)
            .unwrap_or(0);
        (0..max_index)
            .map(|i| self.gamepads.get(&i).filter(|g| g.connected))
            .collect()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gamepad_standard() {
        let gp = Gamepad::new_standard(0, "Xbox Controller");
        assert_eq!(gp.buttons.len(), 17);
        assert_eq!(gp.axes.len(), 4);
        assert_eq!(gp.mapping, GamepadMappingType::Standard);
        assert!(gp.connected);
    }

    #[test]
    fn gamepad_button_update() {
        let mut gp = Gamepad::new_standard(0, "Controller");
        gp.set_button(StandardButton::A.index(), GamepadButton::pressed(1.0));
        assert!(gp.button(StandardButton::A).pressed);
        assert_eq!(gp.button(StandardButton::A).value, 1.0);
    }

    #[test]
    fn gamepad_axis_clamped() {
        let mut gp = Gamepad::new_standard(0, "Controller");
        gp.set_axis(0, 1.5);
        assert_eq!(gp.left_stick_x(), 1.0);
        gp.set_axis(1, -2.0);
        assert_eq!(gp.left_stick_y(), -1.0);
    }

    #[test]
    fn gamepad_manager_connect_disconnect() {
        let mut mgr = GamepadManager::new();
        let idx = mgr.connect("Xbox Controller");
        assert!(mgr.get(idx).is_some());
        assert!(mgr.disconnect(idx));
        assert!(mgr.get(idx).is_none());
    }

    #[test]
    fn gamepad_manager_multiple() {
        let mut mgr = GamepadManager::new();
        let idx0 = mgr.connect("Controller 1");
        let idx1 = mgr.connect("Controller 2");
        let gamepads = mgr.get_gamepads();
        assert_eq!(gamepads.len(), 2);
        assert!(gamepads[idx0 as usize].is_some());
        assert!(gamepads[idx1 as usize].is_some());
    }

    #[test]
    fn standard_button_indices() {
        assert_eq!(StandardButton::A.index(), 0);
        assert_eq!(StandardButton::Home.index(), 16);
        assert_eq!(StandardButton::DPadRight.index(), 15);
    }

    #[test]
    fn gamepad_mapping_type_str() {
        assert_eq!(GamepadMappingType::Standard.as_str(), "standard");
        assert_eq!(GamepadMappingType::None.as_str(), "");
    }
}
