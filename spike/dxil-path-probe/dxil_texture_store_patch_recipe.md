# texture store DXIL 后端 LLVM patch — 可复现 recipe + dev 工具链解锁说明

> 跟踪条目:`registry/deferred.json` **RD-025**(受控、dev-only、临时工具链偏差,镜像 RD-011)。
> 目标:GRX-009 纹理线 —— 让 rurixc 的 DXIL compute `RWTexture2D<f32>` texel store 走通真实 llc,
> 与上游已合并的 texture load(`llvm.dx.resource.load.level`,PR #193343)配套。
> Provenance:`Assisted-by: fable:claude-opus-4-8`。

---

## 0. 这是什么 / 边界

上游 LLVM DirectX 后端已合并 **texture load**（`llvm.dx.resource.load.level` → `dx.op.textureLoad(66)`，
PR #193343），但 **texture store 是上游缺口**（`llvm/llvm-project` issue #194930 逐文件设计，未合入）。
rurixc 的 `RWTexture2D<f32>` texel store 需要 llc 能把一个 store intrinsic 降为 `dx.op.textureStore(67)`。
本 patch 在仓外 LLVM 工作区实现该 store 路径，并顺带修一处 load/store 共有的动态坐标 validation 缺口。

**严格边界（同 RD-011）**：
- patch 二进制 / 重建 llc **不入库**，仅隔离于仓库外。本仓库内只存本 recipe（diff 文本 + 步骤）。
- **不静默改** committed D-205 pin（`C:\Program Files\LLVM`）/ `toolchain.rs` / `src/`。dev 使用经**显式 `RURIX_LLC` env 覆盖**。
- 本偏差为**临时**：退役条件 = 上游 merge（issue #194930）+ release + D-205/`RURIX_LLC` pin bump（属 owner 独立决策）。
- 本 recipe 不做 canonical 切换（`texture_artifact_provenance_policy.md` revert 条款 2-4 未满足：math parity + 真 D3D12 dispatch）。

## 1. 隔离环境（仓库外，不入库）

| 项 | 路径 | 说明 |
|---|---|---|
| LLVM fork worktree | `H:\llvm-clean-82c5bce5-src` | HEAD 含本地 PSV commit（626063a6b，RD-011）；本 patch 追加单 commit **2afad69a7** |
| LLVM build（assertions+PDB） | `H:\llvm-clean-82c5bce5-build` | 含 `bin/llc.exe`、`bin/opt.exe`、`bin/FileCheck.exe`（LLVM 23.0.0git + 本 patch） |
| 2026 签名 validator | `H:\dxc-round7\extractedind` | `dxc.exe` / `dxv.exe` / `dxil.dll`，DXC 1.9.2602.24 |
| ×8 稳定性探针 | `spike/godot-rurix/passes/luminance_reduction/artifacts/toolchain_probe/case_L.ll` | load.level + store.texture 综合案例 |

## 2. 缺陷定位

- **store intrinsic 缺失**：`llvm/include/llvm/IR/IntrinsicsDirectX.td` 只有 `int_dx_resource_load_level`，无 store 对应物；任何 llc 按名拒 store。
- **DXILOp 缺失**：`llvm/lib/Target/DirectX/DXIL.td` 有 `textureStore` opClass（占位）但无 `DXILOp<67, textureStore>` 定义。
- **lowering 缺失**：`DXILOpLowering.cpp` 无 `lowerTextureStore`；`DXILResourceAccess.cpp::createStoreIntrinsic` 对 texture kind `reportFatalUsageError("DXIL Store not implemented for texture resources")`。
- **动态坐标 validation 缺口（load+store 共有）**：texel 坐标以 `insertelement` 组装 `<2 x i32>`；`extractElementsIntoArgs` 的 IRBuilder 只做常量折叠 → 动态 `insertelement`/`extractelement` 残留进最终 DXIL，dxv 拒（"Instructions must be of an allowed type"）。常量 `zeroinitializer` 坐标（1D `(idx,0)`）不受影响。

## 3. patch diff 全文（commit 2afad69a7 的 4 个源文件；lit 测试见 §5）

```diff
commit 2afad69a74a182e06ec724973f6c78b550a59515
Author: qwasg <141140493+qwasg@users.noreply.github.com>
Date:   Sat Jul 11 19:20:41 2026 +0800

    [DirectX] Implement dx.op.textureStore (67) lowering for RWTexture
    
    Add the texture store path that mirrors the merged texture load.level
    support (PR #193343). Introduces the `llvm.dx.resource.store.texture`
    intrinsic (handle, coords, value), the `DXILOp<67, textureStore>`
    operation, its lowering in DXILOpLowering (coords extracted into
    Coord0..2, value splatted across Val0..3 with write mask 15 to match
    DXC's typed-resource store), and the DXILResourceAccess transform from
    `getpointer` + `store` (whole-texel direct store; scalar-component store
    via load/insertelement/store).
    
    Also fold texel coordinates assembled from an insertelement chain (e.g.
    SV_DispatchThreadID components) directly into the dx.op scalar arguments
    and erase the now-dead coordinate vector, so dynamic texture load/store
    coordinates do not leave residual vector insert/extractelement ops that
    fail DXIL validation. This applies to both the new store path and the
    existing texture load path.
    
    Tests: TextureStore.ll, ResourceAccess/store_texture.ll, and
    TextureLoadStoreDynamicCoords.ll (dynamic-coordinate folding).
    
    This is a local, not-yet-upstreamed patch tracking the design in
    llvm/llvm-project issue #194930; the load side is already upstream.

diff --git a/llvm/include/llvm/IR/IntrinsicsDirectX.td b/llvm/include/llvm/IR/IntrinsicsDirectX.td
index af360dfc7..45eb42ac8 100644
--- a/llvm/include/llvm/IR/IntrinsicsDirectX.td
+++ b/llvm/include/llvm/IR/IntrinsicsDirectX.td
@@ -133,6 +133,10 @@ def int_dx_resource_load_level
                             [llvm_any_ty, llvm_any_ty, llvm_any_ty,
                              llvm_any_ty],
                             [IntrReadMem]>;
+def int_dx_resource_store_texture
+    : DefaultAttrsIntrinsic<[],
+                            [llvm_any_ty, llvm_any_ty, llvm_any_ty],
+                            [IntrWriteMem]>;
 
 def int_dx_resource_calculate_lod
     : DefaultAttrsIntrinsic<[llvm_float_ty],
diff --git a/llvm/lib/Target/DirectX/DXIL.td b/llvm/lib/Target/DirectX/DXIL.td
index 299d2d113..d4e13309d 100644
--- a/llvm/lib/Target/DirectX/DXIL.td
+++ b/llvm/lib/Target/DirectX/DXIL.td
@@ -959,6 +959,18 @@ def TextureLoad : DXILOp<66, textureLoad> {
   let attributes = [Attributes<DXIL1_0, [ReadOnly]>];
 }
 
+def TextureStore : DXILOp<67, textureStore> {
+  let Doc = "writes to a texture resource";
+  // Handle, Coord0, Coord1, Coord2, Val0, Val1, Val2, Val3, Mask
+  let arguments = [
+    HandleTy, Int32Ty, Int32Ty, Int32Ty, OverloadTy, OverloadTy, OverloadTy,
+    OverloadTy, Int8Ty
+  ];
+  let result = VoidTy;
+  let overloads = [Overloads<DXIL1_0, [HalfTy, FloatTy, Int16Ty, Int32Ty]>];
+  let stages = [Stages<DXIL1_0, [all_stages]>];
+}
+
 def BufferLoad : DXILOp<68, bufferLoad> {
   let Doc = "reads from a TypedBuffer";
   // Handle, Coord0, Coord1
diff --git a/llvm/lib/Target/DirectX/DXILOpLowering.cpp b/llvm/lib/Target/DirectX/DXILOpLowering.cpp
index 93d5a08a6..2bb5ac32e 100644
--- a/llvm/lib/Target/DirectX/DXILOpLowering.cpp
+++ b/llvm/lib/Target/DirectX/DXILOpLowering.cpp
@@ -590,9 +590,47 @@ public:
     });
   }
 
+  // Folds `extractelement(insertelement(..., V, Idx), Idx)` to `V` by walking a
+  // constant-indexed insertelement chain. Returns nullptr if the element cannot
+  // be resolved statically (non-constant insert index, or a base that is not an
+  // insertelement). DXIL forbids vector insert/extractelement in the final
+  // module, and the op-lowering IRBuilder only constant-folds; without this,
+  // dynamic texel coordinates (assembled as an insertelement vector, e.g. from
+  // SV_DispatchThreadID components) leave residual vector ops that fail
+  // validation.
+  static Value *foldExtractFromInsertChain(Value *Src, unsigned Idx) {
+    Value *Cur = Src;
+    while (auto *IEI = dyn_cast<InsertElementInst>(Cur)) {
+      auto *CIdx = dyn_cast<ConstantInt>(IEI->getOperand(2));
+      if (!CIdx)
+        return nullptr;
+      if (CIdx->getZExtValue() == Idx)
+        return IEI->getOperand(1);
+      Cur = IEI->getOperand(0);
+    }
+    return nullptr;
+  }
+
+  // Erases a now-dead insertelement chain (e.g. a coordinate vector whose
+  // elements were forwarded by foldExtractFromInsertChain and whose only other
+  // use, the original resource intrinsic, has been removed). Mirrors the
+  // leftover-insertelement cleanup in lowerBufferStore so no residual vector
+  // instruction reaches the DXIL validator.
+  static void eraseDeadInsertChain(Value *V) {
+    auto *IEI = dyn_cast<InsertElementInst>(V);
+    while (IEI && IEI->use_empty()) {
+      InsertElementInst *Tmp = IEI;
+      IEI = dyn_cast<InsertElementInst>(IEI->getOperand(0));
+      Tmp->eraseFromParent();
+    }
+  }
+
   // Copies `Src` into `Args` starting at `ArgIdx`. If `Src` is a vector, its
   // elements are extracted and stored in consecutive slots; otherwise `Src`
-  // is stored directly. At most `MaxElements` elements are expected.
+  // is stored directly. At most `MaxElements` elements are expected. When an
+  // element comes from a statically-resolvable insertelement chain the inserted
+  // scalar is used directly (see foldExtractFromInsertChain) so no residual
+  // vector extract survives into the final DXIL.
   static void extractElementsIntoArgs(IRBuilder<> &IRB,
                                       MutableArrayRef<Value *> Args,
                                       unsigned ArgIdx, Value *Src,
@@ -601,8 +639,12 @@ public:
     if (auto *VecTy = dyn_cast<FixedVectorType>(Ty)) {
       unsigned Count = VecTy->getNumElements();
       assert(Count <= MaxElements && "Expected at most 3 elements in vector");
-      for (unsigned I = 0; I < Count; ++I)
-        Args[ArgIdx + I] = IRB.CreateExtractElement(Src, uint64_t(I));
+      for (unsigned I = 0; I < Count; ++I) {
+        if (Value *Folded = foldExtractFromInsertChain(Src, I))
+          Args[ArgIdx + I] = Folded;
+        else
+          Args[ArgIdx + I] = IRB.CreateExtractElement(Src, uint64_t(I));
+      }
     } else {
       Args[ArgIdx] = Src;
     }
@@ -651,6 +693,88 @@ public:
       if (Error E = replaceResRetUses(CI, *OpCall, /*HasCheckBit=*/false))
         return E;
 
+      // The original coordinate vector (if built from an insertelement chain)
+      // is now dead; erase it so no vector op survives into the DXIL.
+      eraseDeadInsertChain(Coords);
+
+      return Error::success();
+    });
+  }
+
+  [[nodiscard]] bool lowerTextureStore(Function &F) {
+    const DataLayout &DL = F.getDataLayout();
+    IRBuilder<> &IRB = OpBuilder.getIRB();
+    Type *Int8Ty = IRB.getInt8Ty();
+    Type *Int32Ty = IRB.getInt32Ty();
+
+    return replaceFunction(F, [&](CallInst *CI) -> Error {
+      IRB.SetInsertPoint(CI);
+
+      Value *Handle =
+          createTmpHandleCast(CI->getArgOperand(0), OpBuilder.getHandleType());
+      Value *Coords = CI->getArgOperand(1);
+      Value *Data = CI->getArgOperand(2);
+
+      Type *DataTy = Data->getType();
+      Type *ScalarTy = DataTy->getScalarType();
+      uint64_t NumElements =
+          DL.getTypeSizeInBits(DataTy) / DL.getTypeSizeInBits(ScalarTy);
+      if (NumElements > 4)
+        return make_error<StringError>(
+            "Texture store data must have at most 4 elements",
+            inconvertibleErrorCode());
+
+      std::array<Value *, 4> DataElements{nullptr, nullptr, nullptr, nullptr};
+      if (DataTy == ScalarTy)
+        DataElements[0] = Data;
+      else {
+        // Since we're post-scalarizer, if we see a vector here it's likely
+        // constructed solely for the argument of the store. Just use the scalar
+        // values from before they're inserted into the temporary.
+        auto *IEI = dyn_cast<InsertElementInst>(Data);
+        while (IEI) {
+          auto *IndexOp = dyn_cast<ConstantInt>(IEI->getOperand(2));
+          if (!IndexOp)
+            break;
+          size_t IndexVal = IndexOp->getZExtValue();
+          assert(IndexVal < 4 && "Too many elements for texture store");
+          DataElements[IndexVal] = IEI->getOperand(1);
+          IEI = dyn_cast<InsertElementInst>(IEI->getOperand(0));
+        }
+      }
+
+      // If for some reason we weren't able to forward the arguments from the
+      // scalarizer artifact, then extract elements from the vector directly.
+      for (int I = 0, E = NumElements; I < E; ++I)
+        if (DataElements[I] == nullptr)
+          DataElements[I] =
+              IRB.CreateExtractElement(Data, ConstantInt::get(Int32Ty, I));
+
+      // For any elements beyond the length of the value, repeat the first
+      // element to match DXC (typed resource store).
+      for (int I = NumElements, E = 4; I < E; ++I)
+        if (DataElements[I] == nullptr)
+          DataElements[I] = DataElements[0];
+
+      // Coord0..2, undef-filled for lower-dimensional textures.
+      Value *UndefI = UndefValue::get(Int32Ty);
+      std::array<Value *, 9> Args{Handle,          UndefI,
+                                  UndefI,          UndefI,
+                                  DataElements[0], DataElements[1],
+                                  DataElements[2], DataElements[3],
+                                  ConstantInt::get(Int8Ty, 15U)};
+      extractElementsIntoArgs(IRB, Args, 1, Coords, 3);
+
+      Expected<CallInst *> OpCall =
+          OpBuilder.tryCreateOp(OpCode::TextureStore, Args, CI->getName());
+      if (Error E = OpCall.takeError())
+        return E;
+
+      CI->eraseFromParent();
+      // Clean up any leftover `insertelement`s (coordinate vector + value).
+      eraseDeadInsertChain(Coords);
+      eraseDeadInsertChain(Data);
+
       return Error::success();
     });
   }
@@ -1172,6 +1296,9 @@ public:
       case Intrinsic::dx_resource_load_level:
         HasErrors |= lowerTextureLoad(F);
         break;
+      case Intrinsic::dx_resource_store_texture:
+        HasErrors |= lowerTextureStore(F);
+        break;
       case Intrinsic::dx_resource_sample:
         HasErrors |= lowerSample(F, /*HasClamp=*/false);
         break;
diff --git a/llvm/lib/Target/DirectX/DXILResourceAccess.cpp b/llvm/lib/Target/DirectX/DXILResourceAccess.cpp
index 25d860e61..36ef0174b 100644
--- a/llvm/lib/Target/DirectX/DXILResourceAccess.cpp
+++ b/llvm/lib/Target/DirectX/DXILResourceAccess.cpp
@@ -198,6 +198,51 @@ static void createRawStores(IntrinsicInst *II, StoreInst *SI,
     emitRawStore(Builder, II->getOperand(0), Index, Offset, V, RTI);
 }
 
+static void createTextureStore(IntrinsicInst *II, StoreInst *SI,
+                               dxil::ResourceTypeInfo &RTI) {
+  const DataLayout &DL = SI->getDataLayout();
+  IRBuilder<> Builder(SI);
+  Type *ContainedType = RTI.getHandleTy()->getTypeParameter(0);
+  Type *ScalarType = ContainedType->getScalarType();
+
+  Value *Handle = II->getOperand(0);
+  Value *Coords = II->getOperand(1);
+
+  Value *V = SI->getValueOperand();
+  if (V->getType() == ContainedType) {
+    // V is already the right (whole-texel) type.
+    assert(SI->getPointerOperand() == II &&
+           "Store of whole element has mismatched address to store to");
+  } else if (V->getType() == ScalarType) {
+    // We're storing a scalar into one component, so we need to load the current
+    // texel and only replace the relevant part.
+    Value *MipLevel = Builder.getInt32(0);
+    Type *OffsetTy;
+    if (auto *VecTy = dyn_cast<FixedVectorType>(Coords->getType()))
+      OffsetTy =
+          FixedVectorType::get(Builder.getInt32Ty(), VecTy->getNumElements());
+    else
+      OffsetTy = Builder.getInt32Ty();
+    Value *Offsets = Constant::getNullValue(OffsetTy);
+
+    Value *Load = Builder.CreateIntrinsic(
+        ContainedType, Intrinsic::dx_resource_load_level,
+        {Handle, Coords, MipLevel, Offsets});
+
+    uint64_t AccessSize = DL.getTypeSizeInBits(ScalarType) / 8;
+    Value *Offset =
+        traverseGEPOffsets(DL, Builder, SI->getPointerOperand(), AccessSize);
+    V = Builder.CreateInsertElement(Load, V, Offset);
+  } else {
+    llvm_unreachable("Store to texture resource has invalid type");
+  }
+
+  auto *Inst = Builder.CreateIntrinsic(Builder.getVoidTy(),
+                                       Intrinsic::dx_resource_store_texture,
+                                       {Handle, Coords, V});
+  SI->replaceAllUsesWith(Inst);
+}
+
 static void createStoreIntrinsic(IntrinsicInst *II, StoreInst *SI,
                                  dxil::ResourceTypeInfo &RTI) {
   switch (RTI.getResourceKind()) {
@@ -215,6 +260,7 @@ static void createStoreIntrinsic(IntrinsicInst *II, StoreInst *SI,
   case dxil::ResourceKind::Texture2DArray:
   case dxil::ResourceKind::Texture2DMSArray:
   case dxil::ResourceKind::TextureCubeArray:
+    return createTextureStore(II, SI, RTI);
   case dxil::ResourceKind::FeedbackTexture2D:
   case dxil::ResourceKind::FeedbackTexture2DArray:
     reportFatalUsageError("DXIL Store not implemented for texture resources");

```

## 4. 重建 + 复验步骤

```
:: vcvars64 环境下，严格 -j 6（.td 改动触发 tablegen 再生成）
ninja -C H:\llvm-clean-82c5bce5-build -j 6 llc llvm-as opt FileCheck not count

:: lit 回归（新增 3 测试 pass + 存量不回归；DirectX 全套 465 pass / 1 unsupported(zlib) / 1 XFAIL）
py -3 H:\llvm-clean-82c5bce5-buildin\llvm-lit.py -j 6 H:\llvm-clean-82c5bce5-src\llvm	est\CodeGen\DirectX
```

## 5. lit 测试（同 commit 2afad69a7）

| 测试 | pass | 作用 |
|---|---|---|
| `TextureStore.ll`（`opt -dxil-op-lower`） | ✅ | store.texture → dx.op.textureStore(67)：coord0..2 + value splat 4×（scalar splat / int3 repeat-first）+ mask 15 |
| `ResourceAccess/store_texture.ll`（`opt -dxil-resource-access`） | ✅ | `getpointer`+`store` → store.texture（整 texel 直写；标量分量走 load/insertelement/store） |
| `TextureLoadStoreDynamicCoords.ll`（`opt -dxil-op-lower`） | ✅ | 动态 `insertelement` 坐标折叠为标量、无残留向量 op（load + store 两向） |

## 6. ×8 稳定性 + validator 期望表（`case_L.ll`）

`llc.exe case_L.ll -filetype=obj -o case_L.obj` 连发 8 次：

| 项 | 期望 | 实测 |
|---|---|---|
| exit code | 8/8 = 0 | 8/8 = 0 |
| obj 产出 | 8/8 存在 | 8/8 存在 |
| obj sha256 | 8 发全等（deterministic） | 单一 sha256 `f853b4fdcc0d30ad…` |
| dxv 验证 | `Validation succeeded.` | ✅ |
| 容器 opcodes | `dx.op.textureLoad(66)` + `dx.op.textureStore(67)` | ✅（coord0/coord1 为标量，无残留 insert/extractelement） |

复现：见 `spike/godot-rurix/passes/luminance_reduction/artifacts/toolchain_probe/case_L.ll` 头注释；load 侧 `case_K.ll` + `texture_load_stage0_probe.py`（stage 0 ×8）。

## 7. 退役条件

1. 上游 LLVM 合并 texture store intrinsic + DXILOp<67> + lowering（issue #194930）。
2. 上游 release，`H:\llvm-clean-82c5bce5-build` 或 D-205 pin bump 到含 store 的版本。
3. rurixc emit 若与上游最终 store 拼写有差异，切到上游拼写。
4. 移除 RD-025 + 本 recipe（或标注 closed）。

在退役前，本地 patch 仅经 `RURIX_LLC=H:\llvm-clean-82c5bce5-buildin\llc.exe` 显式覆盖使用；
committed D-205 pin 与 `toolchain.rs` 不改。
