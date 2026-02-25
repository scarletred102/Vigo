// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_MEDIA_ORCHESTRATION_VIGO_PIP_CONTROLLER_H_
#define VIGO_COMPONENTS_MEDIA_ORCHESTRATION_VIGO_PIP_CONTROLLER_H_

#include <string>

#include "base/sequence_checker.h"

namespace vigo {
namespace media {

// PiP window size presets.
enum class PipSize {
  kSmall,   // 320×180 (corner overlay)
  kMedium,  // 480×270 (default)
  kLarge,   // 640×360 (comfortable viewing)
  kCustom,  // User-resized
};

// PiP window position on screen.
enum class PipPosition {
  kBottomRight,  // Default — out of the way
  kBottomLeft,
  kTopRight,
  kTopLeft,
  kCustom,  // User-dragged
};

// Configuration for Vigo's enhanced PiP window.
struct PipConfig {
  PipSize size = PipSize::kMedium;
  PipPosition position = PipPosition::kBottomRight;
  double opacity = 1.0;             // 0.0–1.0, default fully opaque.
  bool always_on_top = true;        // PiP stays above other windows.
  bool show_controls = true;        // Play/pause, seek, close buttons.
  bool auto_pip_on_tab_switch = false;  // Auto-PiP when switching tabs.
  bool remember_position = true;     // Remember last position/size.
  int custom_width = 0;
  int custom_height = 0;
};

// VigoPipController manages Vigo's enhanced Picture-in-Picture mode.
//
// Enhancements over Chrome's default PiP:
//   - Configurable size presets (Small/Medium/Large)
//   - Configurable position (4 corners or user drag)
//   - Opacity control (semi-transparent overlay mode)
//   - Auto-PiP on tab switch (opt-in)
//   - Position/size memory across sessions
//   - Enhanced controls: seek bar, volume, playback speed
//   - Subtitle rendering in PiP window
//
// Thread safety: UI sequence only.
class VigoPipController {
 public:
  VigoPipController();
  ~VigoPipController();

  VigoPipController(const VigoPipController&) = delete;
  VigoPipController& operator=(const VigoPipController&) = delete;

  // Get/set the current PiP configuration.
  const PipConfig& GetConfig() const;
  void SetConfig(const PipConfig& config);

  // Individual setters for common adjustments.
  void SetSize(PipSize size);
  void SetPosition(PipPosition position);
  void SetOpacity(double opacity);
  void SetAlwaysOnTop(bool always_on_top);
  void SetAutoPipOnTabSwitch(bool enabled);
  void SetShowControls(bool show);

  // Calculate the actual pixel dimensions for a PiP size preset.
  // Takes screen DPI into account.
  static void GetPixelDimensions(PipSize size,
                                 float screen_scale_factor,
                                 int* out_width,
                                 int* out_height);

  // Calculate the screen coordinates for a PiP position preset.
  // |screen_width|, |screen_height|: monitor resolution.
  // |pip_width|, |pip_height|: PiP window size.
  // |margin|: edge margin in pixels.
  static void GetScreenCoordinates(PipPosition position,
                                   int screen_width,
                                   int screen_height,
                                   int pip_width,
                                   int pip_height,
                                   int margin,
                                   int* out_x,
                                   int* out_y);

  // Returns true if the controller should auto-trigger PiP when
  // the user switches away from a tab playing video.
  bool ShouldAutoPip() const;

  // Save/restore PiP position for cross-session memory.
  // TODO(Phase 2.5): Persist via PrefService.
  void SavePosition(int x, int y, int width, int height);
  void RestorePosition(int* x, int* y, int* width, int* height) const;

 private:
  PipConfig config_;

  // Saved position from last PiP session.
  int saved_x_ = -1;
  int saved_y_ = -1;
  int saved_width_ = 0;
  int saved_height_ = 0;

  SEQUENCE_CHECKER(sequence_checker_);
};

}  // namespace media
}  // namespace vigo

#endif  // VIGO_COMPONENTS_MEDIA_ORCHESTRATION_VIGO_PIP_CONTROLLER_H_
