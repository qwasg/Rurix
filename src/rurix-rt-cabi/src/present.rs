//! rxp present 会话 C ABI(MS1.2b,RFC-0009 §4.6;spec/host_orchestration.md
//! RXS-0197/0198)。
//!
//! host `.rx` 的 present typestate 面(RXS-0197,affine 消费式,错序 = 编译期
//! RX4xxx move 违例)降级为本模块 `rxp_*` 符号;本层再做**运行期状态断言**转发
//! ([`OwnedPresentSession`] 内部状态机:错序 → 确定性诊断 + 失败值,双保险不
//! UB)。fence 偶/奇协议(acquire 2n / cuda_done 2n+1 / d3d_done 2n+2)单一事实源
//! 留 rurix-rt `interop.rs`,本层零 fence 细节(RFC-0009 §7 否决重述)。
//!
//! - **会话句柄表 = `thread_local`**(区别 lib.rs 进程级 `Mutex` 表):shim 窗口/
//!   泵对象固定创建线程(RFC-0001 §4.2.1)、[`OwnedPresentSession`] 持线程绑定
//!   `Context`——**不做 `unsafe impl Send` 豁免**,跨线程句柄查无 → 确定性失败
//!   (correct-by-construction;宿主 `.rx` 首期单线程,RFC-0009 §8)。
//! - **backbuffer 借用句柄**(RXS-0198):[`rxp_backbuffer`] 仅 `Acquired` 态可得;
//!   首次注册为全局句柄表 [`BufKind::Borrowed`] 条目(owned = false)后缓存复用
//!   (共享 backbuffer 单一映射,同句柄跨帧稳定)——`rxrt_buf_free` 对其 no-op,
//!   设备内存释放责任留会话([`rxp_destroy`] 清表)。
//! - **poisoned 传播一致**(RXS-0193):每 op 先查所属 rxrt ctx poisoned;会话
//!   CUDA/shim 失败(状态错序除外——错序未触 GPU,不 poison)→ 诊断 + 失败值 +
//!   poison 所属 ctx。
//! - **stub / 无 GPU / 非交互桌面**:[`rxp_create`] 确定性失败 → 诊断 + `0`(不假
//!   绿,不 poison ctx——呈现不可用是环境事实,非该 ctx 的 CUDA 失败);真实呈现
//!   须 cabi feature `present-real`(→ rurix-rt/d3d12-interop-real)。
//!
//! 本模块**零新 unsafe**:入参全为 u64 句柄/标量,会话操作经 rurix-rt safe 面
//! (其内部 external-resource FFI 注册于 unsafe-audit/rurix-rt.md U17/U18)。

use std::cell::RefCell;
use std::collections::HashMap;

use rurix_rt::interop::{InteropError, OwnedPresentSession};

use crate::{BufEntry, BufKind, POISONED, RXRT_FAIL, diag, lock};

/// 会话条目:所属 rxrt ctx 句柄 + 借用 backbuffer 句柄缓存(`0` = 未注册)+ 会话。
struct SessEntry {
    ctx: u64,
    bb: u64,
    session: OwnedPresentSession,
}

/// thread_local 会话句柄表(句柄 `0` 恒无效;线程绑定论证见模块文档)。
struct SessTable {
    next: u64,
    map: HashMap<u64, SessEntry>,
}

thread_local! {
    static SESSIONS: RefCell<SessTable> = RefCell::new(SessTable {
        next: 0,
        map: HashMap::new(),
    });
}

/// rxp op 公共骨架:句柄查找 → 所属 ctx poisoned 检查 → 会话操作;失败分类
/// (错序 [`InteropError::InvalidState`] → 仅诊断,不 poison;CUDA/shim 失败 →
/// 诊断 + poison 所属 ctx,RXS-0193 传播一致)。
fn with_session<R>(
    op: &'static str,
    sess: u64,
    f: impl FnOnce(&mut SessEntry) -> Result<R, InteropError>,
) -> Option<R> {
    SESSIONS.with(|table| {
        let mut table = table.borrow_mut();
        let Some(entry) = table.map.get_mut(&sess) else {
            diag(
                op,
                format!("unknown present session handle {sess} (wrong handle or wrong thread)"),
            );
            return None;
        };
        {
            let mut t = lock();
            let Some(ce) = t.ctxs.get_mut(&entry.ctx) else {
                diag(
                    op,
                    format!("ctx of present session {sess} already destroyed"),
                );
                return None;
            };
            if ce.poisoned {
                diag(op, POISONED);
                return None;
            }
        } // 释放全局锁再进会话操作(会话对象线程独占,无表内共享)。
        match f(entry) {
            Ok(r) => Some(r),
            Err(InteropError::InvalidState {
                op: state_op,
                state,
            }) => {
                // 错序未触 GPU:确定性诊断,不 poison(RXS-0197)。
                diag(
                    op,
                    format!("state machine misorder: op={state_op} in state {state} (RXS-0197)"),
                );
                None
            }
            Err(e) => {
                // CUDA / shim 失败:poison 所属 ctx(RXS-0193 传播一致)后确定性失败。
                let mut t = lock();
                if let Some(ce) = t.ctxs.get_mut(&entry.ctx) {
                    ce.poisoned = true;
                }
                diag(op, format!("{e:?}"));
                None
            }
        }
    })
}

/// C ABI:建立 present 会话(RFC-0009 §4.6)。`ctx` = [`crate::rxrt_ctx_create`] 句柄
/// (device 0 primary context,与会话同设备共享指针地址空间);`rw`/`rh` = 渲染尺寸,
/// `ww`/`wh` = 窗口尺寸。建链(presenter/LUID 核对/import/帧机)单一事实源在
/// rurix-rt [`OwnedPresentSession::create`]。失败(未知/poisoned ctx、stub、无桌面、
/// 无 GPU)→ 确定性诊断 + `0`。
//@ spec: RXS-0197
#[unsafe(no_mangle)]
pub extern "C" fn rxp_create(ctx: u64, rw: u32, rh: u32, ww: u32, wh: u32) -> u64 {
    const OP: &str = "rxp_create";
    {
        let mut t = lock();
        let Some(ce) = t.ctxs.get_mut(&ctx) else {
            diag(OP, format!("unknown ctx handle {ctx}"));
            return 0;
        };
        if ce.poisoned {
            diag(OP, POISONED);
            return 0;
        }
    }
    // 建链(stub/无桌面/无 GPU → 确定性错误,不假绿;不 poison ctx——呈现不可用是
    // 环境事实,非该 ctx 的 CUDA 失败)。device ordinal 0 = rxrt ctx 同设备。
    let session = match OwnedPresentSession::create(0, [rw, rh], [ww, wh]) {
        Ok(session) => session,
        Err(e) => {
            diag(OP, format!("{e:?}"));
            return 0;
        }
    };
    SESSIONS.with(|table| {
        let mut table = table.borrow_mut();
        table.next += 1;
        let h = table.next;
        table.map.insert(
            h,
            SessEntry {
                ctx,
                bb: 0,
                session,
            },
        );
        h
    })
}

/// C ABI:取得本帧写权(`Ready → Acquired`;CUDA wait `acquire(n)=2n`)。`0` 成功;
/// 错序/失败 → 诊断 + 负值。
//@ spec: RXS-0197
#[unsafe(no_mangle)]
pub extern "C" fn rxp_wait(sess: u64) -> i32 {
    match with_session("rxp_wait", sess, |e| e.session.wait()) {
        Some(()) => 0,
        None => RXRT_FAIL,
    }
}

/// C ABI:取共享 backbuffer 的**借用** buffer 句柄(仅 `Acquired` 态可得,RXS-0198)。
/// 首次调用注册全局句柄表 [`BufKind::Borrowed`] 条目(owned = false:`rxrt_buf_free`
/// no-op、无 upload/download 面,内容由 blit kernel 经 [`crate::rxrt_launch`] 写入),
/// 后续帧复用同句柄。错序/失败 → 诊断 + `0`。
//@ spec: RXS-0198
#[unsafe(no_mangle)]
pub extern "C" fn rxp_backbuffer(sess: u64) -> u64 {
    const OP: &str = "rxp_backbuffer";
    SESSIONS.with(|table| {
        let mut table = table.borrow_mut();
        let Some(entry) = table.map.get_mut(&sess) else {
            diag(
                OP,
                format!("unknown present session handle {sess} (wrong handle or wrong thread)"),
            );
            return 0;
        };
        let mut t = lock();
        let Some(ce) = t.ctxs.get_mut(&entry.ctx) else {
            diag(
                OP,
                format!("ctx of present session {sess} already destroyed"),
            );
            return 0;
        };
        if ce.poisoned {
            diag(OP, POISONED);
            return 0;
        }
        // 仅 Acquired 态可得(RXS-0198;错序 → 确定性诊断 + 0,不 poison)。
        let (dptr, bytes) = match entry.session.backbuffer() {
            Ok(v) => v,
            Err(InteropError::InvalidState { op, state }) => {
                diag(
                    OP,
                    format!("state machine misorder: op={op} in state {state} (RXS-0197)"),
                );
                return 0;
            }
            Err(e) => {
                ce.poisoned = true;
                diag(OP, format!("{e:?}"));
                return 0;
            }
        };
        if entry.bb == 0 {
            let h = t.alloc_handle();
            t.bufs.insert(
                h,
                BufEntry {
                    ctx: entry.ctx,
                    bytes: bytes as u64,
                    kind: BufKind::Borrowed { dptr, sess },
                },
            );
            entry.bb = h;
        }
        entry.bb
    })
}

/// C ABI:完成本帧写入(`Acquired → Presentable`;CUDA signal `cuda_done(n)=2n+1`)。
/// `0` 成功;错序/失败 → 诊断 + 负值。
//@ spec: RXS-0197
#[unsafe(no_mangle)]
pub extern "C" fn rxp_signal(sess: u64) -> i32 {
    match with_session("rxp_signal", sess, |e| e.session.signal()) {
        Some(()) => 0,
        None => RXRT_FAIL,
    }
}

/// C ABI:抽干窗口消息泵(仅 `Presentable` 态,镜像 typestate 面)。`0` 继续 /
/// `1` 收到关窗请求;错序/失败 → 诊断 + 负值。
//@ spec: RXS-0197
#[unsafe(no_mangle)]
pub extern "C" fn rxp_pump(sess: u64) -> i32 {
    match with_session("rxp_pump", sess, |e| e.session.pump()) {
        Some(true) => 1,
        Some(false) => 0,
        None => RXRT_FAIL,
    }
}

/// C ABI:提交 D3D12 present(`Presentable → Ready`,帧 +1;shim wait `2n+1` →
/// 固定 present pass → Present → signal `2n+2`)。`0` 成功;错序/失败 → 诊断 + 负值。
//@ spec: RXS-0197
#[unsafe(no_mangle)]
pub extern "C" fn rxp_present(sess: u64) -> i32 {
    match with_session("rxp_present", sess, |e| e.session.present()) {
        Some(()) => 0,
        None => RXRT_FAIL,
    }
}

/// C ABI:销毁 present 会话(任意态可销毁;借用 backbuffer 条目随之清表——防悬垂
/// 设备指针进后续 launch;清表不释放设备内存,所有权在会话,随 Drop 链销毁:帧核心
/// buffer → semaphore → stream → presenter → primary context)。重复/未知句柄 =
/// no-op + 诊断。
//@ spec: RXS-0197
#[unsafe(no_mangle)]
pub extern "C" fn rxp_destroy(sess: u64) {
    const OP: &str = "rxp_destroy";
    SESSIONS.with(|table| {
        let mut table = table.borrow_mut();
        let Some(entry) = table.map.remove(&sess) else {
            diag(
                OP,
                format!("unknown or already destroyed present session handle {sess} (no-op)"),
            );
            return;
        };
        if entry.bb != 0 {
            let mut t = lock();
            t.bufs.remove(&entry.bb);
        }
        drop(entry); // 会话 Drop:帧核心 → primary context(RFC-0001 §4.4 近似)
    });
}

// -- tests ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifacts;
    use crate::{rxrt_ctx_create, rxrt_ctx_destroy, rxrt_ctx_sync};

    //@ spec: RXS-0193, RXS-0197
    #[test]
    fn unknown_session_handles_fail_deterministically() {
        // 句柄 0 恒无效;未知句柄一律确定性失败(诊断 + 失败值),不 panic 不 UB。
        assert!(rxp_wait(0) < 0);
        assert!(rxp_wait(u64::MAX) < 0);
        assert_eq!(rxp_backbuffer(0), 0);
        assert!(rxp_signal(0) < 0);
        assert!(rxp_pump(0) < 0);
        assert!(rxp_present(0) < 0);
        // 未知句柄销毁 = no-op + 诊断。
        rxp_destroy(0);
        // 未知 ctx 的 create → 触 shim 前确定性拒绝。
        assert_eq!(rxp_create(0, 4, 4, 8, 8), 0);
        assert_eq!(rxp_create(u64::MAX, 4, 4, 8, 8), 0);
    }

    //@ spec: RXS-0197
    #[test]
    fn stub_create_fails_deterministically_without_poisoning_ctx() {
        if !crate::gpu_available() {
            eprintln!(
                "[rurix-rt-cabi] SKIP stub_create_fails_deterministically_without_poisoning_ctx: 无可用 GPU/驱动(降级 SKIP)"
            );
            return;
        }
        // ctx_create 只解析描述表 + 保留 primary context,PTX 至 launch 才装载。
        let ptx = b".version 8.0\n.target sm_89\n";
        let blob = artifacts::make_artifacts_blob(ptx, &[], b"\0\0\0\0\0\0\0\0");
        let ctx = rxrt_ctx_create(blob.as_ptr());
        assert_ne!(ctx, 0, "ctx_create");

        if cfg!(feature = "present-real") {
            // real-shim 交互设备:建链成功,驱动一帧后销毁(仅设备真跑编排启用)。
            let sess = rxp_create(ctx, 4, 4, 64, 64);
            assert_ne!(sess, 0, "present-real 会话建立");
            assert_eq!(rxp_backbuffer(sess), 0, "Ready 态无 backbuffer(错序 = 0)");
            assert_eq!(rxp_wait(sess), 0, "wait acquire(0)");
            let bb = rxp_backbuffer(sess);
            assert_ne!(bb, 0, "Acquired 态可得借用 backbuffer 句柄");
            assert_eq!(rxp_backbuffer(sess), bb, "同句柄跨调用稳定(缓存复用)");
            crate::rxrt_buf_free(bb); // 借用句柄 free = no-op(RXS-0198)
            assert!(crate::rxrt_buf_len(bb) > 0, "free 后借用句柄仍有效");
            assert_eq!(rxp_signal(sess), 0, "signal cuda_done(1)");
            assert!(rxp_wait(sess) < 0, "Presentable 态 wait 错序 = 负值");
            let pumped = rxp_pump(sess);
            assert!(pumped == 0 || pumped == 1, "pump 返回 0 继续 / 1 关窗");
            assert_eq!(rxp_present(sess), 0, "present d3d_done(2)");
            rxp_destroy(sess);
            assert!(rxp_wait(sess) < 0, "销毁后句柄失效");
            assert_eq!(crate::rxrt_buf_len(bb), 0, "借用条目随会话清表");
        } else {
            // stub 态:create 确定性失败 → 0(诊断已落 stderr,不假绿)。
            let sess = rxp_create(ctx, 4, 4, 64, 64);
            assert_eq!(sess, 0, "stub 态 create 须确定性失败");
            // 呈现不可用不得 poison ctx(环境事实 ≠ CUDA 失败):后续 GPU 编排照常。
            assert_eq!(rxrt_ctx_sync(ctx), 0, "ctx 未被 poison");
        }
        rxrt_ctx_destroy(ctx);
    }
}
