// G8 M66 关节 capture 一次性布局真值测量(第 2 段:第 1 段未覆盖的轴/极限/弹簧
// 字段补测)。数值消费方 = ffi.rs `ffi_layout_anchors`(Hinge 段)。
// 画像:x86_64-pc-windows-msvc / 单精度(DOUBLE_PRECISION=OFF)/ OBJECT_LAYER_BITS=16。
// 编译复测:cl /std:c++17 /I vendor/JoltC tools/layout_hinge2.cpp(见 tools/README.md)。
#include <cstdio>
#include <cstddef>
#include "JoltC/JoltC.h"
int main() {
  printf("HingeAxis1 %zu\n", offsetof(JPC_HingeConstraintSettings, HingeAxis1));
  printf("NormalAxis1 %zu\n", offsetof(JPC_HingeConstraintSettings, NormalAxis1));
  printf("HingeAxis2 %zu\n", offsetof(JPC_HingeConstraintSettings, HingeAxis2));
  printf("NormalAxis2 %zu\n", offsetof(JPC_HingeConstraintSettings, NormalAxis2));
  printf("LimitsMax %zu\n", offsetof(JPC_HingeConstraintSettings, LimitsMax));
  printf("LimitsSpring %zu\n", offsetof(JPC_HingeConstraintSettings, LimitsSpringSettings));
  printf("MaxFriction %zu\n", offsetof(JPC_HingeConstraintSettings, MaxFrictionTorque));
  printf("CS Draw %zu NumVel %zu NumPos %zu\n",
    offsetof(JPC_ConstraintSettings, DrawConstraintSize),
    offsetof(JPC_ConstraintSettings, NumVelocityStepsOverride),
    offsetof(JPC_ConstraintSettings, NumPositionStepsOverride));
  printf("Spring Mode %zu Freq %zu Damp %zu\n",
    offsetof(JPC_SpringSettings, Mode),
    offsetof(JPC_SpringSettings, FrequencyOrStiffness),
    offsetof(JPC_SpringSettings, Damping));
}
