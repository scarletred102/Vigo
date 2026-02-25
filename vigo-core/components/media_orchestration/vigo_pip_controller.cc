// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/media_orchestration/vigo_pip_controller.h"

#include <algorithm>

#include "base/logging.h"

namespace vigo {
namespace media {

namespace {

// Default margin from screen edges (pixels).
constexpr int kDefaultMargin = 16;

// Base dimensions for PiP size presets (16:9 aspect ratio).
// These are scaled by screen DPI.
constexpr int kSmallWidth = 320;
constexpr int kSmallHeight = 180;
constexpr int kMediumWidth = 480;
constexpr int kMediumHeight = 270;
constexpr int kLargeWidth = 640;
constexpr int kLargeHeight = 360;

}  // namespace

VigoPipController::VigoPipController() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  VLOG(1) << "VigoPipController: Initialised with default config";
}

VigoPipController::~VigoPipController() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

const PipConfig& VigoPipController::GetConfig() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return config_;
}

void VigoPipController::SetConfig(const PipConfig& config) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  config_ = config;
  // Clamp opacity to valid range.
  config_.opacity = std::clamp(config_.opacity, 0.1, 1.0);
  VLOG(1) << "VigoPipController: Config updated";
}

void VigoPipController::SetSize(PipSize size) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  config_.size = size;
}

void VigoPipController::SetPosition(PipPosition position) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  config_.position = position;
}

void VigoPipController::SetOpacity(double opacity) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  config_.opacity = std::clamp(opacity, 0.1, 1.0);
}

void VigoPipController::SetAlwaysOnTop(bool always_on_top) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  config_.always_on_top = always_on_top;
}

void VigoPipController::SetAutoPipOnTabSwitch(bool enabled) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  config_.auto_pip_on_tab_switch = enabled;
}

void VigoPipController::SetShowControls(bool show) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  config_.show_controls = show;
}

// static
void VigoPipController::GetPixelDimensions(PipSize size,
                                            float screen_scale_factor,
                                            int* out_width,
                                            int* out_height) {
  float scale = std::max(screen_scale_factor, 1.0f);

  switch (size) {
    case PipSize::kSmall:
      *out_width = static_cast<int>(kSmallWidth * scale);
      *out_height = static_cast<int>(kSmallHeight * scale);
      break;
    case PipSize::kMedium:
      *out_width = static_cast<int>(kMediumWidth * scale);
      *out_height = static_cast<int>(kMediumHeight * scale);
      break;
    case PipSize::kLarge:
      *out_width = static_cast<int>(kLargeWidth * scale);
      *out_height = static_cast<int>(kLargeHeight * scale);
      break;
    case PipSize::kCustom:
      // Custom size is handled by the caller.
      break;
  }
}

// static
void VigoPipController::GetScreenCoordinates(PipPosition position,
                                              int screen_width,
                                              int screen_height,
                                              int pip_width,
                                              int pip_height,
                                              int margin,
                                              int* out_x,
                                              int* out_y) {
  if (margin <= 0) {
    margin = kDefaultMargin;
  }

  switch (position) {
    case PipPosition::kBottomRight:
      *out_x = screen_width - pip_width - margin;
      *out_y = screen_height - pip_height - margin;
      break;
    case PipPosition::kBottomLeft:
      *out_x = margin;
      *out_y = screen_height - pip_height - margin;
      break;
    case PipPosition::kTopRight:
      *out_x = screen_width - pip_width - margin;
      *out_y = margin;
      break;
    case PipPosition::kTopLeft:
      *out_x = margin;
      *out_y = margin;
      break;
    case PipPosition::kCustom:
      // Custom position is handled by the caller.
      break;
  }
}

bool VigoPipController::ShouldAutoPip() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return config_.auto_pip_on_tab_switch;
}

void VigoPipController::SavePosition(int x, int y, int width, int height) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  saved_x_ = x;
  saved_y_ = y;
  saved_width_ = width;
  saved_height_ = height;
  VLOG(2) << "VigoPipController: Position saved (" << x << "," << y
          << " " << width << "x" << height << ")";
}

void VigoPipController::RestorePosition(int* x,
                                         int* y,
                                         int* width,
                                         int* height) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  *x = saved_x_;
  *y = saved_y_;
  *width = saved_width_;
  *height = saved_height_;
}

}  // namespace media
}  // namespace vigo
