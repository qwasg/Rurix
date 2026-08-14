#include <Jolt/Jolt.h>

#include <Jolt/Core/Factory.h>
#include <Jolt/Core/JobSystem.h>
#include <Jolt/Core/JobSystemSingleThreaded.h>
#include <Jolt/Core/JobSystemThreadPool.h>
#include <Jolt/Core/TempAllocator.h>
#include <Jolt/Physics/Body/BodyActivationListener.h>
#include <Jolt/Physics/Body/BodyCreationSettings.h>
#include <Jolt/Physics/Body/BodyLockMulti.h>
#include <Jolt/Physics/Collision/CastResult.h>
#include <Jolt/Physics/Collision/CollideShape.h>
#include <Jolt/Physics/Collision/CollisionCollectorImpl.h>
#include <Jolt/Physics/Collision/ContactListener.h>
#include <Jolt/Physics/Collision/EstimateCollisionResponse.h>
#include <Jolt/Physics/Collision/RayCast.h>
#include <Jolt/Physics/Collision/Shape/BoxShape.h>
#include <Jolt/Physics/Collision/Shape/CapsuleShape.h>
#include <Jolt/Physics/Collision/Shape/CompoundShape.h>
#include <Jolt/Physics/Collision/Shape/ConvexHullShape.h>
#include <Jolt/Physics/Collision/Shape/CylinderShape.h>
#include <Jolt/Physics/Collision/Shape/MeshShape.h>
#include <Jolt/Physics/Collision/Shape/MutableCompoundShape.h>
#include <Jolt/Physics/Collision/Shape/PlaneShape.h>
#include <Jolt/Physics/Collision/Shape/SphereShape.h>
#include <Jolt/Physics/Collision/Shape/StaticCompoundShape.h>
#include <Jolt/Physics/Collision/Shape/TriangleShape.h>
#include <Jolt/Physics/Collision/ShapeCast.h>
#include <Jolt/Physics/Collision/SimShapeFilter.h>
#include <Jolt/Physics/Constraints/ConstraintPart/SwingTwistConstraintPart.h>
#include <Jolt/Physics/Constraints/FixedConstraint.h>
#include <Jolt/Physics/Constraints/SixDOFConstraint.h>
#include <Jolt/Physics/Constraints/HingeConstraint.h>
#include <Jolt/Physics/Constraints/DistanceConstraint.h>
#include <Jolt/Physics/Constraints/SliderConstraint.h>
#include <Jolt/Physics/PhysicsSettings.h>
#include <Jolt/Physics/PhysicsSystem.h>
#include <Jolt/RegisterTypes.h>

#include <Jolt/Renderer/DebugRendererSimple.h>

#include <JoltC/JoltC.h>

#define JPC56_IMPL static

#define OPAQUE_WRAPPER(c_type, cpp_type) \
	static c_type* to_jpc(cpp_type *in) { return reinterpret_cast<c_type*>(in); } \
	static const c_type* to_jpc(const cpp_type *in) { return reinterpret_cast<const c_type*>(in); } \
	static cpp_type* to_jph(c_type *in) { return reinterpret_cast<cpp_type*>(in); } \
	static const cpp_type* to_jph(const c_type *in) { return reinterpret_cast<const cpp_type*>(in); } \
	static cpp_type** to_jph(c_type **in) { return reinterpret_cast<cpp_type**>(in); }

#define DESTRUCTOR(c_type) \
	JPC56_API void c_type##_delete(c_type* object) { \
		delete to_jph(object); \
	}

#define ENUM_CONVERSION(c_type, cpp_type) \
	static c_type to_jpc(cpp_type in) { return static_cast<c_type>(in); } \
	static cpp_type to_jph(c_type in) { return static_cast<cpp_type>(in); }

#define LAYOUT_COMPATIBLE(c_type, cpp_type) \
	static c_type to_jpc(cpp_type in) { \
		c_type out; \
		memcpy(&out, &in, sizeof(c_type)); \
		return out; \
	} \
	static cpp_type to_jph(c_type in) { \
		cpp_type out; \
		memcpy(&out, &in, sizeof(cpp_type)); \
		return out; \
	} \
	static c_type* to_jpc(cpp_type* in) { \
		return reinterpret_cast<c_type*>(in); \
	} \
	static cpp_type* to_jph(c_type* in) { \
		return reinterpret_cast<cpp_type*>(in); \
	} \
	static const c_type* to_jpc(const cpp_type* in) { \
		return reinterpret_cast<const c_type*>(in); \
	} \
	static const cpp_type* to_jph(const c_type* in) { \
		return reinterpret_cast<const cpp_type*>(in); \
	} \
	static_assert(sizeof(c_type) == sizeof(cpp_type), "size of " #c_type " did not match size of " #cpp_type); \
	static_assert(alignof(c_type) == alignof(cpp_type), "align of " #c_type " did not match align of " #cpp_type); \
	static_assert(!std::is_polymorphic_v<cpp_type>, #cpp_type " is polymorphic and cannot be made layout compatible");

template<typename E>
constexpr auto to_integral(E e) -> typename std::underlying_type<E>::type
{
	return static_cast<typename std::underlying_type<E>::type>(e);
}

ENUM_CONVERSION(JPC56_MotionType, JPH56::EMotionType)
ENUM_CONVERSION(JPC56_AllowedDOFs, JPH56::EAllowedDOFs)
ENUM_CONVERSION(JPC56_Activation, JPH56::EActivation)
ENUM_CONVERSION(JPC56_BodyType, JPH56::EBodyType)
ENUM_CONVERSION(JPC56_MotionQuality, JPH56::EMotionQuality)
ENUM_CONVERSION(JPC56_OverrideMassProperties, JPH56::EOverrideMassProperties)
ENUM_CONVERSION(JPC56_ShapeType, JPH56::EShapeType)
ENUM_CONVERSION(JPC56_ShapeSubType, JPH56::EShapeSubType)
ENUM_CONVERSION(JPC56_SpringMode, JPH56::ESpringMode)
ENUM_CONVERSION(JPC56_MotorState, JPH56::EMotorState)
ENUM_CONVERSION(JPC56_ValidateResult, JPH56::ValidateResult)

OPAQUE_WRAPPER(JPC56_PhysicsSystem, JPH56::PhysicsSystem)
DESTRUCTOR(JPC56_PhysicsSystem)

OPAQUE_WRAPPER(JPC56_BodyInterface, JPH56::BodyInterface)
OPAQUE_WRAPPER(JPC56_BodyLockInterface, JPH56::BodyLockInterface)
OPAQUE_WRAPPER(JPC56_BodyLockRead, JPH56::BodyLockRead)
OPAQUE_WRAPPER(JPC56_BodyLockWrite, JPH56::BodyLockWrite)
OPAQUE_WRAPPER(JPC56_BodyLockMultiRead, JPH56::BodyLockMultiRead)
OPAQUE_WRAPPER(JPC56_BodyLockMultiWrite, JPH56::BodyLockMultiWrite)
OPAQUE_WRAPPER(JPC56_NarrowPhaseQuery, JPH56::NarrowPhaseQuery)

OPAQUE_WRAPPER(JPC56_TempAllocatorImpl, JPH56::TempAllocatorImpl)
DESTRUCTOR(JPC56_TempAllocatorImpl)

OPAQUE_WRAPPER(JPC56_JobSystem, JPH56::JobSystem)
DESTRUCTOR(JPC56_JobSystem)

OPAQUE_WRAPPER(JPC56_JobSystemThreadPool, JPH56::JobSystemThreadPool)
DESTRUCTOR(JPC56_JobSystemThreadPool)

OPAQUE_WRAPPER(JPC56_JobSystemSingleThreaded, JPH56::JobSystemSingleThreaded)
DESTRUCTOR(JPC56_JobSystemSingleThreaded)

OPAQUE_WRAPPER(JPC56_Shape, JPH56::Shape)
OPAQUE_WRAPPER(JPC56_CompoundShape, JPH56::CompoundShape)
OPAQUE_WRAPPER(JPC56_Body, JPH56::Body)

OPAQUE_WRAPPER(JPC56_VertexList, JPH56::VertexList)
DESTRUCTOR(JPC56_VertexList)

OPAQUE_WRAPPER(JPC56_IndexedTriangleList, JPH56::IndexedTriangleList)
DESTRUCTOR(JPC56_IndexedTriangleList)

OPAQUE_WRAPPER(JPC56_String, JPH56::String)
DESTRUCTOR(JPC56_String)

LAYOUT_COMPATIBLE(JPC56_BodyManager_DrawSettings, JPH56::BodyManager::DrawSettings)

LAYOUT_COMPATIBLE(JPC56_ShapeCastSettings, JPH56::ShapeCastSettings)
LAYOUT_COMPATIBLE(JPC56_CollideShapeSettings, JPH56::CollideShapeSettings)

LAYOUT_COMPATIBLE(JPC56_BodyID, JPH56::BodyID)

static auto to_jpc(JPH56::BroadPhaseLayer in) { return in.GetValue(); }
static auto to_jph(JPC56_BroadPhaseLayer in) { return JPH56::BroadPhaseLayer(in); }

static JPC56_Vec2 to_jpc(JPH56::Vector<2> in) {
	return JPC56_Vec2{in[0], in[1]};
}
static JPH56::Vector<2> to_jph(JPC56_Vec2 in) {
	JPH56::Vector<2> out;
	out[0] = in.x;
	out[1] = in.y;
	return out;
}

static JPC56_Vec3 to_jpc(JPH56::Vec3 in) {
	return JPC56_Vec3{in.GetX(), in.GetY(), in.GetZ(), in.GetZ()};
}
static JPH56::Vec3 to_jph(JPC56_Vec3 in) {
	return JPH56::Vec3(in.x, in.y, in.z);
}

static JPC56_Vec4 to_jpc(JPH56::Vec4 in) {
	return JPC56_Vec4{in.GetX(), in.GetY(), in.GetZ(), in.GetW()};
}
static JPH56::Vec4 to_jph(JPC56_Vec4 in) {
	return JPH56::Vec4(in.x, in.y, in.z, in.w);
}

static JPH56::Array<JPH56::Vec3> to_jph(const JPC56_Vec3* src, size_t n) {
	JPH56::Array<JPH56::Vec3> vec;
	vec.resize(n);

	if (src != nullptr) {
		memcpy(vec.data(), src, n * sizeof(*src));
	}

	return vec;
}

static JPC56_DVec3 to_jpc(JPH56::DVec3 in) {
	return JPC56_DVec3{in.GetX(), in.GetY(), in.GetZ(), in.GetZ()};
}
static JPH56::DVec3 to_jph(JPC56_DVec3 in) {
	return JPH56::DVec3(in.x, in.y, in.z);
}

static JPC56_Quat to_jpc(JPH56::Quat in) {
	return JPC56_Quat{in.GetX(), in.GetY(), in.GetZ(), in.GetW()};
}
static JPH56::Quat to_jph(JPC56_Quat in) {
	return JPH56::Quat(in.x, in.y, in.z, in.w);
}

static JPC56_Mat44 to_jpc(JPH56::Mat44 in) {
	JPC56_Mat44 out;
	in.StoreFloat4x4(reinterpret_cast<JPH56::Float4*>(&out));
	return out;
}
static JPH56::Mat44 to_jph(JPC56_Mat44 in) {
	return JPH56::Mat44::sLoadFloat4x4Aligned(reinterpret_cast<const JPH56::Float4*>(&in));
}

static JPC56_DMat44 to_jpc(JPH56::DMat44 in) {
	JPC56_DMat44 out;
	out.col[0] = to_jpc(in.GetColumn4(0));
	out.col[1] = to_jpc(in.GetColumn4(1));
	out.col[2] = to_jpc(in.GetColumn4(2));
	out.col3 = to_jpc(in.GetTranslation());
	return out;
}
static JPH56::DMat44 to_jph(JPC56_DMat44 in) {
	JPH56::DVec3 col3 = to_jph(in.col3);

	JPH56::DMat44 out(
		to_jph(in.col[0]),
		to_jph(in.col[1]),
		to_jph(in.col[2]),
		col3);
	return out;
}

static JPC56_Color to_jpc(JPH56::Color in) {
	return JPC56_Color{in.r, in.g, in.b, in.a};
}
static JPH56::Color to_jph(JPC56_Color in) {
	return JPH56::Color(in.r, in.g, in.b, in.a);
}

static JPH56::RayCast to_jph(JPC56_RayCast in) {
	return JPH56::RayCast(to_jph(in.Origin), to_jph(in.Direction));
}

static JPH56::RRayCast to_jph(JPC56_RRayCast in) {
	return JPH56::RRayCast(to_jph(in.Origin), to_jph(in.Direction));
}

static JPH56::RShapeCast to_jph(JPC56_RShapeCast in) {
	return JPH56::RShapeCast(
		to_jph(in.Shape),
		to_jph(in.Scale),
		to_jph(in.CenterOfMassStart),
		to_jph(in.Direction));
}

static JPH56::SubShapeID JPC56_SubShapeID_to_jph(JPC56_SubShapeID in) {
	JPH56::SubShapeID out;
	out.SetValue(in);
	return out;
}

static JPC56_SubShapeID to_jpc(JPH56::SubShapeID in) {
	return in.GetValue();
}

static JPC56_RayCastResult to_jpc(JPH56::RayCastResult in) {
	JPC56_RayCastResult out{0};
	out.BodyID = to_jpc(in.mBodyID);
	out.Fraction = in.mFraction;
	out.SubShapeID2 = to_jpc(in.mSubShapeID2);

	return out;
}

JPC56_IMPL JPC56_ShapeCastResult JPC56_ShapeCastResult_to_jpc(JPH56::ShapeCastResult in) {
	JPC56_ShapeCastResult out{};
	// CollideShapeResult
	out.ContactPointOn1 = to_jpc(in.mContactPointOn1);
	out.ContactPointOn2 = to_jpc(in.mContactPointOn2);
	out.PenetrationAxis = to_jpc(in.mPenetrationAxis);
	out.PenetrationDepth = in.mPenetrationDepth;
	out.SubShapeID1 = to_jpc(in.mSubShapeID1);
	out.SubShapeID2 = to_jpc(in.mSubShapeID2);
	out.BodyID2 = to_jpc(in.mBodyID2);
	// Face Shape1Face;
	// Face Shape2Face;

	// ShapeCastResult
	out.Fraction = in.mFraction;
	out.IsBackFaceHit = in.mIsBackFaceHit;

	return out;
}

JPC56_IMPL JPH56::ShapeCastSettings JPC56_ShapeCastSettings_to_jph(JPC56_ShapeCastSettings in) {
	JPH56::ShapeCastSettings out{};

	// JPH56::CollideSettingsBase
	// EActiveEdgeMode ActiveEdgeMode;
	// ECollectFacesMode CollectFacesMode;
	out.mCollisionTolerance = in.CollisionTolerance;
	out.mPenetrationTolerance = in.PenetrationTolerance;
	out.mActiveEdgeMovementDirection = to_jph(in.ActiveEdgeMovementDirection);

	// JPH56::ShapeCastSettings
	out.mExtraConvexRadius = in.ExtraConvexRadius; // rurix M125: Jolt 5.6 新字段
	out.mBackFaceModeTriangles = static_cast<JPH56::EBackFaceMode>(in.BackFaceModeTriangles);
	out.mBackFaceModeConvex = static_cast<JPH56::EBackFaceMode>(in.BackFaceModeConvex);
	out.mUseShrunkenShapeAndConvexRadius = in.UseShrunkenShapeAndConvexRadius;
	out.mReturnDeepestPoint = in.ReturnDeepestPoint;

	return out;
}

JPC56_IMPL JPC56_CollideShapeResult JPC56_CollideShapeResult_to_jpc(JPH56::CollideShapeResult in) {
	JPC56_CollideShapeResult out{};
	// CollideShapeResult
	out.ContactPointOn1 = to_jpc(in.mContactPointOn1);
	out.ContactPointOn2 = to_jpc(in.mContactPointOn2);
	out.PenetrationAxis = to_jpc(in.mPenetrationAxis);
	out.PenetrationDepth = in.mPenetrationDepth;
	out.SubShapeID1 = to_jpc(in.mSubShapeID1);
	out.SubShapeID2 = to_jpc(in.mSubShapeID2);
	out.BodyID2 = to_jpc(in.mBodyID2);
	// Face Shape1Face;
	// Face Shape2Face;

	return out;
}

JPC56_API void JPC56_RegisterDefaultAllocator() {
	JPH56::RegisterDefaultAllocator();
}

JPC56_API void JPC56_FactoryInit() {
	JPH56::Factory::sInstance = new JPH56::Factory();
}

JPC56_API void JPC56_FactoryDelete() {
	delete JPH56::Factory::sInstance;
	JPH56::Factory::sInstance = nullptr;
}

JPC56_API void JPC56_RegisterTypes() {
	JPH56::RegisterTypes();
}

JPC56_API void JPC56_UnregisterTypes() {
	JPH56::UnregisterTypes();
}

////////////////////////////////////////////////////////////////////////////////
// VertexList == Array<Float3> == std::vector<Float3>

JPC56_API JPC56_VertexList* JPC56_VertexList_new(const JPC56_Float3* storage, size_t len) {
	const JPH56::Float3* new_storage = (const JPH56::Float3*)storage;
	return to_jpc(new JPH56::VertexList(new_storage, new_storage + len));
}

////////////////////////////////////////////////////////////////////////////////
// IndexedTriangleList == Array<IndexedTriangle> == std::vector<IndexedTriangle>

JPC56_API JPC56_IndexedTriangleList* JPC56_IndexedTriangleList_new(const JPC56_IndexedTriangle* storage, size_t len) {
	const JPH56::IndexedTriangle* new_storage = (const JPH56::IndexedTriangle*)storage;
	return to_jpc(new JPH56::IndexedTriangleList(new_storage, new_storage + len));
}

////////////////////////////////////////////////////////////////////////////////
// TempAllocatorImpl

JPC56_API JPC56_TempAllocatorImpl* JPC56_TempAllocatorImpl_new(uint size) {
	return to_jpc(new JPH56::TempAllocatorImpl(size));
}

////////////////////////////////////////////////////////////////////////////////
// JobSystemThreadPool

JPC56_API JPC56_JobSystemThreadPool* JPC56_JobSystemThreadPool_new2(
	uint inMaxJobs,
	uint inMaxBarriers)
{
	return to_jpc(new JPH56::JobSystemThreadPool(inMaxJobs, inMaxBarriers));
}

JPC56_API JPC56_JobSystemThreadPool* JPC56_JobSystemThreadPool_new3(
	uint inMaxJobs,
	uint inMaxBarriers,
	int inNumThreads)
{
	return to_jpc(new JPH56::JobSystemThreadPool(inMaxJobs, inMaxBarriers, inNumThreads));
}

////////////////////////////////////////////////////////////////////////////////
// JobSystemSingleThreaded

JPC56_API JPC56_JobSystemSingleThreaded* JPC56_JobSystemSingleThreaded_new(uint inMaxJobs) {
	return to_jpc(new JPH56::JobSystemSingleThreaded(inMaxJobs));
}

////////////////////////////////////////////////////////////////////////////////
// CollisionGroup

JPC56_IMPL JPC56_CollisionGroup JPC56_CollisionGroup_to_jpc(const JPH56::CollisionGroup* input);

class JPC56_GroupFilterBridge final : public JPH56::GroupFilter {
public:
	explicit JPC56_GroupFilterBridge(const void *self, JPC56_GroupFilterFns fns) : self(self), fns(fns) {}

	bool CanCollide(const JPH56::CollisionGroup &inGroup1, const JPH56::CollisionGroup &inGroup2) const override {
		JPC56_CollisionGroup jpcGroup1 = JPC56_CollisionGroup_to_jpc(&inGroup1);
		JPC56_CollisionGroup jpcGroup2 = JPC56_CollisionGroup_to_jpc(&inGroup2);

		return fns.CanCollide(self, &jpcGroup1, &jpcGroup2);
	}

	void SaveBinaryState([[maybe_unused]] JPH56::StreamOut &inStream) const override {}
	void RestoreBinaryState([[maybe_unused]] JPH56::StreamIn &inStream) override {}

private:
	const void* self;
	JPC56_GroupFilterFns fns;
};

OPAQUE_WRAPPER(JPC56_GroupFilter, JPC56_GroupFilterBridge)
DESTRUCTOR(JPC56_GroupFilter)

JPC56_IMPL JPH56::CollisionGroup JPC56_CollisionGroup_to_jph(const JPC56_CollisionGroup* self) {
	const JPC56_GroupFilterBridge* filter_group = to_jph(self->GroupFilter);

	JPH56::CollisionGroup group(filter_group, self->GroupID, self->SubGroupID);
	return group;
}

JPC56_IMPL JPC56_CollisionGroup JPC56_CollisionGroup_to_jpc(const JPH56::CollisionGroup* input) {
	JPC56_CollisionGroup group{};
	group.GroupFilter; // NOTE: This member doesn't matter for callers of this function
	group.GroupID = input->GetGroupID();
	group.SubGroupID = input->GetSubGroupID();
	return group;
}

JPC56_API JPC56_GroupFilter* JPC56_GroupFilter_new(
	const void *self,
	JPC56_GroupFilterFns fns)
{
	return to_jpc(new JPC56_GroupFilterBridge(self, fns));
}

////////////////////////////////////////////////////////////////////////////////
// BroadPhaseLayerInterface

class JPC56_BroadPhaseLayerInterfaceBridge final : public JPH56::BroadPhaseLayerInterface {
public:
	explicit JPC56_BroadPhaseLayerInterfaceBridge(const void *self, JPC56_BroadPhaseLayerInterfaceFns fns) : self(self), fns(fns) {}

	virtual uint GetNumBroadPhaseLayers() const override {
		return fns.GetNumBroadPhaseLayers(self);
	}

	virtual JPH56::BroadPhaseLayer GetBroadPhaseLayer(JPH56::ObjectLayer inLayer) const override {
		return to_jph(fns.GetBroadPhaseLayer(self, inLayer));
	}

#if defined(JPH_EXTERNAL_PROFILE) || defined(JPH_PROFILE_ENABLED)
	virtual const char * GetBroadPhaseLayerName([[maybe_unused]] JPH56::BroadPhaseLayer inLayer) const override {
		return "FIXME";
	}
#endif

private:
	const void* self;
	JPC56_BroadPhaseLayerInterfaceFns fns;
};

OPAQUE_WRAPPER(JPC56_BroadPhaseLayerInterface, JPC56_BroadPhaseLayerInterfaceBridge)
DESTRUCTOR(JPC56_BroadPhaseLayerInterface)

JPC56_API JPC56_BroadPhaseLayerInterface* JPC56_BroadPhaseLayerInterface_new(
	const void *self,
	JPC56_BroadPhaseLayerInterfaceFns fns)
{
	return to_jpc(new JPC56_BroadPhaseLayerInterfaceBridge(self, fns));
}

////////////////////////////////////////////////////////////////////////////////
// ObjectVsBroadPhaseLayerFilter

class JPC56_ObjectVsBroadPhaseLayerFilterBridge final : public JPH56::ObjectVsBroadPhaseLayerFilter {
public:
	explicit JPC56_ObjectVsBroadPhaseLayerFilterBridge(const void *self, JPC56_ObjectVsBroadPhaseLayerFilterFns fns) : self(self), fns(fns) {}

	virtual bool ShouldCollide(JPH56::ObjectLayer inLayer1, JPH56::BroadPhaseLayer inLayer2) const override {
		return fns.ShouldCollide(self, inLayer1, to_jpc(inLayer2));
	}

private:
	const void* self;
	JPC56_ObjectVsBroadPhaseLayerFilterFns fns;
};

OPAQUE_WRAPPER(JPC56_ObjectVsBroadPhaseLayerFilter, JPC56_ObjectVsBroadPhaseLayerFilterBridge)
DESTRUCTOR(JPC56_ObjectVsBroadPhaseLayerFilter)

JPC56_API JPC56_ObjectVsBroadPhaseLayerFilter* JPC56_ObjectVsBroadPhaseLayerFilter_new(
	const void *self,
	JPC56_ObjectVsBroadPhaseLayerFilterFns fns)
{
	return to_jpc(new JPC56_ObjectVsBroadPhaseLayerFilterBridge(self, fns));
}

////////////////////////////////////////////////////////////////////////////////
// BroadPhaseLayerFilter

class JPC56_BroadPhaseLayerFilterBridge final : public JPH56::BroadPhaseLayerFilter {
public:
	explicit JPC56_BroadPhaseLayerFilterBridge(const void *self, JPC56_BroadPhaseLayerFilterFns fns) : self(self), fns(fns) {}

	virtual bool ShouldCollide(JPH56::BroadPhaseLayer inLayer) const override {
		return fns.ShouldCollide(self, to_jpc(inLayer));
	}

private:
	const void* self;
	JPC56_BroadPhaseLayerFilterFns fns;
};

OPAQUE_WRAPPER(JPC56_BroadPhaseLayerFilter, JPC56_BroadPhaseLayerFilterBridge)
DESTRUCTOR(JPC56_BroadPhaseLayerFilter)

JPC56_API JPC56_BroadPhaseLayerFilter* JPC56_BroadPhaseLayerFilter_new(
	const void *self,
	JPC56_BroadPhaseLayerFilterFns fns)
{
	return to_jpc(new JPC56_BroadPhaseLayerFilterBridge(self, fns));
}

////////////////////////////////////////////////////////////////////////////////
// ObjectLayerFilter

class JPC56_ObjectLayerFilterBridge final : public JPH56::ObjectLayerFilter {
public:
	explicit JPC56_ObjectLayerFilterBridge(const void *self, JPC56_ObjectLayerFilterFns fns) : self(self), fns(fns) {}

	virtual bool ShouldCollide(JPH56::ObjectLayer inLayer) const override {
		return fns.ShouldCollide(self, inLayer);
	}

private:
	const void* self;
	JPC56_ObjectLayerFilterFns fns;
};

OPAQUE_WRAPPER(JPC56_ObjectLayerFilter, JPC56_ObjectLayerFilterBridge)
DESTRUCTOR(JPC56_ObjectLayerFilter)

JPC56_API JPC56_ObjectLayerFilter* JPC56_ObjectLayerFilter_new(
	const void *self,
	JPC56_ObjectLayerFilterFns fns)
{
	return to_jpc(new JPC56_ObjectLayerFilterBridge(self, fns));
}

////////////////////////////////////////////////////////////////////////////////
// BodyFilter

class JPC56_BodyFilterBridge final : public JPH56::BodyFilter {
public:
	explicit JPC56_BodyFilterBridge(const void *self, JPC56_BodyFilterFns fns) : self(self), fns(fns) {}

	virtual bool ShouldCollide(const JPH56::BodyID &inBodyID) const override {
		return fns.ShouldCollide(self, to_jpc(inBodyID));
	}

	virtual bool ShouldCollideLocked(const JPH56::Body &inBody) const override {
		return fns.ShouldCollideLocked(self, to_jpc(&inBody));
	}

private:
	const void* self;
	JPC56_BodyFilterFns fns;
};

OPAQUE_WRAPPER(JPC56_BodyFilter, JPC56_BodyFilterBridge)
DESTRUCTOR(JPC56_BodyFilter)

JPC56_API JPC56_BodyFilter* JPC56_BodyFilter_new(
	const void *self,
	JPC56_BodyFilterFns fns)
{
	return to_jpc(new JPC56_BodyFilterBridge(self, fns));
}

////////////////////////////////////////////////////////////////////////////////
// ShapeFilter

class JPC56_ShapeFilterBridge final : public JPH56::ShapeFilter {
public:
	explicit JPC56_ShapeFilterBridge(const void *self, JPC56_ShapeFilterFns fns) : self(self), fns(fns) {}

	virtual bool ShouldCollide(const JPH56::Shape *inShape2, const JPH56::SubShapeID &inSubShapeIDOfShape2) const override {
		if (fns.ShouldCollide == nullptr) {
			return true;
		}

		return fns.ShouldCollide(self, to_jpc(inShape2), to_jpc(inSubShapeIDOfShape2));
	}

	virtual bool ShouldCollide(
		const JPH56::Shape *inShape1, const JPH56::SubShapeID &inSubShapeIDOfShape1,
		const JPH56::Shape *inShape2, const JPH56::SubShapeID &inSubShapeIDOfShape2) const override
	{
		if (fns.ShouldCollideTwoShapes == nullptr) {
			return true;
		}

		return fns.ShouldCollideTwoShapes(self,
			to_jpc(inShape1), to_jpc(inSubShapeIDOfShape1),
			to_jpc(inShape2), to_jpc(inSubShapeIDOfShape2));
	}

private:
	const void* self;
	JPC56_ShapeFilterFns fns;
};

OPAQUE_WRAPPER(JPC56_ShapeFilter, JPC56_ShapeFilterBridge)
DESTRUCTOR(JPC56_ShapeFilter)

JPC56_API JPC56_ShapeFilter* JPC56_ShapeFilter_new(
	const void *self,
	JPC56_ShapeFilterFns fns)
{
	return to_jpc(new JPC56_ShapeFilterBridge(self, fns));
}

////////////////////////////////////////////////////////////////////////////////
// SimShapeFilter

class JPC56_SimShapeFilterBridge final : public JPH56::SimShapeFilter {
public:
	explicit JPC56_SimShapeFilterBridge(const void *self, JPC56_SimShapeFilterFns fns) : self(self), fns(fns) {}

	virtual bool ShouldCollide(
		const JPH56::Body &inBody1, const JPH56::Shape *inShape1, const JPH56::SubShapeID &inSubShapeIDOfShape1,
		const JPH56::Body &inBody2, const JPH56::Shape *inShape2, const JPH56::SubShapeID &inSubShapeIDOfShape2) const override
	{
		if (fns.ShouldCollide == nullptr) {
			return true;
		}

		return fns.ShouldCollide(self,
			to_jpc(&inBody1), to_jpc(inShape1), to_jpc(inSubShapeIDOfShape1),
			to_jpc(&inBody2), to_jpc(inShape2), to_jpc(inSubShapeIDOfShape2));
	}

private:
	const void* self;
	JPC56_SimShapeFilterFns fns;
};

OPAQUE_WRAPPER(JPC56_SimShapeFilter, JPC56_SimShapeFilterBridge)
DESTRUCTOR(JPC56_SimShapeFilter)

JPC56_API JPC56_SimShapeFilter* JPC56_SimShapeFilter_new(
	const void *self,
	JPC56_SimShapeFilterFns fns)
{
	return to_jpc(new JPC56_SimShapeFilterBridge(self, fns));
}

////////////////////////////////////////////////////////////////////////////////
// JPC56_ObjectLayerPairFilter

class JPC56_ObjectLayerPairFilterBridge final : public JPH56::ObjectLayerPairFilter {
public:
	explicit JPC56_ObjectLayerPairFilterBridge(const void *self, JPC56_ObjectLayerPairFilterFns fns) : self(self), fns(fns) {}

	virtual bool ShouldCollide(JPH56::ObjectLayer inLayer1, JPH56::ObjectLayer inLayer2) const override {
		return fns.ShouldCollide(self, inLayer1, inLayer2);
	}

private:
	const void* self;
	JPC56_ObjectLayerPairFilterFns fns;
};

OPAQUE_WRAPPER(JPC56_ObjectLayerPairFilter, JPC56_ObjectLayerPairFilterBridge)
DESTRUCTOR(JPC56_ObjectLayerPairFilter)

JPC56_API JPC56_ObjectLayerPairFilter* JPC56_ObjectLayerPairFilter_new(
	const void *self,
	JPC56_ObjectLayerPairFilterFns fns)
{
	return to_jpc(new JPC56_ObjectLayerPairFilterBridge(self, fns));
}

////////////////////////////////////////////////////////////////////////////////
// JPC56_ContactListener

class JPC56_ContactListenerBridge final : public JPH56::ContactListener {
public:
	explicit JPC56_ContactListenerBridge(void *self, JPC56_ContactListenerFns fns) : self(self), fns(fns) {}

	JPH56::ValidateResult OnContactValidate(
		const JPH56::Body &inBody1,
		const JPH56::Body &inBody2,
		JPH56::RVec3Arg inBaseOffset,
		const JPH56::CollideShapeResult &inCollisionResult) override
	{
		if (fns.OnContactValidate != nullptr) {
			JPC56_CollideShapeResult collisionResult = JPC56_CollideShapeResult_to_jpc(inCollisionResult);
			return to_jph(fns.OnContactValidate(self, to_jpc(&inBody1), to_jpc(&inBody2), to_jpc(inBaseOffset), &collisionResult));
		}
		return ContactListener::OnContactValidate(inBody1, inBody2, inBaseOffset, inCollisionResult);
	}

	void OnContactAdded(
		const JPH56::Body &inBody1,
		const JPH56::Body &inBody2,
		const JPH56::ContactManifold &inManifold,
		JPH56::ContactSettings &ioSettings) override
	{
		if (fns.OnContactAdded != nullptr) {
			const auto* cManifold = reinterpret_cast<const JPC56_ContactManifold*>(&inManifold);
			auto* cSettings = reinterpret_cast<JPC56_ContactSettings*>(&ioSettings);

			fns.OnContactAdded(self, to_jpc(&inBody1), to_jpc(&inBody2), cManifold, cSettings);
		}
	}

	void OnContactPersisted(
		const JPH56::Body &inBody1,
		const JPH56::Body &inBody2,
		const JPH56::ContactManifold &inManifold,
		JPH56::ContactSettings &ioSettings) override
	{
		if (fns.OnContactPersisted != nullptr) {
			const auto* cManifold = reinterpret_cast<const JPC56_ContactManifold*>(&inManifold);
			auto* cSettings = reinterpret_cast<JPC56_ContactSettings*>(&ioSettings);

			fns.OnContactPersisted(self, to_jpc(&inBody1), to_jpc(&inBody2), cManifold, cSettings);
		}
	}

	void OnContactRemoved(const JPH56::SubShapeIDPair &inSubShapePair) override {
		if (fns.OnContactRemoved != nullptr) {
			const auto* cSubShapePair = reinterpret_cast<const JPC56_SubShapeIDPair*>(&inSubShapePair);

			fns.OnContactRemoved(self, cSubShapePair);
		}
	}

private:
	void* self;
	JPC56_ContactListenerFns fns;
};

OPAQUE_WRAPPER(JPC56_ContactListener, JPC56_ContactListenerBridge)
DESTRUCTOR(JPC56_ContactListener)

JPC56_API JPC56_ContactListener* JPC56_ContactListener_new(
	void *self,
	JPC56_ContactListenerFns fns)
{
	return to_jpc(new JPC56_ContactListenerBridge(self, fns));
}

JPC56_API void JPC56_EstimateCollisionResponse(
	const JPC56_Body* inBody1,
	const JPC56_Body* inBody2,
	const JPC56_ContactManifold* inManifold,
	JPC56_CollisionEstimationResult* outResult,
	float inCombinedFriction,
	float inCombinedRestitution,
	float inMinVelocityForRestitution,	///< = 1.0f
	uint inNumIterations				///< = 10
) {
	const auto* jphManifold = reinterpret_cast<const JPH56::ContactManifold*>(inManifold);
	auto* jphResult = reinterpret_cast<JPH56::CollisionEstimationResult*>(outResult);

	JPH56::EstimateCollisionResponse(
		*to_jph(inBody1),
		*to_jph(inBody2),
		*jphManifold,
		*jphResult,
		inCombinedFriction,
		inCombinedRestitution,
		inMinVelocityForRestitution,
		inNumIterations);
}

////////////////////////////////////////////////////////////////////////////////
// JPC56_CastShapeCollector

class JPC56_CastShapeCollectorBridge;
OPAQUE_WRAPPER(JPC56_CastShapeCollector, JPC56_CastShapeCollectorBridge)

class JPC56_CastShapeCollectorBridge final : public JPH56::CastShapeCollector {
	using ResultType = JPH56::ShapeCastResult;

public:
	explicit JPC56_CastShapeCollectorBridge(void *self, JPC56_CastShapeCollectorFns fns) : self(self), fns(fns) {}

	void Reset() override {
		JPH56::CastShapeCollector::Reset();

		if (fns.Reset != nullptr) {
			fns.Reset(self);
		}
	}

	void AddHit(const ResultType &inResult) override {
		JPC56_ShapeCastResult result = JPC56_ShapeCastResult_to_jpc(inResult);
		JPC56_CastShapeCollector *base = to_jpc(this);

		fns.AddHit(self, base, &result);
	}

private:
	void* self;
	JPC56_CastShapeCollectorFns fns;
};

DESTRUCTOR(JPC56_CastShapeCollector)

JPC56_API JPC56_CastShapeCollector* JPC56_CastShapeCollector_new(
	void *self,
	JPC56_CastShapeCollectorFns fns)
{
	return to_jpc(new JPC56_CastShapeCollectorBridge(self, fns));
}

JPC56_API void JPC56_CastShapeCollector_UpdateEarlyOutFraction(JPC56_CastShapeCollector* self, float inFraction) {
	to_jph(self)->UpdateEarlyOutFraction(inFraction);
}

////////////////////////////////////////////////////////////////////////////////
// JPC56_CollideShapeCollector

class JPC56_CollideShapeCollectorBridge;
OPAQUE_WRAPPER(JPC56_CollideShapeCollector, JPC56_CollideShapeCollectorBridge)

class JPC56_CollideShapeCollectorBridge final : public JPH56::CollideShapeCollector {
	using ResultType = JPH56::CollideShapeResult;

public:
	explicit JPC56_CollideShapeCollectorBridge(void *self, JPC56_CollideShapeCollectorFns fns) : self(self), fns(fns) {}

	void Reset() override {
		JPH56::CollideShapeCollector::Reset();

		if (fns.Reset != nullptr) {
			fns.Reset(self);
		}
	}

	void AddHit(const ResultType &inResult) override {
		JPC56_CollideShapeResult result = JPC56_CollideShapeResult_to_jpc(inResult);
		JPC56_CollideShapeCollector *base = to_jpc(this);

		fns.AddHit(self, base, &result);
	}

private:
	void* self;
	JPC56_CollideShapeCollectorFns fns;
};

DESTRUCTOR(JPC56_CollideShapeCollector)

JPC56_API JPC56_CollideShapeCollector* JPC56_CollideShapeCollector_new(
	void *self,
	JPC56_CollideShapeCollectorFns fns)
{
	return to_jpc(new JPC56_CollideShapeCollectorBridge(self, fns));
}

JPC56_API void JPC56_CollideShapeCollector_UpdateEarlyOutFraction(JPC56_CollideShapeCollector* self, float inFraction) {
	to_jph(self)->UpdateEarlyOutFraction(inFraction);
}

////////////////////////////////////////////////////////////////////////////////
// BodyManager::DrawSettings

JPC56_API void JPC56_BodyManager_DrawSettings_default(JPC56_BodyManager_DrawSettings* object) {
	*object = to_jpc(JPH56::BodyManager::DrawSettings());
}

////////////////////////////////////////////////////////////////////////////////
// DebugRendererSimple

class JPC56_DebugRendererSimpleBridge final : public JPH56::DebugRendererSimple {
public:
	explicit JPC56_DebugRendererSimpleBridge(const void *self, JPC56_DebugRendererSimpleFns fns) : self(self), fns(fns) {}

	virtual void DrawLine(JPH56::RVec3Arg inFrom, JPH56::RVec3Arg inTo, JPH56::ColorArg inColor) override {
		fns.DrawLine(self, to_jpc(inFrom), to_jpc(inTo), to_jpc(inColor));
	}

	virtual void DrawText3D(
		[[maybe_unused]] JPH56::RVec3Arg inPosition,
		[[maybe_unused]] const std::string_view &inString,
		[[maybe_unused]] JPH56::ColorArg inColor = JPH56::Color::sWhite,
		[[maybe_unused]] float inHeight = 0.5f) override
	{
		// TODO
	}

private:
	const void* self;
	JPC56_DebugRendererSimpleFns fns;
};

OPAQUE_WRAPPER(JPC56_DebugRendererSimple, JPC56_DebugRendererSimpleBridge)
DESTRUCTOR(JPC56_DebugRendererSimple)

JPC56_API JPC56_DebugRendererSimple* JPC56_DebugRendererSimple_new(
	const void *self,
	JPC56_DebugRendererSimpleFns fns)
{
	return to_jpc(new JPC56_DebugRendererSimpleBridge(self, fns));
}

////////////////////////////////////////////////////////////////////////////////
// String

JPC56_API const char* JPC56_String_c_str(JPC56_String* self) {
	return to_jph(self)->c_str();
}

////////////////////////////////////////////////////////////////////////////////
// Constraint -> RefTarget<Constraint>

OPAQUE_WRAPPER(JPC56_Constraint, JPH56::Constraint);

// RefTarget<Constraint>
JPC56_API uint32_t JPC56_Constraint_GetRefCount(const JPC56_Constraint* self) {
	return to_jph(self)->GetRefCount();
}

JPC56_API void JPC56_Constraint_AddRef(const JPC56_Constraint* self) {
	to_jph(self)->AddRef();
}

JPC56_API void JPC56_Constraint_Release(const JPC56_Constraint* self) {
	to_jph(self)->Release();
}

// Constraint
JPC56_API void JPC56_Constraint_delete(JPC56_Constraint* self) {
	delete to_jph(self);
}

// JPC56_API JPC56_ConstraintType JPC56_Constraint_GetType(const JPC56_Constraint* self);
// JPC56_API JPC56_ConstraintSubType JPC56_Constraint_GetSubType(const JPC56_Constraint* self);

JPC56_API uint32_t JPC56_Constraint_GetConstraintPriority(const JPC56_Constraint* self) {
	return to_jph(self)->GetConstraintPriority();
}

JPC56_API void JPC56_Constraint_SetConstraintPriority(JPC56_Constraint* self, uint32_t inPriority) {
	to_jph(self)->SetConstraintPriority(inPriority);
}

JPC56_API uint JPC56_Constraint_GetNumVelocityStepsOverride(const JPC56_Constraint* self) {
	return to_jph(self)->GetNumVelocityStepsOverride();
}

JPC56_API void JPC56_Constraint_SetNumVelocityStepsOverride(JPC56_Constraint* self, uint inN) {
	to_jph(self)->SetNumVelocityStepsOverride(inN);
}

JPC56_API uint JPC56_Constraint_GetNumPositionStepsOverride(const JPC56_Constraint* self) {
	return to_jph(self)->GetNumPositionStepsOverride();
}

JPC56_API void JPC56_Constraint_SetNumPositionStepsOverride(JPC56_Constraint* self, uint inN) {
	to_jph(self)->SetNumPositionStepsOverride(inN);
}

JPC56_API bool JPC56_Constraint_GetEnabled(const JPC56_Constraint* self) {
	return to_jph(self)->GetEnabled();
}

JPC56_API void JPC56_Constraint_SetEnabled(JPC56_Constraint* self, bool inEnabled) {
	to_jph(self)->SetEnabled(inEnabled);
}

JPC56_API uint64_t JPC56_Constraint_GetUserData(const JPC56_Constraint* self) {
	return to_jph(self)->GetUserData();
}

JPC56_API void JPC56_Constraint_SetUserData(JPC56_Constraint* self, uint64_t inUserData) {
	to_jph(self)->SetUserData(inUserData);
}

JPC56_API void JPC56_Constraint_NotifyShapeChanged(JPC56_Constraint* self, JPC56_BodyID inBodyID, JPC56_Vec3 inDeltaCOM) {
	to_jph(self)->NotifyShapeChanged(to_jph(inBodyID), to_jph(inDeltaCOM));
}

////////////////////////////////////////////////////////////////////////////////
// TwoBodyConstraint -> Constraint -> RefTarget<Constraint>

OPAQUE_WRAPPER(JPC56_TwoBodyConstraint, JPH56::TwoBodyConstraint);

JPC56_API JPC56_Body* JPC56_TwoBodyConstraint_GetBody1(const JPC56_TwoBodyConstraint* self) {
	return to_jpc(to_jph(self)->GetBody1());
}

JPC56_API JPC56_Body* JPC56_TwoBodyConstraint_GetBody2(const JPC56_TwoBodyConstraint* self) {
	return to_jpc(to_jph(self)->GetBody2());
}

JPC56_API JPC56_Mat44 JPC56_TwoBodyConstraint_GetConstraintToBody1Matrix(const JPC56_TwoBodyConstraint* self) {
	return to_jpc(to_jph(self)->GetConstraintToBody1Matrix());
}

JPC56_API JPC56_Mat44 JPC56_TwoBodyConstraint_GetConstraintToBody2Matrix(const JPC56_TwoBodyConstraint* self) {
	return to_jpc(to_jph(self)->GetConstraintToBody2Matrix());
}

////////////////////////////////////////////////////////////////////////////////
// FixedConstraint -> TwoBodyConstraint -> Constraint -> RefTarget<Constraint>

OPAQUE_WRAPPER(JPC56_FixedConstraint, JPH56::FixedConstraint);

JPC56_API JPC56_Vec3 JPC56_FixedConstraint_GetTotalLambdaPosition(const JPC56_FixedConstraint* self) {
	return to_jpc(to_jph(self)->GetTotalLambdaPosition());
}

JPC56_API JPC56_Vec3 JPC56_FixedConstraint_GetTotalLambdaRotation(const JPC56_FixedConstraint* self) {
	return to_jpc(to_jph(self)->GetTotalLambdaRotation());
}

////////////////////////////////////////////////////////////////////////////////
// DistanceConstraint -> TwoBodyConstraint -> Constraint -> RefTarget<Constraint>

OPAQUE_WRAPPER(JPC56_DistanceConstraint, JPH56::DistanceConstraint);

JPC56_API float JPC56_DistanceConstraint_GetTotalLambdaPosition(const JPC56_DistanceConstraint* self) {
	return to_jph(self)->GetTotalLambdaPosition();
}

////////////////////////////////////////////////////////////////////////////////
// SixDOFConstraint -> TwoBodyConstraint -> Constraint -> RefTarget<Constraint>

OPAQUE_WRAPPER(JPC56_SixDOFConstraint, JPH56::SixDOFConstraint);

JPC56_API JPC56_Vec3 JPC56_SixDOFConstraint_GetTranslationLimitsMin(const JPC56_SixDOFConstraint* self) {
	return to_jpc(to_jph(self)->GetTranslationLimitsMin());
}

JPC56_API JPC56_Vec3 JPC56_SixDOFConstraint_GetTranslationLimitsMax(const JPC56_SixDOFConstraint* self) {
	return to_jpc(to_jph(self)->GetTranslationLimitsMax());
}

JPC56_API void JPC56_SixDOFConstraint_SetTranslationLimits(JPC56_SixDOFConstraint* self, JPC56_Vec3 inLimitMin, JPC56_Vec3 inLimitMax) {
	to_jph(self)->SetTranslationLimits(to_jph(inLimitMin), to_jph(inLimitMax));
}

JPC56_API JPC56_Vec3 JPC56_SixDOFConstraint_GetRotationLimitsMin(const JPC56_SixDOFConstraint* self) {
	return to_jpc(to_jph(self)->GetRotationLimitsMin());
}

JPC56_API JPC56_Vec3 JPC56_SixDOFConstraint_GetRotationLimitsMax(const JPC56_SixDOFConstraint* self) {
	return to_jpc(to_jph(self)->GetRotationLimitsMax());
}

JPC56_API void JPC56_SixDOFConstraint_SetRotationLimits(JPC56_SixDOFConstraint* self, JPC56_Vec3 inLimitMin, JPC56_Vec3 inLimitMax) {
	to_jph(self)->SetRotationLimits(to_jph(inLimitMin), to_jph(inLimitMax));
}

JPC56_API float JPC56_SixDOFConstraint_GetLimitsMin(const JPC56_SixDOFConstraint* self, JPC56_SixDOFConstraint_Axis inAxis) {
	return to_jph(self)->GetLimitsMin((JPH56::SixDOFConstraint::EAxis)inAxis);
}

JPC56_API float JPC56_SixDOFConstraint_GetLimitsMax(const JPC56_SixDOFConstraint* self, JPC56_SixDOFConstraint_Axis inAxis) {
	return to_jph(self)->GetLimitsMax((JPH56::SixDOFConstraint::EAxis)inAxis);
}

JPC56_API bool JPC56_SixDOFConstraint_IsFreeAxis(const JPC56_SixDOFConstraint* self, JPC56_SixDOFConstraint_Axis inAxis);

// const SpringSettings & GetLimitsSpringSettings(JPC56_SixDOFConstraint_Axis inAxis) const { JPH_ASSERT(inAxis < JPC56_SixDOFConstraint_Axis::NumTranslation); return mLimitsSpringSettings[inAxis]; }
// void SetLimitsSpringSettings(JPC56_SixDOFConstraint_Axis inAxis, const SpringSettings& inLimitsSpringSettings) { JPH_ASSERT(inAxis < JPC56_SixDOFConstraint_Axis::NumTranslation); mLimitsSpringSettings[inAxis] = inLimitsSpringSettings; CacheHasSpringLimits(); }

JPC56_API void JPC56_SixDOFConstraint_SetMaxFriction(JPC56_SixDOFConstraint* self, JPC56_SixDOFConstraint_Axis inAxis, float inFriction) {
	to_jph(self)->SetMaxFriction((JPH56::SixDOFConstraint::EAxis)inAxis, inFriction);
}

JPC56_API float JPC56_SixDOFConstraint_GetMaxFriction(const JPC56_SixDOFConstraint* self, JPC56_SixDOFConstraint_Axis inAxis) {
	return to_jph(self)->GetMaxFriction((JPH56::SixDOFConstraint::EAxis)inAxis);
}

JPC56_API JPC56_Quat JPC56_SixDOFConstraint_GetRotationInConstraintSpace(const JPC56_SixDOFConstraint* self) {
	return to_jpc(to_jph(self)->GetRotationInConstraintSpace());
}

/// Motor settings
// MotorSettings & GetMotorSettings(EAxis inAxis)
// const MotorSettings & GetMotorSettings(EAxis inAxis) const

// void SetMotorState(EAxis inAxis, EMotorState inState);
// EMotorState GetMotorState(EAxis inAxis) const

JPC56_API JPC56_Vec3 JPC56_SixDOFConstraint_GetTargetVelocityCS(const JPC56_SixDOFConstraint* self) {
	return to_jpc(to_jph(self)->GetTargetVelocityCS());
}

JPC56_API void JPC56_SixDOFConstraint_SetTargetVelocityCS(JPC56_SixDOFConstraint* self, JPC56_Vec3 inVelocity) {
	to_jph(self)->SetTargetVelocityCS(to_jph(inVelocity));
}

JPC56_API JPC56_Vec3 JPC56_SixDOFConstraint_GetTargetAngularVelocityCS(const JPC56_SixDOFConstraint* self) {
	return to_jpc(to_jph(self)->GetTargetAngularVelocityCS());
}

JPC56_API void JPC56_SixDOFConstraint_SetTargetAngularVelocityCS(JPC56_SixDOFConstraint* self, JPC56_Vec3 inAngularVelocity) {
	to_jph(self)->SetTargetAngularVelocityCS(to_jph(inAngularVelocity));
}

JPC56_API JPC56_Vec3 JPC56_SixDOFConstraint_GetTargetPositionCS(const JPC56_SixDOFConstraint* self) {
	return to_jpc(to_jph(self)->GetTargetPositionCS());
}

JPC56_API void JPC56_SixDOFConstraint_SetTargetPositionCS(JPC56_SixDOFConstraint* self, JPC56_Vec3 inPosition) {
	to_jph(self)->SetTargetPositionCS(to_jph(inPosition));
}

JPC56_API JPC56_Quat JPC56_SixDOFConstraint_GetTargetOrientationCS(const JPC56_SixDOFConstraint* self) {
	return to_jpc(to_jph(self)->GetTargetOrientationCS());
}

JPC56_API void JPC56_SixDOFConstraint_SetTargetOrientationCS(JPC56_SixDOFConstraint* self, JPC56_Quat inOrientation) {
	to_jph(self)->SetTargetOrientationCS(to_jph(inOrientation));
}

JPC56_API void JPC56_SixDOFConstraint_SetTargetOrientationBS(JPC56_SixDOFConstraint* self, JPC56_Quat inOrientation) {
	to_jph(self)->SetTargetOrientationBS(to_jph(inOrientation));
}

JPC56_API JPC56_Vec3 JPC56_SixDOFConstraint_GetTotalLambdaPosition(JPC56_SixDOFConstraint* self) {
	return to_jpc(to_jph(self)->GetTotalLambdaPosition());
}

JPC56_API JPC56_Vec3 JPC56_SixDOFConstraint_GetTotalLambdaRotation(JPC56_SixDOFConstraint* self) {
	return to_jpc(to_jph(self)->GetTotalLambdaRotation());
}

JPC56_API JPC56_Vec3 JPC56_SixDOFConstraint_GetTotalLambdaMotorTranslation(JPC56_SixDOFConstraint* self) {
	return to_jpc(to_jph(self)->GetTotalLambdaMotorTranslation());
}

JPC56_API JPC56_Vec3 JPC56_SixDOFConstraint_GetTotalLambdaMotorRotation(JPC56_SixDOFConstraint* self) {
	return to_jpc(to_jph(self)->GetTotalLambdaMotorRotation());
}

////////////////////////////////////////////////////////////////////////////////
// HingeConstraint -> TwoBodyConstraint -> Constraint -> RefTarget<Constraint>

OPAQUE_WRAPPER(JPC56_HingeConstraint, JPH56::HingeConstraint);

JPC56_API JPC56_Constraint* JPC56_HingeConstraint_to_Constraint(JPC56_HingeConstraint* self) {
	return (JPC56_Constraint*)(self);
}

JPC56_API void JPC56_HingeConstraint_SetMotorState(JPC56_HingeConstraint* self, JPC56_MotorState inState) {
	to_jph(self)->SetMotorState(to_jph(inState));
}

JPC56_API JPC56_MotorState JPC56_HingeConstraint_GetMotorState(const JPC56_HingeConstraint* self) {
	return to_jpc(to_jph(self)->GetMotorState());
}

JPC56_API void JPC56_HingeConstraint_SetTargetAngularVelocity(JPC56_HingeConstraint* self, float inAngularVelocity) {
	to_jph(self)->SetTargetAngularVelocity(inAngularVelocity);
}

JPC56_API float JPC56_HingeConstraint_GetTargetAngularVelocity(const JPC56_HingeConstraint* self) {
	return to_jph(self)->GetTargetAngularVelocity();
}

JPC56_API void JPC56_HingeConstraint_SetTargetAngle(JPC56_HingeConstraint* self, float inAngle) {
	to_jph(self)->SetTargetAngle(inAngle);
}

JPC56_API float JPC56_HingeConstraint_GetTargetAngle(const JPC56_HingeConstraint* self) {
	return to_jph(self)->GetTargetAngle();
}

JPC56_API JPC56_Vec3 JPC56_HingeConstraint_GetTotalLambdaPosition(const JPC56_HingeConstraint* self) {
	return to_jpc(to_jph(self)->GetTotalLambdaPosition());
}

JPC56_API JPC56_Vec2 JPC56_HingeConstraint_GetTotalLambdaRotation(const JPC56_HingeConstraint* self) {
	return to_jpc(to_jph(self)->GetTotalLambdaRotation());
}

JPC56_API float JPC56_HingeConstraint_GetTotalLambdaRotationLimits(const JPC56_HingeConstraint* self) {
	return to_jph(self)->GetTotalLambdaRotationLimits();
}

JPC56_API float JPC56_HingeConstraint_GetTotalLambdaMotor(const JPC56_HingeConstraint* self) {
	return to_jph(self)->GetTotalLambdaMotor();
}


////////////////////////////////////////////////////////////////////////////////
// SliderConstraint -> TwoBodyConstraint -> Constraint -> RefTarget<Constraint>

OPAQUE_WRAPPER(JPC56_SliderConstraint, JPH56::SliderConstraint);

JPC56_API void JPC56_SliderConstraint_SetMotorState(JPC56_SliderConstraint* self, JPC56_MotorState inState) {
	to_jph(self)->SetMotorState(to_jph(inState));
}

JPC56_API JPC56_MotorState JPC56_SliderConstraint_GetMotorState(const JPC56_SliderConstraint* self) {
	return to_jpc(to_jph(self)->GetMotorState());
}

JPC56_API void JPC56_SliderConstraint_SetTargetVelocity(JPC56_SliderConstraint* self, float inVelocity) {
	to_jph(self)->SetTargetVelocity(inVelocity);
}

JPC56_API float JPC56_SliderConstraint_GetTargetVelocity(const JPC56_SliderConstraint* self) {
	return to_jph(self)->GetTargetVelocity();
}

JPC56_API void JPC56_SliderConstraint_SetTargetPosition(JPC56_SliderConstraint* self, float inPosition) {
	to_jph(self)->SetTargetPosition(inPosition);
}

JPC56_API float JPC56_SliderConstraint_GetTargetPosition(const JPC56_SliderConstraint* self) {
	return to_jph(self)->GetTargetPosition();
}

JPC56_API JPC56_Vec2 JPC56_SliderConstraint_GetTotalLambdaPosition(const JPC56_SliderConstraint* self) {
	return to_jpc(to_jph(self)->GetTotalLambdaPosition());
}

JPC56_API float JPC56_SliderConstraint_GetTotalLambdaPositionLimits(const JPC56_SliderConstraint* self) {
	return to_jph(self)->GetTotalLambdaPositionLimits();
}

JPC56_API JPC56_Vec3 JPC56_SliderConstraint_GetTotalLambdaRotation(const JPC56_SliderConstraint* self) {
	return to_jpc(to_jph(self)->GetTotalLambdaRotation());
}

JPC56_API float JPC56_SliderConstraint_GetTotalLambdaMotor(const JPC56_SliderConstraint* self) {
	return to_jph(self)->GetTotalLambdaMotor();
}


////////////////////////////////////////////////////////////////////////////////
// ConstraintSettings

JPC56_IMPL void JPC56_ConstraintSettings_to_jpc(
	JPC56_ConstraintSettings* outJpc,
	const JPH56::ConstraintSettings* inJph)
{
	outJpc->Enabled = inJph->mEnabled;
	outJpc->ConstraintPriority = inJph->mConstraintPriority;
	outJpc->NumVelocityStepsOverride = inJph->mNumVelocityStepsOverride;
	outJpc->NumPositionStepsOverride = inJph->mNumPositionStepsOverride;
	outJpc->DrawConstraintSize = inJph->mDrawConstraintSize;
	outJpc->UserData = inJph->mUserData;
}

JPC56_IMPL void JPC56_ConstraintSettings_to_jph(
	const JPC56_ConstraintSettings* inJpc,
	JPH56::ConstraintSettings* outJph)
{
	outJph->mEnabled = inJpc->Enabled;
	outJph->mConstraintPriority = inJpc->ConstraintPriority;
	outJph->mNumVelocityStepsOverride = inJpc->NumVelocityStepsOverride;
	outJph->mNumPositionStepsOverride = inJpc->NumPositionStepsOverride;
	outJph->mDrawConstraintSize = inJpc->DrawConstraintSize;
	outJph->mUserData = inJpc->UserData;
}

JPC56_API void JPC56_ConstraintSettings_default(JPC56_ConstraintSettings* settings) {
	// rurix M125: Jolt 5.6 起 ConstraintSettings 基类 ctor 为 protected(Constraint.h),
	// 经派生 shim 取默认值(零行为变化)。
	struct JPCConstraintSettingsDefaults : JPH56::ConstraintSettings {};
	JPCConstraintSettingsDefaults defaultSettings{};
	JPC56_ConstraintSettings_to_jpc(settings, &defaultSettings);
}

////////////////////////////////////////////////////////////////////////////////
// SpringSettings

JPC56_IMPL void JPC56_SpringSettings_to_jpc(
	JPC56_SpringSettings* outJpc,
	const JPH56::SpringSettings* inJph)
{
	outJpc->Mode = to_jpc(inJph->mMode);
	outJpc->FrequencyOrStiffness = inJph->mFrequency;
	outJpc->Damping = inJph->mDamping;
}

JPC56_IMPL void JPC56_SpringSettings_to_jph(
	const JPC56_SpringSettings* inJpc,
	JPH56::SpringSettings* outJph)
{
	outJph->mMode = to_jph(inJpc->Mode);
	outJph->mFrequency = inJpc->FrequencyOrStiffness;
	outJph->mDamping = inJpc->Damping;
}

JPC56_API void JPC56_SpringSettings_default(JPC56_SpringSettings* settings) {
	JPH56::SpringSettings defaultSettings{};
	JPC56_SpringSettings_to_jpc(settings, &defaultSettings);
}

////////////////////////////////////////////////////////////////////////////////
// MotorSettings

JPC56_IMPL void JPC56_MotorSettings_to_jpc(
	JPC56_MotorSettings* outJpc,
	const JPH56::MotorSettings* inJph)
{
	JPC56_SpringSettings_to_jpc(&outJpc->SpringSettings, &inJph->mSpringSettings);
	outJpc->MinForceLimit = inJph->mMinForceLimit;
	outJpc->MaxForceLimit = inJph->mMaxForceLimit;
	outJpc->MinTorqueLimit = inJph->mMinTorqueLimit;
	outJpc->MaxTorqueLimit = inJph->mMaxTorqueLimit;
}

JPC56_IMPL void JPC56_MotorSettings_to_jph(
	const JPC56_MotorSettings* inJpc,
	JPH56::MotorSettings* outJph)
{
	JPC56_SpringSettings_to_jph(&inJpc->SpringSettings, &outJph->mSpringSettings);
	outJph->mMinForceLimit = inJpc->MinForceLimit;
	outJph->mMaxForceLimit = inJpc->MaxForceLimit;
	outJph->mMinTorqueLimit = inJpc->MinTorqueLimit;
	outJph->mMaxTorqueLimit = inJpc->MaxTorqueLimit;
}

JPC56_API void JPC56_MotorSettings_default(JPC56_MotorSettings* settings) {
	JPH56::MotorSettings defaultSettings{};
	JPC56_MotorSettings_to_jpc(settings, &defaultSettings);
}

////////////////////////////////////////////////////////////////////////////////
// FixedConstraintSettings -> TwoBodyConstraintSettings -> ConstraintSettings

JPC56_IMPL void JPC56_FixedConstraintSettings_to_jpc(
	JPC56_FixedConstraintSettings* outJpc,
	const JPH56::FixedConstraintSettings* inJph)
{
	JPC56_ConstraintSettings_to_jpc(&outJpc->ConstraintSettings, inJph);

	outJpc->Space = static_cast<JPC56_ConstraintSpace>(inJph->mSpace);
	outJpc->AutoDetectPoint = inJph->mAutoDetectPoint;
	outJpc->Point1 = to_jpc(inJph->mPoint1);
	outJpc->AxisX1 = to_jpc(inJph->mAxisX1);
	outJpc->AxisY1 = to_jpc(inJph->mAxisY1);
	outJpc->Point2 = to_jpc(inJph->mPoint2);
	outJpc->AxisX2 = to_jpc(inJph->mAxisX2);
	outJpc->AxisY2 = to_jpc(inJph->mAxisY2);
}

JPC56_IMPL void JPC56_FixedConstraintSettings_to_jph(
	const JPC56_FixedConstraintSettings* inJpc,
	JPH56::FixedConstraintSettings* outJph)
{
	JPC56_ConstraintSettings_to_jph(&inJpc->ConstraintSettings, outJph);

	outJph->mSpace = static_cast<JPH56::EConstraintSpace>(inJpc->Space);
	outJph->mAutoDetectPoint = inJpc->AutoDetectPoint;
	outJph->mPoint1 = to_jph(inJpc->Point1);
	outJph->mAxisX1 = to_jph(inJpc->AxisX1);
	outJph->mAxisY1 = to_jph(inJpc->AxisY1);
	outJph->mPoint2 = to_jph(inJpc->Point2);
	outJph->mAxisX2 = to_jph(inJpc->AxisX2);
	outJph->mAxisY2 = to_jph(inJpc->AxisY2);
}

JPC56_API void JPC56_FixedConstraintSettings_default(JPC56_FixedConstraintSettings* settings) {
	JPH56::FixedConstraintSettings defaultSettings{};
	JPC56_FixedConstraintSettings_to_jpc(settings, &defaultSettings);
}

JPC56_API JPC56_Constraint* JPC56_FixedConstraintSettings_Create(
	const JPC56_FixedConstraintSettings* self,
	JPC56_Body* inBody1,
	JPC56_Body* inBody2)
{
	JPH56::FixedConstraintSettings jphSettings;
	JPC56_FixedConstraintSettings_to_jph(self, &jphSettings);

	JPH56::FixedConstraint* outJph = new JPH56::FixedConstraint(*to_jph(inBody1), *to_jph(inBody2), jphSettings);
	return (JPC56_Constraint*)outJph;
}

////////////////////////////////////////////////////////////////////////////////
// SixDOFConstraintSettings -> TwoBodyConstraintSettings -> ConstraintSettings

JPC56_IMPL void JPC56_SixDOFConstraintSettings_to_jpc(
	JPC56_SixDOFConstraintSettings* outJpc,
	const JPH56::SixDOFConstraintSettings* inJph)
{
	JPC56_ConstraintSettings_to_jpc(&outJpc->ConstraintSettings, inJph);

	outJpc->Space = static_cast<JPC56_ConstraintSpace>(inJph->mSpace);
	outJpc->Position1 = to_jpc(inJph->mPosition1);
	outJpc->AxisX1 = to_jpc(inJph->mAxisX1);
	outJpc->AxisY1 = to_jpc(inJph->mAxisY1);
	outJpc->Position2 = to_jpc(inJph->mPosition2);
	outJpc->AxisX2 = to_jpc(inJph->mAxisX2);
	outJpc->AxisY2 = to_jpc(inJph->mAxisY2);
	std::copy(inJph->mMaxFriction, inJph->mMaxFriction + 6, outJpc->MaxFriction);
	std::copy(inJph->mLimitMin, inJph->mLimitMin + 6, outJpc->LimitMin);
	std::copy(inJph->mLimitMax, inJph->mLimitMax + 6, outJpc->LimitMax);

	// TODO: LimitsSpringSettings
}

JPC56_IMPL void JPC56_SixDOFConstraintSettings_to_jph(
	const JPC56_SixDOFConstraintSettings* inJpc,
	JPH56::SixDOFConstraintSettings* outJph)
{
	JPC56_ConstraintSettings_to_jph(&inJpc->ConstraintSettings, outJph);

	outJph->mSpace = static_cast<JPH56::EConstraintSpace>(inJpc->Space);
	outJph->mPosition1 = to_jph(inJpc->Position1);
	outJph->mAxisX1 = to_jph(inJpc->AxisX1);
	outJph->mAxisY1 = to_jph(inJpc->AxisY1);
	outJph->mPosition2 = to_jph(inJpc->Position2);
	outJph->mAxisX2 = to_jph(inJpc->AxisX2);
	outJph->mAxisY2 = to_jph(inJpc->AxisY2);
	std::copy(inJpc->MaxFriction, inJpc->MaxFriction + 6, outJph->mMaxFriction);
	std::copy(inJpc->LimitMin, inJpc->LimitMin + 6, outJph->mLimitMin);
	std::copy(inJpc->LimitMax, inJpc->LimitMax + 6, outJph->mLimitMax);

	// TODO: LimitsSpringSettings
}

JPC56_API void JPC56_SixDOFConstraintSettings_default(JPC56_SixDOFConstraintSettings* settings) {
	JPH56::SixDOFConstraintSettings defaultSettings{};
	JPC56_SixDOFConstraintSettings_to_jpc(settings, &defaultSettings);
}

JPC56_API JPC56_Constraint* JPC56_SixDOFConstraintSettings_Create(
	const JPC56_SixDOFConstraintSettings* self,
	JPC56_Body* inBody1,
	JPC56_Body* inBody2)
{
	JPH56::SixDOFConstraintSettings jphSettings;
	JPC56_SixDOFConstraintSettings_to_jph(self, &jphSettings);

	JPH56::SixDOFConstraint* outJph = new JPH56::SixDOFConstraint(*to_jph(inBody1), *to_jph(inBody2), jphSettings);
	return (JPC56_Constraint*)outJph;
}

////////////////////////////////////////////////////////////////////////////////
// HingeConstraintSettings -> TwoBodyConstraintSettings -> ConstraintSettings

JPC56_IMPL void JPC56_HingeConstraintSettings_to_jpc(
	JPC56_HingeConstraintSettings* outJpc,
	const JPH56::HingeConstraintSettings* inJph)
{
	JPC56_ConstraintSettings_to_jpc(&outJpc->ConstraintSettings, inJph);

	outJpc->Space = static_cast<JPC56_ConstraintSpace>(inJph->mSpace);
	outJpc->Point1 = to_jpc(inJph->mPoint1);
	outJpc->HingeAxis1 = to_jpc(inJph->mHingeAxis1);
	outJpc->NormalAxis1 = to_jpc(inJph->mNormalAxis1);
	outJpc->Point2 = to_jpc(inJph->mPoint2);
	outJpc->HingeAxis2 = to_jpc(inJph->mHingeAxis2);
	outJpc->NormalAxis2 = to_jpc(inJph->mNormalAxis2);
	outJpc->LimitsMin = inJph->mLimitsMin;
	outJpc->LimitsMax = inJph->mLimitsMax;
	JPC56_SpringSettings_to_jpc(&outJpc->LimitsSpringSettings, &inJph->mLimitsSpringSettings);
	outJpc->MaxFrictionTorque = inJph->mMaxFrictionTorque;
	JPC56_MotorSettings_to_jpc(&outJpc->MotorSettings, &inJph->mMotorSettings);
}

JPC56_IMPL void JPC56_HingeConstraintSettings_to_jph(
	const JPC56_HingeConstraintSettings* inJpc,
	JPH56::HingeConstraintSettings* outJph)
{
	JPC56_ConstraintSettings_to_jph(&inJpc->ConstraintSettings, outJph);

	outJph->mSpace = static_cast<JPH56::EConstraintSpace>(inJpc->Space);
	outJph->mPoint1 = to_jph(inJpc->Point1);
	outJph->mHingeAxis1 = to_jph(inJpc->HingeAxis1);
	outJph->mNormalAxis1 = to_jph(inJpc->NormalAxis1);
	outJph->mPoint2 = to_jph(inJpc->Point2);
	outJph->mHingeAxis2 = to_jph(inJpc->HingeAxis2);
	outJph->mNormalAxis2 = to_jph(inJpc->NormalAxis2);
	outJph->mLimitsMin = inJpc->LimitsMin;
	outJph->mLimitsMax = inJpc->LimitsMax;
	JPC56_SpringSettings_to_jph(&inJpc->LimitsSpringSettings, &outJph->mLimitsSpringSettings);
	outJph->mMaxFrictionTorque = inJpc->MaxFrictionTorque;
	JPC56_MotorSettings_to_jph(&inJpc->MotorSettings, &outJph->mMotorSettings);
}

JPC56_API void JPC56_HingeConstraintSettings_default(JPC56_HingeConstraintSettings* settings) {
	JPH56::HingeConstraintSettings defaultSettings{};
	JPC56_HingeConstraintSettings_to_jpc(settings, &defaultSettings);
}

JPC56_API JPC56_HingeConstraint* JPC56_HingeConstraintSettings_Create(
	const JPC56_HingeConstraintSettings* self,
	JPC56_Body* inBody1,
	JPC56_Body* inBody2)
{
	JPH56::HingeConstraintSettings jphSettings;
	JPC56_HingeConstraintSettings_to_jph(self, &jphSettings);

	JPH56::HingeConstraint* outJph = new JPH56::HingeConstraint(*to_jph(inBody1), *to_jph(inBody2), jphSettings);
	return (JPC56_HingeConstraint*)outJph;
}

////////////////////////////////////////////////////////////////////////////////
// DistanceConstraintSettings -> TwoBodyConstraintSettings -> ConstraintSettings

JPC56_IMPL void JPC56_DistanceConstraintSettings_to_jpc(
	JPC56_DistanceConstraintSettings* outJpc,
	const JPH56::DistanceConstraintSettings* inJph)
{
	JPC56_ConstraintSettings_to_jpc(&outJpc->ConstraintSettings, inJph);

	outJpc->Space = static_cast<JPC56_ConstraintSpace>(inJph->mSpace);
	outJpc->Point1 = to_jpc(inJph->mPoint1);
	outJpc->Point2 = to_jpc(inJph->mPoint2);
	outJpc->MinDistance = inJph->mMinDistance;
	outJpc->MaxDistance = inJph->mMaxDistance;
	// TODO: Spring settings
}

JPC56_IMPL void JPC56_DistanceConstraintSettings_to_jph(
	const JPC56_DistanceConstraintSettings* inJpc,
	JPH56::DistanceConstraintSettings* outJph)
{
	JPC56_ConstraintSettings_to_jph(&inJpc->ConstraintSettings, outJph);

	outJph->mSpace = static_cast<JPH56::EConstraintSpace>(inJpc->Space);
	outJph->mPoint1 = to_jph(inJpc->Point1);
	outJph->mPoint2 = to_jph(inJpc->Point2);
	outJph->mMinDistance = inJpc->MinDistance;
	outJph->mMaxDistance = inJpc->MaxDistance;
	// TODO: Spring settings
}

JPC56_API void JPC56_DistanceConstraintSettings_default(JPC56_DistanceConstraintSettings* settings) {
	JPH56::DistanceConstraintSettings defaultSettings{};
	JPC56_DistanceConstraintSettings_to_jpc(settings, &defaultSettings);
}

JPC56_API JPC56_DistanceConstraint* JPC56_DistanceConstraintSettings_Create(
	const JPC56_DistanceConstraintSettings* self,
	JPC56_Body* inBody1,
	JPC56_Body* inBody2)
	{
		JPH56::DistanceConstraintSettings jphSettings;
		JPC56_DistanceConstraintSettings_to_jph(self, &jphSettings);

		JPH56::DistanceConstraint* outJph = new JPH56::DistanceConstraint(*to_jph(inBody1), *to_jph(inBody2), jphSettings);
		return (JPC56_DistanceConstraint*)outJph;
	}

////////////////////////////////////////////////////////////////////////////////
// SliderConstraintSettings -> TwoBodyConstraintSettings -> ConstraintSettings

JPC56_IMPL void JPC56_SliderConstraintSettings_to_jpc(
	JPC56_SliderConstraintSettings* outJpc,
	const JPH56::SliderConstraintSettings* inJph)
{
	JPC56_ConstraintSettings_to_jpc(&outJpc->ConstraintSettings, inJph);

	outJpc->Space = static_cast<JPC56_ConstraintSpace>(inJph->mSpace);
	outJpc->AutoDetectPoint = inJph->mAutoDetectPoint;
	outJpc->Point1 = to_jpc(inJph->mPoint1);
	outJpc->SliderAxis1 = to_jpc(inJph->mSliderAxis1);
	outJpc->NormalAxis1 = to_jpc(inJph->mNormalAxis1);
	outJpc->Point2 = to_jpc(inJph->mPoint2);
	outJpc->SliderAxis2 = to_jpc(inJph->mSliderAxis2);
	outJpc->NormalAxis2 = to_jpc(inJph->mNormalAxis2);
	outJpc->LimitsMin = inJph->mLimitsMin;
	outJpc->LimitsMax = inJph->mLimitsMax;
	JPC56_SpringSettings_to_jpc(&outJpc->LimitsSpringSettings, &inJph->mLimitsSpringSettings);
	outJpc->MaxFrictionForce = inJph->mMaxFrictionForce;
	JPC56_MotorSettings_to_jpc(&outJpc->MotorSettings, &inJph->mMotorSettings);
}

JPC56_IMPL void JPC56_SliderConstraintSettings_to_jph(
	const JPC56_SliderConstraintSettings* inJpc,
	JPH56::SliderConstraintSettings* outJph)
{
	JPC56_ConstraintSettings_to_jph(&inJpc->ConstraintSettings, outJph);

	outJph->mSpace = static_cast<JPH56::EConstraintSpace>(inJpc->Space);
	outJph->mAutoDetectPoint = inJpc->AutoDetectPoint;
	outJph->mPoint1 = to_jph(inJpc->Point1);
	outJph->mSliderAxis1 = to_jph(inJpc->SliderAxis1);
	outJph->mNormalAxis1 = to_jph(inJpc->NormalAxis1);
	outJph->mPoint2 = to_jph(inJpc->Point2);
	outJph->mSliderAxis2 = to_jph(inJpc->SliderAxis2);
	outJph->mNormalAxis2 = to_jph(inJpc->NormalAxis2);
	outJph->mLimitsMin = inJpc->LimitsMin;
	outJph->mLimitsMax = inJpc->LimitsMax;
	JPC56_SpringSettings_to_jph(&inJpc->LimitsSpringSettings, &outJph->mLimitsSpringSettings);
	outJph->mMaxFrictionForce = inJpc->MaxFrictionForce;
	JPC56_MotorSettings_to_jph(&inJpc->MotorSettings, &outJph->mMotorSettings);
}

JPC56_API void JPC56_SliderConstraintSettings_default(JPC56_SliderConstraintSettings* settings) {
	JPH56::SliderConstraintSettings defaultSettings{};
	JPC56_SliderConstraintSettings_to_jpc(settings, &defaultSettings);
}

JPC56_API JPC56_SliderConstraint* JPC56_SliderConstraintSettings_Create(
	const JPC56_SliderConstraintSettings* self,
	JPC56_Body* inBody1,
	JPC56_Body* inBody2)
{
	JPH56::SliderConstraintSettings jphSettings;
	JPC56_SliderConstraintSettings_to_jph(self, &jphSettings);

	JPH56::SliderConstraint* outJph = new JPH56::SliderConstraint(*to_jph(inBody1), *to_jph(inBody2), jphSettings);
	return (JPC56_SliderConstraint*)outJph;
}

////////////////////////////////////////////////////////////////////////////////
// Shape -> RefTarget<Shape>

// RefTarget<Shape>
JPC56_API uint32_t JPC56_Shape_GetRefCount(const JPC56_Shape* self) {
	return to_jph(self)->GetRefCount();
}

JPC56_API void JPC56_Shape_AddRef(const JPC56_Shape* self) {
	to_jph(self)->AddRef();
}

JPC56_API void JPC56_Shape_Release(const JPC56_Shape* self) {
	to_jph(self)->Release();
}

// Shape
JPC56_API uint64_t JPC56_Shape_GetUserData(const JPC56_Shape* self) {
	return to_jph(self)->GetUserData();
}

JPC56_API void JPC56_Shape_SetUserData(JPC56_Shape* self, uint64_t userData) {
	to_jph(self)->SetUserData(userData);
}

JPC56_API JPC56_ShapeType JPC56_Shape_GetType(const JPC56_Shape* self) {
	return to_jpc(to_jph(self)->GetType());
}

JPC56_API JPC56_ShapeSubType JPC56_Shape_GetSubType(const JPC56_Shape* self) {
	return to_jpc(to_jph(self)->GetSubType());
}

JPC56_API uint64_t JPC56_Shape_GetSubShapeUserData(const JPC56_Shape* self, JPC56_SubShapeID inSubShapeID) {
	return to_jph(self)->GetSubShapeUserData(JPC56_SubShapeID_to_jph(inSubShapeID));
}

JPC56_API JPC56_Vec3 JPC56_Shape_GetCenterOfMass(const JPC56_Shape* self) {
	return to_jpc(to_jph(self)->GetCenterOfMass());
}

JPC56_API float JPC56_Shape_GetVolume(const JPC56_Shape* self) {
	return to_jph(self)->GetVolume();
}

////////////////////////////////////////////////////////////////////////////////
// CompoundShape

JPC56_API const JPC56_Shape* JPC56_CompoundShape_GetSubShape_Shape(
	const JPC56_CompoundShape* self,
	uint inIdx)
{
	return to_jpc(to_jph(self)->GetSubShape(inIdx).mShape.GetPtr());
}

JPC56_API uint32_t JPC56_CompoundShape_GetSubShapeIndexFromID(
	const JPC56_CompoundShape* self,
	JPC56_SubShapeID inSubShapeID,
	JPC56_SubShapeID* outRemainder)
{
	JPH56::SubShapeID jphRemainder;
	uint32_t res = to_jph(self)->GetSubShapeIndexFromID(JPC56_SubShapeID_to_jph(inSubShapeID), jphRemainder);
	*outRemainder = to_jpc(jphRemainder);
	return res;
}

////////////////////////////////////////////////////////////////////////////////
// ShapeSettings

// Unpack a ShapeResult into a bool and two pointers to be friendlier to C.
static bool HandleShapeResult(JPH56::ShapeSettings::ShapeResult res, JPC56_Shape** outShape, JPC56_String** outError) {
	if (res.HasError()) {
		if (outError != nullptr) {
			JPH56::String* created = new JPH56::String(std::move(res.GetError()));
			*outError = to_jpc(created);
		}

		return false;
	} else {
		JPH56::Ref<JPH56::Shape> shape = res.Get();
		shape->AddRef();
		*outShape = to_jpc((JPH56::Shape*)shape);

		return true;
	}
}

////////////////////////////////////////////////////////////////////////////////
// TriangleShapeSettings

static void to_jph(const JPC56_TriangleShapeSettings* input, JPH56::TriangleShapeSettings* output) {
	output->mUserData = input->UserData;

	// TODO: Material
	output->mDensity = input->Density;

	output->mV1 = to_jph(input->V1);
	output->mV2 = to_jph(input->V2);
	output->mV3 = to_jph(input->V3);
	output->mConvexRadius = input->ConvexRadius;
}

JPC56_API void JPC56_TriangleShapeSettings_default(JPC56_TriangleShapeSettings* object) {
	object->UserData = 0;

	// TODO: Material
	object->Density = 1000.0;

	object->V1 = {0};
	object->V2 = {0};
	object->V3 = {0};
	object->ConvexRadius = 0.0;
}

JPC56_API bool JPC56_TriangleShapeSettings_Create(const JPC56_TriangleShapeSettings* self, JPC56_Shape** outShape, JPC56_String** outError) {
	JPH56::TriangleShapeSettings settings;
	to_jph(self, &settings);

	return HandleShapeResult(settings.Create(), outShape, outError);
}

////////////////////////////////////////////////////////////////////////////////
// MeshShapeSettings

JPC56_IMPL void JPC56_MeshShapeSettings_to_jpc_borrowed(
	JPC56_MeshShapeSettings* outJpc,
	const JPH56::MeshShapeSettings* inJph)
{
	outJpc->UserData = inJph->mUserData;

	outJpc->TriangleVertices = (JPC56_Float3*)inJph->mTriangleVertices.data();
	outJpc->TriangleVerticesLen = inJph->mTriangleVertices.size();
	outJpc->IndexedTriangles = (JPC56_IndexedTriangle*)inJph->mIndexedTriangles.data();
	outJpc->IndexedTrianglesLen = inJph->mIndexedTriangles.size();
}

JPC56_IMPL void JPC56_MeshShapeSettings_to_jph(
	const JPC56_MeshShapeSettings* inJpc,
	JPH56::MeshShapeSettings* outJph)
{
	outJph->mUserData = inJpc->UserData;

	auto triangleVertices = (const JPH56::Float3*)inJpc->TriangleVertices;
	outJph->mTriangleVertices = JPH56::VertexList(triangleVertices, triangleVertices + inJpc->TriangleVerticesLen);

	auto indexedTriangles = (const JPH56::IndexedTriangle*)inJpc->IndexedTriangles;
	outJph->mIndexedTriangles = JPH56::IndexedTriangleList(indexedTriangles, indexedTriangles + inJpc->IndexedTrianglesLen);
}

JPC56_API void JPC56_MeshShapeSettings_default(JPC56_MeshShapeSettings* object) {
	JPH56::MeshShapeSettings settings;
	JPC56_MeshShapeSettings_to_jpc_borrowed(object, &settings);

	// Overwrite all pointers and lengths so that the default value doesn't
	// contain pointers to freed memory.
	object->TriangleVertices = nullptr;
	object->TriangleVerticesLen = 0;
	object->IndexedTriangles = nullptr;
	object->IndexedTrianglesLen = 0;
}

JPC56_API bool JPC56_MeshShapeSettings_Create(const JPC56_MeshShapeSettings* self, JPC56_Shape** outShape, JPC56_String** outError) {
	JPH56::MeshShapeSettings settings;
	JPC56_MeshShapeSettings_to_jph(self, &settings);

	// MeshShapeSettings calls Sanitize in its default constructor, but we don't
	// have constructors in C. It's probably fine to always Sanitize.
	settings.Sanitize();

	return HandleShapeResult(settings.Create(), outShape, outError);
}

////////////////////////////////////////////////////////////////////////////////
// BoxShapeSettings

static void to_jph(const JPC56_BoxShapeSettings* input, JPH56::BoxShapeSettings* output) {
	output->mUserData = input->UserData;

	// TODO: Material
	output->mDensity = input->Density;

	output->mHalfExtent = to_jph(input->HalfExtent);
	output->mConvexRadius = input->ConvexRadius;
}

JPC56_API void JPC56_BoxShapeSettings_default(JPC56_BoxShapeSettings* object) {
	object->UserData = 0;

	// TODO: Material
	object->Density = 1000.0;

	object->HalfExtent = JPC56_Vec3{0};
	object->ConvexRadius = 0.0;
}

JPC56_API bool JPC56_BoxShapeSettings_Create(const JPC56_BoxShapeSettings* self, JPC56_Shape** outShape, JPC56_String** outError) {
	JPH56::BoxShapeSettings settings;
	to_jph(self, &settings);

	return HandleShapeResult(settings.Create(), outShape, outError);
}

////////////////////////////////////////////////////////////////////////////////
// SphereShapeSettings

static void to_jph(const JPC56_SphereShapeSettings* input, JPH56::SphereShapeSettings* output) {
	output->mUserData = input->UserData;

	// TODO: Material
	output->mDensity = input->Density;

	output->mRadius = input->Radius;
}

JPC56_API void JPC56_SphereShapeSettings_default(JPC56_SphereShapeSettings* object) {
	object->UserData = 0;

	// TODO: Material
	object->Density = 1000.0;

	object->Radius = 0.0;
}

JPC56_API bool JPC56_SphereShapeSettings_Create(const JPC56_SphereShapeSettings* self, JPC56_Shape** outShape, JPC56_String** outError) {
	JPH56::SphereShapeSettings settings;
	to_jph(self, &settings);

	return HandleShapeResult(settings.Create(), outShape, outError);
}

////////////////////////////////////////////////////////////////////////////////
// CapsuleShapeSettings

static void to_jph(const JPC56_CapsuleShapeSettings* input, JPH56::CapsuleShapeSettings* output) {
	output->mUserData = input->UserData;

	// TODO: Material
	output->mDensity = input->Density;

	output->mRadius = input->Radius;
	output->mHalfHeightOfCylinder = input->HalfHeightOfCylinder;
}

JPC56_API void JPC56_CapsuleShapeSettings_default(JPC56_CapsuleShapeSettings* object) {
	object->UserData = 0;

	// TODO: Material
	object->Density = 1000.0;

	object->Radius = 0.0;
	object->HalfHeightOfCylinder = 0.0;
}

JPC56_API bool JPC56_CapsuleShapeSettings_Create(const JPC56_CapsuleShapeSettings* self, JPC56_Shape** outShape, JPC56_String** outError) {
	JPH56::CapsuleShapeSettings settings;
	to_jph(self, &settings);

	return HandleShapeResult(settings.Create(), outShape, outError);
}

////////////////////////////////////////////////////////////////////////////////
// CylinderShapeSettings

static void to_jph(const JPC56_CylinderShapeSettings* input, JPH56::CylinderShapeSettings* output) {
	output->mUserData = input->UserData;

	// TODO: Material
	output->mDensity = input->Density;

	output->mHalfHeight = input->HalfHeight;
	output->mRadius = input->Radius;
	output->mConvexRadius = input->ConvexRadius;
}

JPC56_API void JPC56_CylinderShapeSettings_default(JPC56_CylinderShapeSettings* object) {
	object->UserData = 0;

	// TODO: Material
	object->Density = 1000.0;

	object->HalfHeight = 0.0;
	object->Radius = 0.0;
	object->ConvexRadius = 0.0;
}

JPC56_API bool JPC56_CylinderShapeSettings_Create(const JPC56_CylinderShapeSettings* self, JPC56_Shape** outShape, JPC56_String** outError) {
	JPH56::CylinderShapeSettings settings;
	to_jph(self, &settings);

	return HandleShapeResult(settings.Create(), outShape, outError);
}

////////////////////////////////////////////////////////////////////////////////
// PlaneShapeSettings

static void to_jph(const JPC56_PlaneShapeSettings* input, JPH56::PlaneShapeSettings* output) {
	output->mUserData = input->UserData;

	// TODO: Material
	output->mPlane = JPH56::Plane(to_jph(input->Normal), input->Constant);
	output->mHalfExtent = input->HalfExtent;
}

JPC56_API void JPC56_PlaneShapeSettings_default(JPC56_PlaneShapeSettings* object) {
	object->UserData = 0;

	// TODO: Material
	object->Normal = JPC56_Vec3{0, 1, 0, 1};
	object->Constant = 0.0;
	object->HalfExtent = JPH56::PlaneShapeSettings::cDefaultHalfExtent;
}

JPC56_API bool JPC56_PlaneShapeSettings_Create(const JPC56_PlaneShapeSettings* self, JPC56_Shape** outShape, JPC56_String** outError) {
	JPH56::PlaneShapeSettings settings;
	to_jph(self, &settings);

	return HandleShapeResult(settings.Create(), outShape, outError);
}

////////////////////////////////////////////////////////////////////////////////
// ConvexHullShapeSettings

static void to_jph(const JPC56_ConvexHullShapeSettings* input, JPH56::ConvexHullShapeSettings* output) {
	output->mUserData = input->UserData;

	// TODO: Material
	output->mDensity = input->Density;

	output->mPoints = to_jph(input->Points, input->PointsLen);
	output->mMaxConvexRadius = input->MaxConvexRadius;
	output->mMaxErrorConvexRadius = input->MaxErrorConvexRadius;
	output->mHullTolerance = input->HullTolerance;
}

JPC56_API void JPC56_ConvexHullShapeSettings_default(JPC56_ConvexHullShapeSettings* object) {
	object->UserData = 0;

	// TODO: Material
	object->Density = 1000.0;

	object->Points = nullptr;
	object->PointsLen = 0;
	object->MaxConvexRadius = 0.0;
	object->MaxErrorConvexRadius = 0.05f;
	object->HullTolerance = 1.0e-3f;
}

JPC56_API bool JPC56_ConvexHullShapeSettings_Create(const JPC56_ConvexHullShapeSettings* self, JPC56_Shape** outShape, JPC56_String** outError) {
	JPH56::ConvexHullShapeSettings settings;
	to_jph(self, &settings);

	return HandleShapeResult(settings.Create(), outShape, outError);
}

////////////////////////////////////////////////////////////////////////////////
// CompoundShape::SubShapeSettings

static JPH56::CompoundShapeSettings::SubShapeSettings to_jph(const JPC56_SubShapeSettings* input) {
	const JPH56::Shape* shape = to_jph(input->Shape);

	JPH56::CompoundShapeSettings::SubShapeSettings output;
	output.mShape = nullptr;
	output.mShapePtr = shape;
	output.mPosition = to_jph(input->Position);
	output.mRotation = to_jph(input->Rotation);
	output.mUserData = input->UserData;
	return output;
}

static JPH56::Array<JPH56::CompoundShapeSettings::SubShapeSettings> to_jph(const JPC56_SubShapeSettings* src, size_t n) {
	JPH56::Array<JPH56::CompoundShapeSettings::SubShapeSettings> vec;
	vec.reserve(n);

	for (size_t i = 0; i < n; i++) {
		vec.push_back(to_jph(&src[i]));
	}

	return vec;
}

JPC56_API void JPC56_SubShapeSettings_default(JPC56_SubShapeSettings* object) {
	object->Shape = nullptr;
	object->Position = JPC56_Vec3{0};
	object->Rotation = JPC56_Quat{0, 0, 0, 1};
	object->UserData = 0;
}

////////////////////////////////////////////////////////////////////////////////
// StaticCompoundShapeSettings -> CompoundShapeSettings -> ShapeSettings

static void to_jph(const JPC56_StaticCompoundShapeSettings* input, JPH56::StaticCompoundShapeSettings* output) {
	output->mUserData = input->UserData;

	output->mSubShapes = to_jph(input->SubShapes, input->SubShapesLen);
}

JPC56_API void JPC56_StaticCompoundShapeSettings_default(JPC56_StaticCompoundShapeSettings* object) {
	object->UserData = 0;

	object->SubShapes = nullptr;
	object->SubShapesLen = 0;
}

JPC56_API bool JPC56_StaticCompoundShapeSettings_Create(const JPC56_StaticCompoundShapeSettings* self, JPC56_Shape** outShape, JPC56_String** outError) {
	JPH56::StaticCompoundShapeSettings settings;
	to_jph(self, &settings);

	return HandleShapeResult(settings.Create(), outShape, outError);
}

////////////////////////////////////////////////////////////////////////////////
// MutableCompoundShape -> CompoundShape -> Shape

JPC56_IMPL JPH56::MutableCompoundShape* JPC56_MutableCompoundShape_to_jph(JPC56_MutableCompoundShape* self) {
	return reinterpret_cast<JPH56::MutableCompoundShape*>(self);
}

JPC56_API uint JPC56_MutableCompoundShape_AddShape(
	JPC56_MutableCompoundShape* self,
	JPC56_Vec3 inPosition,
	JPC56_Quat inRotation,
	const JPC56_Shape* inShape,
	uint32_t inUserData)
{
	JPH56::MutableCompoundShape* self_jph = JPC56_MutableCompoundShape_to_jph(self);

	return self_jph->AddShape(to_jph(inPosition), to_jph(inRotation), to_jph(inShape), inUserData);
}

JPC56_API void JPC56_MutableCompoundShape_RemoveShape(JPC56_MutableCompoundShape* self, uint inIndex) {
	JPH56::MutableCompoundShape* self_jph = JPC56_MutableCompoundShape_to_jph(self);

	self_jph->RemoveShape(inIndex);
}

JPC56_API void JPC56_MutableCompoundShape_ModifyShape(JPC56_MutableCompoundShape* self, uint inIndex, JPC56_Vec3 inPosition, JPC56_Quat inRotation) {
	JPH56::MutableCompoundShape* self_jph = JPC56_MutableCompoundShape_to_jph(self);

	self_jph->ModifyShape(inIndex, to_jph(inPosition), to_jph(inRotation));
}

JPC56_API void JPC56_MutableCompoundShape_ModifyShape2(JPC56_MutableCompoundShape* self, uint inIndex, JPC56_Vec3 inPosition, JPC56_Quat inRotation, const JPC56_Shape* inShape) {
	JPH56::MutableCompoundShape* self_jph = JPC56_MutableCompoundShape_to_jph(self);

	self_jph->ModifyShape(inIndex, to_jph(inPosition), to_jph(inRotation), to_jph(inShape));
}

JPC56_API void JPC56_MutableCompoundShape_AdjustCenterOfMass(JPC56_MutableCompoundShape* self) {
	JPH56::MutableCompoundShape* self_jph = JPC56_MutableCompoundShape_to_jph(self);

	self_jph->AdjustCenterOfMass();
}

////////////////////////////////////////////////////////////////////////////////
// MutableCompoundShapeSettings -> CompoundShapeSettings -> ShapeSettings

static void to_jph(const JPC56_MutableCompoundShapeSettings* input, JPH56::MutableCompoundShapeSettings* output) {
	output->mUserData = input->UserData;

	output->mSubShapes = to_jph(input->SubShapes, input->SubShapesLen);
}

JPC56_API void JPC56_MutableCompoundShapeSettings_default(JPC56_MutableCompoundShapeSettings* object) {
	object->UserData = 0;

	object->SubShapes = nullptr;
	object->SubShapesLen = 0;
}

JPC56_API bool JPC56_MutableCompoundShapeSettings_Create(const JPC56_MutableCompoundShapeSettings* self, JPC56_MutableCompoundShape** outShape, JPC56_String** outError) {
	JPH56::MutableCompoundShapeSettings settings;
	to_jph(self, &settings);

	return HandleShapeResult(settings.Create(), (JPC56_Shape**)outShape, outError);
}

////////////////////////////////////////////////////////////////////////////////
// BodyCreationSettings

static JPH56::BodyCreationSettings to_jph(const JPC56_BodyCreationSettings* settings) {
	JPH56::BodyCreationSettings output{};

	output.mPosition = to_jph(settings->Position);
	output.mRotation = to_jph(settings->Rotation);
	output.mLinearVelocity = to_jph(settings->LinearVelocity);
	output.mAngularVelocity = to_jph(settings->AngularVelocity);
	output.mUserData = settings->UserData;
	output.mObjectLayer = settings->ObjectLayer;
	// CollisionGroup
	output.mMotionType = to_jph(settings->MotionType);
	output.mAllowedDOFs = to_jph(settings->AllowedDOFs);
	output.mAllowDynamicOrKinematic = settings->AllowDynamicOrKinematic;
	output.mIsSensor = settings->IsSensor;
	output.mCollideKinematicVsNonDynamic = settings->CollideKinematicVsNonDynamic;
	output.mUseManifoldReduction = settings->UseManifoldReduction;
	output.mApplyGyroscopicForce = settings->ApplyGyroscopicForce;
	output.mMotionQuality = to_jph(settings->MotionQuality);
	output.mEnhancedInternalEdgeRemoval = settings->EnhancedInternalEdgeRemoval;
	output.mAllowSleeping = settings->AllowSleeping;
	output.mFriction = settings->Friction;
	output.mRestitution = settings->Restitution;
	output.mLinearDamping = settings->LinearDamping;
	output.mAngularDamping = settings->AngularDamping;
	output.mMaxLinearVelocity = settings->MaxLinearVelocity;
	output.mMaxAngularVelocity = settings->MaxAngularVelocity;
	output.mGravityFactor = settings->GravityFactor;
	output.mNumVelocityStepsOverride = settings->NumVelocityStepsOverride;
	output.mNumPositionStepsOverride = settings->NumPositionStepsOverride;
	output.mOverrideMassProperties = to_jph(settings->OverrideMassProperties);
	output.mInertiaMultiplier = settings->InertiaMultiplier;
	// output.mMassPropertiesOverride = settings->MassPropertiesOverride;
	output.SetShape(to_jph(settings->Shape));

	return output;
}

JPC56_API void JPC56_BodyCreationSettings_default(JPC56_BodyCreationSettings* settings) {
	JPH56::BodyCreationSettings defaultSettings{};

	settings->Position = to_jpc(defaultSettings.mPosition);
	settings->Rotation = to_jpc(defaultSettings.mRotation);
	settings->LinearVelocity = to_jpc(defaultSettings.mLinearVelocity);
	settings->AngularVelocity = to_jpc(defaultSettings.mAngularVelocity);
	settings->UserData = defaultSettings.mUserData;
	settings->ObjectLayer = defaultSettings.mObjectLayer;
	// CollisionGroup
	settings->MotionType = to_jpc(defaultSettings.mMotionType);
	settings->AllowedDOFs = to_jpc(defaultSettings.mAllowedDOFs);
	settings->AllowDynamicOrKinematic = defaultSettings.mAllowDynamicOrKinematic;
	settings->IsSensor = defaultSettings.mIsSensor;
	settings->CollideKinematicVsNonDynamic = defaultSettings.mCollideKinematicVsNonDynamic;
	settings->UseManifoldReduction = defaultSettings.mUseManifoldReduction;
	settings->ApplyGyroscopicForce = defaultSettings.mApplyGyroscopicForce;
	settings->MotionQuality = to_jpc(defaultSettings.mMotionQuality);
	settings->EnhancedInternalEdgeRemoval = defaultSettings.mEnhancedInternalEdgeRemoval;
	settings->AllowSleeping = defaultSettings.mAllowSleeping;
	settings->Friction = defaultSettings.mFriction;
	settings->Restitution = defaultSettings.mRestitution;
	settings->LinearDamping = defaultSettings.mLinearDamping;
	settings->AngularDamping = defaultSettings.mAngularDamping;
	settings->MaxLinearVelocity = defaultSettings.mMaxLinearVelocity;
	settings->MaxAngularVelocity = defaultSettings.mMaxAngularVelocity;
	settings->GravityFactor = defaultSettings.mGravityFactor;
	settings->NumVelocityStepsOverride = defaultSettings.mNumVelocityStepsOverride;
	settings->NumPositionStepsOverride = defaultSettings.mNumPositionStepsOverride;
	settings->OverrideMassProperties = to_jpc(defaultSettings.mOverrideMassProperties);
	settings->InertiaMultiplier = defaultSettings.mInertiaMultiplier;
	// MassPropertiesOverride
}

////////////////////////////////////////////////////////////////////////////////
// Body

JPC56_API JPC56_BodyID JPC56_Body_GetID(const JPC56_Body* self) {
	return to_jpc(to_jph(self)->GetID());
}

JPC56_API JPC56_BodyType JPC56_Body_GetBodyType(const JPC56_Body* self) {
	return to_jpc(to_jph(self)->GetBodyType());
}

JPC56_API bool JPC56_Body_IsRigidBody(const JPC56_Body* self) {
	return to_jph(self)->IsRigidBody();
}

JPC56_API bool JPC56_Body_IsSoftBody(const JPC56_Body* self) {
	return to_jph(self)->IsSoftBody();
}

JPC56_API bool JPC56_Body_IsActive(const JPC56_Body* self) {
	return to_jph(self)->IsActive();
}

JPC56_API bool JPC56_Body_IsStatic(const JPC56_Body* self) {
	return to_jph(self)->IsStatic();
}

JPC56_API bool JPC56_Body_IsKinematic(const JPC56_Body* self) {
	return to_jph(self)->IsKinematic();
}

JPC56_API bool JPC56_Body_IsDynamic(const JPC56_Body* self) {
	return to_jph(self)->IsDynamic();
}

JPC56_API bool JPC56_Body_CanBeKinematicOrDynamic(const JPC56_Body* self) {
	return to_jph(self)->CanBeKinematicOrDynamic();
}

JPC56_API void JPC56_Body_SetIsSensor(JPC56_Body* self, bool inIsSensor) {
	to_jph(self)->SetIsSensor(inIsSensor);
}

JPC56_API bool JPC56_Body_IsSensor(const JPC56_Body* self) {
	return to_jph(self)->IsSensor();
}

JPC56_API void JPC56_Body_SetCollideKinematicVsNonDynamic(JPC56_Body* self, bool inCollide) {
	to_jph(self)->SetCollideKinematicVsNonDynamic(inCollide);
}

JPC56_API bool JPC56_Body_GetCollideKinematicVsNonDynamic(const JPC56_Body* self) {
	return to_jph(self)->GetCollideKinematicVsNonDynamic();
}

JPC56_API void JPC56_Body_SetUseManifoldReduction(JPC56_Body* self, bool inUseReduction) {
	to_jph(self)->SetUseManifoldReduction(inUseReduction);
}

JPC56_API bool JPC56_Body_GetUseManifoldReduction(const JPC56_Body* self) {
	return to_jph(self)->GetUseManifoldReduction();
}

JPC56_API bool JPC56_Body_GetUseManifoldReductionWithBody(const JPC56_Body* self, const JPC56_Body* inBody2) {
	return to_jph(self)->GetUseManifoldReductionWithBody(*to_jph(inBody2));
}

JPC56_API void JPC56_Body_SetApplyGyroscopicForce(JPC56_Body* self, bool inApply) {
	to_jph(self)->SetApplyGyroscopicForce(inApply);
}

JPC56_API bool JPC56_Body_GetApplyGyroscopicForce(const JPC56_Body* self) {
	return to_jph(self)->GetApplyGyroscopicForce();
}

JPC56_API void JPC56_Body_SetEnhancedInternalEdgeRemoval(JPC56_Body* self, bool inApply) {
	to_jph(self)->SetEnhancedInternalEdgeRemoval(inApply);
}

JPC56_API bool JPC56_Body_GetEnhancedInternalEdgeRemoval(const JPC56_Body* self) {
	return to_jph(self)->GetEnhancedInternalEdgeRemoval();
}

JPC56_API bool JPC56_Body_GetEnhancedInternalEdgeRemovalWithBody(const JPC56_Body* self, const JPC56_Body* inBody2) {
	return to_jph(self)->GetEnhancedInternalEdgeRemovalWithBody(*to_jph(inBody2));
}

JPC56_API JPC56_MotionType JPC56_Body_GetMotionType(const JPC56_Body* self) {
	return to_jpc(to_jph(self)->GetMotionType());
}

JPC56_API void JPC56_Body_SetMotionType(JPC56_Body* self, JPC56_MotionType inMotionType) {
	to_jph(self)->SetMotionType(to_jph(inMotionType));
}

JPC56_API JPC56_BroadPhaseLayer JPC56_Body_GetBroadPhaseLayer(const JPC56_Body* self) {
	return to_jpc(to_jph(self)->GetBroadPhaseLayer());
}

JPC56_API JPC56_ObjectLayer JPC56_Body_GetObjectLayer(const JPC56_Body* self) {
	return to_jph(self)->GetObjectLayer();
}

// JPC56_API const CollisionGroup & JPC56_Body_GetCollisionGroup(const JPC56_Body* self);
// JPC56_API CollisionGroup & JPC56_Body_GetCollisionGroup(JPC56_Body* self);
// JPC56_API void JPC56_Body_SetCollisionGroup(JPC56_Body* self, const CollisionGroup &inGroup);

JPC56_API bool JPC56_Body_GetAllowSleeping(const JPC56_Body* self) {
	return to_jph(self)->GetAllowSleeping();
}

JPC56_API void JPC56_Body_SetAllowSleeping(JPC56_Body* self, bool inAllow) {
	to_jph(self)->SetAllowSleeping(inAllow);
}

JPC56_API void JPC56_Body_ResetSleepTimer(JPC56_Body* self) {
	to_jph(self)->ResetSleepTimer();
}

JPC56_API float JPC56_Body_GetFriction(const JPC56_Body* self) {
	return to_jph(self)->GetFriction();
}

JPC56_API void JPC56_Body_SetFriction(JPC56_Body* self, float inFriction) {
	to_jph(self)->SetFriction(inFriction);
}

JPC56_API float JPC56_Body_GetRestitution(const JPC56_Body* self) {
	return to_jph(self)->GetRestitution();
}

JPC56_API void JPC56_Body_SetRestitution(JPC56_Body* self, float inRestitution) {
	to_jph(self)->SetRestitution(inRestitution);
}

JPC56_API JPC56_Vec3 JPC56_Body_GetLinearVelocity(const JPC56_Body* self) {
	return to_jpc(to_jph(self)->GetLinearVelocity());
}

JPC56_API void JPC56_Body_SetLinearVelocity(JPC56_Body* self, JPC56_Vec3 inLinearVelocity) {
	to_jph(self)->SetLinearVelocity(to_jph(inLinearVelocity));
}

JPC56_API void JPC56_Body_SetLinearVelocityClamped(JPC56_Body* self, JPC56_Vec3 inLinearVelocity) {
	to_jph(self)->SetLinearVelocityClamped(to_jph(inLinearVelocity));
}

JPC56_API JPC56_Vec3 JPC56_Body_GetAngularVelocity(const JPC56_Body* self) {
	return to_jpc(to_jph(self)->GetAngularVelocity());
}

JPC56_API void JPC56_Body_SetAngularVelocity(JPC56_Body* self, JPC56_Vec3 inAngularVelocity) {
	to_jph(self)->SetAngularVelocity(to_jph(inAngularVelocity));
}

JPC56_API void JPC56_Body_SetAngularVelocityClamped(JPC56_Body* self, JPC56_Vec3 inAngularVelocity) {
	to_jph(self)->SetAngularVelocityClamped(to_jph(inAngularVelocity));
}

JPC56_API JPC56_Vec3 JPC56_Body_GetPointVelocityCOM(const JPC56_Body* self, JPC56_Vec3 inPointRelativeToCOM) {
	return to_jpc(to_jph(self)->GetPointVelocityCOM(to_jph(inPointRelativeToCOM)));
}

JPC56_API JPC56_Vec3 JPC56_Body_GetPointVelocity(const JPC56_Body* self, JPC56_RVec3 inPoint) {
	return to_jpc(to_jph(self)->GetPointVelocity(to_jph(inPoint)));
}

JPC56_API void JPC56_Body_AddForce(JPC56_Body* self, JPC56_Vec3 inForce) {
	to_jph(self)->AddForce(to_jph(inForce));
}

// overload of Body::AddForce
JPC56_API void JPC56_Body_AddForceAtPoint(JPC56_Body* self, JPC56_Vec3 inForce, JPC56_RVec3 inPosition) {
	to_jph(self)->AddForce(to_jph(inForce), to_jph(inPosition));
}

JPC56_API void JPC56_Body_AddTorque(JPC56_Body* self, JPC56_Vec3 inTorque) {
	to_jph(self)->AddTorque(to_jph(inTorque));
}

JPC56_API JPC56_Vec3 JPC56_Body_GetAccumulatedForce(const JPC56_Body* self) {
	return to_jpc(to_jph(self)->GetAccumulatedForce());
}

JPC56_API JPC56_Vec3 JPC56_Body_GetAccumulatedTorque(const JPC56_Body* self) {
	return to_jpc(to_jph(self)->GetAccumulatedTorque());
}

JPC56_API void JPC56_Body_ResetForce(JPC56_Body* self) {
	to_jph(self)->ResetForce();
}

JPC56_API void JPC56_Body_ResetTorque(JPC56_Body* self) {
	to_jph(self)->ResetTorque();
}

JPC56_API void JPC56_Body_ResetMotion(JPC56_Body* self) {
	to_jph(self)->ResetMotion();
}

JPC56_API void JPC56_Body_GetInverseInertia(const JPC56_Body* self, JPC56_Mat44* outMatrix) {
	to_jph(self)->GetInverseInertia().StoreFloat4x4(reinterpret_cast<JPH56::Float4*>(outMatrix));
}

JPC56_API void JPC56_Body_AddImpulse(JPC56_Body* self, JPC56_Vec3 inImpulse) {
	to_jph(self)->AddImpulse(to_jph(inImpulse));
}

JPC56_API void JPC56_Body_AddImpulse2(JPC56_Body* self, JPC56_Vec3 inImpulse, JPC56_RVec3 inPosition) {
	to_jph(self)->AddImpulse(to_jph(inImpulse), to_jph(inPosition));
}

JPC56_API void JPC56_Body_AddAngularImpulse(JPC56_Body* self, JPC56_Vec3 inAngularImpulse) {
	to_jph(self)->AddAngularImpulse(to_jph(inAngularImpulse));
}

JPC56_API void JPC56_Body_MoveKinematic(JPC56_Body* self, JPC56_RVec3 inTargetPosition, JPC56_Quat inTargetRotation, float inDeltaTime) {
	to_jph(self)->MoveKinematic(to_jph(inTargetPosition), to_jph(inTargetRotation), inDeltaTime);
}

JPC56_API bool JPC56_Body_ApplyBuoyancyImpulse(JPC56_Body* self, JPC56_RVec3 inSurfacePosition, JPC56_Vec3 inSurfaceNormal, float inBuoyancy, float inLinearDrag, float inAngularDrag, JPC56_Vec3 inFluidVelocity, JPC56_Vec3 inGravity, float inDeltaTime) {
	return to_jph(self)->ApplyBuoyancyImpulse(to_jph(inSurfacePosition), to_jph(inSurfaceNormal), inBuoyancy, inLinearDrag, inAngularDrag, to_jph(inFluidVelocity), to_jph(inGravity), inDeltaTime);
}

JPC56_API bool JPC56_Body_IsInBroadPhase(const JPC56_Body* self) {
	return to_jph(self)->IsInBroadPhase();
}

JPC56_API bool JPC56_Body_IsCollisionCacheInvalid(const JPC56_Body* self) {
	return to_jph(self)->IsCollisionCacheInvalid();
}

JPC56_API const JPC56_Shape* JPC56_Body_GetShape(const JPC56_Body* self) {
	return to_jpc(to_jph(self)->GetShape());
}

JPC56_API JPC56_RVec3 JPC56_Body_GetPosition(const JPC56_Body* self) {
	return to_jpc(to_jph(self)->GetPosition());
}

JPC56_API JPC56_Quat JPC56_Body_GetRotation(const JPC56_Body* self) {
	return to_jpc(to_jph(self)->GetRotation());
}

JPC56_API JPC56_RMat44 JPC56_Body_GetWorldTransform(const JPC56_Body* self) {
	return to_jpc(to_jph(self)->GetWorldTransform());
}

JPC56_API JPC56_RVec3 JPC56_Body_GetCenterOfMassPosition(const JPC56_Body* self) {
	return to_jpc(to_jph(self)->GetCenterOfMassPosition());
}

JPC56_API JPC56_RMat44 JPC56_Body_GetCenterOfMassTransform(const JPC56_Body* self) {
	return to_jpc(to_jph(self)->GetCenterOfMassTransform());
}

JPC56_API JPC56_RMat44 JPC56_Body_GetInverseCenterOfMassTransform(const JPC56_Body* self) {
	return to_jpc(to_jph(self)->GetInverseCenterOfMassTransform());
}

// JPC56_API const AABox & JPC56_Body_GetWorldSpaceBounds(const JPC56_Body* self);
// JPC56_API const MotionProperties *JPC56_Body_GetMotionProperties(const JPC56_Body* self)
// JPC56_API MotionProperties * JPC56_Body_GetMotionProperties(JPC56_Body* self);
// JPC56_API const MotionProperties *JPC56_Body_GetMotionPropertiesUnchecked(const JPC56_Body* self)
// JPC56_API MotionProperties * JPC56_Body_GetMotionPropertiesUnchecked(JPC56_Body* self);

JPC56_API uint64_t JPC56_Body_GetUserData(const JPC56_Body* self) {
	return to_jph(self)->GetUserData();
}

JPC56_API void JPC56_Body_SetUserData(JPC56_Body* self, uint64_t inUserData) {
	to_jph(self)->SetUserData(inUserData);
}

JPC56_API JPC56_Vec3 JPC56_Body_GetWorldSpaceSurfaceNormal(const JPC56_Body* self, JPC56_SubShapeID inSubShapeID, JPC56_RVec3 inPosition) {
	JPH56::SubShapeID jph_id = JPC56_SubShapeID_to_jph(inSubShapeID);

	return to_jpc(to_jph(self)->GetWorldSpaceSurfaceNormal(jph_id, to_jph(inPosition)));
}

// JPC56_API TransformedShape JPC56_Body_GetTransformedShape(const JPC56_Body* self);
// JPC56_API BodyCreationSettings JPC56_Body_GetBodyCreationSettings(const JPC56_Body* self);
// JPC56_API SoftBodyCreationSettings JPC56_Body_GetSoftBodyCreationSettings(const JPC56_Body* self);

////////////////////////////////////////////////////////////////////////////////
// BodyLockRead

JPC56_API JPC56_BodyLockRead* JPC56_BodyLockRead_new(const JPC56_BodyLockInterface* interface, JPC56_BodyID bodyID) {
	JPH56::BodyLockRead* lockRead = new JPH56::BodyLockRead(*to_jph(interface), to_jph(bodyID));
	return to_jpc(lockRead);
}

JPC56_API void JPC56_BodyLockRead_delete(JPC56_BodyLockRead* self) {
	delete to_jph(self);
}

JPC56_API bool JPC56_BodyLockRead_Succeeded(JPC56_BodyLockRead* self) {
	return to_jph(self)->Succeeded();
}

JPC56_API const JPC56_Body* JPC56_BodyLockRead_GetBody(JPC56_BodyLockRead* self) {
	return to_jpc(&to_jph(self)->GetBody());
}

////////////////////////////////////////////////////////////////////////////////
// BodyLockWrite

JPC56_API JPC56_BodyLockWrite* JPC56_BodyLockWrite_new(const JPC56_BodyLockInterface* interface, JPC56_BodyID bodyID) {
	JPH56::BodyLockWrite* lockWrite = new JPH56::BodyLockWrite(*to_jph(interface), to_jph(bodyID));
	return to_jpc(lockWrite);
}

JPC56_API void JPC56_BodyLockWrite_delete(JPC56_BodyLockWrite* self) {
	delete to_jph(self);
}

JPC56_API bool JPC56_BodyLockWrite_Succeeded(JPC56_BodyLockWrite* self) {
	return to_jph(self)->Succeeded();
}

JPC56_API JPC56_Body* JPC56_BodyLockWrite_GetBody(JPC56_BodyLockWrite* self) {
	return to_jpc(&to_jph(self)->GetBody());
}

////////////////////////////////////////////////////////////////////////////////
// BodyLockMultiRead

typedef struct JPC56_BodyLockMultiRead JPC56_BodyLockMultiRead;

JPC56_API JPC56_BodyLockMultiRead* JPC56_BodyLockMultiRead_new(
	const JPC56_BodyLockInterface* interface,
	const JPC56_BodyID *inBodyIDs,
	int inNumber)
{
	JPH56::BodyLockMultiRead* lockRead = new JPH56::BodyLockMultiRead(*to_jph(interface), to_jph(inBodyIDs), inNumber);
	return to_jpc(lockRead);
}

JPC56_API void JPC56_BodyLockMultiRead_delete(JPC56_BodyLockMultiRead* self) {
	delete to_jph(self);
}

JPC56_API const JPC56_Body* JPC56_BodyLockMultiRead_GetBody(JPC56_BodyLockMultiRead* self, int inBodyIndex) {
	return to_jpc(to_jph(self)->GetBody(inBodyIndex));
}

////////////////////////////////////////////////////////////////////////////////
// BodyLockMultiWrite

typedef struct JPC56_BodyLockMultiWrite JPC56_BodyLockMultiWrite;

JPC56_API JPC56_BodyLockMultiWrite* JPC56_BodyLockMultiWrite_new(
	const JPC56_BodyLockInterface* interface,
	const JPC56_BodyID *inBodyIDs,
	int inNumber)
{
	JPH56::BodyLockMultiWrite* lockWrite = new JPH56::BodyLockMultiWrite(*to_jph(interface), to_jph(inBodyIDs), inNumber);
	return to_jpc(lockWrite);
}

JPC56_API void JPC56_BodyLockMultiWrite_delete(JPC56_BodyLockMultiWrite* self) {
	delete to_jph(self);
}

JPC56_API JPC56_Body* JPC56_BodyLockMultiWrite_GetBody(JPC56_BodyLockMultiWrite* self, int inBodyIndex) {
	return to_jpc(to_jph(self)->GetBody(inBodyIndex));
}

////////////////////////////////////////////////////////////////////////////////
// BodyInterface

JPC56_API JPC56_Body* JPC56_BodyInterface_CreateBody(JPC56_BodyInterface* self, const JPC56_BodyCreationSettings* inSettings) {
	return to_jpc(to_jph(self)->CreateBody(to_jph(inSettings)));
}

// JPC56_API JPC56_Body* JPC56_BodyInterface_CreateSoftBody(JPC56_BodyInterface *self, const SoftBodyCreationSettings &inSettings);

JPC56_API JPC56_Body* JPC56_BodyInterface_CreateBodyWithID(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, const JPC56_BodyCreationSettings* inSettings) {
	return to_jpc(to_jph(self)->CreateBodyWithID(to_jph(inBodyID), to_jph(inSettings)));
}

// JPC56_API JPC56_Body* JPC56_BodyInterface_CreateSoftBodyWithID(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, const SoftBodyCreationSettings* inSettings);

JPC56_API JPC56_Body* JPC56_BodyInterface_CreateBodyWithoutID(const JPC56_BodyInterface *self, const JPC56_BodyCreationSettings* inSettings) {
	return to_jpc(to_jph(self)->CreateBodyWithoutID(to_jph(inSettings)));
}

// JPC56_API JPC56_Body* JPC56_BodyInterface_CreateSoftBodyWithoutID(const JPC56_BodyInterface *self, const SoftBodyCreationSettings* inSettings);

JPC56_API void JPC56_BodyInterface_DestroyBodyWithoutID(const JPC56_BodyInterface *self, JPC56_Body *inBody) {
	to_jph(self)->DestroyBodyWithoutID(to_jph(inBody));
}

JPC56_API bool JPC56_BodyInterface_AssignBodyID(JPC56_BodyInterface *self, JPC56_Body *ioBody) {
	return to_jph(self)->AssignBodyID(to_jph(ioBody));
}

// JPC56_API bool JPC56_BodyInterface_AssignBodyID(JPC56_BodyInterface *self, JPC56_Body *ioBody, JPC56_BodyID inBodyID);

JPC56_API JPC56_Body* JPC56_BodyInterface_UnassignBodyID(JPC56_BodyInterface *self, JPC56_BodyID inBodyID) {
	return to_jpc(to_jph(self)->UnassignBodyID(to_jph(inBodyID)));
}

// JPC56_API void JPC56_BodyInterface_UnassignBodyIDs(JPC56_BodyInterface *self, const JPC56_BodyID *inBodyIDs, int inNumber, JPC56_Body **outBodies) {
// 	return to_jph(self)->UnassignBodyIDs(to_jph(inBodyIDs), inNumber, to_jph(outBodies));
// }

JPC56_API void JPC56_BodyInterface_DestroyBody(JPC56_BodyInterface *self, JPC56_BodyID inBodyID) {
	to_jph(self)->DestroyBody(to_jph(inBodyID));
}

// JPC56_API void JPC56_BodyInterface_DestroyBodies(JPC56_BodyInterface *self, const JPC56_BodyID *inBodyIDs, int inNumber) {
// 	return to_jph(self)->DestroyBodies(to_jph(inBodyIDs), int inNumber);
// }

JPC56_API void JPC56_BodyInterface_AddBody(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Activation inActivationMode) {
	to_jph(self)->AddBody(to_jph(inBodyID), to_jph(inActivationMode));
}

JPC56_API void JPC56_BodyInterface_RemoveBody(JPC56_BodyInterface *self, JPC56_BodyID inBodyID) {
	to_jph(self)->RemoveBody(to_jph(inBodyID));
}

JPC56_API bool JPC56_BodyInterface_IsAdded(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID) {
	return to_jph(self)->IsAdded(to_jph(inBodyID));
}

JPC56_API JPC56_BodyID JPC56_BodyInterface_CreateAndAddBody(JPC56_BodyInterface *self, const JPC56_BodyCreationSettings* inSettings, JPC56_Activation inActivationMode) {
	return to_jpc(to_jph(self)->CreateAndAddBody(to_jph(inSettings), to_jph(inActivationMode)));
}

// JPC56_API JPC56_BodyID JPC56_BodyInterface_CreateAndAddSoftBody(JPC56_BodyInterface *self, const SoftBodyCreationSettings &inSettings, JPC56_Activation inActivationMode);

JPC56_API void* JPC56_BodyInterface_AddBodiesPrepare(JPC56_BodyInterface *self, JPC56_BodyID *ioBodies, int inNumber) {
	return to_jph(self)->AddBodiesPrepare(to_jph(ioBodies), inNumber);
}

JPC56_API void JPC56_BodyInterface_AddBodiesFinalize(JPC56_BodyInterface *self, JPC56_BodyID *ioBodies, int inNumber, void* inAddState, JPC56_Activation inActivationMode) {
	to_jph(self)->AddBodiesFinalize(to_jph(ioBodies), inNumber, inAddState, to_jph(inActivationMode));
}

JPC56_API void JPC56_BodyInterface_AddBodiesAbort(JPC56_BodyInterface *self, JPC56_BodyID *ioBodies, int inNumber, void* inAddState) {
	to_jph(self)->AddBodiesAbort(to_jph(ioBodies), inNumber, inAddState);
}

JPC56_API void JPC56_BodyInterface_RemoveBodies(JPC56_BodyInterface *self, JPC56_BodyID *ioBodies, int inNumber) {
	to_jph(self)->RemoveBodies(to_jph(ioBodies), inNumber);
}

JPC56_API void JPC56_BodyInterface_ActivateBody(JPC56_BodyInterface *self, JPC56_BodyID inBodyID) {
	to_jph(self)->ActivateBody(to_jph(inBodyID));
}

JPC56_API void JPC56_BodyInterface_ActivateBodies(JPC56_BodyInterface *self, JPC56_BodyID *inBodyIDs, int inNumber) {
	to_jph(self)->ActivateBodies(to_jph(inBodyIDs), inNumber);
}

// JPC56_API void JPC56_BodyInterface_ActivateBodiesInAABox(JPC56_BodyInterface *self, const AABox &inBox, const BroadPhaseLayerFilter &inBroadPhaseLayerFilter, const ObjectLayerFilter &inObjectLayerFilter);

JPC56_API void JPC56_BodyInterface_DeactivateBody(JPC56_BodyInterface *self, JPC56_BodyID inBodyID) {
	to_jph(self)->DeactivateBody(to_jph(inBodyID));
}

JPC56_API void JPC56_BodyInterface_DeactivateBodies(JPC56_BodyInterface *self, JPC56_BodyID *inBodyIDs, int inNumber) {
	to_jph(self)->DeactivateBodies(to_jph(inBodyIDs), inNumber);
}

JPC56_API bool JPC56_BodyInterface_IsActive(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID) {
	return to_jph(self)->IsActive(to_jph(inBodyID));
}

// TwoBodyConstraint * JPC56_BodyInterface_CreateConstraint(JPC56_BodyInterface *self, const TwoBodyConstraintSettings *inSettings, JPC56_BodyID inBodyID1, JPC56_BodyID inBodyID2);
// JPC56_API void JPC56_BodyInterface_ActivateConstraint(JPC56_BodyInterface *self, const TwoBodyConstraint *inConstraint);

JPC56_API const JPC56_Shape* JPC56_BodyInterface_GetShape(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID) {
	// NOTE: This pointer will only be alive as long as BodyInterface holds onto it!
	return to_jpc(to_jph(self)->GetShape(to_jph(inBodyID)).GetPtr());
}

JPC56_API void JPC56_BodyInterface_SetShape(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID, const JPC56_Shape *inShape, bool inUpdateMassProperties, JPC56_Activation inActivationMode) {
	to_jph(self)->SetShape(to_jph(inBodyID), to_jph(inShape), inUpdateMassProperties, to_jph(inActivationMode));
}

JPC56_API void JPC56_BodyInterface_NotifyShapeChanged(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Vec3 inPreviousCenterOfMass, bool inUpdateMassProperties, JPC56_Activation inActivationMode) {
	to_jph(self)->NotifyShapeChanged(to_jph(inBodyID), to_jph(inPreviousCenterOfMass), inUpdateMassProperties, to_jph(inActivationMode));
}

JPC56_API void JPC56_BodyInterface_SetObjectLayer(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_ObjectLayer inLayer) {
	to_jph(self)->SetObjectLayer(to_jph(inBodyID), inLayer);
}

JPC56_API JPC56_ObjectLayer JPC56_BodyInterface_GetObjectLayer(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID) {
	return to_jph(self)->GetObjectLayer(to_jph(inBodyID));
}

JPC56_API void JPC56_BodyInterface_SetPositionAndRotation(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_RVec3 inPosition, JPC56_Quat inRotation, JPC56_Activation inActivationMode) {
	to_jph(self)->SetPositionAndRotation(to_jph(inBodyID), to_jph(inPosition), to_jph(inRotation), to_jph(inActivationMode));
}

JPC56_API void JPC56_BodyInterface_SetPositionAndRotationWhenChanged(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_RVec3 inPosition, JPC56_Quat inRotation, JPC56_Activation inActivationMode) {
	to_jph(self)->SetPositionAndRotationWhenChanged(to_jph(inBodyID), to_jph(inPosition), to_jph(inRotation), to_jph(inActivationMode));
}

JPC56_API void JPC56_BodyInterface_GetPositionAndRotation(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_RVec3 *outPosition, JPC56_Quat *outRotation) {
	JPH56::RVec3 outPos{};
	JPH56::Quat outRot{};

	to_jph(self)->GetPositionAndRotation(to_jph(inBodyID), outPos, outRot);

	*outPosition = to_jpc(outPos);
	*outRotation = to_jpc(outRot);
}

JPC56_API void JPC56_BodyInterface_SetPosition(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_RVec3 inPosition, JPC56_Activation inActivationMode) {
	to_jph(self)->SetPosition(to_jph(inBodyID), to_jph(inPosition), to_jph(inActivationMode));
}

JPC56_API JPC56_RVec3 JPC56_BodyInterface_GetPosition(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID) {
	return to_jpc(to_jph(self)->GetPosition(to_jph(inBodyID)));
}

JPC56_API JPC56_RVec3 JPC56_BodyInterface_GetCenterOfMassPosition(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID) {
	return to_jpc(to_jph(self)->GetCenterOfMassPosition(to_jph(inBodyID)));
}

JPC56_API void JPC56_BodyInterface_SetRotation(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Quat inRotation, JPC56_Activation inActivationMode) {
	to_jph(self)->SetRotation(to_jph(inBodyID), to_jph(inRotation), to_jph(inActivationMode));
}

JPC56_API JPC56_Quat JPC56_BodyInterface_GetRotation(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID) {
	return to_jpc(to_jph(self)->GetRotation(to_jph(inBodyID)));
}

JPC56_API JPC56_RMat44 JPC56_BodyInterface_GetWorldTransform(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID) {
	return to_jpc(to_jph(self)->GetWorldTransform(to_jph(inBodyID)));
}

JPC56_API JPC56_RMat44 JPC56_BodyInterface_GetCenterOfMassTransform(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID) {
	return to_jpc(to_jph(self)->GetCenterOfMassTransform(to_jph(inBodyID)));
}

JPC56_API void JPC56_BodyInterface_MoveKinematic(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_RVec3 inTargetPosition, JPC56_Quat inTargetRotation, float inDeltaTime) {
	to_jph(self)->MoveKinematic(to_jph(inBodyID), to_jph(inTargetPosition), to_jph(inTargetRotation), inDeltaTime);
}

JPC56_API void JPC56_BodyInterface_SetLinearAndAngularVelocity(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Vec3 inLinearVelocity, JPC56_Vec3 inAngularVelocity) {
	to_jph(self)->SetLinearAndAngularVelocity(to_jph(inBodyID), to_jph(inLinearVelocity), to_jph(inAngularVelocity));
}

JPC56_API void JPC56_BodyInterface_GetLinearAndAngularVelocity(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Vec3 *outLinearVelocity, JPC56_Vec3 *outAngularVelocity) {
	JPH56::Vec3 outLinVel;
	JPH56::Vec3 outAngVel;

	to_jph(self)->GetLinearAndAngularVelocity(to_jph(inBodyID), outLinVel, outAngVel);

	*outLinearVelocity = to_jpc(outLinVel);
	*outAngularVelocity = to_jpc(outAngVel);
}

JPC56_API void JPC56_BodyInterface_SetLinearVelocity(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Vec3 inLinearVelocity) {
	to_jph(self)->SetLinearVelocity(to_jph(inBodyID), to_jph(inLinearVelocity));
}

JPC56_API JPC56_Vec3 JPC56_BodyInterface_GetLinearVelocity(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID) {
	return to_jpc(to_jph(self)->GetLinearVelocity(to_jph(inBodyID)));
}

JPC56_API void JPC56_BodyInterface_AddLinearVelocity(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Vec3 inLinearVelocity) {
	to_jph(self)->AddLinearVelocity(to_jph(inBodyID), to_jph(inLinearVelocity));
}

JPC56_API void JPC56_BodyInterface_AddLinearAndAngularVelocity(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Vec3 inLinearVelocity, JPC56_Vec3 inAngularVelocity) {
	to_jph(self)->AddLinearAndAngularVelocity(to_jph(inBodyID), to_jph(inLinearVelocity), to_jph(inAngularVelocity));
}

JPC56_API void JPC56_BodyInterface_SetAngularVelocity(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Vec3 inAngularVelocity) {
	to_jph(self)->SetAngularVelocity(to_jph(inBodyID), to_jph(inAngularVelocity));
}

JPC56_API JPC56_Vec3 JPC56_BodyInterface_GetAngularVelocity(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID) {
	return to_jpc(to_jph(self)->GetAngularVelocity(to_jph(inBodyID)));
}

JPC56_API JPC56_Vec3 JPC56_BodyInterface_GetPointVelocity(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_RVec3 inPoint) {
	return to_jpc(to_jph(self)->GetPointVelocity(to_jph(inBodyID), to_jph(inPoint)));
}

JPC56_API void JPC56_BodyInterface_SetPositionRotationAndVelocity(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_RVec3 inPosition, JPC56_Quat inRotation, JPC56_Vec3 inLinearVelocity, JPC56_Vec3 inAngularVelocity) {
	to_jph(self)->SetPositionRotationAndVelocity(to_jph(inBodyID), to_jph(inPosition), to_jph(inRotation), to_jph(inLinearVelocity), to_jph(inAngularVelocity));
}

JPC56_API void JPC56_BodyInterface_AddForce(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Vec3 inForce) {
	to_jph(self)->AddForce(to_jph(inBodyID), to_jph(inForce));
}

// overload of BodyInterface::AddForce
JPC56_API void JPC56_BodyInterface_AddForceAtPoint(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Vec3 inForce, JPC56_RVec3 inPoint) {
	to_jph(self)->AddForce(to_jph(inBodyID), to_jph(inForce), to_jph(inPoint));
}

JPC56_API void JPC56_BodyInterface_AddTorque(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Vec3 inTorque) {
	to_jph(self)->AddTorque(to_jph(inBodyID), to_jph(inTorque));
}

JPC56_API void JPC56_BodyInterface_AddForceAndTorque(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Vec3 inForce, JPC56_Vec3 inTorque) {
	to_jph(self)->AddForceAndTorque(to_jph(inBodyID), to_jph(inForce), to_jph(inTorque));
}

JPC56_API void JPC56_BodyInterface_AddImpulse(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Vec3 inImpulse) {
	to_jph(self)->AddImpulse(to_jph(inBodyID), to_jph(inImpulse));
}

JPC56_API void JPC56_BodyInterface_AddImpulse3(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Vec3 inImpulse, JPC56_RVec3 inPoint) {
	to_jph(self)->AddImpulse(to_jph(inBodyID), to_jph(inImpulse), to_jph(inPoint));
}

JPC56_API void JPC56_BodyInterface_AddAngularImpulse(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Vec3 inAngularImpulse) {
	to_jph(self)->AddAngularImpulse(to_jph(inBodyID), to_jph(inAngularImpulse));
}

JPC56_API JPC56_BodyType JPC56_BodyInterface_GetBodyType(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID) {
	return to_jpc(to_jph(self)->GetBodyType(to_jph(inBodyID)));
}

JPC56_API void JPC56_BodyInterface_SetMotionType(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_MotionType inMotionType, JPC56_Activation inActivationMode) {
	to_jph(self)->SetMotionType(to_jph(inBodyID), to_jph(inMotionType), to_jph(inActivationMode));
}

JPC56_API JPC56_MotionType JPC56_BodyInterface_GetMotionType(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID) {
	return to_jpc(to_jph(self)->GetMotionType(to_jph(inBodyID)));
}

JPC56_API void JPC56_BodyInterface_SetMotionQuality(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_MotionQuality inMotionQuality) {
	to_jph(self)->SetMotionQuality(to_jph(inBodyID), to_jph(inMotionQuality));
}

JPC56_API JPC56_MotionQuality JPC56_BodyInterface_GetMotionQuality(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID) {
	return to_jpc(to_jph(self)->GetMotionQuality(to_jph(inBodyID)));
}

JPC56_API void JPC56_BodyInterface_GetInverseInertia(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID, JPC56_Mat44 *outMatrix) {
	to_jph(self)->GetInverseInertia(to_jph(inBodyID)).StoreFloat4x4(reinterpret_cast<JPH56::Float4*>(outMatrix));
}

JPC56_API void JPC56_BodyInterface_SetRestitution(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, float inRestitution) {
	to_jph(self)->SetRestitution(to_jph(inBodyID), inRestitution);
}

JPC56_API float JPC56_BodyInterface_GetRestitution(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID) {
	return to_jph(self)->GetRestitution(to_jph(inBodyID));
}

JPC56_API void JPC56_BodyInterface_SetFriction(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, float inFriction) {
	to_jph(self)->SetFriction(to_jph(inBodyID), inFriction);
}

JPC56_API float JPC56_BodyInterface_GetFriction(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID) {
	return to_jph(self)->GetFriction(to_jph(inBodyID));
}

JPC56_API void JPC56_BodyInterface_SetGravityFactor(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, float inGravityFactor) {
	to_jph(self)->SetGravityFactor(to_jph(inBodyID), inGravityFactor);
}

JPC56_API float JPC56_BodyInterface_GetGravityFactor(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID) {
	return to_jph(self)->GetGravityFactor(to_jph(inBodyID));
}

JPC56_API void JPC56_BodyInterface_SetUseManifoldReduction(JPC56_BodyInterface *self, JPC56_BodyID inBodyID, bool inUseReduction) {
	to_jph(self)->SetUseManifoldReduction(to_jph(inBodyID), inUseReduction);
}

JPC56_API bool JPC56_BodyInterface_GetUseManifoldReduction(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID) {
	return to_jph(self)->GetUseManifoldReduction(to_jph(inBodyID));
}

// TransformedShape JPC56_BodyInterface_GetTransformedShape(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID);

JPC56_API uint64_t JPC56_BodyInterface_GetUserData(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID) {
	return to_jph(self)->GetUserData(to_jph(inBodyID));
}

JPC56_API void JPC56_BodyInterface_SetUserData(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID, uint64_t inUserData) {
	to_jph(self)->SetUserData(to_jph(inBodyID), inUserData);
}

// const PhysicsMaterial* JPC56_BodyInterface_GetMaterial(const JPC56_BodyInterface *self, JPC56_BodyID inBodyID, const SubShapeID &inSubShapeID);

JPC56_API void JPC56_BodyInterface_InvalidateContactCache(JPC56_BodyInterface *self, JPC56_BodyID inBodyID) {
	to_jph(self)->InvalidateContactCache(to_jph(inBodyID));
}

////////////////////////////////////////////////////////////////////////////////
// NarrowPhaseQuery

JPC56_API bool JPC56_NarrowPhaseQuery_CastRay(const JPC56_NarrowPhaseQuery* self, JPC56_NarrowPhaseQuery_CastRayArgs* args) {
	JPH56::RayCastResult result;

	JPH56::RayCastSettings settings;

	JPH56::BroadPhaseLayerFilter defaultBplFilter{};
	const JPH56::BroadPhaseLayerFilter* bplFilter = &defaultBplFilter;
	if (args->BroadPhaseLayerFilter != nullptr) {
		bplFilter = to_jph(args->BroadPhaseLayerFilter);
	}

	JPH56::ObjectLayerFilter defaultOlFilter{};
	const JPH56::ObjectLayerFilter* olFilter = &defaultOlFilter;
	if (args->ObjectLayerFilter != nullptr) {
		olFilter = to_jph(args->ObjectLayerFilter);
	}

	JPH56::BodyFilter defaultBodyFilter{};
	const JPH56::BodyFilter* bodyFilter = &defaultBodyFilter;
	if (args->BodyFilter != nullptr) {
		bodyFilter = to_jph(args->BodyFilter);
	}

	JPH56::ShapeFilter defaultShapeFilter{};
	const JPH56::ShapeFilter* shapeFilter = &defaultShapeFilter;
	if (args->ShapeFilter != nullptr) {
		shapeFilter = to_jph(args->ShapeFilter);
	}

	JPH56::ClosestHitCollisionCollector<JPH56::CastRayCollector> collector;

	to_jph(self)->CastRay(
		to_jph(args->Ray),
		settings,
		collector,
		*bplFilter,
		*olFilter,
		*bodyFilter,
		*shapeFilter);

	bool hit = collector.HadHit();
	if (hit) {
		args->Result = to_jpc(collector.mHit);
	}

	return hit;
}

JPC56_API void JPC56_ShapeCastSettings_default(JPC56_ShapeCastSettings* object) {
	JPH56::ShapeCastSettings defaultSettings{};
	*object = to_jpc(defaultSettings);
}

JPC56_API void JPC56_NarrowPhaseQuery_CastShape(const JPC56_NarrowPhaseQuery* self, JPC56_NarrowPhaseQuery_CastShapeArgs* args) {
	JPH56::ShapeCastSettings settings = to_jph(args->Settings);

	JPH56::ClosestHitCollisionCollector<JPH56::CastShapeCollector> defaultCollector{};
	JPH56::CastShapeCollector* collector = &defaultCollector;
	if (args->Collector != nullptr) {
		collector = to_jph(args->Collector);
	}

	JPH56::BroadPhaseLayerFilter defaultBplFilter{};
	const JPH56::BroadPhaseLayerFilter* bplFilter = &defaultBplFilter;
	if (args->BroadPhaseLayerFilter != nullptr) {
		bplFilter = to_jph(args->BroadPhaseLayerFilter);
	}

	JPH56::ObjectLayerFilter defaultOlFilter{};
	const JPH56::ObjectLayerFilter* olFilter = &defaultOlFilter;
	if (args->ObjectLayerFilter != nullptr) {
		olFilter = to_jph(args->ObjectLayerFilter);
	}

	JPH56::BodyFilter defaultBodyFilter{};
	const JPH56::BodyFilter* bodyFilter = &defaultBodyFilter;
	if (args->BodyFilter != nullptr) {
		bodyFilter = to_jph(args->BodyFilter);
	}

	JPH56::ShapeFilter defaultShapeFilter{};
	const JPH56::ShapeFilter* shapeFilter = &defaultShapeFilter;
	if (args->ShapeFilter != nullptr) {
		shapeFilter = to_jph(args->ShapeFilter);
	}

	to_jph(self)->CastShape(
		to_jph(args->ShapeCast),
		settings,
		to_jph(args->BaseOffset),
		*collector,
		*bplFilter,
		*olFilter,
		*bodyFilter,
		*shapeFilter);
}

JPC56_API void JPC56_CollideShapeSettings_default(JPC56_CollideShapeSettings* object) {
	JPH56::CollideShapeSettings defaultSettings{};
	*object = to_jpc(defaultSettings);
}

JPC56_API void JPC56_NarrowPhaseQuery_CollideShape(const JPC56_NarrowPhaseQuery* self, JPC56_NarrowPhaseQuery_CollideShapeArgs* args) {
	JPH56::CollideShapeSettings settings = to_jph(args->Settings);

	JPH56::ClosestHitCollisionCollector<JPH56::CollideShapeCollector> defaultCollector{};
	JPH56::CollideShapeCollector* collector = &defaultCollector;
	if (args->Collector != nullptr) {
		collector = to_jph(args->Collector);
	}

	JPH56::BroadPhaseLayerFilter defaultBplFilter{};
	const JPH56::BroadPhaseLayerFilter* bplFilter = &defaultBplFilter;
	if (args->BroadPhaseLayerFilter != nullptr) {
		bplFilter = to_jph(args->BroadPhaseLayerFilter);
	}

	JPH56::ObjectLayerFilter defaultOlFilter{};
	const JPH56::ObjectLayerFilter* olFilter = &defaultOlFilter;
	if (args->ObjectLayerFilter != nullptr) {
		olFilter = to_jph(args->ObjectLayerFilter);
	}

	JPH56::BodyFilter defaultBodyFilter{};
	const JPH56::BodyFilter* bodyFilter = &defaultBodyFilter;
	if (args->BodyFilter != nullptr) {
		bodyFilter = to_jph(args->BodyFilter);
	}

	JPH56::ShapeFilter defaultShapeFilter{};
	const JPH56::ShapeFilter* shapeFilter = &defaultShapeFilter;
	if (args->ShapeFilter != nullptr) {
		shapeFilter = to_jph(args->ShapeFilter);
	}

	to_jph(self)->CollideShape(
		to_jph(args->Shape),
		to_jph(args->ShapeScale),
		to_jph(args->CenterOfMassTransform),
		settings,
		to_jph(args->BaseOffset),
		*collector,
		*bplFilter,
		*olFilter,
		*bodyFilter,
		*shapeFilter);
}

////////////////////////////////////////////////////////////////////////////////
// PhysicsSystem

JPC56_API JPC56_PhysicsSystem* JPC56_PhysicsSystem_new() {
	return to_jpc(new JPH56::PhysicsSystem());
}

JPC56_API void JPC56_PhysicsSystem_Init(
	JPC56_PhysicsSystem* self,
	uint inMaxBodies,
	uint inNumBodyMutexes,
	uint inMaxBodyPairs,
	uint inMaxContactConstraints,
	JPC56_BroadPhaseLayerInterface* inBroadPhaseLayerInterface,
	JPC56_ObjectVsBroadPhaseLayerFilter* inObjectVsBroadPhaseLayerFilter,
	JPC56_ObjectLayerPairFilter* inObjectLayerPairFilter)
{
	JPC56_BroadPhaseLayerInterfaceBridge* impl_inBroadPhaseLayerInterface = to_jph(inBroadPhaseLayerInterface);
	JPC56_ObjectVsBroadPhaseLayerFilterBridge* impl_inObjectVsBroadPhaseLayerFilter = to_jph(inObjectVsBroadPhaseLayerFilter);
	JPC56_ObjectLayerPairFilterBridge* impl_inObjectLayerPairFilter = to_jph(inObjectLayerPairFilter);

	to_jph(self)->Init(
		inMaxBodies,
		inNumBodyMutexes,
		inMaxBodyPairs,
		inMaxContactConstraints,
		*impl_inBroadPhaseLayerInterface,
		*impl_inObjectVsBroadPhaseLayerFilter,
		*impl_inObjectLayerPairFilter);
}

JPC56_API void JPC56_PhysicsSystem_OptimizeBroadPhase(JPC56_PhysicsSystem* self) {
	to_jph(self)->OptimizeBroadPhase();
}

JPC56_API void JPC56_PhysicsSystem_AddConstraint(JPC56_PhysicsSystem* self, JPC56_Constraint* constraint) {
	to_jph(self)->AddConstraint(to_jph(constraint));
}

JPC56_API void JPC56_PhysicsSystem_RemoveConstraint(JPC56_PhysicsSystem* self, JPC56_Constraint* constraint) {
	to_jph(self)->RemoveConstraint(to_jph(constraint));
}

JPC56_API void JPC56_PhysicsSystem_SetGravity(JPC56_PhysicsSystem* self, JPC56_Vec3 inGravity) {
	to_jph(self)->SetGravity(to_jph(inGravity));
}

JPC56_API JPC56_Vec3 JPC56_PhysicsSystem_GetGravity(const JPC56_PhysicsSystem* self) {
	return to_jpc(to_jph(self)->GetGravity());
}

JPC56_API JPC56_BodyInterface* JPC56_PhysicsSystem_GetBodyInterface(JPC56_PhysicsSystem* self) {
	return to_jpc(&to_jph(self)->GetBodyInterface());
}

JPC56_API const JPC56_BodyLockInterface* JPC56_PhysicsSystem_GetBodyLockInterface(JPC56_PhysicsSystem* self) {
	return to_jpc(&to_jph(self)->GetBodyLockInterface());
}

JPC56_API const JPC56_NarrowPhaseQuery* JPC56_PhysicsSystem_GetNarrowPhaseQuery(const JPC56_PhysicsSystem* self) {
	return to_jpc(&to_jph(self)->GetNarrowPhaseQuery());
}

JPC56_API JPC56_PhysicsUpdateError JPC56_PhysicsSystem_Update(
	JPC56_PhysicsSystem* self,
	float inDeltaTime,
	int inCollisionSteps,
	JPC56_TempAllocatorImpl *inTempAllocator,
	JPC56_JobSystem *inJobSystem)
{
	auto res = to_jph(self)->Update(
		inDeltaTime,
		inCollisionSteps,
		to_jph(inTempAllocator),
		to_jph(inJobSystem));

	return to_integral(res);
}

JPC56_API void JPC56_PhysicsSystem_DrawBodies(
	JPC56_PhysicsSystem* self,
	JPC56_BodyManager_DrawSettings* inSettings,
	JPC56_DebugRendererSimple* inRenderer,
	[[maybe_unused]] const void* inBodyFilter)
{
	to_jph(self)->DrawBodies(to_jph(*inSettings), to_jph(inRenderer), nullptr);
}

JPC56_API void JPC56_PhysicsSystem_DrawConstraints(
	JPC56_PhysicsSystem* self,
	JPC56_DebugRendererSimple* inRenderer)
{
	to_jph(self)->DrawConstraints(to_jph(inRenderer));
}


JPC56_API void JPC56_PhysicsSystem_SetSimShapeFilter(
	JPC56_PhysicsSystem* self,
	const JPC56_SimShapeFilter* inShapeFilter)
{
	to_jph(self)->SetSimShapeFilter(to_jph(inShapeFilter));
}

JPC56_API void JPC56_PhysicsSystem_SetContactListener(
	JPC56_PhysicsSystem* self,
	JPC56_ContactListener* inContactListener)
{
	to_jph(self)->SetContactListener(to_jph(inContactListener));
}
