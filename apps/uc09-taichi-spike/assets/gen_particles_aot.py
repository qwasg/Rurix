"""G6.5 Taichi Vulkan AOT spike — particles AOT 资产生成脚本（Task 2）。

产物契约（供 Rust 宿主侧消费）：
  - kernel 名：fill_particles
  - NdArray 参数 p：elem_type = f32，shape = (64,)
  - 期望值：p[i] = i * 1.5 + 1.0（i 从 0 起）
  - AOT 目标：ti.vulkan
  - 产出物：Module.archive() 的 .tcm（zip 形态，非逐位可复现——
    sha256 核验对象为入仓产物本体，而非再生成物）

幂等：重跑覆盖同路径产物 particles.tcm。
"""

import os

import taichi as ti

ASSET_DIR = os.path.dirname(os.path.abspath(__file__))
ARCHIVE_PATH = os.path.join(ASSET_DIR, "particles.tcm")


def main() -> None:
    ti.init(arch=ti.vulkan)

    @ti.kernel
    def fill_particles(p: ti.types.ndarray(ti.f32, 1)):
        for i in p:
            p[i] = i * 1.5 + 1.0

    particles = ti.ndarray(dtype=ti.f32, shape=(64,))

    module = ti.aot.Module(ti.vulkan)
    module.add_kernel(fill_particles, template_args={"p": particles})
    module.archive(ARCHIVE_PATH)

    size = os.path.getsize(ARCHIVE_PATH)
    print(f"archived {ARCHIVE_PATH} ({size} bytes)")


if __name__ == "__main__":
    main()
