// G9.6 M125 layout 探针(Jolt 5.6 臂):rurix-physics-sys56 ffi_layout_anchors 数据源。
// 沿 G6.2 layout_dump.cpp + G8 M66 layout_hinge.cpp/hinge2.cpp 体例合并为单一探针——
// RXS-0377 L5「所有 *Settings 结构 sizeof/offsetof 静态断言重跑纳入 vendor 升级/新 FFI
// 检查单固定项;探针源码入库」;本探针 = 5.6 臂全量消费结构 + 全部 *Settings 覆盖。
// 仅包含 vendored JoltC(5.6 线)公共头(ENSURE_* 宏无 ENSURE_TESTS 时为空,无需 Jolt include)。
// 画像:x86_64-pc-windows-msvc / 单精度(DOUBLE_PRECISION=OFF)/ OBJECT_LAYER_BITS=16。
// 编译复测(在 crate 根 src/rurix-physics-sys56/ 执行;二进制产物不入库):
//   cl /std:c++17 /I vendor/JoltC tools/layout_dump56.cpp
#include <cstdio>
#include <cstddef>
#include "JoltC/JoltC.h"

#define P_SIZE(T) std::printf("sizeof(%s) = %zu, align = %zu\n", #T, sizeof(T), alignof(T))
#define P_OFF(T, F) std::printf("offsetof(%s.%s) = %zu\n", #T, #F, offsetof(T, F))

int main() {
    P_SIZE(JPC56_Vec3); P_SIZE(JPC56_Vec4); P_SIZE(JPC56_Quat); P_SIZE(JPC56_Float3);
    P_SIZE(JPC56_Mat44); P_OFF(JPC56_Mat44, col); P_OFF(JPC56_Mat44, col3);
    P_SIZE(JPC56_RayCastResult); P_OFF(JPC56_RayCastResult, BodyID); P_OFF(JPC56_RayCastResult, Fraction); P_OFF(JPC56_RayCastResult, SubShapeID2);
    P_SIZE(JPC56_RRayCast); P_OFF(JPC56_RRayCast, Origin); P_OFF(JPC56_RRayCast, Direction);
    P_SIZE(JPC56_NarrowPhaseQuery_CastRayArgs);
    P_OFF(JPC56_NarrowPhaseQuery_CastRayArgs, Ray); P_OFF(JPC56_NarrowPhaseQuery_CastRayArgs, Result);
    P_OFF(JPC56_NarrowPhaseQuery_CastRayArgs, BroadPhaseLayerFilter); P_OFF(JPC56_NarrowPhaseQuery_CastRayArgs, ObjectLayerFilter);
    P_OFF(JPC56_NarrowPhaseQuery_CastRayArgs, BodyFilter); P_OFF(JPC56_NarrowPhaseQuery_CastRayArgs, ShapeFilter);
    P_SIZE(JPC56_ShapeCastResult);
    P_OFF(JPC56_ShapeCastResult, ContactPointOn1); P_OFF(JPC56_ShapeCastResult, ContactPointOn2);
    P_OFF(JPC56_ShapeCastResult, PenetrationAxis); P_OFF(JPC56_ShapeCastResult, PenetrationDepth);
    P_OFF(JPC56_ShapeCastResult, SubShapeID1); P_OFF(JPC56_ShapeCastResult, SubShapeID2);
    P_OFF(JPC56_ShapeCastResult, BodyID2); P_OFF(JPC56_ShapeCastResult, Fraction); P_OFF(JPC56_ShapeCastResult, IsBackFaceHit);
    P_SIZE(JPC56_CollideShapeResult);
    P_OFF(JPC56_CollideShapeResult, ContactPointOn1); P_OFF(JPC56_CollideShapeResult, PenetrationAxis);
    P_OFF(JPC56_CollideShapeResult, PenetrationDepth); P_OFF(JPC56_CollideShapeResult, BodyID2);
    // Jolt 5.6 delta:ShapeCastSettings 新增 ExtraConvexRadius(上游 5.6 新面,
    // 5.3 无此字段;offset 后移段为本探针重点)。
    P_SIZE(JPC56_ShapeCastSettings);
    P_OFF(JPC56_ShapeCastSettings, ActiveEdgeMode); P_OFF(JPC56_ShapeCastSettings, CollectFacesMode);
    P_OFF(JPC56_ShapeCastSettings, CollisionTolerance); P_OFF(JPC56_ShapeCastSettings, PenetrationTolerance);
    P_OFF(JPC56_ShapeCastSettings, ActiveEdgeMovementDirection); P_OFF(JPC56_ShapeCastSettings, ExtraConvexRadius);
    P_OFF(JPC56_ShapeCastSettings, BackFaceModeTriangles); P_OFF(JPC56_ShapeCastSettings, BackFaceModeConvex);
    P_OFF(JPC56_ShapeCastSettings, UseShrunkenShapeAndConvexRadius); P_OFF(JPC56_ShapeCastSettings, ReturnDeepestPoint);
    // Jolt 5.6 delta:CollideShapeSettings 新增 InternalEdgeRemovalVertexToleranceSq。
    P_SIZE(JPC56_CollideShapeSettings);
    P_OFF(JPC56_CollideShapeSettings, MaxSeparationDistance); P_OFF(JPC56_CollideShapeSettings, BackFaceMode);
    P_OFF(JPC56_CollideShapeSettings, InternalEdgeRemovalVertexToleranceSq);
    P_SIZE(JPC56_RShapeCast);
    P_OFF(JPC56_RShapeCast, Shape); P_OFF(JPC56_RShapeCast, Scale);
    P_OFF(JPC56_RShapeCast, CenterOfMassStart); P_OFF(JPC56_RShapeCast, Direction);
    P_SIZE(JPC56_NarrowPhaseQuery_CastShapeArgs);
    P_OFF(JPC56_NarrowPhaseQuery_CastShapeArgs, ShapeCast); P_OFF(JPC56_NarrowPhaseQuery_CastShapeArgs, Settings);
    P_OFF(JPC56_NarrowPhaseQuery_CastShapeArgs, BaseOffset); P_OFF(JPC56_NarrowPhaseQuery_CastShapeArgs, Collector);
    P_OFF(JPC56_NarrowPhaseQuery_CastShapeArgs, ShapeFilter);
    P_SIZE(JPC56_NarrowPhaseQuery_CollideShapeArgs);
    P_OFF(JPC56_NarrowPhaseQuery_CollideShapeArgs, Shape); P_OFF(JPC56_NarrowPhaseQuery_CollideShapeArgs, ShapeScale);
    P_OFF(JPC56_NarrowPhaseQuery_CollideShapeArgs, CenterOfMassTransform); P_OFF(JPC56_NarrowPhaseQuery_CollideShapeArgs, Settings);
    P_OFF(JPC56_NarrowPhaseQuery_CollideShapeArgs, BaseOffset); P_OFF(JPC56_NarrowPhaseQuery_CollideShapeArgs, Collector);
    P_SIZE(JPC56_BodyCreationSettings);
    P_OFF(JPC56_BodyCreationSettings, Position); P_OFF(JPC56_BodyCreationSettings, Rotation);
    P_OFF(JPC56_BodyCreationSettings, LinearVelocity); P_OFF(JPC56_BodyCreationSettings, AngularVelocity);
    P_OFF(JPC56_BodyCreationSettings, UserData); P_OFF(JPC56_BodyCreationSettings, ObjectLayer);
    P_OFF(JPC56_BodyCreationSettings, MotionType); P_OFF(JPC56_BodyCreationSettings, AllowedDOFs);
    P_OFF(JPC56_BodyCreationSettings, AllowDynamicOrKinematic); P_OFF(JPC56_BodyCreationSettings, IsSensor);
    P_OFF(JPC56_BodyCreationSettings, CollideKinematicVsNonDynamic); P_OFF(JPC56_BodyCreationSettings, UseManifoldReduction);
    P_OFF(JPC56_BodyCreationSettings, ApplyGyroscopicForce); P_OFF(JPC56_BodyCreationSettings, MotionQuality);
    P_OFF(JPC56_BodyCreationSettings, EnhancedInternalEdgeRemoval); P_OFF(JPC56_BodyCreationSettings, AllowSleeping);
    P_OFF(JPC56_BodyCreationSettings, Friction); P_OFF(JPC56_BodyCreationSettings, Restitution);
    P_OFF(JPC56_BodyCreationSettings, LinearDamping); P_OFF(JPC56_BodyCreationSettings, AngularDamping);
    P_OFF(JPC56_BodyCreationSettings, MaxLinearVelocity); P_OFF(JPC56_BodyCreationSettings, MaxAngularVelocity);
    P_OFF(JPC56_BodyCreationSettings, GravityFactor); P_OFF(JPC56_BodyCreationSettings, NumVelocityStepsOverride);
    P_OFF(JPC56_BodyCreationSettings, NumPositionStepsOverride); P_OFF(JPC56_BodyCreationSettings, OverrideMassProperties);
    P_OFF(JPC56_BodyCreationSettings, InertiaMultiplier); P_OFF(JPC56_BodyCreationSettings, Shape);
    P_SIZE(JPC56_ContactPoints); P_OFF(JPC56_ContactPoints, length); P_OFF(JPC56_ContactPoints, points);
    P_SIZE(JPC56_ContactManifold);
    P_OFF(JPC56_ContactManifold, BaseOffset); P_OFF(JPC56_ContactManifold, WorldSpaceNormal);
    P_OFF(JPC56_ContactManifold, PenetrationDepth); P_OFF(JPC56_ContactManifold, SubShapeID1);
    P_OFF(JPC56_ContactManifold, SubShapeID2); P_OFF(JPC56_ContactManifold, RelativeContactPointsOn1);
    P_OFF(JPC56_ContactManifold, RelativeContactPointsOn2);
    P_SIZE(JPC56_ContactSettings);
    P_OFF(JPC56_ContactSettings, IsSensor); P_OFF(JPC56_ContactSettings, RelativeLinearSurfaceVelocity);
    P_SIZE(JPC56_SubShapeIDPair);
    P_OFF(JPC56_SubShapeIDPair, Body1ID); P_OFF(JPC56_SubShapeIDPair, SubShapeID1);
    P_OFF(JPC56_SubShapeIDPair, Body2ID); P_OFF(JPC56_SubShapeIDPair, SubShapeID2);
    P_SIZE(JPC56_SphereShapeSettings);
    P_OFF(JPC56_SphereShapeSettings, UserData); P_OFF(JPC56_SphereShapeSettings, Density); P_OFF(JPC56_SphereShapeSettings, Radius);
    P_SIZE(JPC56_BoxShapeSettings);
    P_OFF(JPC56_BoxShapeSettings, UserData); P_OFF(JPC56_BoxShapeSettings, Density);
    P_OFF(JPC56_BoxShapeSettings, HalfExtent); P_OFF(JPC56_BoxShapeSettings, ConvexRadius);
    P_SIZE(JPC56_CapsuleShapeSettings);
    P_OFF(JPC56_CapsuleShapeSettings, UserData); P_OFF(JPC56_CapsuleShapeSettings, Density);
    P_OFF(JPC56_CapsuleShapeSettings, Radius); P_OFF(JPC56_CapsuleShapeSettings, HalfHeightOfCylinder);
    P_SIZE(JPC56_ConvexHullShapeSettings);
    P_OFF(JPC56_ConvexHullShapeSettings, UserData); P_OFF(JPC56_ConvexHullShapeSettings, Density);
    P_OFF(JPC56_ConvexHullShapeSettings, Points); P_OFF(JPC56_ConvexHullShapeSettings, PointsLen);
    P_OFF(JPC56_ConvexHullShapeSettings, MaxConvexRadius); P_OFF(JPC56_ConvexHullShapeSettings, MaxErrorConvexRadius);
    P_OFF(JPC56_ConvexHullShapeSettings, HullTolerance);
    P_SIZE(JPC56_MeshShapeSettings);
    P_OFF(JPC56_MeshShapeSettings, UserData); P_OFF(JPC56_MeshShapeSettings, TriangleVertices);
    P_OFF(JPC56_MeshShapeSettings, TriangleVerticesLen); P_OFF(JPC56_MeshShapeSettings, IndexedTriangles);
    P_OFF(JPC56_MeshShapeSettings, IndexedTrianglesLen);
    P_SIZE(JPC56_IndexedTriangle);
    P_OFF(JPC56_IndexedTriangle, idx); P_OFF(JPC56_IndexedTriangle, materialIndex); P_OFF(JPC56_IndexedTriangle, userData);
    // Constraint/Hinge 段(沿 layout_hinge.cpp/hinge2.cpp 覆盖面合并):
    // Jolt 5.6 ConstraintSettings 基类 ctor protected(vendor56 适配补丁 JoltC.cpp
    // 派生 shim),字段面不变——断言重跑仍强制(RXS-0377 L5 检查单)。
    std::printf("ConstraintSettings %zu align %zu\n", sizeof(JPC56_ConstraintSettings), alignof(JPC56_ConstraintSettings));
    std::printf("  Enabled %zu\n", offsetof(JPC56_ConstraintSettings, Enabled));
    std::printf("  Priority %zu\n", offsetof(JPC56_ConstraintSettings, ConstraintPriority));
    std::printf("  UserData %zu\n", offsetof(JPC56_ConstraintSettings, UserData));
    std::printf("  DrawConstraintSize %zu\n", offsetof(JPC56_ConstraintSettings, DrawConstraintSize));
    std::printf("  NumVelocityStepsOverride %zu\n", offsetof(JPC56_ConstraintSettings, NumVelocityStepsOverride));
    std::printf("  NumPositionStepsOverride %zu\n", offsetof(JPC56_ConstraintSettings, NumPositionStepsOverride));
    std::printf("SpringSettings %zu\n", sizeof(JPC56_SpringSettings));
    std::printf("  Mode %zu\n", offsetof(JPC56_SpringSettings, Mode));
    std::printf("  FrequencyOrStiffness %zu\n", offsetof(JPC56_SpringSettings, FrequencyOrStiffness));
    std::printf("  Damping %zu\n", offsetof(JPC56_SpringSettings, Damping));
    std::printf("MotorSettings %zu\n", sizeof(JPC56_MotorSettings));
    std::printf("HingeConstraintSettings %zu align %zu\n", sizeof(JPC56_HingeConstraintSettings), alignof(JPC56_HingeConstraintSettings));
    std::printf("  Space %zu\n", offsetof(JPC56_HingeConstraintSettings, Space));
    std::printf("  Point1 %zu\n", offsetof(JPC56_HingeConstraintSettings, Point1));
    std::printf("  Point2 %zu\n", offsetof(JPC56_HingeConstraintSettings, Point2));
    std::printf("  HingeAxis1 %zu\n", offsetof(JPC56_HingeConstraintSettings, HingeAxis1));
    std::printf("  NormalAxis1 %zu\n", offsetof(JPC56_HingeConstraintSettings, NormalAxis1));
    std::printf("  HingeAxis2 %zu\n", offsetof(JPC56_HingeConstraintSettings, HingeAxis2));
    std::printf("  NormalAxis2 %zu\n", offsetof(JPC56_HingeConstraintSettings, NormalAxis2));
    std::printf("  LimitsMin %zu\n", offsetof(JPC56_HingeConstraintSettings, LimitsMin));
    std::printf("  LimitsMax %zu\n", offsetof(JPC56_HingeConstraintSettings, LimitsMax));
    std::printf("  LimitsSpring %zu\n", offsetof(JPC56_HingeConstraintSettings, LimitsSpringSettings));
    std::printf("  MaxFrictionTorque %zu\n", offsetof(JPC56_HingeConstraintSettings, MaxFrictionTorque));
    std::printf("  MotorSettings %zu\n", offsetof(JPC56_HingeConstraintSettings, MotorSettings));
    // Jolt 5.6 delta:新摩擦模型(平均接触点)CollisionEstimationResult 重排——
    // 聚合摩擦字段(FrictionPoint/Tangent1/Tangent2/FrictionImpulse1/2/
    // AngularFrictionImpulse)+ 逐点 ContactImpulse float 数组。
    P_SIZE(JPC56_CollisionEstimationResult);
    P_OFF(JPC56_CollisionEstimationResult, LinearVelocity1); P_OFF(JPC56_CollisionEstimationResult, AngularVelocity1);
    P_OFF(JPC56_CollisionEstimationResult, LinearVelocity2); P_OFF(JPC56_CollisionEstimationResult, AngularVelocity2);
    P_OFF(JPC56_CollisionEstimationResult, FrictionPoint); P_OFF(JPC56_CollisionEstimationResult, Tangent1);
    P_OFF(JPC56_CollisionEstimationResult, Tangent2); P_OFF(JPC56_CollisionEstimationResult, FrictionImpulse1);
    P_OFF(JPC56_CollisionEstimationResult, FrictionImpulse2); P_OFF(JPC56_CollisionEstimationResult, AngularFrictionImpulse);
    P_OFF(JPC56_CollisionEstimationResult, NumImpulses); P_OFF(JPC56_CollisionEstimationResult, Impulses);
    // Jolt 5.6 delta:BodyManager_DrawSettings 新增 strand-hair 调试三字段。
    P_SIZE(JPC56_BodyManager_DrawSettings);
    P_OFF(JPC56_BodyManager_DrawSettings, mDrawSoftBodyLRAConstraints);
    P_OFF(JPC56_BodyManager_DrawSettings, mDrawSoftBodyRods);
    P_OFF(JPC56_BodyManager_DrawSettings, mDrawSoftBodyRodStates);
    P_OFF(JPC56_BodyManager_DrawSettings, mDrawSoftBodyRodBendTwistConstraints);
    P_OFF(JPC56_BodyManager_DrawSettings, mDrawSoftBodyPredictedBounds);
    P_SIZE(JPC56_ContactListenerFns); P_SIZE(JPC56_CastShapeCollectorFns); P_SIZE(JPC56_CollideShapeCollectorFns);
    P_SIZE(JPC56_BroadPhaseLayerInterfaceFns); P_SIZE(JPC56_ObjectVsBroadPhaseLayerFilterFns);
    P_SIZE(JPC56_ObjectLayerPairFilterFns); P_SIZE(JPC56_ObjectLayerFilterFns); P_SIZE(JPC56_BodyFilterFns);
    P_SIZE(JPC56_BodyID); P_SIZE(JPC56_ObjectLayer); P_SIZE(JPC56_BroadPhaseLayer); P_SIZE(JPC56_SubShapeID);
    P_SIZE(JPC56_MotionType); P_SIZE(JPC56_MotionQuality); P_SIZE(JPC56_Activation); P_SIZE(JPC56_OverrideMassProperties);
    P_SIZE(JPC56_PhysicsUpdateError); P_SIZE(JPC56_ValidateResult); P_SIZE(JPC56_BackFaceMode);
    P_SIZE(JPC56_ActiveEdgeMode); P_SIZE(JPC56_CollectFacesMode); P_SIZE(JPC56_AllowedDOFs);
    return 0;
}
