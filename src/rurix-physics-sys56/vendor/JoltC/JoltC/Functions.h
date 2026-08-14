#pragma once

#include <stdalign.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef _MSC_VER
	#define JPC56_API extern __declspec(dllexport)
#else
	#define JPC56_API
#endif

// Mirror JPH56's per-arch VECTOR / DVECTOR alignment logic from
// `JoltPhysics/Jolt/Core/Core.h`. These two values MUST match what JPH56
// computes, otherwise the LAYOUT_COMPATIBLE static_asserts in
// `JoltCImpl/JoltC.cpp` fail at build time.
//
// The load-bearing case is 32-bit ARM (e.g. armv7 Android, armv7 Linux):
// JPH56 falls back to 8-byte alignment because 32-bit ARM cannot stack-align
// to 16 bytes. Without mirroring that fallback here, the C-side `alignas(16)`
// over-constrains `JPC56_Vec3` / `Vec4` / `Quat` / `Mat44` / `DVec3` / `DMat44`
// (and every higher-level struct that embeds them: `JPC56_ShapeCastSettings`,
// `JPC56_CollideShapeSettings`, `JPC56_BodyManager_DrawSettings`,
// `JPC56_ContactPoints`, `JPC56_ContactManifold`, …) and the LAYOUT_COMPATIBLE
// static_assert fires for every settings struct that contains a vector
// field — `align of JPC56_<X> did not match align of JPH56::<X> (16 == 8)`.
#if defined(__aarch64__) || defined(_M_ARM64) || \
    defined(__x86_64__)  || defined(_M_X64)   || \
    defined(__i386__)    || defined(_M_IX86)  || \
    defined(__wasm__)    || defined(__e2k__)
	#define JPC56_VECTOR_ALIGNMENT  16
	#define JPC56_DVECTOR_ALIGNMENT 32
#elif defined(__arm__) || defined(_M_ARM)
	// 32-bit ARM (the aarch64 case above takes precedence on 64-bit).
	#define JPC56_VECTOR_ALIGNMENT  8
	#define JPC56_DVECTOR_ALIGNMENT 8
#elif defined(__riscv)
	#define JPC56_VECTOR_ALIGNMENT  16
	#if __riscv_xlen == 64
		#define JPC56_DVECTOR_ALIGNMENT 32
	#else
		#define JPC56_DVECTOR_ALIGNMENT 8
	#endif
#elif defined(__powerpc__) || defined(__powerpc64__) || defined(__loongarch__)
	#define JPC56_VECTOR_ALIGNMENT  16
	#define JPC56_DVECTOR_ALIGNMENT 8
#else
	// Conservative fallback: assume the common 64-bit shape.
	#define JPC56_VECTOR_ALIGNMENT  16
	#define JPC56_DVECTOR_ALIGNMENT 32
#endif

static float JPC56_PI = 3.14159265358979323846f;

// C-compatible typedefs that match Jolt's internal primitive typedefs.
#define uint unsigned int

#ifdef __cplusplus
extern "C" {
#endif

JPC56_API void JPC56_RegisterDefaultAllocator();
JPC56_API void JPC56_FactoryInit();
JPC56_API void JPC56_FactoryDelete();
JPC56_API void JPC56_RegisterTypes();
JPC56_API void JPC56_UnregisterTypes();

////////////////////////////////////////////////////////////////////////////////
// Primitive types

typedef struct JPC56_Float3 {
	float x;
	float y;
	float z;
} JPC56_Float3;

ENSURE_SIZE_ALIGN(JPC56_Float3, JPH56::Float3)

// Jolt has no type named Vec2 but uses Vector<2> in its API sometimes
typedef struct JPC56_Vec2 {
	float x;
	float y;
} JPC56_Vec2;

ENSURE_SIZE_ALIGN(JPC56_Vec2, JPH56::Vector<2>)

typedef struct JPC56_Vec3 {
	alignas(JPC56_VECTOR_ALIGNMENT) float x;
	float y;
	float z;
	float _w;
} JPC56_Vec3;

ENSURE_SIZE_ALIGN(JPC56_Vec3, JPH56::Vec3)

typedef struct JPC56_Vec4 {
	alignas(JPC56_VECTOR_ALIGNMENT) float x;
	float y;
	float z;
	float w;
} JPC56_Vec4;

ENSURE_SIZE_ALIGN(JPC56_Vec4, JPH56::Vec4)

typedef struct JPC56_DVec3 {
	alignas(JPC56_DVECTOR_ALIGNMENT) double x;
	double y;
	double z;
	double _w;
} JPC56_DVec3;

ENSURE_SIZE_ALIGN(JPC56_DVec3, JPH56::DVec3)

typedef struct JPC56_Quat {
	alignas(JPC56_VECTOR_ALIGNMENT) float x;
	float y;
	float z;
	float w;
} JPC56_Quat;

ENSURE_SIZE_ALIGN(JPC56_Quat, JPH56::Quat)

typedef struct JPC56_Mat44 {
	alignas(JPC56_VECTOR_ALIGNMENT) JPC56_Vec4 col[3];
	JPC56_Vec3 col3;
} JPC56_Mat44;

ENSURE_SIZE_ALIGN(JPC56_Mat44, JPH56::Mat44)

typedef struct JPC56_DMat44 {
	alignas(JPC56_DVECTOR_ALIGNMENT) JPC56_Vec4 col[3];
	JPC56_DVec3 col3;
} JPC56_DMat44;

ENSURE_SIZE_ALIGN(JPC56_DMat44, JPH56::DMat44)

typedef struct JPC56_Color {
	alignas(uint32_t) uint8_t r;
	uint8_t g;
	uint8_t b;
	uint8_t a;
} JPC56_Color;

ENSURE_SIZE_ALIGN(JPC56_Color, JPH56::Color)

#ifdef JPC56_DOUBLE_PRECISION
	typedef JPC56_DVec3 JPC56_RVec3;
	typedef JPC56_DMat44 JPC56_RMat44;
	typedef double Real;
#else
	typedef JPC56_Vec3 JPC56_RVec3;
	typedef JPC56_Mat44 JPC56_RMat44;
	typedef float Real;
#endif

ENSURE_SIZE_ALIGN(JPC56_RVec3, JPH56::RVec3)

typedef uint32_t JPC56_BodyID;
ENSURE_SIZE_ALIGN(JPC56_BodyID, JPH56::BodyID)

typedef uint32_t JPC56_SubShapeID;
ENSURE_SIZE_ALIGN(JPC56_SubShapeID, JPH56::SubShapeID)

typedef uint8_t JPC56_BroadPhaseLayer;
ENSURE_SIZE_ALIGN(JPC56_BroadPhaseLayer, JPH56::BroadPhaseLayer)

#ifndef JPC56_OBJECT_LAYER_BITS
	#define JPC56_OBJECT_LAYER_BITS 16
#endif

#if JPC56_OBJECT_LAYER_BITS == 16
	typedef uint16_t JPC56_ObjectLayer;
#elif JPC56_OBJECT_LAYER_BITS == 32
	typedef uint32_t JPC56_ObjectLayer;
#else
	#error "JPC56_OBJECT_LAYER_BITS must be 16 or 32"
#endif

ENSURE_SIZE_ALIGN(JPC56_ObjectLayer, JPH56::ObjectLayer)

typedef struct JPC56_IndexedTriangleNoMaterial {
	uint32_t idx[3];
} JPC56_IndexedTriangleNoMaterial;

ENSURE_SIZE_ALIGN(JPC56_IndexedTriangleNoMaterial, JPH56::IndexedTriangleNoMaterial)

typedef struct JPC56_IndexedTriangle {
	uint32_t idx[3];
	uint32_t materialIndex;
	uint32_t userData;
} JPC56_IndexedTriangle;

ENSURE_SIZE_ALIGN(JPC56_IndexedTriangle, JPH56::IndexedTriangle)

typedef struct JPC56_RayCast {
	JPC56_Vec3 Origin;
	JPC56_Vec3 Direction;
} JPC56_RayCast;

typedef struct JPC56_RRayCast {
	JPC56_RVec3 Origin;
	JPC56_Vec3 Direction;
} JPC56_RRayCast;

typedef struct JPC56_RayCastResult {
	JPC56_BodyID BodyID;
	float Fraction;
	JPC56_SubShapeID SubShapeID2;
} JPC56_RayCastResult;

typedef struct JPC56_ShapeCastResult {
	// From CollideShapeResult
	JPC56_Vec3 ContactPointOn1;
	JPC56_Vec3 ContactPointOn2;
	JPC56_Vec3 PenetrationAxis;
	float PenetrationDepth;
	JPC56_SubShapeID SubShapeID1;
	JPC56_SubShapeID SubShapeID2;
	JPC56_BodyID BodyID2;
	// Face Shape1Face;
	// Face Shape2Face;

	// From ShapeCastResult
	float Fraction;
	bool IsBackFaceHit;
} JPC56_ShapeCastResult;

typedef struct JPC56_CollideShapeResult {
	JPC56_Vec3 ContactPointOn1;
	JPC56_Vec3 ContactPointOn2;
	JPC56_Vec3 PenetrationAxis;
	float PenetrationDepth;
	JPC56_SubShapeID SubShapeID1;
	JPC56_SubShapeID SubShapeID2;
	JPC56_BodyID BodyID2;
	// Face Shape1Face;
	// Face Shape2Face;
} JPC56_CollideShapeResult;

typedef struct JPC56_Body JPC56_Body;

////////////////////////////////////////////////////////////////////////////////
// VertexList == Array<Float3> == std::vector<Float3>

typedef struct JPC56_VertexList JPC56_VertexList;

JPC56_API JPC56_VertexList* JPC56_VertexList_new(const JPC56_Float3* storage, size_t len);
JPC56_API void JPC56_VertexList_delete(JPC56_VertexList* object);

////////////////////////////////////////////////////////////////////////////////
// IndexedTriangleList == Array<IndexedTriangle> == std::vector<IndexedTriangle>

typedef struct JPC56_IndexedTriangleList JPC56_IndexedTriangleList;

JPC56_API JPC56_IndexedTriangleList* JPC56_IndexedTriangleList_new(const JPC56_IndexedTriangle* storage, size_t len);
JPC56_API void JPC56_IndexedTriangleList_delete(JPC56_IndexedTriangleList* object);

////////////////////////////////////////////////////////////////////////////////
// Shape -> RefTarget<Shape>

typedef struct JPC56_Shape JPC56_Shape;

JPC56_API uint32_t JPC56_Shape_GetRefCount(const JPC56_Shape* self);
JPC56_API void JPC56_Shape_AddRef(const JPC56_Shape* self);
JPC56_API void JPC56_Shape_Release(const JPC56_Shape* self);

JPC56_API uint64_t JPC56_Shape_GetUserData(const JPC56_Shape* self);
JPC56_API void JPC56_Shape_SetUserData(JPC56_Shape* self, uint64_t userData);

JPC56_API JPC56_ShapeType JPC56_Shape_GetType(const JPC56_Shape* self);
JPC56_API JPC56_ShapeSubType JPC56_Shape_GetSubType(const JPC56_Shape* self);

JPC56_API uint64_t JPC56_Shape_GetSubShapeUserData(const JPC56_Shape* self, JPC56_SubShapeID inSubShapeID);

JPC56_API JPC56_Vec3 JPC56_Shape_GetCenterOfMass(const JPC56_Shape* self);
JPC56_API float JPC56_Shape_GetVolume(const JPC56_Shape* self);

////////////////////////////////////////////////////////////////////////////////
// CompoundShape -> Shape -> RefTarget<Shape>

typedef struct JPC56_CompoundShape JPC56_CompoundShape;

// FIXME: The real API should return a new type, JPC56_CompoundShape_SubShape*
JPC56_API const JPC56_Shape* JPC56_CompoundShape_GetSubShape_Shape(
	const JPC56_CompoundShape* self,
	uint inIdx);

JPC56_API uint32_t JPC56_CompoundShape_GetSubShapeIndexFromID(
	const JPC56_CompoundShape* self,
	JPC56_SubShapeID inSubShapeID,
	JPC56_SubShapeID* outRemainder);

////////////////////////////////////////////////////////////////////////////////
// TempAllocatorImpl

typedef struct JPC56_TempAllocatorImpl JPC56_TempAllocatorImpl;

JPC56_API JPC56_TempAllocatorImpl* JPC56_TempAllocatorImpl_new(uint size);
JPC56_API void JPC56_TempAllocatorImpl_delete(JPC56_TempAllocatorImpl* object);

////////////////////////////////////////////////////////////////////////////////
// JobSystem

typedef struct JPC56_JobSystem JPC56_JobSystem;
typedef struct JPC56_JobSystemThreadPool JPC56_JobSystemThreadPool;
typedef struct JPC56_JobSystemSingleThreaded JPC56_JobSystemSingleThreaded;

JPC56_API JPC56_JobSystemThreadPool* JPC56_JobSystemThreadPool_new2(
	uint inMaxJobs,
	uint inMaxBarriers);
JPC56_API JPC56_JobSystemThreadPool* JPC56_JobSystemThreadPool_new3(
	uint inMaxJobs,
	uint inMaxBarriers,
	int inNumThreads);

JPC56_API void JPC56_JobSystemThreadPool_delete(JPC56_JobSystemThreadPool* object);

JPC56_API JPC56_JobSystemSingleThreaded* JPC56_JobSystemSingleThreaded_new(uint inMaxJobs);
JPC56_API void JPC56_JobSystemSingleThreaded_delete(JPC56_JobSystemSingleThreaded* object);

////////////////////////////////////////////////////////////////////////////////
// CollisionGroup and GroupFilter

typedef uint32_t JPC56_GroupID;
typedef uint32_t JPC56_SubGroupID;
typedef struct JPC56_GroupFilter JPC56_GroupFilter;

typedef struct JPC56_CollisionGroup {
	const JPC56_GroupFilter* GroupFilter;
	JPC56_GroupID GroupID;
	JPC56_SubGroupID SubGroupID;
} JPC56_CollisionGroup;

typedef struct JPC56_GroupFilterFns {
	bool (*CanCollide)(const void *self, const JPC56_CollisionGroup* inGroup1, const JPC56_CollisionGroup* inGroup2);
} JPC56_GroupFilterFns;

JPC56_API JPC56_GroupFilter* JPC56_GroupFilter_new(
	const void *self,
	JPC56_GroupFilterFns fns);

JPC56_API void JPC56_GroupFilter_delete(JPC56_GroupFilter* object);

////////////////////////////////////////////////////////////////////////////////
// BroadPhaseLayerInterface

typedef struct JPC56_BroadPhaseLayerInterfaceFns {
	uint (*GetNumBroadPhaseLayers)(const void *self);
	JPC56_BroadPhaseLayer (*GetBroadPhaseLayer)(const void *self, JPC56_ObjectLayer inLayer);
} JPC56_BroadPhaseLayerInterfaceFns;

typedef struct JPC56_BroadPhaseLayerInterface JPC56_BroadPhaseLayerInterface;

JPC56_API JPC56_BroadPhaseLayerInterface* JPC56_BroadPhaseLayerInterface_new(
	const void *self,
	JPC56_BroadPhaseLayerInterfaceFns fns);

JPC56_API void JPC56_BroadPhaseLayerInterface_delete(JPC56_BroadPhaseLayerInterface* object);

////////////////////////////////////////////////////////////////////////////////
// BroadPhaseLayerFilter

typedef struct JPC56_BroadPhaseLayerFilterFns {
	bool (*ShouldCollide)(const void *self, JPC56_BroadPhaseLayer inLayer);
} JPC56_BroadPhaseLayerFilterFns;

typedef struct JPC56_BroadPhaseLayerFilter JPC56_BroadPhaseLayerFilter;

JPC56_API JPC56_BroadPhaseLayerFilter* JPC56_BroadPhaseLayerFilter_new(
	const void *self,
	JPC56_BroadPhaseLayerFilterFns fns);

JPC56_API void JPC56_BroadPhaseLayerFilter_delete(JPC56_BroadPhaseLayerFilter* object);

////////////////////////////////////////////////////////////////////////////////
// ObjectLayerFilter

typedef struct JPC56_ObjectLayerFilterFns {
	bool (*ShouldCollide)(const void *self, JPC56_ObjectLayer inLayer);
} JPC56_ObjectLayerFilterFns;

typedef struct JPC56_ObjectLayerFilter JPC56_ObjectLayerFilter;

JPC56_API JPC56_ObjectLayerFilter* JPC56_ObjectLayerFilter_new(
	const void *self,
	JPC56_ObjectLayerFilterFns fns);

JPC56_API void JPC56_ObjectLayerFilter_delete(JPC56_ObjectLayerFilter* object);

////////////////////////////////////////////////////////////////////////////////
// BodyFilter

typedef struct JPC56_BodyFilterFns {
	bool (*ShouldCollide)(const void *self, JPC56_BodyID inBodyID);
	bool (*ShouldCollideLocked)(const void *self, const JPC56_Body *inBodyID);
} JPC56_BodyFilterFns;

typedef struct JPC56_BodyFilter JPC56_BodyFilter;

JPC56_API JPC56_BodyFilter* JPC56_BodyFilter_new(
	const void *self,
	JPC56_BodyFilterFns fns);

JPC56_API void JPC56_BodyFilter_delete(JPC56_BodyFilter* object);

////////////////////////////////////////////////////////////////////////////////
// ShapeFilter

typedef struct JPC56_ShapeFilterFns {
	bool (*ShouldCollide)(const void *self, const JPC56_Shape *inShape2, JPC56_SubShapeID inSubShapeIDOfShape2);

	bool (*ShouldCollideTwoShapes)(const void *self,
		const JPC56_Shape *inShape1, JPC56_SubShapeID inSubShapeIDOfShape1,
		const JPC56_Shape *inShape2, JPC56_SubShapeID inSubShapeIDOfShape2);
} JPC56_ShapeFilterFns;

typedef struct JPC56_ShapeFilter JPC56_ShapeFilter;

JPC56_API JPC56_ShapeFilter* JPC56_ShapeFilter_new(
	const void *self,
	JPC56_ShapeFilterFns fns);

JPC56_API void JPC56_ShapeFilter_delete(JPC56_ShapeFilter* object);

////////////////////////////////////////////////////////////////////////////////
// SimShapeFilter

typedef struct JPC56_SimShapeFilterFns {
	bool (*ShouldCollide)(
		const void *self,
		const JPC56_Body *inBody1, const JPC56_Shape *inShape1, JPC56_SubShapeID inSubShapeIDOfShape1,
		const JPC56_Body *inBody2, const JPC56_Shape *inShape2, JPC56_SubShapeID inSubShapeIDOfShape2);
} JPC56_SimShapeFilterFns;

typedef struct JPC56_SimShapeFilter JPC56_SimShapeFilter;

JPC56_API JPC56_SimShapeFilter* JPC56_SimShapeFilter_new(
	const void *self,
	JPC56_SimShapeFilterFns fns);

JPC56_API void JPC56_SimShapeFilter_delete(JPC56_SimShapeFilter* object);

////////////////////////////////////////////////////////////////////////////////
// ObjectVsBroadPhaseLayerFilter

typedef struct JPC56_ObjectVsBroadPhaseLayerFilterFns {
	bool (*ShouldCollide)(const void *self, JPC56_ObjectLayer inLayer1, JPC56_BroadPhaseLayer inLayer2);
} JPC56_ObjectVsBroadPhaseLayerFilterFns;

typedef struct JPC56_ObjectVsBroadPhaseLayerFilter JPC56_ObjectVsBroadPhaseLayerFilter;

JPC56_API JPC56_ObjectVsBroadPhaseLayerFilter* JPC56_ObjectVsBroadPhaseLayerFilter_new(
	const void *self,
	JPC56_ObjectVsBroadPhaseLayerFilterFns fns);

JPC56_API void JPC56_ObjectVsBroadPhaseLayerFilter_delete(JPC56_ObjectVsBroadPhaseLayerFilter* object);

////////////////////////////////////////////////////////////////////////////////
// ObjectLayerPairFilter

typedef struct JPC56_ObjectLayerPairFilterFns {
	bool (*ShouldCollide)(const void *self, JPC56_ObjectLayer inLayer1, JPC56_ObjectLayer inLayer2);
} JPC56_ObjectLayerPairFilterFns;

typedef struct JPC56_ObjectLayerPairFilter JPC56_ObjectLayerPairFilter;

JPC56_API JPC56_ObjectLayerPairFilter* JPC56_ObjectLayerPairFilter_new(
	const void *self,
	JPC56_ObjectLayerPairFilterFns fns);

JPC56_API void JPC56_ObjectLayerPairFilter_delete(JPC56_ObjectLayerPairFilter* object);

////////////////////////////////////////////////////////////////////////////////
// ContactListener

typedef struct JPC56_ContactPoints {
	uint length;
	JPC56_Vec3 points[64];
} JPC56_ContactPoints;

ENSURE_SIZE_ALIGN(JPC56_ContactPoints, JPH56::ContactPoints)

typedef struct JPC56_ContactManifold {
	JPC56_RVec3 BaseOffset;
	JPC56_Vec3 WorldSpaceNormal;
	float PenetrationDepth;
	JPC56_SubShapeID SubShapeID1;
	JPC56_SubShapeID SubShapeID2;
	JPC56_ContactPoints RelativeContactPointsOn1;
	JPC56_ContactPoints RelativeContactPointsOn2;
} JPC56_ContactManifold;

ENSURE_SIZE_ALIGN(JPC56_ContactManifold, JPH56::ContactManifold)
ENSURE_NORMAL_FIELD(  ContactManifold, BaseOffset)
ENSURE_NORMAL_FIELD(  ContactManifold, WorldSpaceNormal)
ENSURE_NORMAL_FIELD(  ContactManifold, PenetrationDepth)
ENSURE_NORMAL_FIELD(  ContactManifold, SubShapeID1)
ENSURE_NORMAL_FIELD(  ContactManifold, SubShapeID2)
ENSURE_NORMAL_FIELD(  ContactManifold, RelativeContactPointsOn1)
ENSURE_NORMAL_FIELD(  ContactManifold, RelativeContactPointsOn2)

typedef struct JPC56_ContactSettings {
	float CombinedFriction;
	float CombinedRestitution;
	float InvMassScale1;
	float InvInertiaScale1;
	float InvMassScale2;
	float InvInertiaScale2;
	bool IsSensor;
	JPC56_Vec3 RelativeLinearSurfaceVelocity;
	JPC56_Vec3 RelativeAngularSurfaceVelocity;
} JPC56_ContactSettings;

ENSURE_SIZE_ALIGN(JPC56_ContactSettings, JPH56::ContactSettings)
ENSURE_NORMAL_FIELD(  ContactSettings, CombinedFriction)
ENSURE_NORMAL_FIELD(  ContactSettings, CombinedRestitution)
ENSURE_NORMAL_FIELD(  ContactSettings, InvMassScale1)
ENSURE_NORMAL_FIELD(  ContactSettings, InvInertiaScale1)
ENSURE_NORMAL_FIELD(  ContactSettings, InvMassScale2)
ENSURE_NORMAL_FIELD(  ContactSettings, InvInertiaScale2)
ENSURE_NORMAL_FIELD(  ContactSettings, IsSensor)
ENSURE_NORMAL_FIELD(  ContactSettings, RelativeLinearSurfaceVelocity)
ENSURE_NORMAL_FIELD(  ContactSettings, RelativeAngularSurfaceVelocity)

typedef struct JPC56_SubShapeIDPair {
	JPC56_BodyID Body1ID;
	JPC56_SubShapeID SubShapeID1;
	JPC56_BodyID Body2ID;
	JPC56_SubShapeID SubShapeID2;
} JPC56_SubShapeIDPair;

ENSURE_SIZE_ALIGN(JPC56_SubShapeIDPair, JPH56::SubShapeIDPair)
// These fields are private, so we can't test them directly!
// ENSURE_NORMAL_FIELD(  SubShapeIDPair, Body1ID)
// ENSURE_NORMAL_FIELD(  SubShapeIDPair, SubShapeID1)
// ENSURE_NORMAL_FIELD(  SubShapeIDPair, Body2ID)
// ENSURE_NORMAL_FIELD(  SubShapeIDPair, SubShapeID2)

typedef struct JPC56_ShapeCastSettings {
	// JPH56::CollideSettingsBase
	JPC56_ActiveEdgeMode ActiveEdgeMode;
	JPC56_CollectFacesMode CollectFacesMode;
	float CollisionTolerance;
	float PenetrationTolerance;
	JPC56_Vec3 ActiveEdgeMovementDirection;

	// JPH56::ShapeCastSettings
	float ExtraConvexRadius; // rurix M125: Jolt 5.6 mExtraConvexRadius(ShapeCastSettings.h)
	JPC56_BackFaceMode BackFaceModeTriangles;
	JPC56_BackFaceMode BackFaceModeConvex;
	bool UseShrunkenShapeAndConvexRadius;
	bool ReturnDeepestPoint;
} JPC56_ShapeCastSettings;

ENSURE_SIZE_ALIGN(JPC56_ShapeCastSettings, JPH56::ShapeCastSettings)
ENSURE_NORMAL_FIELD(  ShapeCastSettings, ActiveEdgeMode)
ENSURE_NORMAL_FIELD(  ShapeCastSettings, CollectFacesMode)
ENSURE_NORMAL_FIELD(  ShapeCastSettings, CollisionTolerance)
ENSURE_NORMAL_FIELD(  ShapeCastSettings, PenetrationTolerance)
ENSURE_NORMAL_FIELD(  ShapeCastSettings, ActiveEdgeMovementDirection)
ENSURE_NORMAL_FIELD(  ShapeCastSettings, BackFaceModeTriangles)
ENSURE_NORMAL_FIELD(  ShapeCastSettings, BackFaceModeConvex)
ENSURE_NORMAL_FIELD(  ShapeCastSettings, UseShrunkenShapeAndConvexRadius)
ENSURE_NORMAL_FIELD(  ShapeCastSettings, ReturnDeepestPoint)

typedef struct JPC56_CollideShapeSettings {
	// CollideSettingsBase
	JPC56_ActiveEdgeMode ActiveEdgeMode;
	JPC56_CollectFacesMode CollectFacesMode;
	float CollisionTolerance;
	float PenetrationTolerance;
	JPC56_Vec3 ActiveEdgeMovementDirection;

	// CollideShapeSettings
	float MaxSeparationDistance;
	JPC56_BackFaceMode BackFaceMode;
	float InternalEdgeRemovalVertexToleranceSq; // rurix M125: Jolt 5.6 mInternalEdgeRemovalVertexToleranceSq(CollideShape.h)
} JPC56_CollideShapeSettings;

ENSURE_SIZE_ALIGN(JPC56_CollideShapeSettings, JPH56::CollideShapeSettings)
ENSURE_NORMAL_FIELD(  CollideShapeSettings, ActiveEdgeMode)
ENSURE_NORMAL_FIELD(  CollideShapeSettings, CollectFacesMode)
ENSURE_NORMAL_FIELD(  CollideShapeSettings, CollisionTolerance)
ENSURE_NORMAL_FIELD(  CollideShapeSettings, PenetrationTolerance)
ENSURE_NORMAL_FIELD(  CollideShapeSettings, ActiveEdgeMovementDirection)
ENSURE_NORMAL_FIELD(  CollideShapeSettings, MaxSeparationDistance)
ENSURE_NORMAL_FIELD(  CollideShapeSettings, BackFaceMode)

typedef struct JPC56_ContactListenerFns {
	JPC56_ValidateResult (*OnContactValidate)(
		void *self,
		const JPC56_Body *inBody1,
		const JPC56_Body *inBody2,
		JPC56_RVec3 inBaseOffset,
		const JPC56_CollideShapeResult *inCollisionResult);

	void (*OnContactAdded)(
		void *self,
		const JPC56_Body *inBody1,
		const JPC56_Body *inBody2,
		const JPC56_ContactManifold *inManifold,
		JPC56_ContactSettings *ioSettings);

	void (*OnContactPersisted)(
		void *self,
		const JPC56_Body *inBody1,
		const JPC56_Body *inBody2,
		const JPC56_ContactManifold *inManifold,
		JPC56_ContactSettings *ioSettings);

	void (*OnContactRemoved)(
		void *self,
		const JPC56_SubShapeIDPair *inSubShapePair);
} JPC56_ContactListenerFns;

typedef struct JPC56_ContactListener JPC56_ContactListener;

JPC56_API JPC56_ContactListener* JPC56_ContactListener_new(
	void *self,
	JPC56_ContactListenerFns fns);

JPC56_API void JPC56_ContactListener_delete(JPC56_ContactListener* object);

static const uint JPC56_ContactPointsCapacity = 64;

// rurix M125: Jolt 5.6 新摩擦模型(平均接触点)——逐点 Impulse{Contact,Friction1,Friction2}
// 已删除,改为聚合摩擦字段(FrictionPoint/Tangent1/Tangent2 + FrictionImpulse1/2 +
// AngularFrictionImpulse)+ 逐点 ContactImpulse float 数组(EstimateCollisionResponse.h @v5.6.0)。
typedef struct JPC56_CollisionEstimationResult {
	JPC56_Vec3 LinearVelocity1;				///< The estimated linear velocity of body 1 after collision
	JPC56_Vec3 AngularVelocity1;				///< The estimated angular velocity of body 1 after collision
	JPC56_Vec3 LinearVelocity2;				///< The estimated linear velocity of body 2 after collision
	JPC56_Vec3 AngularVelocity2;				///< The estimated angular velocity of body 2 after collision

	JPC56_Vec3 FrictionPoint;					///< Point at which friction was applied (relative to mBaseOffset of the manifold)
	JPC56_Vec3 Tangent1;						///< Normalized tangent of contact normal
	JPC56_Vec3 Tangent2;						///< Second normalized tangent of contact normal (forms a basis with mTangent1 and mWorldSpaceNormal)

	float FrictionImpulse1;					///< Estimated friction impulses in the direction of tangent 1 (kg m / s)
	float FrictionImpulse2;					///< Estimated friction impulses in the direction of tangent 2 (kg m / s)
	float AngularFrictionImpulse;			///< Estimated angular friction impulse around the world space normal (kg m^2 / s)

	uint NumImpulses;
	float Impulses[JPC56_ContactPointsCapacity];	///< Estimated contact impulses (kg m / s)
} JPC56_CollisionEstimationResult;

ENSURE_SIZE_ALIGN(JPC56_CollisionEstimationResult, JPH56::CollisionEstimationResult)

JPC56_API void JPC56_EstimateCollisionResponse(
	const JPC56_Body* inBody1,
	const JPC56_Body* inBody2,
	const JPC56_ContactManifold* inManifold,
	JPC56_CollisionEstimationResult* outResult,
	float inCombinedFriction,
	float inCombinedRestitution,
	float inMinVelocityForRestitution,	///< = 1.0f
	uint inNumIterations				///< = 10
);

////////////////////////////////////////////////////////////////////////////////
// CastShapeCollector

typedef struct JPC56_CastShapeCollector JPC56_CastShapeCollector;

typedef struct JPC56_CastShapeCollectorFns {
	void (*Reset)(void *self);
	void (*AddHit)(void *self, JPC56_CastShapeCollector *base, const JPC56_ShapeCastResult *Result);
} JPC56_CastShapeCollectorFns;

JPC56_API JPC56_CastShapeCollector* JPC56_CastShapeCollector_new(
	void *self,
	JPC56_CastShapeCollectorFns fns);

JPC56_API void JPC56_CastShapeCollector_delete(JPC56_CastShapeCollector* object);

JPC56_API void JPC56_CastShapeCollector_UpdateEarlyOutFraction(JPC56_CastShapeCollector *self, float inFraction);

////////////////////////////////////////////////////////////////////////////////
// CollideShapeCollector

typedef struct JPC56_CollideShapeCollector JPC56_CollideShapeCollector;

typedef struct JPC56_CollideShapeCollectorFns {
	void (*Reset)(void *self);
	void (*AddHit)(void *self, JPC56_CollideShapeCollector *base, const JPC56_CollideShapeResult *Result);
} JPC56_CollideShapeCollectorFns;

JPC56_API JPC56_CollideShapeCollector* JPC56_CollideShapeCollector_new(
	void *self,
	JPC56_CollideShapeCollectorFns fns);

JPC56_API void JPC56_CollideShapeCollector_delete(JPC56_CollideShapeCollector* object);

JPC56_API void JPC56_CollideShapeCollector_UpdateEarlyOutFraction(JPC56_CollideShapeCollector *self, float inFraction);

////////////////////////////////////////////////////////////////////////////////
// DrawSettings

typedef struct JPC56_BodyManager_DrawSettings {
	bool mDrawGetSupportFunction;
	bool mDrawSupportDirection;
	bool mDrawGetSupportingFace;
	bool mDrawShape;
	bool mDrawShapeWireframe;
	JPC56_ShapeColor mDrawShapeColor;
	bool mDrawBoundingBox;
	bool mDrawCenterOfMassTransform;
	bool mDrawWorldTransform;
	bool mDrawVelocity;
	bool mDrawMassAndInertia;
	bool mDrawSleepStats;
	bool mDrawSoftBodyVertices;
	bool mDrawSoftBodyVertexVelocities;
	bool mDrawSoftBodyEdgeConstraints;
	bool mDrawSoftBodyBendConstraints;
	bool mDrawSoftBodyVolumeConstraints;
	bool mDrawSoftBodySkinConstraints;
	bool mDrawSoftBodyLRAConstraints;
	bool mDrawSoftBodyRods; // rurix M125: Jolt 5.6 strand-hair debug fields(BodyManager.h)
	bool mDrawSoftBodyRodStates;
	bool mDrawSoftBodyRodBendTwistConstraints;
	bool mDrawSoftBodyPredictedBounds;
	JPC56_SoftBodyConstraintColor DrawSoftBodyConstraintColor;
} JPC56_BodyManager_DrawSettings;

ENSURE_SIZE_ALIGN(JPC56_BodyManager_DrawSettings, JPH56::BodyManager::DrawSettings)

JPC56_API void JPC56_BodyManager_DrawSettings_default(JPC56_BodyManager_DrawSettings* object);

////////////////////////////////////////////////////////////////////////////////
// DebugRendererSimple

typedef struct JPC56_DebugRendererSimpleFns {
	void (*DrawLine)(const void *self, JPC56_RVec3 inFrom, JPC56_RVec3 inTo, JPC56_Color inColor);
} JPC56_DebugRendererSimpleFns;

typedef struct JPC56_DebugRendererSimple JPC56_DebugRendererSimple;

JPC56_API JPC56_DebugRendererSimple* JPC56_DebugRendererSimple_new(
	const void *self,
	JPC56_DebugRendererSimpleFns fns);

JPC56_API void JPC56_DebugRendererSimple_delete(JPC56_DebugRendererSimple* object);

////////////////////////////////////////////////////////////////////////////////
// String

typedef struct JPC56_String JPC56_String;

JPC56_API void JPC56_String_delete(JPC56_String* self);
JPC56_API const char* JPC56_String_c_str(JPC56_String* self);

////////////////////////////////////////////////////////////////////////////////
// Constraint -> RefTarget<Constraint>

typedef struct JPC56_Constraint JPC56_Constraint;

JPC56_API uint32_t JPC56_Constraint_GetRefCount(const JPC56_Constraint* self);
JPC56_API void JPC56_Constraint_AddRef(const JPC56_Constraint* self);
JPC56_API void JPC56_Constraint_Release(const JPC56_Constraint* self);

JPC56_API void JPC56_Constraint_delete(JPC56_Constraint* self);

// JPC56_API JPC56_ConstraintType JPC56_Constraint_GetType(const JPC56_Constraint* self);
// JPC56_API JPC56_ConstraintSubType JPC56_Constraint_GetSubType(const JPC56_Constraint* self);

JPC56_API uint32_t JPC56_Constraint_GetConstraintPriority(const JPC56_Constraint* self);
JPC56_API void JPC56_Constraint_SetConstraintPriority(JPC56_Constraint* self, uint32_t inPriority);

JPC56_API uint JPC56_Constraint_GetNumVelocityStepsOverride(const JPC56_Constraint* self);
JPC56_API void JPC56_Constraint_SetNumVelocityStepsOverride(JPC56_Constraint* self, uint inN);

JPC56_API uint JPC56_Constraint_GetNumPositionStepsOverride(const JPC56_Constraint* self);
JPC56_API void JPC56_Constraint_SetNumPositionStepsOverride(JPC56_Constraint* self, uint inN);

JPC56_API bool JPC56_Constraint_GetEnabled(const JPC56_Constraint* self);
JPC56_API void JPC56_Constraint_SetEnabled(JPC56_Constraint* self, bool inEnabled);

JPC56_API uint64_t JPC56_Constraint_GetUserData(const JPC56_Constraint* self);
JPC56_API void JPC56_Constraint_SetUserData(JPC56_Constraint* self, uint64_t inUserData);

JPC56_API void JPC56_Constraint_NotifyShapeChanged(JPC56_Constraint* self, JPC56_BodyID inBodyID, JPC56_Vec3 inDeltaCOM);

////////////////////////////////////////////////////////////////////////////////
// TwoBodyConstraint -> Constraint -> RefTarget<Constraint>

typedef struct JPC56_TwoBodyConstraint JPC56_TwoBodyConstraint;

JPC56_API JPC56_Body* JPC56_TwoBodyConstraint_GetBody1(const JPC56_TwoBodyConstraint* self);
JPC56_API JPC56_Body* JPC56_TwoBodyConstraint_GetBody2(const JPC56_TwoBodyConstraint* self);

JPC56_API JPC56_Mat44 JPC56_TwoBodyConstraint_GetConstraintToBody1Matrix(const JPC56_TwoBodyConstraint* self);
JPC56_API JPC56_Mat44 JPC56_TwoBodyConstraint_GetConstraintToBody2Matrix(const JPC56_TwoBodyConstraint* self);

////////////////////////////////////////////////////////////////////////////////
// FixedConstraint -> TwoBodyConstraint -> Constraint -> RefTarget<Constraint>

typedef struct JPC56_FixedConstraint JPC56_FixedConstraint;

JPC56_API JPC56_Vec3 JPC56_FixedConstraint_GetTotalLambdaPosition(const JPC56_FixedConstraint* self);
JPC56_API JPC56_Vec3 JPC56_FixedConstraint_GetTotalLambdaRotation(const JPC56_FixedConstraint* self);

////////////////////////////////////////////////////////////////////////////////
// DistanceConstraint -> TwoBodyConstraint -> Constraint -> RefTarget<Constraint>

typedef struct JPC56_DistanceConstraint JPC56_DistanceConstraint;

JPC56_API float JPC56_DistanceConstraint_GetTotalLambdaPosition(const JPC56_DistanceConstraint* self);

////////////////////////////////////////////////////////////////////////////////
// SixDOFConstraint -> TwoBodyConstraint -> Constraint -> RefTarget<Constraint>

typedef struct JPC56_SixDOFConstraint JPC56_SixDOFConstraint;

JPC56_API JPC56_Vec3 JPC56_SixDOFConstraint_GetTranslationLimitsMin(const JPC56_SixDOFConstraint* self);
JPC56_API JPC56_Vec3 JPC56_SixDOFConstraint_GetTranslationLimitsMax(const JPC56_SixDOFConstraint* self);
JPC56_API void JPC56_SixDOFConstraint_SetTranslationLimits(JPC56_SixDOFConstraint* self, JPC56_Vec3 inLimitMin, JPC56_Vec3 inLimitMax);

JPC56_API JPC56_Vec3 JPC56_SixDOFConstraint_GetRotationLimitsMin(const JPC56_SixDOFConstraint* self);
JPC56_API JPC56_Vec3 JPC56_SixDOFConstraint_GetRotationLimitsMax(const JPC56_SixDOFConstraint* self);
JPC56_API void JPC56_SixDOFConstraint_SetRotationLimits(JPC56_SixDOFConstraint* self, JPC56_Vec3 inLimitMin, JPC56_Vec3 inLimitMax);

JPC56_API float JPC56_SixDOFConstraint_GetLimitsMin(const JPC56_SixDOFConstraint* self, JPC56_SixDOFConstraint_Axis inAxis);
JPC56_API float JPC56_SixDOFConstraint_GetLimitsMax(const JPC56_SixDOFConstraint* self, JPC56_SixDOFConstraint_Axis inAxis);

JPC56_API bool JPC56_SixDOFConstraint_IsFreeAxis(const JPC56_SixDOFConstraint* self, JPC56_SixDOFConstraint_Axis inAxis);

// const SpringSettings & GetLimitsSpringSettings(JPC56_SixDOFConstraint_Axis inAxis) const { JPH_ASSERT(inAxis < JPC56_SixDOFConstraint_Axis::NumTranslation); return mLimitsSpringSettings[inAxis]; }
// void SetLimitsSpringSettings(JPC56_SixDOFConstraint_Axis inAxis, const SpringSettings& inLimitsSpringSettings) { JPH_ASSERT(inAxis < JPC56_SixDOFConstraint_Axis::NumTranslation); mLimitsSpringSettings[inAxis] = inLimitsSpringSettings; CacheHasSpringLimits(); }

JPC56_API void JPC56_SixDOFConstraint_SetMaxFriction(JPC56_SixDOFConstraint* self, JPC56_SixDOFConstraint_Axis inAxis, float inFriction);
JPC56_API float JPC56_SixDOFConstraint_GetMaxFriction(const JPC56_SixDOFConstraint* self, JPC56_SixDOFConstraint_Axis inAxis);

JPC56_API JPC56_Quat JPC56_SixDOFConstraint_GetRotationInConstraintSpace(const JPC56_SixDOFConstraint* self);

/// Motor settings
// MotorSettings & GetMotorSettings(EAxis inAxis)
// const MotorSettings & GetMotorSettings(EAxis inAxis) const

// void SetMotorState(EAxis inAxis, EMotorState inState);
// EMotorState GetMotorState(EAxis inAxis) const

JPC56_API JPC56_Vec3 JPC56_SixDOFConstraint_GetTargetVelocityCS(const JPC56_SixDOFConstraint* self);
JPC56_API void JPC56_SixDOFConstraint_SetTargetVelocityCS(JPC56_SixDOFConstraint* self, JPC56_Vec3 inVelocity);

JPC56_API JPC56_Vec3 JPC56_SixDOFConstraint_GetTargetAngularVelocityCS(const JPC56_SixDOFConstraint* self);
JPC56_API void JPC56_SixDOFConstraint_SetTargetAngularVelocityCS(JPC56_SixDOFConstraint* self, JPC56_Vec3 inAngularVelocity);

JPC56_API JPC56_Vec3 JPC56_SixDOFConstraint_GetTargetPositionCS(const JPC56_SixDOFConstraint* self);
JPC56_API void JPC56_SixDOFConstraint_SetTargetPositionCS(JPC56_SixDOFConstraint* self, JPC56_Vec3 inPosition);

JPC56_API JPC56_Quat JPC56_SixDOFConstraint_GetTargetOrientationCS(const JPC56_SixDOFConstraint* self);
JPC56_API void JPC56_SixDOFConstraint_SetTargetOrientationCS(JPC56_SixDOFConstraint* self, JPC56_Quat inOrientation);

JPC56_API void JPC56_SixDOFConstraint_SetTargetOrientationBS(JPC56_SixDOFConstraint* self, JPC56_Quat inOrientation);

JPC56_API JPC56_Vec3 JPC56_SixDOFConstraint_GetTotalLambdaPosition(JPC56_SixDOFConstraint* self);
JPC56_API JPC56_Vec3 JPC56_SixDOFConstraint_GetTotalLambdaRotation(JPC56_SixDOFConstraint* self);
JPC56_API JPC56_Vec3 JPC56_SixDOFConstraint_GetTotalLambdaMotorTranslation(JPC56_SixDOFConstraint* self);
JPC56_API JPC56_Vec3 JPC56_SixDOFConstraint_GetTotalLambdaMotorRotation(JPC56_SixDOFConstraint* self);

////////////////////////////////////////////////////////////////////////////////
// HingeConstraint

typedef struct JPC56_HingeConstraint JPC56_HingeConstraint;

JPC56_API void JPC56_HingeConstraint_SetMotorState(JPC56_HingeConstraint* self, JPC56_MotorState inState);
JPC56_API JPC56_MotorState JPC56_HingeConstraint_GetMotorState(const JPC56_HingeConstraint* self);
JPC56_API void JPC56_HingeConstraint_SetTargetAngularVelocity(JPC56_HingeConstraint* self, float inAngularVelocity);
JPC56_API float JPC56_HingeConstraint_GetTargetAngularVelocity(const JPC56_HingeConstraint* self);
JPC56_API void JPC56_HingeConstraint_SetTargetAngle(JPC56_HingeConstraint* self, float inAngle);
JPC56_API float JPC56_HingeConstraint_GetTargetAngle(const JPC56_HingeConstraint* self);

JPC56_API JPC56_Vec3 JPC56_HingeConstraint_GetTotalLambdaPosition(const JPC56_HingeConstraint* self);
JPC56_API JPC56_Vec2 JPC56_HingeConstraint_GetTotalLambdaRotation(const JPC56_HingeConstraint* self);
JPC56_API float JPC56_HingeConstraint_GetTotalLambdaRotationLimits(const JPC56_HingeConstraint* self);
JPC56_API float JPC56_HingeConstraint_GetTotalLambdaMotor(const JPC56_HingeConstraint* self);

////////////////////////////////////////////////////////////////////////////////
// SliderConstraint

typedef struct JPC56_SliderConstraint JPC56_SliderConstraint;

JPC56_API void JPC56_SliderConstraint_SetMotorState(JPC56_SliderConstraint* self, JPC56_MotorState inState);
JPC56_API JPC56_MotorState JPC56_SliderConstraint_GetMotorState(const JPC56_SliderConstraint* self);
JPC56_API void JPC56_SliderConstraint_SetTargetVelocity(JPC56_SliderConstraint* self, float inVelocity);
JPC56_API float JPC56_SliderConstraint_GetTargetVelocity(const JPC56_SliderConstraint* self);
JPC56_API void JPC56_SliderConstraint_SetTargetPosition(JPC56_SliderConstraint* self, float inPosition);
JPC56_API float JPC56_SliderConstraint_GetTargetPosition(const JPC56_SliderConstraint* self);
JPC56_API JPC56_Vec2 JPC56_SliderConstraint_GetTotalLambdaPosition(const JPC56_SliderConstraint* self);
JPC56_API float JPC56_SliderConstraint_GetTotalLambdaPositionLimits(const JPC56_SliderConstraint* self);
JPC56_API JPC56_Vec3 JPC56_SliderConstraint_GetTotalLambdaRotation(const JPC56_SliderConstraint* self);
JPC56_API float JPC56_SliderConstraint_GetTotalLambdaMotor(const JPC56_SliderConstraint* self);

////////////////////////////////////////////////////////////////////////////////
// ConstraintSettings

typedef struct JPC56_ConstraintSettings {
	bool Enabled;
	uint32_t ConstraintPriority;
	uint NumVelocityStepsOverride;
	uint NumPositionStepsOverride;
	float DrawConstraintSize;
	uint64_t UserData;
} JPC56_ConstraintSettings;

JPC56_API void JPC56_ConstraintSettings_default(JPC56_ConstraintSettings* settings);

////////////////////////////////////////////////////////////////////////////////
// SpringSettings

typedef struct JPC56_SpringSettings {
	JPC56_SpringMode Mode;
	float FrequencyOrStiffness;
	float Damping;
} JPC56_SpringSettings;

JPC56_API void JPC56_SpringSettings_default(JPC56_SpringSettings* settings);

////////////////////////////////////////////////////////////////////////////////
// MotorSettings

typedef struct JPC56_MotorSettings {
	JPC56_SpringSettings SpringSettings;
	float MinForceLimit;
	float MaxForceLimit;
	float MinTorqueLimit;
	float MaxTorqueLimit;
} JPC56_MotorSettings;

JPC56_API void JPC56_MotorSettings_default(JPC56_MotorSettings* settings);

////////////////////////////////////////////////////////////////////////////////
// FixedConstraintSettings -> TwoBodyConstraintSettings -> ConstraintSettings

typedef struct JPC56_FixedConstraintSettings {
	JPC56_ConstraintSettings ConstraintSettings;

	// TwoBodyConstraintSettings: no extra members

	// FixedConstraintSettings
	JPC56_ConstraintSpace Space;
	bool AutoDetectPoint;

	JPC56_RVec3 Point1;
	JPC56_Vec3 AxisX1;
	JPC56_Vec3 AxisY1;

	JPC56_RVec3 Point2;
	JPC56_Vec3 AxisX2;
	JPC56_Vec3 AxisY2;
} JPC56_FixedConstraintSettings;

JPC56_API void JPC56_FixedConstraintSettings_default(JPC56_FixedConstraintSettings* settings);
JPC56_API JPC56_Constraint* JPC56_FixedConstraintSettings_Create(
	const JPC56_FixedConstraintSettings* self,
	JPC56_Body* inBody1,
	JPC56_Body* inBody2);

////////////////////////////////////////////////////////////////////////////////
// SixDOFConstraintSettings -> TwoBodyConstraintSettings -> ConstraintSettings

typedef struct JPC56_SixDOFConstraintSettings {
	JPC56_ConstraintSettings ConstraintSettings;

	// TwoBodyConstraintSettings: no extra members

	// SixDOFConstraintSettings
	JPC56_ConstraintSpace Space;

	JPC56_RVec3 Position1;
	JPC56_Vec3 AxisX1;
	JPC56_Vec3 AxisY1;

	JPC56_RVec3 Position2;
	JPC56_Vec3 AxisX2;
	JPC56_Vec3 AxisY2;

	float MaxFriction[6];

	float LimitMin[6];
	float LimitMax[6];

	// TODO: LimitsSpringSettings
} JPC56_SixDOFConstraintSettings;

JPC56_API void JPC56_SixDOFConstraintSettings_default(JPC56_SixDOFConstraintSettings* settings);
JPC56_API JPC56_Constraint* JPC56_SixDOFConstraintSettings_Create(
	const JPC56_SixDOFConstraintSettings* self,
	JPC56_Body* inBody1,
	JPC56_Body* inBody2);

////////////////////////////////////////////////////////////////////////////////
// HingeConstraintSettings -> TwoBodyConstraintSettings -> ConstraintSettings

typedef struct JPC56_HingeConstraintSettings {
	JPC56_ConstraintSettings ConstraintSettings;

	// TwoBodyConstraintSettings: no extra members

	// HingeConstraintSettings
	JPC56_ConstraintSpace Space;

	JPC56_RVec3 Point1;
	JPC56_Vec3 HingeAxis1;
	JPC56_Vec3 NormalAxis1;

	JPC56_RVec3 Point2;
	JPC56_Vec3 HingeAxis2;
	JPC56_Vec3 NormalAxis2;

	float LimitsMin;
	float LimitsMax;

	JPC56_SpringSettings LimitsSpringSettings;

	float MaxFrictionTorque;

	JPC56_MotorSettings MotorSettings;
} JPC56_HingeConstraintSettings;

JPC56_API void JPC56_HingeConstraintSettings_default(JPC56_HingeConstraintSettings* settings);
JPC56_API JPC56_HingeConstraint* JPC56_HingeConstraintSettings_Create(
	const JPC56_HingeConstraintSettings* self,
	JPC56_Body* inBody1,
	JPC56_Body* inBody2);

////////////////////////////////////////////////////////////////////////////////
// DistanceConstraintSettings -> TwoBodyConstraintSettings -> ConstraintSettings

typedef struct JPC56_DistanceConstraintSettings {
	JPC56_ConstraintSettings ConstraintSettings;

	// TwoBodyConstraintSettings: no extra members

	// DistanceConstraintSettings
	JPC56_ConstraintSpace Space;

	JPC56_RVec3 Point1;
	JPC56_RVec3 Point2;

	float MinDistance;
	float MaxDistance;

	JPC56_SpringSettings LimitsSpringSettings;
} JPC56_DistanceConstraintSettings;

JPC56_API void JPC56_DistanceConstraintSettings_default(JPC56_DistanceConstraintSettings* settings);
JPC56_API JPC56_DistanceConstraint* JPC56_DistanceConstraintSettings_Create(
	const JPC56_DistanceConstraintSettings* self,
	JPC56_Body* inBody1,
	JPC56_Body* inBody2);

////////////////////////////////////////////////////////////////////////////////
// SliderConstraintSettings -> TwoBodyConstraintSettings -> ConstraintSettings

typedef struct JPC56_SliderConstraintSettings {
	JPC56_ConstraintSettings ConstraintSettings;

	// TwoBodyConstraintSettings: no extra members

	// SliderConstraintSettings
	JPC56_ConstraintSpace Space;
	bool AutoDetectPoint;

	JPC56_RVec3 Point1;
	JPC56_Vec3 SliderAxis1;
	JPC56_Vec3 NormalAxis1;

	JPC56_RVec3 Point2;
	JPC56_Vec3 SliderAxis2;
	JPC56_Vec3 NormalAxis2;

	float LimitsMin;
	float LimitsMax;

	JPC56_SpringSettings LimitsSpringSettings;

	float MaxFrictionForce;

	JPC56_MotorSettings MotorSettings;
} JPC56_SliderConstraintSettings;

JPC56_API void JPC56_SliderConstraintSettings_default(JPC56_SliderConstraintSettings* settings);
JPC56_API JPC56_SliderConstraint* JPC56_SliderConstraintSettings_Create(
	const JPC56_SliderConstraintSettings* self,
	JPC56_Body* inBody1,
	JPC56_Body* inBody2);

////////////////////////////////////////////////////////////////////////////////
// TriangleShapeSettings

typedef struct JPC56_TriangleShapeSettings {
	// ShapeSettings
	uint64_t UserData;

	// ConvexShapeSettings
	// TODO: Material
	float Density;

	// TriangleShapeSettings
	JPC56_Vec3 V1;
	JPC56_Vec3 V2;
	JPC56_Vec3 V3;
	float ConvexRadius;
} JPC56_TriangleShapeSettings;

JPC56_API void JPC56_TriangleShapeSettings_default(JPC56_TriangleShapeSettings* object);
JPC56_API bool JPC56_TriangleShapeSettings_Create(const JPC56_TriangleShapeSettings* self, JPC56_Shape** outShape, JPC56_String** outError);

////////////////////////////////////////////////////////////////////////////////
// MeshShapeSettings -> ShapeSettings

typedef struct JPC56_MeshShapeSettings {
	// ShapeSettings
	uint64_t UserData;

	// MeshShapeSettings
	JPC56_Float3* TriangleVertices;
	size_t TriangleVerticesLen;
	JPC56_IndexedTriangle* IndexedTriangles;
	size_t IndexedTrianglesLen;
	// PhysicsMaterialList				mMaterials;
	// uint							mMaxTrianglesPerLeaf = 8;
	// float							mActiveEdgeCosThresholdAngle = 0.996195f;
	// bool							mPerTriangleUserData = false;
} JPC56_MeshShapeSettings;

JPC56_API void JPC56_MeshShapeSettings_default(JPC56_MeshShapeSettings* object);
JPC56_API bool JPC56_MeshShapeSettings_Create(const JPC56_MeshShapeSettings* self, JPC56_Shape** outShape, JPC56_String** outError);

////////////////////////////////////////////////////////////////////////////////
// BoxShapeSettings -> ConvexShapeSettings -> ShapeSettings

typedef struct JPC56_BoxShapeSettings {
	// ShapeSettings
	uint64_t UserData;

	// ConvexShapeSettings
	// TODO: Material
	float Density;

	// BoxShapeSettings
	JPC56_Vec3 HalfExtent;
	float ConvexRadius;
} JPC56_BoxShapeSettings;

JPC56_API void JPC56_BoxShapeSettings_default(JPC56_BoxShapeSettings* object);
JPC56_API bool JPC56_BoxShapeSettings_Create(const JPC56_BoxShapeSettings* self, JPC56_Shape** outShape, JPC56_String** outError);

////////////////////////////////////////////////////////////////////////////////
// SphereShapeSettings -> ConvexShapeSettings -> ShapeSettings

typedef struct JPC56_SphereShapeSettings {
	// ShapeSettings
	uint64_t UserData;

	// ConvexShapeSettings
	// TODO: Material
	float Density;

	// SphereShapeSettings
	float Radius;
} JPC56_SphereShapeSettings;

JPC56_API void JPC56_SphereShapeSettings_default(JPC56_SphereShapeSettings* object);
JPC56_API bool JPC56_SphereShapeSettings_Create(const JPC56_SphereShapeSettings* self, JPC56_Shape** outShape, JPC56_String** outError);

////////////////////////////////////////////////////////////////////////////////
// CapsuleShapeSettings -> ConvexShapeSettings -> ShapeSettings

typedef struct JPC56_CapsuleShapeSettings {
	// ShapeSettings
	uint64_t UserData;

	// ConvexShapeSettings
	// TODO: Material
	float Density;

	// CapsuleShapeSettings
	float Radius;
	float HalfHeightOfCylinder;
} JPC56_CapsuleShapeSettings;

JPC56_API void JPC56_CapsuleShapeSettings_default(JPC56_CapsuleShapeSettings* object);
JPC56_API bool JPC56_CapsuleShapeSettings_Create(const JPC56_CapsuleShapeSettings* self, JPC56_Shape** outShape, JPC56_String** outError);

////////////////////////////////////////////////////////////////////////////////
// CylinderShapeSettings -> ConvexShapeSettings -> ShapeSettings

typedef struct JPC56_CylinderShapeSettings {
	// ShapeSettings
	uint64_t UserData;

	// ConvexShapeSettings
	// TODO: Material
	float Density;

	// CylinderShapeSettings
	float HalfHeight;
	float Radius;
	float ConvexRadius;
} JPC56_CylinderShapeSettings;

JPC56_API void JPC56_CylinderShapeSettings_default(JPC56_CylinderShapeSettings* object);
JPC56_API bool JPC56_CylinderShapeSettings_Create(const JPC56_CylinderShapeSettings* self, JPC56_Shape** outShape, JPC56_String** outError);

////////////////////////////////////////////////////////////////////////////////
// PlaneShapeSettings -> ShapeSettings

typedef struct JPC56_PlaneShapeSettings {
	// ShapeSettings
	uint64_t UserData;

	// PlaneShapeSettings
	// TODO: Material
	JPC56_Vec3 Normal;
	float Constant;
	float HalfExtent;
} JPC56_PlaneShapeSettings;

JPC56_API void JPC56_PlaneShapeSettings_default(JPC56_PlaneShapeSettings* object);
JPC56_API bool JPC56_PlaneShapeSettings_Create(const JPC56_PlaneShapeSettings* self, JPC56_Shape** outShape, JPC56_String** outError);

////////////////////////////////////////////////////////////////////////////////
// ConvexHullShapeSettings -> ConvexShapeSettings -> ShapeSettings

typedef struct JPC56_ConvexHullShapeSettings {
	// ShapeSettings
	uint64_t UserData;

	// ConvexShapeSettings
	// TODO: Material
	float Density;

	// ConvexHullShapeSettings
	const JPC56_Vec3* Points;
	size_t PointsLen;
	float MaxConvexRadius;
	float MaxErrorConvexRadius;
	float HullTolerance;
} JPC56_ConvexHullShapeSettings;

JPC56_API void JPC56_ConvexHullShapeSettings_default(JPC56_ConvexHullShapeSettings* object);
JPC56_API bool JPC56_ConvexHullShapeSettings_Create(const JPC56_ConvexHullShapeSettings* self, JPC56_Shape** outShape, JPC56_String** outError);

////////////////////////////////////////////////////////////////////////////////
// CompoundShape::SubShapeSettings

typedef struct JPC56_SubShapeSettings {
	const JPC56_Shape* Shape;
	JPC56_Vec3 Position;
	JPC56_Quat Rotation;
	uint32_t UserData;
} JPC56_SubShapeSettings;

JPC56_API void JPC56_SubShapeSettings_default(JPC56_SubShapeSettings* object);

////////////////////////////////////////////////////////////////////////////////
// StaticCompoundShapeSettings -> CompoundShapeSettings -> ShapeSettings

typedef struct JPC56_StaticCompoundShapeSettings {
	// ShapeSettings
	uint64_t UserData;

	// CompoundShapeSettings
	const JPC56_SubShapeSettings* SubShapes;
	size_t SubShapesLen;

	// StaticCompoundShapeSettings
	// (no fields)
} JPC56_StaticCompoundShapeSettings;

JPC56_API void JPC56_StaticCompoundShapeSettings_default(JPC56_StaticCompoundShapeSettings* object);
JPC56_API bool JPC56_StaticCompoundShapeSettings_Create(const JPC56_StaticCompoundShapeSettings* self, JPC56_Shape** outShape, JPC56_String** outError);

////////////////////////////////////////////////////////////////////////////////
// MutableCompoundShape -> CompoundShape -> Shape

typedef struct JPC56_MutableCompoundShape JPC56_MutableCompoundShape;

JPC56_API uint JPC56_MutableCompoundShape_AddShape(
	JPC56_MutableCompoundShape* self,
	JPC56_Vec3 inPosition,
	JPC56_Quat inRotation,
	const JPC56_Shape* inShape,
	uint32_t inUserData);

JPC56_API void JPC56_MutableCompoundShape_RemoveShape(JPC56_MutableCompoundShape* self, uint inIndex);
JPC56_API void JPC56_MutableCompoundShape_ModifyShape(JPC56_MutableCompoundShape* self, uint inIndex, JPC56_Vec3 inPosition, JPC56_Quat inRotation);
JPC56_API void JPC56_MutableCompoundShape_ModifyShape2(JPC56_MutableCompoundShape* self, uint inIndex, JPC56_Vec3 inPosition, JPC56_Quat inRotation, const JPC56_Shape* inShape);
JPC56_API void JPC56_MutableCompoundShape_AdjustCenterOfMass(JPC56_MutableCompoundShape* self);

// TODO:
// JPC56_API void JPC56_MutableCompoundShape_ModifyShapes(JPC56_MutableCompoundShape* self, ...);
// JPC56_API JPC56_MutableCompoundShape* JPC56_MutableCompoundShape_Clone(JPC56_MutableCompoundShape* self);

////////////////////////////////////////////////////////////////////////////////
// MutableCompoundShapeSettings -> CompoundShapeSettings -> ShapeSettings

typedef struct JPC56_MutableCompoundShapeSettings {
	// ShapeSettings
	uint64_t UserData;

	// CompoundShapeSettings
	const JPC56_SubShapeSettings* SubShapes;
	size_t SubShapesLen;

	// MutableCompoundShapeSettings
	// (no fields)
} JPC56_MutableCompoundShapeSettings;

JPC56_API void JPC56_MutableCompoundShapeSettings_default(JPC56_MutableCompoundShapeSettings* object);
JPC56_API bool JPC56_MutableCompoundShapeSettings_Create(const JPC56_MutableCompoundShapeSettings* self, JPC56_MutableCompoundShape** outShape, JPC56_String** outError);

////////////////////////////////////////////////////////////////////////////////
// BodyCreationSettings

typedef struct JPC56_BodyCreationSettings {
	JPC56_RVec3 Position;
	JPC56_Quat Rotation;
	JPC56_Vec3 LinearVelocity;
	JPC56_Vec3 AngularVelocity;
	uint64_t UserData;
	JPC56_ObjectLayer ObjectLayer;
	// CollisionGroup CollisionGroup;
	JPC56_MotionType MotionType;
	JPC56_AllowedDOFs AllowedDOFs;
	bool AllowDynamicOrKinematic;
	bool IsSensor;
	bool CollideKinematicVsNonDynamic;
	bool UseManifoldReduction;
	bool ApplyGyroscopicForce;
	JPC56_MotionQuality MotionQuality;
	bool EnhancedInternalEdgeRemoval;
	bool AllowSleeping;
	float Friction;
	float Restitution;
	float LinearDamping;
	float AngularDamping;
	float MaxLinearVelocity;
	float MaxAngularVelocity;
	float GravityFactor;
	uint NumVelocityStepsOverride;
	uint NumPositionStepsOverride;
	JPC56_OverrideMassProperties OverrideMassProperties;
	float InertiaMultiplier;

	// MassProperties MassPropertiesOverride;

	const JPC56_Shape* Shape;
} JPC56_BodyCreationSettings;

JPC56_API void JPC56_BodyCreationSettings_default(JPC56_BodyCreationSettings* settings);

typedef struct JPC56_BodyCreationSettings JPC56_BodyCreationSettings;

JPC56_API JPC56_BodyCreationSettings* JPC56_BodyCreationSettings_new();

////////////////////////////////////////////////////////////////////////////////
// Body

JPC56_API JPC56_BodyID JPC56_Body_GetID(const JPC56_Body* self);
JPC56_API JPC56_BodyType JPC56_Body_GetBodyType(const JPC56_Body* self);
JPC56_API bool JPC56_Body_IsRigidBody(const JPC56_Body* self);
JPC56_API bool JPC56_Body_IsSoftBody(const JPC56_Body* self);
JPC56_API bool JPC56_Body_IsActive(const JPC56_Body* self);
JPC56_API bool JPC56_Body_IsStatic(const JPC56_Body* self);
JPC56_API bool JPC56_Body_IsKinematic(const JPC56_Body* self);
JPC56_API bool JPC56_Body_IsDynamic(const JPC56_Body* self);
JPC56_API bool JPC56_Body_CanBeKinematicOrDynamic(const JPC56_Body* self);
JPC56_API void JPC56_Body_SetIsSensor(JPC56_Body* self, bool inIsSensor);
JPC56_API bool JPC56_Body_IsSensor(const JPC56_Body* self);
JPC56_API void JPC56_Body_SetCollideKinematicVsNonDynamic(JPC56_Body* self, bool inCollide);
JPC56_API bool JPC56_Body_GetCollideKinematicVsNonDynamic(const JPC56_Body* self);
JPC56_API void JPC56_Body_SetUseManifoldReduction(JPC56_Body* self, bool inUseReduction);
JPC56_API bool JPC56_Body_GetUseManifoldReduction(const JPC56_Body* self);
JPC56_API bool JPC56_Body_GetUseManifoldReductionWithBody(const JPC56_Body* self, const JPC56_Body* inBody2);
JPC56_API void JPC56_Body_SetApplyGyroscopicForce(JPC56_Body* self, bool inApply);
JPC56_API bool JPC56_Body_GetApplyGyroscopicForce(const JPC56_Body* self);
JPC56_API void JPC56_Body_SetEnhancedInternalEdgeRemoval(JPC56_Body* self, bool inApply);
JPC56_API bool JPC56_Body_GetEnhancedInternalEdgeRemoval(const JPC56_Body* self);
JPC56_API bool JPC56_Body_GetEnhancedInternalEdgeRemovalWithBody(const JPC56_Body* self, const JPC56_Body* inBody2);
JPC56_API JPC56_MotionType JPC56_Body_GetMotionType(const JPC56_Body* self);
JPC56_API void JPC56_Body_SetMotionType(JPC56_Body* self, JPC56_MotionType inMotionType);
JPC56_API JPC56_BroadPhaseLayer JPC56_Body_GetBroadPhaseLayer(const JPC56_Body* self);
JPC56_API JPC56_ObjectLayer JPC56_Body_GetObjectLayer(const JPC56_Body* self);

// JPC56_API const CollisionGroup & JPC56_Body_GetCollisionGroup(const JPC56_Body* self);
// JPC56_API CollisionGroup & JPC56_Body_GetCollisionGroup(JPC56_Body* self);
// JPC56_API void JPC56_Body_SetCollisionGroup(JPC56_Body* self, const CollisionGroup &inGroup);

JPC56_API bool JPC56_Body_GetAllowSleeping(const JPC56_Body* self);
JPC56_API void JPC56_Body_SetAllowSleeping(JPC56_Body* self, bool inAllow);
JPC56_API void JPC56_Body_ResetSleepTimer(JPC56_Body* self);
JPC56_API float JPC56_Body_GetFriction(const JPC56_Body* self);
JPC56_API void JPC56_Body_SetFriction(JPC56_Body* self, float inFriction);
JPC56_API float JPC56_Body_GetRestitution(const JPC56_Body* self);
JPC56_API void JPC56_Body_SetRestitution(JPC56_Body* self, float inRestitution);
JPC56_API JPC56_Vec3 JPC56_Body_GetLinearVelocity(const JPC56_Body* self);
JPC56_API void JPC56_Body_SetLinearVelocity(JPC56_Body* self, JPC56_Vec3 inLinearVelocity);
JPC56_API void JPC56_Body_SetLinearVelocityClamped(JPC56_Body* self, JPC56_Vec3 inLinearVelocity);
JPC56_API JPC56_Vec3 JPC56_Body_GetAngularVelocity(const JPC56_Body* self);
JPC56_API void JPC56_Body_SetAngularVelocity(JPC56_Body* self, JPC56_Vec3 inAngularVelocity);
JPC56_API void JPC56_Body_SetAngularVelocityClamped(JPC56_Body* self, JPC56_Vec3 inAngularVelocity);
JPC56_API JPC56_Vec3 JPC56_Body_GetPointVelocityCOM(const JPC56_Body* self, JPC56_Vec3 inPointRelativeToCOM);
JPC56_API JPC56_Vec3 JPC56_Body_GetPointVelocity(const JPC56_Body* self, JPC56_RVec3 inPoint);
JPC56_API void JPC56_Body_AddForce(JPC56_Body* self, JPC56_Vec3 inForce);
// overload of Body::AddForce
JPC56_API void JPC56_Body_AddForceAtPoint(JPC56_Body* self, JPC56_Vec3 inForce, JPC56_RVec3 inPosition);
JPC56_API void JPC56_Body_AddTorque(JPC56_Body* self, JPC56_Vec3 inTorque);
JPC56_API JPC56_Vec3 JPC56_Body_GetAccumulatedForce(const JPC56_Body* self);
JPC56_API JPC56_Vec3 JPC56_Body_GetAccumulatedTorque(const JPC56_Body* self);
JPC56_API void JPC56_Body_ResetForce(JPC56_Body* self);
JPC56_API void JPC56_Body_ResetTorque(JPC56_Body* self);
JPC56_API void JPC56_Body_ResetMotion(JPC56_Body* self);
JPC56_API void JPC56_Body_GetInverseInertia(const JPC56_Body* self, JPC56_Mat44* outMatrix);
JPC56_API void JPC56_Body_AddImpulse(JPC56_Body* self, JPC56_Vec3 inImpulse);
JPC56_API void JPC56_Body_AddImpulse2(JPC56_Body* self, JPC56_Vec3 inImpulse, JPC56_RVec3 inPosition);
JPC56_API void JPC56_Body_AddAngularImpulse(JPC56_Body* self, JPC56_Vec3 inAngularImpulse);
JPC56_API void JPC56_Body_MoveKinematic(JPC56_Body* self, JPC56_RVec3 inTargetPosition, JPC56_Quat inTargetRotation, float inDeltaTime);
JPC56_API bool JPC56_Body_ApplyBuoyancyImpulse(JPC56_Body* self, JPC56_RVec3 inSurfacePosition, JPC56_Vec3 inSurfaceNormal, float inBuoyancy, float inLinearDrag, float inAngularDrag, JPC56_Vec3 inFluidVelocity, JPC56_Vec3 inGravity, float inDeltaTime);
JPC56_API bool JPC56_Body_IsInBroadPhase(const JPC56_Body* self);
JPC56_API bool JPC56_Body_IsCollisionCacheInvalid(const JPC56_Body* self);
JPC56_API const JPC56_Shape* JPC56_Body_GetShape(const JPC56_Body* self);
JPC56_API JPC56_RVec3 JPC56_Body_GetPosition(const JPC56_Body* self);
JPC56_API JPC56_Quat JPC56_Body_GetRotation(const JPC56_Body* self);
JPC56_API JPC56_RMat44 JPC56_Body_GetWorldTransform(const JPC56_Body* self);
JPC56_API JPC56_RVec3 JPC56_Body_GetCenterOfMassPosition(const JPC56_Body* self);

JPC56_API JPC56_RMat44 JPC56_Body_GetCenterOfMassTransform(const JPC56_Body* self);
JPC56_API JPC56_RMat44 JPC56_Body_GetInverseCenterOfMassTransform(const JPC56_Body* self);

// JPC56_API const AABox & JPC56_Body_GetWorldSpaceBounds(const JPC56_Body* self);
// JPC56_API const MotionProperties *JPC56_Body_GetMotionProperties(const JPC56_Body* self)
// JPC56_API MotionProperties * JPC56_Body_GetMotionProperties(JPC56_Body* self);
// JPC56_API const MotionProperties *JPC56_Body_GetMotionPropertiesUnchecked(const JPC56_Body* self)
// JPC56_API MotionProperties * JPC56_Body_GetMotionPropertiesUnchecked(JPC56_Body* self);

JPC56_API uint64_t JPC56_Body_GetUserData(const JPC56_Body* self);
JPC56_API void JPC56_Body_SetUserData(JPC56_Body* self, uint64_t inUserData);

JPC56_API JPC56_Vec3 JPC56_Body_GetWorldSpaceSurfaceNormal(const JPC56_Body* self, JPC56_SubShapeID inSubShapeID, JPC56_RVec3 inPosition);

// JPC56_API TransformedShape JPC56_Body_GetTransformedShape(const JPC56_Body* self);
// JPC56_API BodyCreationSettings JPC56_Body_GetBodyCreationSettings(const JPC56_Body* self);
// JPC56_API SoftBodyCreationSettings JPC56_Body_GetSoftBodyCreationSettings(const JPC56_Body* self);

////////////////////////////////////////////////////////////////////////////////
// BodyLockInterface

typedef struct JPC56_BodyLockInterface JPC56_BodyLockInterface;

////////////////////////////////////////////////////////////////////////////////
// BodyLockRead

typedef struct JPC56_BodyLockRead JPC56_BodyLockRead;

JPC56_API JPC56_BodyLockRead* JPC56_BodyLockRead_new(const JPC56_BodyLockInterface* interface, JPC56_BodyID bodyID);
JPC56_API void JPC56_BodyLockRead_delete(JPC56_BodyLockRead* self);

JPC56_API bool JPC56_BodyLockRead_Succeeded(JPC56_BodyLockRead* self);
JPC56_API const JPC56_Body* JPC56_BodyLockRead_GetBody(JPC56_BodyLockRead* self);

////////////////////////////////////////////////////////////////////////////////
// BodyLockWrite

typedef struct JPC56_BodyLockWrite JPC56_BodyLockWrite;

JPC56_API JPC56_BodyLockWrite* JPC56_BodyLockWrite_new(const JPC56_BodyLockInterface* interface, JPC56_BodyID bodyID);
JPC56_API void JPC56_BodyLockWrite_delete(JPC56_BodyLockWrite* self);

JPC56_API bool JPC56_BodyLockWrite_Succeeded(JPC56_BodyLockWrite* self);
JPC56_API JPC56_Body* JPC56_BodyLockWrite_GetBody(JPC56_BodyLockWrite* self);

////////////////////////////////////////////////////////////////////////////////
// BodyLockMultiRead

typedef struct JPC56_BodyLockMultiRead JPC56_BodyLockMultiRead;

JPC56_API JPC56_BodyLockMultiRead* JPC56_BodyLockMultiRead_new(
	const JPC56_BodyLockInterface* interface,
	const JPC56_BodyID *inBodyIDs,
	int inNumber);
JPC56_API void JPC56_BodyLockMultiRead_delete(JPC56_BodyLockMultiRead* self);

JPC56_API const JPC56_Body* JPC56_BodyLockMultiRead_GetBody(JPC56_BodyLockMultiRead* self, int inBodyIndex);

////////////////////////////////////////////////////////////////////////////////
// BodyLockMultiWrite

typedef struct JPC56_BodyLockMultiWrite JPC56_BodyLockMultiWrite;

JPC56_API JPC56_BodyLockMultiWrite* JPC56_BodyLockMultiWrite_new(
	const JPC56_BodyLockInterface* interface,
	const JPC56_BodyID *inBodyIDs,
	int inNumber);
JPC56_API void JPC56_BodyLockMultiWrite_delete(JPC56_BodyLockMultiWrite* self);

JPC56_API JPC56_Body* JPC56_BodyLockMultiWrite_GetBody(JPC56_BodyLockMultiWrite* self, int inBodyIndex);

////////////////////////////////////////////////////////////////////////////////
// BodyInterface

typedef struct JPC56_BodyInterface JPC56_BodyInterface;

JPC56_API JPC56_Body* JPC56_BodyInterface_CreateBody(JPC56_BodyInterface* self, const JPC56_BodyCreationSettings* inSettings);
JPC56_API JPC56_Body* JPC56_BodyInterface_CreateBodyWithID(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, const JPC56_BodyCreationSettings* inSettings);
JPC56_API JPC56_Body* JPC56_BodyInterface_CreateBodyWithoutID(const JPC56_BodyInterface *self, const JPC56_BodyCreationSettings* inSettings);

// JPC56_API JPC56_Body* JPC56_BodyInterface_CreateSoftBody(JPC56_BodyInterface *self, const SoftBodyCreationSettings &inSettings);
// JPC56_API JPC56_Body* JPC56_BodyInterface_CreateSoftBodyWithID(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, const SoftBodyCreationSettings* inSettings);
// JPC56_API JPC56_Body* JPC56_BodyInterface_CreateSoftBodyWithoutID(const JPC56_BodyInterface *self, const SoftBodyCreationSettings* inSettings);

JPC56_API void JPC56_BodyInterface_DestroyBodyWithoutID(const JPC56_BodyInterface *self, JPC56_Body *inBody);
JPC56_API bool JPC56_BodyInterface_AssignBodyID(JPC56_BodyInterface *self, JPC56_Body *ioBody);

// JPC56_API bool JPC56_BodyInterface_AssignBodyID(JPC56_BodyInterface *self, JPC56_Body *ioBody, JPC56_BodyID inBodyID);

JPC56_API JPC56_Body* JPC56_BodyInterface_UnassignBodyID(JPC56_BodyInterface *self, JPC56_BodyID inBodyID);
JPC56_API void JPC56_BodyInterface_UnassignBodyIDs(JPC56_BodyInterface *self, const JPC56_BodyID *inBodyIDs, int inNumber, JPC56_Body **outBodies);
JPC56_API void JPC56_BodyInterface_DestroyBody(JPC56_BodyInterface *self, JPC56_BodyID inBodyID);
JPC56_API void JPC56_BodyInterface_DestroyBodies(JPC56_BodyInterface *self, const JPC56_BodyID *inBodyIDs, int inNumber);
JPC56_API void JPC56_BodyInterface_AddBody(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Activation inActivationMode);
JPC56_API void JPC56_BodyInterface_RemoveBody(JPC56_BodyInterface *self, JPC56_BodyID inBodyID);
JPC56_API bool JPC56_BodyInterface_IsAdded(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID);
JPC56_API JPC56_BodyID JPC56_BodyInterface_CreateAndAddBody(JPC56_BodyInterface *self, const JPC56_BodyCreationSettings* inSettings, JPC56_Activation inActivationMode);

// JPC56_API JPC56_BodyID JPC56_BodyInterface_CreateAndAddSoftBody(JPC56_BodyInterface *self, const SoftBodyCreationSettings &inSettings, JPC56_Activation inActivationMode);

JPC56_API void* JPC56_BodyInterface_AddBodiesPrepare(JPC56_BodyInterface *self, JPC56_BodyID *ioBodies, int inNumber);
JPC56_API void JPC56_BodyInterface_AddBodiesFinalize(JPC56_BodyInterface *self, JPC56_BodyID *ioBodies, int inNumber, void* inAddState, JPC56_Activation inActivationMode);
JPC56_API void JPC56_BodyInterface_AddBodiesAbort(JPC56_BodyInterface *self, JPC56_BodyID *ioBodies, int inNumber, void* inAddState);
JPC56_API void JPC56_BodyInterface_RemoveBodies(JPC56_BodyInterface *self, JPC56_BodyID *ioBodies, int inNumber);
JPC56_API void JPC56_BodyInterface_ActivateBody(JPC56_BodyInterface *self, JPC56_BodyID inBodyID);
JPC56_API void JPC56_BodyInterface_ActivateBodies(JPC56_BodyInterface *self, JPC56_BodyID *inBodyIDs, int inNumber);

// JPC56_API void JPC56_BodyInterface_ActivateBodiesInAABox(JPC56_BodyInterface *self, const AABox &inBox, const BroadPhaseLayerFilter &inBroadPhaseLayerFilter, const ObjectLayerFilter &inObjectLayerFilter);

JPC56_API void JPC56_BodyInterface_DeactivateBody(JPC56_BodyInterface *self, JPC56_BodyID inBodyID);
JPC56_API void JPC56_BodyInterface_DeactivateBodies(JPC56_BodyInterface *self, JPC56_BodyID *inBodyIDs, int inNumber);
JPC56_API bool JPC56_BodyInterface_IsActive(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID);

// TwoBodyConstraint * JPC56_BodyInterface_CreateConstraint(JPC56_BodyInterface *self, const TwoBodyConstraintSettings *inSettings, JPC56_BodyID inBodyID1, JPC56_BodyID inBodyID2);
// JPC56_API void JPC56_BodyInterface_ActivateConstraint(JPC56_BodyInterface *self, const TwoBodyConstraint *inConstraint);
JPC56_API const JPC56_Shape* JPC56_BodyInterface_GetShape(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID);

JPC56_API void JPC56_BodyInterface_SetShape(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID, const JPC56_Shape *inShape, bool inUpdateMassProperties, JPC56_Activation inActivationMode);
JPC56_API void JPC56_BodyInterface_NotifyShapeChanged(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Vec3 inPreviousCenterOfMass, bool inUpdateMassProperties, JPC56_Activation inActivationMode);
JPC56_API void JPC56_BodyInterface_SetObjectLayer(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_ObjectLayer inLayer);
JPC56_API JPC56_ObjectLayer JPC56_BodyInterface_GetObjectLayer(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID);
JPC56_API void JPC56_BodyInterface_SetPositionAndRotation(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_RVec3 inPosition, JPC56_Quat inRotation, JPC56_Activation inActivationMode);
JPC56_API void JPC56_BodyInterface_SetPositionAndRotationWhenChanged(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_RVec3 inPosition, JPC56_Quat inRotation, JPC56_Activation inActivationMode);
JPC56_API void JPC56_BodyInterface_GetPositionAndRotation(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_RVec3 *outPosition, JPC56_Quat *outRotation);
JPC56_API void JPC56_BodyInterface_SetPosition(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_RVec3 inPosition, JPC56_Activation inActivationMode);
JPC56_API JPC56_RVec3 JPC56_BodyInterface_GetPosition(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID);
JPC56_API JPC56_RVec3 JPC56_BodyInterface_GetCenterOfMassPosition(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID);
JPC56_API void JPC56_BodyInterface_SetRotation(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Quat inRotation, JPC56_Activation inActivationMode);
JPC56_API JPC56_Quat JPC56_BodyInterface_GetRotation(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID);
JPC56_API JPC56_RMat44 JPC56_BodyInterface_GetWorldTransform(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID);
JPC56_API JPC56_RMat44 JPC56_BodyInterface_GetCenterOfMassTransform(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID);
JPC56_API void JPC56_BodyInterface_MoveKinematic(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_RVec3 inTargetPosition, JPC56_Quat inTargetRotation, float inDeltaTime);
JPC56_API void JPC56_BodyInterface_SetLinearAndAngularVelocity(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Vec3 inLinearVelocity, JPC56_Vec3 inAngularVelocity);
JPC56_API void JPC56_BodyInterface_GetLinearAndAngularVelocity(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Vec3 *outLinearVelocity, JPC56_Vec3 *outAngularVelocity);
JPC56_API void JPC56_BodyInterface_SetLinearVelocity(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Vec3 inLinearVelocity);
JPC56_API JPC56_Vec3 JPC56_BodyInterface_GetLinearVelocity(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID);
JPC56_API void JPC56_BodyInterface_AddLinearVelocity(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Vec3 inLinearVelocity);
JPC56_API void JPC56_BodyInterface_AddLinearAndAngularVelocity(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Vec3 inLinearVelocity, JPC56_Vec3 inAngularVelocity);
JPC56_API void JPC56_BodyInterface_SetAngularVelocity(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Vec3 inAngularVelocity);
JPC56_API JPC56_Vec3 JPC56_BodyInterface_GetAngularVelocity(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID);
JPC56_API JPC56_Vec3 JPC56_BodyInterface_GetPointVelocity(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_RVec3 inPoint);
JPC56_API void JPC56_BodyInterface_SetPositionRotationAndVelocity(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_RVec3 inPosition, JPC56_Quat inRotation, JPC56_Vec3 inLinearVelocity, JPC56_Vec3 inAngularVelocity);
JPC56_API void JPC56_BodyInterface_AddForce(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Vec3 inForce);
// overload of BodyInterface::AddForce
JPC56_API void JPC56_BodyInterface_AddForceAtPoint(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Vec3 inForce, JPC56_RVec3 inPoint);
JPC56_API void JPC56_BodyInterface_AddTorque(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Vec3 inTorque);
JPC56_API void JPC56_BodyInterface_AddForceAndTorque(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Vec3 inForce, JPC56_Vec3 inTorque);
JPC56_API void JPC56_BodyInterface_AddImpulse(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Vec3 inImpulse);
JPC56_API void JPC56_BodyInterface_AddImpulse3(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Vec3 inImpulse, JPC56_RVec3 inPoint);
JPC56_API void JPC56_BodyInterface_AddAngularImpulse(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Vec3 inAngularImpulse);
JPC56_API JPC56_BodyType JPC56_BodyInterface_GetBodyType(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID);
JPC56_API void JPC56_BodyInterface_SetMotionType(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_MotionType inMotionType, JPC56_Activation inActivationMode);
JPC56_API JPC56_MotionType JPC56_BodyInterface_GetMotionType(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID);
JPC56_API void JPC56_BodyInterface_SetMotionQuality(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_MotionQuality inMotionQuality);
JPC56_API JPC56_MotionQuality JPC56_BodyInterface_GetMotionQuality(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID);
JPC56_API void JPC56_BodyInterface_GetInverseInertia(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Mat44 *outMatrix);
JPC56_API void JPC56_BodyInterface_SetRestitution(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, float inRestitution);
JPC56_API float JPC56_BodyInterface_GetRestitution(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID);
JPC56_API void JPC56_BodyInterface_SetFriction(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, float inFriction);
JPC56_API float JPC56_BodyInterface_GetFriction(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID);
JPC56_API void JPC56_BodyInterface_SetGravityFactor(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, float inGravityFactor);
JPC56_API float JPC56_BodyInterface_GetGravityFactor(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID);
JPC56_API void JPC56_BodyInterface_SetUseManifoldReduction(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, bool inUseReduction);
JPC56_API bool JPC56_BodyInterface_GetUseManifoldReduction(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID);

// TransformedShape JPC56_BodyInterface_GetTransformedShape(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID);

JPC56_API uint64_t JPC56_BodyInterface_GetUserData(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID);
JPC56_API void JPC56_BodyInterface_SetUserData(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID, uint64_t inUserData);

// const PhysicsMaterial* JPC56_BodyInterface_GetMaterial(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID, const SubShapeID &inSubShapeID);

JPC56_API void JPC56_BodyInterface_InvalidateContactCache(JPC56_BodyInterface *self, JPC56_BodyID inBodyID);

////////////////////////////////////////////////////////////////////////////////
// NarrowPhaseQuery

typedef struct JPC56_NarrowPhaseQuery JPC56_NarrowPhaseQuery;

typedef struct JPC56_NarrowPhaseQuery_CastRayArgs {
	JPC56_RRayCast Ray;
	JPC56_RayCastResult Result;
	const JPC56_BroadPhaseLayerFilter *BroadPhaseLayerFilter;
	const JPC56_ObjectLayerFilter *ObjectLayerFilter;
	const JPC56_BodyFilter *BodyFilter;
	const JPC56_ShapeFilter *ShapeFilter;
} JPC56_NarrowPhaseQuery_CastRayArgs;

JPC56_API bool JPC56_NarrowPhaseQuery_CastRay(const JPC56_NarrowPhaseQuery* self, JPC56_NarrowPhaseQuery_CastRayArgs* args);

typedef struct JPC56_RShapeCast {
	const JPC56_Shape *Shape;
	JPC56_Vec3 Scale;
	JPC56_RMat44 CenterOfMassStart;
	JPC56_Vec3 Direction;
	// const JPC56_AABox ShapeWorldBounds;
} JPC56_RShapeCast;

JPC56_API void JPC56_ShapeCastSettings_default(JPC56_ShapeCastSettings* object);

typedef struct JPC56_NarrowPhaseQuery_CastShapeArgs {
	JPC56_RShapeCast ShapeCast;
	JPC56_ShapeCastSettings Settings;
	JPC56_RVec3 BaseOffset;
	JPC56_CastShapeCollector *Collector;
	const JPC56_BroadPhaseLayerFilter *BroadPhaseLayerFilter;
	const JPC56_ObjectLayerFilter *ObjectLayerFilter;
	const JPC56_BodyFilter *BodyFilter;
	const JPC56_ShapeFilter *ShapeFilter;
} JPC56_NarrowPhaseQuery_CastShapeArgs;

JPC56_API void JPC56_NarrowPhaseQuery_CastShape(const JPC56_NarrowPhaseQuery* self, JPC56_NarrowPhaseQuery_CastShapeArgs* args);

JPC56_API void JPC56_CollideShapeSettings_default(JPC56_CollideShapeSettings* object);

typedef struct JPC56_NarrowPhaseQuery_CollideShapeArgs {
	const JPC56_Shape *Shape;
	JPC56_Vec3 ShapeScale;
	JPC56_RMat44 CenterOfMassTransform;
	JPC56_CollideShapeSettings Settings;
	JPC56_RVec3 BaseOffset;
	JPC56_CollideShapeCollector *Collector;
	const JPC56_BroadPhaseLayerFilter *BroadPhaseLayerFilter;
	const JPC56_ObjectLayerFilter *ObjectLayerFilter;
	const JPC56_BodyFilter *BodyFilter;
	const JPC56_ShapeFilter *ShapeFilter;
} JPC56_NarrowPhaseQuery_CollideShapeArgs;

JPC56_API void JPC56_NarrowPhaseQuery_CollideShape(const JPC56_NarrowPhaseQuery* self, JPC56_NarrowPhaseQuery_CollideShapeArgs* args);

////////////////////////////////////////////////////////////////////////////////
// PhysicsSystem

typedef struct JPC56_PhysicsSystem JPC56_PhysicsSystem;

JPC56_API JPC56_PhysicsSystem* JPC56_PhysicsSystem_new();
JPC56_API void JPC56_PhysicsSystem_delete(JPC56_PhysicsSystem* object);
JPC56_API void JPC56_PhysicsSystem_Init(
	JPC56_PhysicsSystem* self,
	uint inMaxBodies,
	uint inNumBodyMutexes,
	uint inMaxBodyPairs,
	uint inMaxContactConstraints,
	JPC56_BroadPhaseLayerInterface* inBroadPhaseLayerInterface,
	JPC56_ObjectVsBroadPhaseLayerFilter* inObjectVsBroadPhaseLayerFilter,
	JPC56_ObjectLayerPairFilter* inObjectLayerPairFilter);

JPC56_API void JPC56_PhysicsSystem_OptimizeBroadPhase(JPC56_PhysicsSystem* self);

JPC56_API JPC56_PhysicsUpdateError JPC56_PhysicsSystem_Update(
	JPC56_PhysicsSystem* self,
	float inDeltaTime,
	int inCollisionSteps,
	JPC56_TempAllocatorImpl *inTempAllocator, // FIXME: un-specialize
	JPC56_JobSystem* inJobSystem);

JPC56_API void JPC56_PhysicsSystem_AddConstraint(JPC56_PhysicsSystem* self, JPC56_Constraint* constraint);
JPC56_API void JPC56_PhysicsSystem_RemoveConstraint(JPC56_PhysicsSystem* self, JPC56_Constraint* constraint);

JPC56_API void JPC56_PhysicsSystem_SetGravity(JPC56_PhysicsSystem* self, JPC56_Vec3 inGravity);
JPC56_API JPC56_Vec3 JPC56_PhysicsSystem_GetGravity(const JPC56_PhysicsSystem* self);

JPC56_API JPC56_BodyInterface* JPC56_PhysicsSystem_GetBodyInterface(JPC56_PhysicsSystem* self);
JPC56_API const JPC56_BodyLockInterface* JPC56_PhysicsSystem_GetBodyLockInterface(JPC56_PhysicsSystem* self);

JPC56_API const JPC56_NarrowPhaseQuery* JPC56_PhysicsSystem_GetNarrowPhaseQuery(const JPC56_PhysicsSystem* self);

JPC56_API void JPC56_PhysicsSystem_DrawBodies(
	JPC56_PhysicsSystem* self,
	JPC56_BodyManager_DrawSettings* inSettings,
	JPC56_DebugRendererSimple* inRenderer, // FIXME: un-specialize
	const void* inBodyFilter); // FIXME: BodyDrawFilter

JPC56_API void JPC56_PhysicsSystem_DrawConstraints(
	JPC56_PhysicsSystem* self,
	JPC56_DebugRendererSimple* inRenderer); // FIXME: un-specialize

JPC56_API void JPC56_PhysicsSystem_SetSimShapeFilter(JPC56_PhysicsSystem* self, const JPC56_SimShapeFilter* inShapeFilter);

JPC56_API void JPC56_PhysicsSystem_SetContactListener(JPC56_PhysicsSystem* self, JPC56_ContactListener* inContactListener);

#ifdef __cplusplus
}
#endif
