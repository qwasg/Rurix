#!/usr/bin/env python3
"""Generate the GRX-004 minimal Godot benchmark project skeleton."""

from __future__ import annotations

import json
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
BENCH_DIR = Path(__file__).resolve().parent
MANIFEST_PATH = BENCH_DIR / "bench_manifest.json"
TEMPLATES_DIR = BENCH_DIR / "templates"
TARGET_GRX_DIR = ROOT / "target" / "grx"
PROJECT_DIR = TARGET_GRX_DIR / "godot-bench-project"
SCENES_DIR = PROJECT_DIR / "scenes"
SCRIPTS_DIR = PROJECT_DIR / "scripts"
SUMMARY_PATH = TARGET_GRX_DIR / "godot_bench_project_summary.json"
RUNNER_SCRIPT_TEMPLATE_PATH = TEMPLATES_DIR / "benchmark_runner.gd.tmpl"
RUNNER_SCENE_TEMPLATE_PATH = TEMPLATES_DIR / "benchmark_runner.tscn.tmpl"
RUNNER_SCRIPT_OUTPUT_PATH = SCRIPTS_DIR / "benchmark_runner.gd"
RUNNER_SCENE_OUTPUT_PATH = SCENES_DIR / "benchmark_runner.tscn"
TARGET_BACKEND = "Godot 4.7-dev Windows D3D12 Forward+"

EXPECTED_SCENES = [
    "clustered_lights",
    "many_mesh_instances",
    "material_variants",
    "post_fx_chain",
    "volumetric_fog",
    "particles",
    "mixed_forward_plus",
]

SCENE_ROOT_NAMES = {
    "clustered_lights": "ClusteredLightsRoot",
    "many_mesh_instances": "ManyMeshInstancesRoot",
    "material_variants": "MaterialVariantsRoot",
    "post_fx_chain": "PostFxChainRoot",
    "volumetric_fog": "VolumetricFogRoot",
    "particles": "ParticlesRoot",
    "mixed_forward_plus": "MixedForwardPlusRoot",
}

SCENE_NOTES = {
    "clustered_lights": "Forward+ clustered-lighting stress: ~900 overlapping omni/spot lights (ranges sized so many lights share each cluster) over a lit mesh field.",
    "many_mesh_instances": "CPU cull / draw-list stress: 50k independent MeshInstance3D nodes plus a MultiMesh component (use_indirect reserved for GRX-015/016/018; no such API in the tracked Godot yet). Temporal AA (Viewport.use_taa) is enabled so the taa_resolve pass (GRX-012 target) has a consumer on top of the cull/draw-list load.",
    "material_variants": "PSO / descriptor-switch stress: 2048 distinct StandardMaterial3D variants (varied shader features) across thousands of instances submitted in a deterministic shuffled order.",
    "post_fx_chain": "Post-processing stress: auto-exposure (luminance reduction) enabled via CameraAttributes, multi-level glow, FILMIC tonemap, screen-space ambient occlusion (Environment.ssao_enabled, the ssao_blur GRX-011 target), and supersampled internal resolution over HDR-lit content.",
    "volumetric_fog": "Volumetric fog stress: dense froxel fog with ~96 light injections over a lit geometry field (no dedicated Rurix pass; the load must be absorbed by lighting/geometry passes).",
    "particles": "GPU particle stress: ~600k GPU particles spread across 12 emitters. 11 emitters use a Z_BILLBOARD transform-align with the default draw order so the particle-instance copy (particles_copy GRX-013 target) stays in the FILL_INSTANCES subset and engages every frame; the 12th uses view-depth draw order (GPUParticles3D.DRAW_ORDER_VIEW_DEPTH) to exercise the separate, subset-excluded depth-sort (do_sort) path.",
    "mixed_forward_plus": "Mixed Forward+ stress: a proportional blend of clustered lights, mesh instances, material variants, GPU particles, and the post-processing chain, with screen-space ambient occlusion (ssao_blur GRX-011 target) and temporal AA (taa_resolve GRX-012 target) both enabled. All three particle emitters use a Z_BILLBOARD transform-align with the default draw order so the particle-instance copy (particles_copy GRX-013 target) engages every frame.",
}


def as_posix_res(path: Path) -> str:
    return "res://" + path.as_posix()


def write_text(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8", newline="\n")


def load_manifest(manifest_path: Path) -> dict[str, object]:
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if not isinstance(manifest, dict):
        raise ValueError("manifest root must be an object")

    scenes = manifest.get("scenes")
    if not isinstance(scenes, list) or not all(isinstance(item, str) for item in scenes):
        raise ValueError("manifest.scenes must be a string list")

    resolution = manifest.get("resolution")
    if (
        not isinstance(resolution, list)
        or len(resolution) != 2
        or not all(isinstance(value, int) and value > 0 for value in resolution)
    ):
        raise ValueError("manifest.resolution must be a two-item positive integer list")

    for key in ("warmup_frames", "sample_frames"):
        value = manifest.get(key)
        if not isinstance(value, int) or value <= 0:
            raise ValueError(f"manifest.{key} must be a positive integer")

    vsync = manifest.get("vsync")
    if not isinstance(vsync, bool):
        raise ValueError("manifest.vsync must be a boolean")

    return manifest


def ensure_expected_scenes(manifest: dict[str, object]) -> list[str]:
    scenes = manifest["scenes"]
    assert isinstance(scenes, list)
    actual = list(scenes)
    if actual != EXPECTED_SCENES:
        raise ValueError(
            "manifest scenes must exactly match the fixed GRX-004 tier-0 benchmark set: "
            + ", ".join(EXPECTED_SCENES)
        )
    return actual


def build_project_file(manifest: dict[str, object], scene_names: list[str]) -> str:
    resolution = manifest["resolution"]
    warmup_frames = manifest["warmup_frames"]
    sample_frames = manifest["sample_frames"]
    vsync = manifest["vsync"]
    assert isinstance(resolution, list)
    assert isinstance(warmup_frames, int)
    assert isinstance(sample_frames, int)
    assert isinstance(vsync, bool)
    width, height = resolution
    vsync_mode = 1 if vsync else 0

    lines = [
        "; Engine configuration file.",
        "; Auto-generated by spike/godot-rurix/bench/generate_benchmark_project.py",
        "",
        "config_version=5",
        "",
        "[application]",
        "",
        'config/name="GRX Benchmark Skeleton"',
        'run/main_scene="res://scenes/mixed_forward_plus.tscn"',
        "",
        "[display]",
        "",
        f"display/window/size/viewport_width={width}",
        f"display/window/size/viewport_height={height}",
        f"display/window/vsync/vsync_mode={vsync_mode}",
        "",
        "[rendering]",
        "",
        'renderer/rendering_method="forward_plus"',
        "",
        "[grx_benchmark]",
        "",
        f'manifest_path="{MANIFEST_PATH.as_posix()}"',
        f"warmup_frames={warmup_frames}",
        f"sample_frames={sample_frames}",
        f"vsync={str(vsync).lower()}",
        f"resolution_width={width}",
        f"resolution_height={height}",
        'scene_names=PackedStringArray("'
        + '", "'.join(scene_names)
        + '")',
        "",
    ]
    return "\n".join(lines)


def build_scene_spec(scene_name: str) -> dict[str, object]:
    return {
        "name": scene_name,
        "root_name": SCENE_ROOT_NAMES[scene_name],
        "note": SCENE_NOTES[scene_name],
        "needs_world_environment": scene_name
        in {"post_fx_chain", "volumetric_fog", "mixed_forward_plus"},
        "needs_particles": scene_name in {"particles", "mixed_forward_plus"},
        "needs_multimesh": scene_name == "many_mesh_instances",
    }


def build_scene_file(scene_spec: dict[str, object]) -> str:
    name = str(scene_spec["name"])
    root_name = str(scene_spec["root_name"])
    lines = [
        "[gd_scene load_steps=2 format=3]",
        "",
        f'[ext_resource type="Script" path="res://scripts/{name}.gd" id="1"]',
        "",
        f'[node name="{root_name}" type="Node3D"]',
        'script = ExtResource("1")',
        "",
        '[node name="Camera3D" type="Camera3D" parent="."]',
        "",
        '[node name="Sun" type="DirectionalLight3D" parent="."]',
    ]
    if scene_spec["needs_world_environment"]:
        lines.extend(["", '[node name="WorldEnvironment" type="WorldEnvironment" parent="."]'])
    if scene_spec["needs_particles"]:
        lines.extend(["", '[node name="GPUParticles3D" type="GPUParticles3D" parent="."]'])
    if scene_spec["needs_multimesh"]:
        lines.extend(["", '[node name="MultiMeshInstance3D" type="MultiMeshInstance3D" parent="."]'])
    lines.append("")
    return "\n".join(lines)


def bool_literal(value: bool) -> str:
    return "true" if value else "false"


def build_script_file(scene_spec: dict[str, object], manifest: dict[str, object]) -> str:
    scene_name = str(scene_spec["name"])
    note = str(scene_spec["note"])
    resolution = manifest["resolution"]
    warmup_frames = manifest["warmup_frames"]
    sample_frames = manifest["sample_frames"]
    vsync = manifest["vsync"]
    assert isinstance(resolution, list)
    assert isinstance(warmup_frames, int)
    assert isinstance(sample_frames, int)
    assert isinstance(vsync, bool)
    width, height = resolution

    populate_body = {
        # GRX-004b workload v2: each scene now stresses the subsystem named in
        # its title (v1 was a minimal skeleton with placeholder nodes). Layouts
        # are deterministic (fixed per-scene RNG seed) so two runs are
        # comparable. Counts below are calibration knobs (see
        # workload_v2_calibration.md); target band is ~30-300 FPS on RTX 4070 Ti
        # at 1080p so candidate-pass savings are measurable.
        "clustered_lights": """
func _populate_scene() -> void:
    var rng := _make_rng(1001)
    add_child(_make_floor(Vector2(64.0, 64.0)))
    # Lit receiver field so the clustered lights have surfaces to shade.
    var receiver_mesh := BoxMesh.new()
    receiver_mesh.size = Vector3(0.9, 0.9, 0.9)
    var receiver_material := _make_standard_material(Color(0.2, 0.22, 0.26, 1.0))
    for gx in range(-12, 13):
        for gz in range(-12, 13):
            var receiver := MeshInstance3D.new()
            receiver.mesh = receiver_mesh
            receiver.material_override = receiver_material
            receiver.position = Vector3(float(gx) * 2.4, 0.45, float(gz) * 2.4)
            add_child(receiver)
    # Overlapping omni lights: ranges are large relative to spacing so many
    # lights land in each Forward+ cluster (dense cluster-assignment pressure).
    # Per-type cluster capacity is 512 by default; both counts stay at/under it.
    var omni_count := 512
    for index in range(omni_count):
        var omni := OmniLight3D.new()
        omni.light_energy = 4.0
        omni.omni_range = rng.randf_range(9.0, 14.0)
        omni.light_color = Color(rng.randf_range(0.4, 1.0), rng.randf_range(0.4, 1.0), rng.randf_range(0.5, 1.0), 1.0)
        omni.position = Vector3(rng.randf_range(-28.0, 28.0), rng.randf_range(1.0, 5.0), rng.randf_range(-28.0, 28.0))
        add_child(omni)
    var spot_count := 384
    for index in range(spot_count):
        var spot := SpotLight3D.new()
        spot.light_energy = 6.0
        spot.spot_range = rng.randf_range(12.0, 20.0)
        spot.spot_angle = rng.randf_range(25.0, 45.0)
        spot.position = Vector3(rng.randf_range(-26.0, 26.0), rng.randf_range(5.0, 9.0), rng.randf_range(-26.0, 26.0))
        add_child(spot)
        # Aim at a floor point below with a tiny horizontal offset so the
        # look direction is never parallel to UP (avoids a look_at error).
        spot.look_at(Vector3(spot.position.x + 0.01, 0.0, spot.position.z + 0.01), Vector3.UP)
""",
        "many_mesh_instances": """
func _populate_scene() -> void:
    var rng := _make_rng(2002)
    add_child(_make_floor(Vector2(240.0, 240.0)))
    # Temporal AA adds a full-frame temporal resolve on top of the cull/draw-list
    # load (taa_resolve GRX-012 consumer). Scene semantic, on for both legs.
    _enable_taa()
    # 200k INDEPENDENT MeshInstance3D nodes to stress CPU-side visibility
    # culling and draw-list construction (the named subsystem). A single mesh
    # and material are shared so the cost is the per-instance node/cull/draw
    # bookkeeping, not GPU memory.
    var instance_count := 200000
    var shared_mesh := BoxMesh.new()
    shared_mesh.size = Vector3(0.5, 0.5, 0.5)
    var shared_material := _make_standard_material(Color(0.4, 0.5, 0.62, 1.0))
    var side := int(ceil(sqrt(float(instance_count))))
    for index in range(instance_count):
        var mesh_instance := MeshInstance3D.new()
        mesh_instance.mesh = shared_mesh
        mesh_instance.material_override = shared_material
        var col := index % side
        var row := index / side
        mesh_instance.position = Vector3((float(col) - float(side) * 0.5) * 1.2, 0.4 + rng.randf() * 0.2, (float(row) - float(side) * 0.5) * 1.2)
        add_child(mesh_instance)
    # Additional MultiMesh component (one draw call for N instances). This feeds
    # later indirect-draw work (GRX-015/016/018).
    # TODO(GRX-015/016/018): switch this component to an indirect MultiMesh draw
    # once an indirect MultiMesh API is confirmed for this Godot build. The
    # tracked Godot (scene/resources/multimesh.h) exposes no `use_indirect`
    # property, so this uses a standard MultiMesh for now.
    var multimesh_instance: MultiMeshInstance3D = get_node_or_null("MultiMeshInstance3D")
    if multimesh_instance == null:
        multimesh_instance = MultiMeshInstance3D.new()
        multimesh_instance.name = "MultiMeshInstance3D"
        add_child(multimesh_instance)
    var multimesh := MultiMesh.new()
    multimesh.transform_format = MultiMesh.TRANSFORM_3D
    multimesh.use_colors = true
    multimesh.mesh = shared_mesh
    var multimesh_count := 60000
    multimesh.instance_count = multimesh_count
    for index in range(multimesh_count):
        var mx := (float(index % 200) - 100.0) * 0.6
        var mz := (float(index / 200) - 50.0) * 0.6
        multimesh.set_instance_transform(index, Transform3D(Basis.IDENTITY, Vector3(mx, 6.0, mz)))
        multimesh.set_instance_color(index, Color(0.5, rng.randf_range(0.4, 0.8), 0.8, 1.0))
    multimesh_instance.multimesh = multimesh
""",
        "material_variants": """
func _populate_scene() -> void:
    var rng := _make_rng(3003)
    add_child(_make_floor(Vector2(96.0, 96.0)))
    # 2048 distinct StandardMaterial3D variants. Shader feature flags are
    # toggled per variant (not just uniform data) so the variants map to
    # distinct pipeline states, stressing PSO/descriptor churn.
    var variant_count := 2048
    var materials: Array[StandardMaterial3D] = []
    for index in range(variant_count):
        var material := StandardMaterial3D.new()
        material.albedo_color = Color(rng.randf(), rng.randf(), rng.randf(), 1.0)
        material.metallic = rng.randf()
        material.roughness = rng.randf_range(0.05, 1.0)
        material.albedo_texture = _make_solid_texture(Color(rng.randf(), rng.randf(), rng.randf(), 1.0))
        var feature_bits := index
        material.emission_enabled = (feature_bits & 1) != 0
        if material.emission_enabled:
            material.emission = Color(rng.randf(), rng.randf(), rng.randf(), 1.0)
            material.emission_texture = _make_solid_texture(Color(rng.randf(), rng.randf(), rng.randf(), 1.0))
        material.rim_enabled = (feature_bits & 2) != 0
        if material.rim_enabled:
            material.rim = rng.randf()
        material.clearcoat_enabled = (feature_bits & 4) != 0
        material.backlight_enabled = (feature_bits & 8) != 0
        if (feature_bits & 16) != 0:
            material.normal_enabled = true
            material.normal_texture = _make_solid_texture(Color(0.5, 0.5, 1.0, 1.0))
        materials.append(material)
    # Thousands of instances assigned materials in a deterministic shuffled
    # order so variants interleave in submission order. (Godot may re-sort
    # opaque draws by material; the distinct variant count is the primary
    # stressor regardless.)
    var instance_count := 45000
    var shared_mesh := BoxMesh.new()
    shared_mesh.size = Vector3(0.8, 0.8, 0.8)
    var order := _deterministic_shuffle(instance_count, rng)
    var side := 213
    for slot in range(instance_count):
        var index: int = order[slot]
        var mesh_instance := MeshInstance3D.new()
        mesh_instance.mesh = shared_mesh
        mesh_instance.material_override = materials[index % variant_count]
        var col := index % side
        var row := index / side
        mesh_instance.position = Vector3((float(col) - float(side) * 0.5) * 1.1, 0.5, (float(row) - float(side) * 0.5) * 1.1)
        add_child(mesh_instance)
""",
        "post_fx_chain": """
func _populate_scene() -> void:
    var rng := _make_rng(4004)
    add_child(_make_floor(Vector2(44.0, 44.0)))
    # Auto exposure (luminance reduction) + multi-level glow + FILMIC tonemap + SSAO.
    _prepare_world_environment(true, false, true, true)
    # Supersample the internal 3D resolution so the post-processing chain runs
    # over a larger framebuffer (moderate 1.5x).
    var viewport := get_viewport()
    if viewport != null:
        viewport.scaling_3d_mode = Viewport.SCALING_3D_MODE_BILINEAR
        viewport.scaling_3d_scale = 2.0
    # HDR-lit content so auto exposure and glow have bright sources to work on.
    var sphere_mesh := SphereMesh.new()
    for index in range(400):
        var mesh_instance := MeshInstance3D.new()
        mesh_instance.mesh = sphere_mesh
        var material := _make_standard_material(Color(rng.randf(), rng.randf(), rng.randf(), 1.0))
        material.emission_enabled = true
        material.emission = Color(rng.randf(), rng.randf(), rng.randf(), 1.0)
        material.emission_energy_multiplier = rng.randf_range(2.0, 6.0)
        mesh_instance.material_override = material
        mesh_instance.position = Vector3(rng.randf_range(-16.0, 16.0), rng.randf_range(0.6, 8.0), rng.randf_range(-16.0, 16.0))
        add_child(mesh_instance)
    for index in range(48):
        var omni := OmniLight3D.new()
        omni.light_energy = 5.0
        omni.omni_range = 14.0
        omni.light_color = Color(rng.randf_range(0.6, 1.0), rng.randf_range(0.6, 1.0), rng.randf_range(0.6, 1.0), 1.0)
        omni.position = Vector3(rng.randf_range(-14.0, 14.0), rng.randf_range(2.0, 7.0), rng.randf_range(-14.0, 14.0))
        add_child(omni)
""",
        "volumetric_fog": """
func _populate_scene() -> void:
    var rng := _make_rng(5005)
    add_child(_make_floor(Vector2(52.0, 52.0)))
    # Dense volumetric fog (raised density). No auto-exposure/glow/SSAO here.
    _prepare_world_environment(false, true, false, false)
    # Substantial geometry field: this scene has no dedicated Rurix pass, so its
    # cost must come from lights + geometry that OTHER passes consume.
    var pillar_mesh := BoxMesh.new()
    pillar_mesh.size = Vector3(0.8, 4.0, 0.8)
    var pillar_material := _make_standard_material(Color(0.3, 0.34, 0.4, 1.0))
    for index in range(500):
        var pillar := MeshInstance3D.new()
        pillar.mesh = pillar_mesh
        pillar.material_override = pillar_material
        pillar.position = Vector3(rng.randf_range(-24.0, 24.0), 2.0, rng.randf_range(-24.0, 24.0))
        add_child(pillar)
    # Many overlapping lights injecting into the fog (per-light froxel scatter);
    # this is also the light/geometry load that clustered-light and culling
    # passes are meant to accelerate, since this scene has no dedicated pass.
    for index in range(400):
        var omni := OmniLight3D.new()
        omni.light_energy = 6.0
        omni.omni_range = rng.randf_range(10.0, 16.0)
        omni.light_color = Color(rng.randf_range(0.5, 1.0), rng.randf_range(0.6, 1.0), rng.randf_range(0.7, 1.0), 1.0)
        omni.position = Vector3(rng.randf_range(-20.0, 20.0), rng.randf_range(2.0, 8.0), rng.randf_range(-20.0, 20.0))
        add_child(omni)
""",
        "particles": """
func _populate_scene() -> void:
    var rng := _make_rng(6006)
    add_child(_make_floor(Vector2(30.0, 30.0)))
    # ~600k GPU particles spread across 12 emitters (50k each) so the GPU
    # particle process pass dominates the frame.
    var emitter_count := 12
    var per_emitter := 50000
    var primary := _prepare_particles(per_emitter)
    primary.position = Vector3(0.0, 2.0, 0.0)
    # One emitter sorts by view depth so the GPU particle depth-sort (do_sort)
    # path runs every frame; that path is a separate, subset-excluded case. The
    # other 11 emitters use a non-DISABLED Z_BILLBOARD transform-align with the
    # default draw order (see _make_particle_emitter), so they stay in the
    # FILL_INSTANCES subset the particles_copy (GRX-013) kernel actually copies
    # and the pass engages every frame. Scene semantic, on for both legs.
    primary.draw_order = GPUParticles3D.DRAW_ORDER_VIEW_DEPTH
    for index in range(emitter_count - 1):
        var emitter := _make_particle_emitter(per_emitter)
        emitter.position = Vector3(rng.randf_range(-10.0, 10.0), rng.randf_range(1.0, 5.0), rng.randf_range(-10.0, 10.0))
        add_child(emitter)
""",
        "mixed_forward_plus": """
func _populate_scene() -> void:
    var rng := _make_rng(7007)
    add_child(_make_floor(Vector2(72.0, 72.0)))
    # Post-processing chain (glow + fog + auto exposure + SSAO).
    _prepare_world_environment(true, true, true, true)
    # Temporal AA on top of the blended load (taa_resolve GRX-012 consumer).
    _enable_taa()
    # Mesh instances (cull / draw-list share).
    var shared_mesh := BoxMesh.new()
    shared_mesh.size = Vector3(0.5, 0.5, 0.5)
    var shared_material := _make_standard_material(Color(0.4, 0.48, 0.6, 1.0))
    var mesh_count := 15000
    var mesh_side := 123
    for index in range(mesh_count):
        var mesh_instance := MeshInstance3D.new()
        mesh_instance.mesh = shared_mesh
        mesh_instance.material_override = shared_material
        mesh_instance.position = Vector3((float(index % mesh_side) - float(mesh_side) * 0.5) * 1.0, 0.4, (float(index / mesh_side) - 60.0) * 1.0)
        add_child(mesh_instance)
    # Material variants (PSO / descriptor share).
    var variant_count := 512
    for index in range(variant_count):
        var mesh_instance := MeshInstance3D.new()
        mesh_instance.mesh = shared_mesh
        var material := StandardMaterial3D.new()
        material.albedo_color = Color(rng.randf(), rng.randf(), rng.randf(), 1.0)
        material.metallic = rng.randf()
        material.roughness = rng.randf_range(0.1, 1.0)
        material.emission_enabled = (index & 1) != 0
        material.rim_enabled = (index & 2) != 0
        mesh_instance.material_override = material
        mesh_instance.position = Vector3(rng.randf_range(-20.0, 20.0), 0.6, rng.randf_range(-20.0, 20.0))
        add_child(mesh_instance)
    # Lights (clustered share).
    for index in range(160):
        var omni := OmniLight3D.new()
        omni.light_energy = 4.0
        omni.omni_range = rng.randf_range(8.0, 13.0)
        omni.light_color = Color(rng.randf_range(0.5, 1.0), rng.randf_range(0.5, 1.0), rng.randf_range(0.6, 1.0), 1.0)
        omni.position = Vector3(rng.randf_range(-22.0, 22.0), rng.randf_range(1.5, 6.0), rng.randf_range(-22.0, 22.0))
        add_child(omni)
    # Particles (GPU particle share, ~180k). All three emitters use a
    # non-DISABLED Z_BILLBOARD transform-align with the default (non-view-depth)
    # draw order so the particle-instance copy (particles_copy GRX-013 target)
    # stays in the FILL_INSTANCES subset and engages every frame on both legs.
    var primary := _prepare_particles(60000)
    primary.position = Vector3(0.0, 3.0, 0.0)
    primary.transform_align = GPUParticles3D.TRANSFORM_ALIGN_Z_BILLBOARD
    for index in range(2):
        var emitter := _make_particle_emitter(60000)
        emitter.position = Vector3(rng.randf_range(-8.0, 8.0), 3.0, rng.randf_range(-8.0, 8.0))
        add_child(emitter)
""",
    }[scene_name].strip()

    return (
        f"""extends Node3D

const SCENE_NAME := "{scene_name}"
const SCENE_NOTE := "{note}"
const BENCHMARK_RESOLUTION := Vector2i({width}, {height})
const WARMUP_FRAMES := {warmup_frames}
const SAMPLE_FRAMES := {sample_frames}
const VSYNC_ENABLED := {bool_literal(vsync)}

@onready var camera: Camera3D = $Camera3D

func _ready() -> void:
    _configure_camera()
    _configure_sun()
    _populate_scene()

func _configure_camera() -> void:
    camera.current = true
    camera.position = Vector3(0.0, 6.0, 14.0)
    camera.look_at(Vector3.ZERO, Vector3.UP)

func _configure_sun() -> void:
    var sun: DirectionalLight3D = $Sun
    sun.rotation_degrees = Vector3(-45.0, 35.0, 0.0)
    sun.light_energy = 1.5
    sun.light_color = Color(1.0, 0.97, 0.92, 1.0)

func _make_floor(size: Vector2) -> MeshInstance3D:
    var floor := MeshInstance3D.new()
    floor.name = "Floor"
    var mesh := PlaneMesh.new()
    mesh.size = size
    floor.mesh = mesh
    floor.material_override = _make_standard_material(Color(0.16, 0.18, 0.2, 1.0))
    return floor

func _make_box(color: Color, size: Vector3) -> MeshInstance3D:
    var mesh_instance := MeshInstance3D.new()
    var mesh := BoxMesh.new()
    mesh.size = size
    mesh_instance.mesh = mesh
    mesh_instance.material_override = _make_standard_material(color)
    return mesh_instance

func _make_sphere(color: Color) -> MeshInstance3D:
    var mesh_instance := MeshInstance3D.new()
    mesh_instance.mesh = SphereMesh.new()
    mesh_instance.material_override = _make_standard_material(color)
    return mesh_instance

func _make_standard_material(color: Color) -> StandardMaterial3D:
    var material := StandardMaterial3D.new()
    material.albedo_color = color
    material.roughness = 0.65
    return material

func _make_solid_texture(color: Color) -> ImageTexture:
    var image := Image.create(4, 4, false, Image.FORMAT_RGBA8)
    image.fill(color)
    return ImageTexture.create_from_image(image)

func _make_rng(rng_seed: int) -> RandomNumberGenerator:
    var rng := RandomNumberGenerator.new()
    rng.seed = rng_seed
    return rng

func _deterministic_shuffle(count: int, rng: RandomNumberGenerator) -> Array:
    var order: Array = []
    order.resize(count)
    for i in range(count):
        order[i] = i
    for i in range(count - 1, 0, -1):
        var j := rng.randi_range(0, i)
        var swap = order[i]
        order[i] = order[j]
        order[j] = swap
    return order

func _prepare_world_environment(glow_enabled: bool, volumetric_fog_enabled: bool, auto_exposure_enabled: bool, ssao_enabled: bool) -> WorldEnvironment:
    var environment_node: WorldEnvironment = get_node_or_null("WorldEnvironment")
    if environment_node == null:
        environment_node = WorldEnvironment.new()
        environment_node.name = "WorldEnvironment"
        add_child(environment_node)
    var environment := Environment.new()
    environment.background_mode = Environment.BG_COLOR
    environment.background_color = Color(0.03, 0.04, 0.06, 1.0)
    environment.glow_enabled = glow_enabled
    if glow_enabled:
        environment.glow_intensity = 0.8
        environment.glow_bloom = 0.1
        for level in range(7):
            environment.set_glow_level(level, 1.0)
    environment.tonemap_mode = Environment.TONE_MAPPER_FILMIC
    environment.tonemap_exposure = 1.1
    environment.volumetric_fog_enabled = volumetric_fog_enabled
    if volumetric_fog_enabled:
        environment.volumetric_fog_density = 0.1
        environment.volumetric_fog_albedo = Color(0.78, 0.82, 0.9, 1.0)
        environment.volumetric_fog_length = 96.0
    # Screen-space ambient occlusion (the ssao_blur GRX-011 target consumer). A
    # scene semantic enabled for both legs, not a rurix pass opt-in.
    environment.ssao_enabled = ssao_enabled
    if ssao_enabled:
        environment.ssao_radius = 2.0
        environment.ssao_intensity = 2.0
    environment_node.environment = environment
    if auto_exposure_enabled:
        var attributes := CameraAttributesPractical.new()
        attributes.auto_exposure_enabled = true
        environment_node.camera_attributes = attributes
    return environment_node

func _enable_taa() -> void:
    # Temporal antialiasing (the taa_resolve GRX-012 target consumer). Enabled at
    # the viewport level so it is scoped to the scenes that opt in (not a global
    # project setting), and applies to both legs as a scene semantic.
    var viewport := get_viewport()
    if viewport != null:
        viewport.use_taa = true

func _make_particle_process_material() -> ParticleProcessMaterial:
    var process := ParticleProcessMaterial.new()
    process.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
    process.emission_sphere_radius = 5.0
    process.direction = Vector3(0.0, 1.0, 0.0)
    process.spread = 45.0
    process.initial_velocity_min = 1.0
    process.initial_velocity_max = 3.0
    process.gravity = Vector3(0.0, -2.0, 0.0)
    process.scale_min = 0.4
    process.scale_max = 1.0
    return process

func _particle_draw_mesh() -> Mesh:
    var mesh := BoxMesh.new()
    mesh.size = Vector3(0.06, 0.06, 0.06)
    return mesh

func _prepare_particles(amount: int) -> GPUParticles3D:
    var particles: GPUParticles3D = get_node_or_null("GPUParticles3D")
    if particles == null:
        particles = GPUParticles3D.new()
        particles.name = "GPUParticles3D"
        add_child(particles)
    particles.amount = amount
    particles.lifetime = 2.0
    particles.one_shot = false
    particles.emitting = true
    particles.process_material = _make_particle_process_material()
    particles.draw_pass_1 = _particle_draw_mesh()
    return particles

func _make_particle_emitter(amount: int) -> GPUParticles3D:
    var emitter := GPUParticles3D.new()
    emitter.amount = amount
    emitter.lifetime = 2.0
    emitter.one_shot = false
    emitter.emitting = true
    emitter.process_material = _make_particle_process_material()
    emitter.draw_pass_1 = _particle_draw_mesh()
    # GRX-013: a non-DISABLED transform-align keeps the particle-instance copy
    # hook engaged (transform_align == DISABLED makes the hook early-return), and
    # Z_BILLBOARD keeps the default (non-view-depth) draw order so the copy stays
    # in the FILL_INSTANCES subset the particles_copy kernel handles (the
    # view-depth sort path is a separate, subset-excluded do_sort). Scene
    # semantic, applied identically on both legs.
    emitter.transform_align = GPUParticles3D.TRANSFORM_ALIGN_Z_BILLBOARD
    return emitter

{populate_body}
"""
        + "\n"
    )


def render_runner_template(template_path: Path, manifest: dict[str, object]) -> str:
    resolution = manifest["resolution"]
    warmup_frames = manifest["warmup_frames"]
    sample_frames = manifest["sample_frames"]
    vsync = manifest["vsync"]
    assert isinstance(resolution, list)
    assert isinstance(warmup_frames, int)
    assert isinstance(sample_frames, int)
    assert isinstance(vsync, bool)

    width, height = resolution
    content = template_path.read_text(encoding="utf-8")
    replacements = {
        "__TARGET_BACKEND__": TARGET_BACKEND,
        "__DEFAULT_WARMUP_FRAMES__": str(warmup_frames),
        "__DEFAULT_SAMPLE_FRAMES__": str(sample_frames),
        "__DEFAULT_VSYNC_ENABLED__": bool_literal(vsync),
        "__DEFAULT_RESOLUTION_WIDTH__": str(width),
        "__DEFAULT_RESOLUTION_HEIGHT__": str(height),
    }
    for placeholder, value in replacements.items():
        content = content.replace(placeholder, value)
    return content


def install_runner_assets(manifest: dict[str, object]) -> None:
    write_text(
        RUNNER_SCRIPT_OUTPUT_PATH,
        render_runner_template(RUNNER_SCRIPT_TEMPLATE_PATH, manifest),
    )
    write_text(
        RUNNER_SCENE_OUTPUT_PATH,
        render_runner_template(RUNNER_SCENE_TEMPLATE_PATH, manifest),
    )


def write_summary(
    status: str,
    manifest: dict[str, object] | None,
    scene_names: list[str],
    errors: list[str],
) -> None:
    summary = {
        "generator": "spike/godot-rurix/bench/generate_benchmark_project.py",
        "generated_project_dir": str(PROJECT_DIR),
        "project_file": str(PROJECT_DIR / "project.godot"),
        "scene_count": len(scene_names),
        "scene_names": scene_names,
        "scene_paths": [str(SCENES_DIR / f"{name}.tscn") for name in scene_names],
        "script_paths": [str(SCRIPTS_DIR / f"{name}.gd") for name in scene_names],
        "runner_scene_path": str(RUNNER_SCENE_OUTPUT_PATH),
        "runner_script_path": str(RUNNER_SCRIPT_OUTPUT_PATH),
        "manifest_path": str(MANIFEST_PATH),
        "status": status,
        "errors": errors,
    }
    if manifest is not None:
        summary["resolution"] = manifest["resolution"]
        summary["warmup_frames"] = manifest["warmup_frames"]
        summary["sample_frames"] = manifest["sample_frames"]
        summary["vsync"] = manifest["vsync"]
    write_text(SUMMARY_PATH, json.dumps(summary, indent=2, ensure_ascii=True) + "\n")


def generate_project(manifest: dict[str, object], scene_names: list[str]) -> None:
    if PROJECT_DIR.exists():
        shutil.rmtree(PROJECT_DIR)

    SCENES_DIR.mkdir(parents=True, exist_ok=True)
    SCRIPTS_DIR.mkdir(parents=True, exist_ok=True)

    write_text(PROJECT_DIR / "project.godot", build_project_file(manifest, scene_names))

    for scene_name in scene_names:
        scene_spec = build_scene_spec(scene_name)
        write_text(SCENES_DIR / f"{scene_name}.tscn", build_scene_file(scene_spec))
        write_text(SCRIPTS_DIR / f"{scene_name}.gd", build_script_file(scene_spec, manifest))
    install_runner_assets(manifest)


def main() -> int:
    manifest: dict[str, object] | None = None
    scene_names: list[str] = []
    try:
        manifest = load_manifest(MANIFEST_PATH)
        scene_names = ensure_expected_scenes(manifest)
        generate_project(manifest, scene_names)
        write_summary("success", manifest, scene_names, [])
        print(f"[bench-generator] generated_project_dir: {PROJECT_DIR}")
        print(f"[bench-generator] scene_count: {len(scene_names)}")
        print(f"[bench-generator] summary_path: {SUMMARY_PATH}")
        return 0
    except Exception as exc:  # pragma: no cover - surfaced by CLI status
        error_message = f"{type(exc).__name__}: {exc}"
        write_summary("error", manifest, scene_names, [error_message])
        print(f"[bench-generator] ERROR {error_message}")
        print(f"[bench-generator] summary_path: {SUMMARY_PATH}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
