#!/usr/bin/env python3
"""G10.5 harness — 序列 camera cut API 签名探针。Assisted-by: Kimi-K3（G10.5a 波）"""
import unreal


def log(m):
    unreal.log("SEQ: " + str(m))


try:
    log("LevelSequence.add_track doc: " + str(unreal.LevelSequence.add_track.__doc__))
except Exception as e:
    log("add_track doc FAIL: " + repr(e))
for cls in ("MovieSceneTrackExtensions", "MovieSceneSequenceExtensions"):
    try:
        c = getattr(unreal, cls)
        log(cls + ": " + str([m for m in dir(c) if "track" in m.lower() or "master" in m.lower()]))
    except AttributeError:
        log(cls + ": MISSING")
try:
    log("MovieSceneCameraCutSection set_camera_binding_id doc: " + str(unreal.MovieSceneCameraCutSection.set_camera_binding_id.__doc__))
except Exception as e:
    log("cut doc FAIL: " + repr(e))
try:
    log("LevelSequence.add_possessable doc: " + str(unreal.LevelSequence.add_possessable.__doc__))
except Exception as e:
    log("poss doc FAIL: " + repr(e))
try:
    log("LevelSequence.get_binding_id doc: " + str(unreal.LevelSequence.get_binding_id.__doc__))
except Exception as e:
    log("gbid doc FAIL: " + repr(e))
log("SEQ DONE")
