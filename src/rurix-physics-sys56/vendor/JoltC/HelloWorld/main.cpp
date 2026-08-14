#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>

#include "JoltC/JoltC.h"

#ifdef _MSC_VER
	#define unreachable() __assume(0)
#else
	#define unreachable() __builtin_unreachable()
#endif

typedef enum Hello_ObjectLayers {
	HELLO_OL_NON_MOVING,
	HELLO_OL_MOVING,
	HELLO_OL_COUNT,
} Hello_ObjectLayers;

typedef enum Hello_BroadPhaseLayers {
	HELLO_BPL_NON_MOVING,
	HELLO_BPL_MOVING,
	HELLO_BPL_COUNT,
} Hello_BroadPhaseLayers;

unsigned int Hello_BPL_GetNumBroadPhaseLayers(const void *self) {
	return HELLO_BPL_COUNT;
}

JPC56_BroadPhaseLayer Hello_BPL_GetBroadPhaseLayer(const void *self, JPC56_ObjectLayer inLayer) {
	switch (inLayer) {
	case HELLO_OL_NON_MOVING:
		return HELLO_BPL_NON_MOVING;

	case HELLO_OL_MOVING:
		return HELLO_BPL_MOVING;

	default:
		unreachable();
	}
}

static JPC56_BroadPhaseLayerInterfaceFns Hello_BPL = {
	.GetNumBroadPhaseLayers = Hello_BPL_GetNumBroadPhaseLayers,
	.GetBroadPhaseLayer = Hello_BPL_GetBroadPhaseLayer,
};

bool Hello_OVB_ShouldCollide(const void *self, JPC56_ObjectLayer inLayer1, JPC56_BroadPhaseLayer inLayer2) {
	switch (inLayer1) {
	case HELLO_OL_NON_MOVING:
		return inLayer2 == HELLO_BPL_MOVING;

	case HELLO_OL_MOVING:
		return true;

	default:
		unreachable();
	}
}

static JPC56_ObjectVsBroadPhaseLayerFilterFns Hello_OVB = {
	.ShouldCollide = Hello_OVB_ShouldCollide,
};

bool Hello_OVO_ShouldCollide(const void *self, JPC56_ObjectLayer inLayer1, JPC56_ObjectLayer inLayer2) {
	switch (inLayer1)
	{
	case HELLO_OL_NON_MOVING:
		return inLayer2 == HELLO_OL_MOVING; // Non moving only collides with moving

	case HELLO_OL_MOVING:
		return true; // Moving collides with everything

	default:
		unreachable();
	}
}

static JPC56_ObjectLayerPairFilterFns Hello_OVO = {
	.ShouldCollide = Hello_OVO_ShouldCollide,
};

void Hello_Debug_DrawLine(const void *self, JPC56_RVec3 inFrom, JPC56_RVec3 inTo, JPC56_Color inColor) {
	// printf("Draw line from (%f, %f, %f) to (%f, %f, %f) with color (%d, %d, %d)\n",
	// 	inFrom.x, inFrom.y, inFrom.z, inTo.x, inTo.y, inTo.z, inColor.r, inColor.g, inColor.b);
}

static JPC56_DebugRendererSimpleFns Hello_DebugRenderer = {
	.DrawLine = Hello_Debug_DrawLine,
};

int main() {
	JPC56_RegisterDefaultAllocator();
	JPC56_FactoryInit();
	JPC56_RegisterTypes();

	JPC56_TempAllocatorImpl* temp_allocator = JPC56_TempAllocatorImpl_new(10 * 1024 * 1024);

	JPC56_JobSystemThreadPool* job_system = JPC56_JobSystemThreadPool_new2(JPC56_MAX_PHYSICS_JOBS, JPC56_MAX_PHYSICS_BARRIERS);

	JPC56_BroadPhaseLayerInterface* broad_phase_layer_interface = JPC56_BroadPhaseLayerInterface_new(nullptr, Hello_BPL);
	JPC56_ObjectVsBroadPhaseLayerFilter* object_vs_broad_phase_layer_filter = JPC56_ObjectVsBroadPhaseLayerFilter_new(nullptr, Hello_OVB);
	JPC56_ObjectLayerPairFilter* object_vs_object_layer_filter = JPC56_ObjectLayerPairFilter_new(nullptr, Hello_OVO);

	const unsigned int cMaxBodies = 1024;
	const unsigned int cNumBodyMutexes = 0;
	const unsigned int cMaxBodyPairs = 1024;
	const unsigned int cMaxContactConstraints = 1024;

	JPC56_PhysicsSystem* physics_system = JPC56_PhysicsSystem_new();
	JPC56_PhysicsSystem_Init(
		physics_system,
		cMaxBodies,
		cNumBodyMutexes,
		cMaxBodyPairs,
		cMaxContactConstraints,
		broad_phase_layer_interface,
		object_vs_broad_phase_layer_filter,
		object_vs_object_layer_filter);

	// TODO: register body activation listener
	// TODO: register contact listener

	JPC56_BodyInterface* body_interface = JPC56_PhysicsSystem_GetBodyInterface(physics_system);

	JPC56_BoxShapeSettings floor_shape_settings;
	JPC56_BoxShapeSettings_default(&floor_shape_settings);
	floor_shape_settings.HalfExtent = JPC56_Vec3{100.0f, 1.0f, 100.0f};
	floor_shape_settings.Density = 500.0;

	JPC56_Shape* floor_shape;
	JPC56_String* err;
	if (!JPC56_BoxShapeSettings_Create(&floor_shape_settings, &floor_shape, &err)) {
		printf("fatal error: %s\n", JPC56_String_c_str(err));

		// the world is ending, but I guess we can still free memory
		JPC56_String_delete(err);

		exit(1);
	}

	JPC56_BodyCreationSettings floor_settings;
	JPC56_BodyCreationSettings_default(&floor_settings);
	floor_settings.Position = JPC56_RVec3{0.0, -1.0, 0.0};
	floor_settings.MotionType = JPC56_MOTION_TYPE_STATIC;
	floor_settings.ObjectLayer = HELLO_OL_NON_MOVING;
	floor_settings.Shape = floor_shape;

	JPC56_Body* floor = JPC56_BodyInterface_CreateBody(body_interface, &floor_settings);
	JPC56_BodyInterface_AddBody(body_interface, JPC56_Body_GetID(floor), JPC56_ACTIVATION_DONT_ACTIVATE);

	JPC56_SphereShapeSettings sphere_shape_settings;
	JPC56_SphereShapeSettings_default(&sphere_shape_settings);
	sphere_shape_settings.Radius = 0.5;

	JPC56_Shape* sphere_shape;
	if (!JPC56_SphereShapeSettings_Create(&sphere_shape_settings, &sphere_shape, &err)) {
		printf("fatal error: %s\n", JPC56_String_c_str(err));

		// the world is ending, but I guess we can still free memory
		JPC56_String_delete(err);

		exit(1);
	}

	JPC56_BodyCreationSettings sphere_settings;
	JPC56_BodyCreationSettings_default(&sphere_settings);
	sphere_settings.Position = JPC56_RVec3{0.0, 2.0, 0.0};
	sphere_settings.MotionType = JPC56_MOTION_TYPE_DYNAMIC;
	sphere_settings.ObjectLayer = HELLO_OL_MOVING;
	sphere_settings.Shape = sphere_shape;

	JPC56_Body* sphere = JPC56_BodyInterface_CreateBody(body_interface, &sphere_settings);
	JPC56_BodyID sphere_id = JPC56_Body_GetID(sphere);
	JPC56_BodyInterface_AddBody(body_interface, sphere_id, JPC56_ACTIVATION_ACTIVATE);

	JPC56_BodyInterface_SetLinearVelocity(body_interface, sphere_id, JPC56_Vec3{0.0, -5.0, 0.0});

	JPC56_DebugRendererSimple* debug_renderer = JPC56_DebugRendererSimple_new(nullptr, Hello_DebugRenderer);
	JPC56_BodyManager_DrawSettings draw_settings;
	JPC56_BodyManager_DrawSettings_default(&draw_settings);
	JPC56_PhysicsSystem_DrawBodies(physics_system, &draw_settings, debug_renderer, nullptr);

	JPC56_PhysicsSystem_OptimizeBroadPhase(physics_system);

	const float cDeltaTime = 1.0f / 60.0f;
	const int cCollisionSteps = 1;

	int step = 0;
	while (JPC56_BodyInterface_IsActive(body_interface, sphere_id)) {
		++step;

		JPC56_RVec3 position = JPC56_BodyInterface_GetCenterOfMassPosition(body_interface, sphere_id);
		JPC56_Vec3 velocity = JPC56_BodyInterface_GetLinearVelocity(body_interface, sphere_id);

		printf("Step %d: Position = (%f, %f, %f), Velocity = (%f, %f, %f)\n", step, position.x, position.y, position.z, velocity.x, velocity.y, velocity.z);

		JPC56_PhysicsSystem_Update(physics_system, cDeltaTime, cCollisionSteps, temp_allocator, (JPC56_JobSystem*) job_system);
	}

	JPC56_BodyInterface_RemoveBody(body_interface, sphere_id);
	JPC56_BodyInterface_DestroyBody(body_interface, sphere_id);

	JPC56_BodyInterface_RemoveBody(body_interface, JPC56_Body_GetID(floor));
	JPC56_BodyInterface_DestroyBody(body_interface, JPC56_Body_GetID(floor));

	JPC56_PhysicsSystem_delete(physics_system);
	JPC56_BroadPhaseLayerInterface_delete(broad_phase_layer_interface);
	JPC56_ObjectVsBroadPhaseLayerFilter_delete(object_vs_broad_phase_layer_filter);
	JPC56_ObjectLayerPairFilter_delete(object_vs_object_layer_filter);

	JPC56_JobSystemThreadPool_delete(job_system);
	JPC56_TempAllocatorImpl_delete(temp_allocator);

	JPC56_UnregisterTypes();
	JPC56_FactoryDelete();

	printf("Hello, world!\n");
}
