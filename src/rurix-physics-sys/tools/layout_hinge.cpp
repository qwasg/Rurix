// G8 M66 关节 capture 一次性布局真值测量(第 1 段:ConstraintSettings / Spring /
// Motor / Hinge 主干偏移)。数值消费方 = ffi.rs `ffi_layout_anchors`(Hinge 段)。
// 画像:x86_64-pc-windows-msvc / 单精度(DOUBLE_PRECISION=OFF)/ OBJECT_LAYER_BITS=16。
// 编译复测:cl /std:c++17 /I vendor/JoltC tools/layout_hinge.cpp(见 tools/README.md)。
#include <cstdio>
#include <cstddef>
#include "JoltC/JoltC.h"
int main() {
  printf("ConstraintSettings %zu align %zu\n", sizeof(JPC_ConstraintSettings), alignof(JPC_ConstraintSettings));
  printf("  Enabled %zu\n", offsetof(JPC_ConstraintSettings, Enabled));
  printf("  Priority %zu\n", offsetof(JPC_ConstraintSettings, ConstraintPriority));
  printf("  UserData %zu\n", offsetof(JPC_ConstraintSettings, UserData));
  printf("SpringSettings %zu\n", sizeof(JPC_SpringSettings));
  printf("MotorSettings %zu\n", sizeof(JPC_MotorSettings));
  printf("HingeConstraintSettings %zu align %zu\n", sizeof(JPC_HingeConstraintSettings), alignof(JPC_HingeConstraintSettings));
  printf("  Space %zu\n", offsetof(JPC_HingeConstraintSettings, Space));
  printf("  Point1 %zu\n", offsetof(JPC_HingeConstraintSettings, Point1));
  printf("  Point2 %zu\n", offsetof(JPC_HingeConstraintSettings, Point2));
  printf("  LimitsMin %zu\n", offsetof(JPC_HingeConstraintSettings, LimitsMin));
  printf("  MotorSettings %zu\n", offsetof(JPC_HingeConstraintSettings, MotorSettings));
}
