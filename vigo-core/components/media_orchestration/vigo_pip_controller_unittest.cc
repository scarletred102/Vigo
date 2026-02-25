// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/media_orchestration/vigo_pip_controller.h"

#include "testing/gtest/include/gtest/gtest.h"

namespace vigo {
namespace media {
namespace {

class VigoPipControllerTest : public testing::Test {
 protected:
  VigoPipController controller_;
};

TEST_F(VigoPipControllerTest, DefaultConfig) {
  const auto& config = controller_.GetConfig();
  EXPECT_EQ(config.size, PipSize::kMedium);
  EXPECT_EQ(config.position, PipPosition::kBottomRight);
  EXPECT_DOUBLE_EQ(config.opacity, 1.0);
  EXPECT_TRUE(config.always_on_top);
  EXPECT_TRUE(config.show_controls);
  EXPECT_FALSE(config.auto_pip_on_tab_switch);
  EXPECT_TRUE(config.remember_position);
}

TEST_F(VigoPipControllerTest, SetSize) {
  controller_.SetSize(PipSize::kLarge);
  EXPECT_EQ(controller_.GetConfig().size, PipSize::kLarge);
}

TEST_F(VigoPipControllerTest, SetPosition) {
  controller_.SetPosition(PipPosition::kTopLeft);
  EXPECT_EQ(controller_.GetConfig().position, PipPosition::kTopLeft);
}

TEST_F(VigoPipControllerTest, SetOpacityClampsMin) {
  controller_.SetOpacity(0.0);
  EXPECT_DOUBLE_EQ(controller_.GetConfig().opacity, 0.1);
}

TEST_F(VigoPipControllerTest, SetOpacityClampsMax) {
  controller_.SetOpacity(2.5);
  EXPECT_DOUBLE_EQ(controller_.GetConfig().opacity, 1.0);
}

TEST_F(VigoPipControllerTest, SetOpacityValid) {
  controller_.SetOpacity(0.5);
  EXPECT_DOUBLE_EQ(controller_.GetConfig().opacity, 0.5);
}

TEST_F(VigoPipControllerTest, AutoPipDefault) {
  EXPECT_FALSE(controller_.ShouldAutoPip());
}

TEST_F(VigoPipControllerTest, AutoPipEnabled) {
  controller_.SetAutoPipOnTabSwitch(true);
  EXPECT_TRUE(controller_.ShouldAutoPip());
}

TEST_F(VigoPipControllerTest, PixelDimensionsSmall) {
  int w = 0, h = 0;
  VigoPipController::GetPixelDimensions(PipSize::kSmall, 1.0f, &w, &h);
  EXPECT_EQ(w, 320);
  EXPECT_EQ(h, 180);
}

TEST_F(VigoPipControllerTest, PixelDimensionsMedium) {
  int w = 0, h = 0;
  VigoPipController::GetPixelDimensions(PipSize::kMedium, 1.0f, &w, &h);
  EXPECT_EQ(w, 480);
  EXPECT_EQ(h, 270);
}

TEST_F(VigoPipControllerTest, PixelDimensionsLargeHiDpi) {
  int w = 0, h = 0;
  VigoPipController::GetPixelDimensions(PipSize::kLarge, 2.0f, &w, &h);
  EXPECT_EQ(w, 1280);
  EXPECT_EQ(h, 720);
}

TEST_F(VigoPipControllerTest, ScreenCoordsBottomRight) {
  int x = 0, y = 0;
  VigoPipController::GetScreenCoordinates(
      PipPosition::kBottomRight, 1920, 1080, 480, 270, 16, &x, &y);
  EXPECT_EQ(x, 1920 - 480 - 16);  // 1424
  EXPECT_EQ(y, 1080 - 270 - 16);  // 794
}

TEST_F(VigoPipControllerTest, ScreenCoordsTopLeft) {
  int x = 0, y = 0;
  VigoPipController::GetScreenCoordinates(
      PipPosition::kTopLeft, 1920, 1080, 480, 270, 16, &x, &y);
  EXPECT_EQ(x, 16);
  EXPECT_EQ(y, 16);
}

TEST_F(VigoPipControllerTest, SaveAndRestorePosition) {
  controller_.SavePosition(100, 200, 480, 270);
  int x, y, w, h;
  controller_.RestorePosition(&x, &y, &w, &h);
  EXPECT_EQ(x, 100);
  EXPECT_EQ(y, 200);
  EXPECT_EQ(w, 480);
  EXPECT_EQ(h, 270);
}

TEST_F(VigoPipControllerTest, RestoreBeforeSaveReturnsDefaults) {
  int x, y, w, h;
  controller_.RestorePosition(&x, &y, &w, &h);
  EXPECT_EQ(x, -1);
  EXPECT_EQ(y, -1);
  EXPECT_EQ(w, 0);
  EXPECT_EQ(h, 0);
}

TEST_F(VigoPipControllerTest, SetAlwaysOnTop) {
  controller_.SetAlwaysOnTop(false);
  EXPECT_FALSE(controller_.GetConfig().always_on_top);
}

TEST_F(VigoPipControllerTest, SetShowControls) {
  controller_.SetShowControls(false);
  EXPECT_FALSE(controller_.GetConfig().show_controls);
}

TEST_F(VigoPipControllerTest, SetFullConfig) {
  PipConfig config;
  config.size = PipSize::kLarge;
  config.position = PipPosition::kTopRight;
  config.opacity = 0.8;
  config.always_on_top = false;
  config.auto_pip_on_tab_switch = true;
  config.show_controls = false;
  controller_.SetConfig(config);

  const auto& result = controller_.GetConfig();
  EXPECT_EQ(result.size, PipSize::kLarge);
  EXPECT_EQ(result.position, PipPosition::kTopRight);
  EXPECT_DOUBLE_EQ(result.opacity, 0.8);
  EXPECT_FALSE(result.always_on_top);
  EXPECT_TRUE(result.auto_pip_on_tab_switch);
  EXPECT_FALSE(result.show_controls);
}

}  // namespace
}  // namespace media
}  // namespace vigo
