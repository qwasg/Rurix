// G6.2 PR-A 一次性布局真值测量(rurix-physics-sys ffi_layout_anchors 数据源)。
// 仅包含 vendored JoltC 公共头(ENSURE_* 宏无 ENSURE_TESTS 时为空,无需 Jolt include)。
// 画像:x86_64-pc-windows-msvc / 单精度(DOUBLE_PRECISION=OFF)/ OBJECT_LAYER_BITS=16。
#include <cstdio>
#include <cstddef>
#include "JoltC/JoltC.h"

#define P_SIZE(T) std::printf("sizeof(%s) = %zu, align = %zu\n", #T, sizeof(T), alignof(T))
#define P_OFF(T, F) std::printf("offsetof(%s.%s) = %zu\n", #T, #F, offsetof(T, F))

int main() {
    P_SIZE(JPC_Vec3); P_SIZE(JPC_Vec4); P_SIZE(JPC_Quat); P_SIZE(JPC_Float3);
    P_SIZE(JPC_Mat44); P_OFF(JPC_Mat44, col); P_OFF(JPC_Mat44, col3);
    P_SIZE(JPC_RayCastResult); P_OFF(JPC_RayCastResult, BodyID); P_OFF(JPC_RayCastResult, Fraction); P_OFF(JPC_RayCastResult, SubShapeID2);
    P_SIZE(JPC_RRayCast); P_OFF(JPC_RRayCast, Origin); P_OFF(JPC_RRayCast, Direction);
    P_SIZE(JPC_NarrowPhaseQuery_CastRayArgs);
    P_OFF(JPC_NarrowPhaseQuery_CastRayArgs, Ray); P_OFF(JPC_NarrowPhaseQuery_CastRayArgs, Result);
    P_OFF(JPC_NarrowPhaseQuery_CastRayArgs, BroadPhaseLayerFilter); P_OFF(JPC_NarrowPhaseQuery_CastRayArgs, ObjectLayerFilter);
    P_OFF(JPC_NarrowPhaseQuery_CastRayArgs, BodyFilter); P_OFF(JPC_NarrowPhaseQuery_CastRayArgs, ShapeFilter);
    P_SIZE(JPC_ShapeCastResult);
    P_OFF(JPC_ShapeCastResult, ContactPointOn1); P_OFF(JPC_ShapeCastResult, ContactPointOn2);
    P_OFF(JPC_ShapeCastResult, PenetrationAxis); P_OFF(JPC_ShapeCastResult, PenetrationDepth);
    P_OFF(JPC_ShapeCastResult, SubShapeID1); P_OFF(JPC_ShapeCastResult, SubShapeID2);
    P_OFF(JPC_ShapeCastResult, BodyID2); P_OFF(JPC_ShapeCastResult, Fraction); P_OFF(JPC_ShapeCastResult, IsBackFaceHit);
    P_SIZE(JPC_CollideShapeResult);
    P_OFF(JPC_CollideShapeResult, ContactPointOn1); P_OFF(JPC_CollideShapeResult, PenetrationAxis);
    P_OFF(JPC_CollideShapeResult, PenetrationDepth); P_OFF(JPC_CollideShapeResult, BodyID2);
    P_SIZE(JPC_ShapeCastSettings);
    P_OFF(JPC_ShapeCastSettings, ActiveEdgeMode); P_OFF(JPC_ShapeCastSettings, CollectFacesMode);
    P_OFF(JPC_ShapeCastSettings, CollisionTolerance); P_OFF(JPC_ShapeCastSettings, PenetrationTolerance);
    P_OFF(JPC_ShapeCastSettings, ActiveEdgeMovementDirection); P_OFF(JPC_ShapeCastSettings, BackFaceModeTriangles);
    P_OFF(JPC_ShapeCastSettings, BackFaceModeConvex); P_OFF(JPC_ShapeCastSettings, UseShrunkenShapeAndConvexRadius);
    P_OFF(JPC_ShapeCastSettings, ReturnDeepestPoint);
    P_SIZE(JPC_CollideShapeSettings);
    P_OFF(JPC_CollideShapeSettings, MaxSeparationDistance); P_OFF(JPC_CollideShapeSettings, BackFaceMode);
    P_SIZE(JPC_RShapeCast);
    P_OFF(JPC_RShapeCast, Shape); P_OFF(JPC_RShapeCast, Scale);
    P_OFF(JPC_RShapeCast, CenterOfMassStart); P_OFF(JPC_RShapeCast, Direction);
    P_SIZE(JPC_NarrowPhaseQuery_CastShapeArgs);
    P_OFF(JPC_NarrowPhaseQuery_CastShapeArgs, ShapeCast); P_OFF(JPC_NarrowPhaseQuery_CastShapeArgs, Settings);
    P_OFF(JPC_NarrowPhaseQuery_CastShapeArgs, BaseOffset); P_OFF(JPC_NarrowPhaseQuery_CastShapeArgs, Collector);
    P_OFF(JPC_NarrowPhaseQuery_CastShapeArgs, ShapeFilter);
    P_SIZE(JPC_NarrowPhaseQuery_CollideShapeArgs);
    P_OFF(JPC_NarrowPhaseQuery_CollideShapeArgs, Shape); P_OFF(JPC_NarrowPhaseQuery_CollideShapeArgs, ShapeScale);
    P_OFF(JPC_NarrowPhaseQuery_CollideShapeArgs, CenterOfMassTransform); P_OFF(JPC_NarrowPhaseQuery_CollideShapeArgs, Settings);
    P_OFF(JPC_NarrowPhaseQuery_CollideShapeArgs, BaseOffset); P_OFF(JPC_NarrowPhaseQuery_CollideShapeArgs, Collector);
    P_SIZE(JPC_BodyCreationSettings);
    P_OFF(JPC_BodyCreationSettings, Position); P_OFF(JPC_BodyCreationSettings, Rotation);
    P_OFF(JPC_BodyCreationSettings, LinearVelocity); P_OFF(JPC_BodyCreationSettings, AngularVelocity);
    P_OFF(JPC_BodyCreationSettings, UserData); P_OFF(JPC_BodyCreationSettings, ObjectLayer);
    P_OFF(JPC_BodyCreationSettings, MotionType); P_OFF(JPC_BodyCreationSettings, AllowedDOFs);
    P_OFF(JPC_BodyCreationSettings, AllowDynamicOrKinematic); P_OFF(JPC_BodyCreationSettings, IsSensor);
    P_OFF(JPC_BodyCreationSettings, CollideKinematicVsNonDynamic); P_OFF(JPC_BodyCreationSettings, UseManifoldReduction);
    P_OFF(JPC_BodyCreationSettings, ApplyGyroscopicForce); P_OFF(JPC_BodyCreationSettings, MotionQuality);
    P_OFF(JPC_BodyCreationSettings, EnhancedInternalEdgeRemoval); P_OFF(JPC_BodyCreationSettings, AllowSleeping);
    P_OFF(JPC_BodyCreationSettings, Friction); P_OFF(JPC_BodyCreationSettings, Restitution);
    P_OFF(JPC_BodyCreationSettings, LinearDamping); P_OFF(JPC_BodyCreationSettings, AngularDamping);
    P_OFF(JPC_BodyCreationSettings, MaxLinearVelocity); P_OFF(JPC_BodyCreationSettings, MaxAngularVelocity);
    P_OFF(JPC_BodyCreationSettings, GravityFactor); P_OFF(JPC_BodyCreationSettings, NumVelocityStepsOverride);
    P_OFF(JPC_BodyCreationSettings, NumPositionStepsOverride); P_OFF(JPC_BodyCreationSettings, OverrideMassProperties);
    P_OFF(JPC_BodyCreationSettings, InertiaMultiplier); P_OFF(JPC_BodyCreationSettings, Shape);
    P_SIZE(JPC_ContactPoints); P_OFF(JPC_ContactPoints, length); P_OFF(JPC_ContactPoints, points);
    P_SIZE(JPC_ContactManifold);
    P_OFF(JPC_ContactManifold, BaseOffset); P_OFF(JPC_ContactManifold, WorldSpaceNormal);
    P_OFF(JPC_ContactManifold, PenetrationDepth); P_OFF(JPC_ContactManifold, SubShapeID1);
    P_OFF(JPC_ContactManifold, SubShapeID2); P_OFF(JPC_ContactManifold, RelativeContactPointsOn1);
    P_OFF(JPC_ContactManifold, RelativeContactPointsOn2);
    P_SIZE(JPC_ContactSettings);
    P_OFF(JPC_ContactSettings, IsSensor); P_OFF(JPC_ContactSettings, RelativeLinearSurfaceVelocity);
    P_SIZE(JPC_SubShapeIDPair);
    P_OFF(JPC_SubShapeIDPair, Body1ID); P_OFF(JPC_SubShapeIDPair, SubShapeID1);
    P_OFF(JPC_SubShapeIDPair, Body2ID); P_OFF(JPC_SubShapeIDPair, SubShapeID2);
    P_SIZE(JPC_SphereShapeSettings);
    P_OFF(JPC_SphereShapeSettings, UserData); P_OFF(JPC_SphereShapeSettings, Density); P_OFF(JPC_SphereShapeSettings, Radius);
    P_SIZE(JPC_BoxShapeSettings);
    P_OFF(JPC_BoxShapeSettings, UserData); P_OFF(JPC_BoxShapeSettings, Density);
    P_OFF(JPC_BoxShapeSettings, HalfExtent); P_OFF(JPC_BoxShapeSettings, ConvexRadius);
    P_SIZE(JPC_CapsuleShapeSettings);
    P_OFF(JPC_CapsuleShapeSettings, UserData); P_OFF(JPC_CapsuleShapeSettings, Density);
    P_OFF(JPC_CapsuleShapeSettings, Radius); P_OFF(JPC_CapsuleShapeSettings, HalfHeightOfCylinder);
    P_SIZE(JPC_ConvexHullShapeSettings);
    P_OFF(JPC_ConvexHullShapeSettings, UserData); P_OFF(JPC_ConvexHullShapeSettings, Density);
    P_OFF(JPC_ConvexHullShapeSettings, Points); P_OFF(JPC_ConvexHullShapeSettings, PointsLen);
    P_OFF(JPC_ConvexHullShapeSettings, MaxConvexRadius); P_OFF(JPC_ConvexHullShapeSettings, MaxErrorConvexRadius);
    P_OFF(JPC_ConvexHullShapeSettings, HullTolerance);
    P_SIZE(JPC_MeshShapeSettings);
    P_OFF(JPC_MeshShapeSettings, UserData); P_OFF(JPC_MeshShapeSettings, TriangleVertices);
    P_OFF(JPC_MeshShapeSettings, TriangleVerticesLen); P_OFF(JPC_MeshShapeSettings, IndexedTriangles);
    P_OFF(JPC_MeshShapeSettings, IndexedTrianglesLen);
    P_SIZE(JPC_IndexedTriangle);
    P_OFF(JPC_IndexedTriangle, idx); P_OFF(JPC_IndexedTriangle, materialIndex); P_OFF(JPC_IndexedTriangle, userData);
    P_SIZE(JPC_ContactListenerFns); P_SIZE(JPC_CastShapeCollectorFns); P_SIZE(JPC_CollideShapeCollectorFns);
    P_SIZE(JPC_BroadPhaseLayerInterfaceFns); P_SIZE(JPC_ObjectVsBroadPhaseLayerFilterFns);
    P_SIZE(JPC_ObjectLayerPairFilterFns); P_SIZE(JPC_ObjectLayerFilterFns); P_SIZE(JPC_BodyFilterFns);
    P_SIZE(JPC_BodyID); P_SIZE(JPC_ObjectLayer); P_SIZE(JPC_BroadPhaseLayer); P_SIZE(JPC_SubShapeID);
    P_SIZE(JPC_MotionType); P_SIZE(JPC_MotionQuality); P_SIZE(JPC_Activation); P_SIZE(JPC_OverrideMassProperties);
    P_SIZE(JPC_PhysicsUpdateError); P_SIZE(JPC_ValidateResult); P_SIZE(JPC_BackFaceMode);
    P_SIZE(JPC_ActiveEdgeMode); P_SIZE(JPC_CollectFacesMode); P_SIZE(JPC_AllowedDOFs);
    return 0;
}
