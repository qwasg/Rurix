//! G9.2 physics gate runner(M121 particle view + M122 gameplay field 骨架期)。
//!
//! 形态照 g8-physics-gates:子命令分发 + 单行 JSON verdict;新建 app 保持
//! G8 门 0-byte 纪律(g8-physics-gates 既有 --mode 字面 0-byte 不动)。
//!
//! 子命令:
//! - `particle-view --source <destruction source.json> --golden <particle_view golden>`
//!   M121 门:五域 adapter + 写路径结构性断言 + M68 迁移 digest 对拍。
//! - `field --golden <field golden>` M122 门:八枚举 accept/非法 RED +
//!   过滤默认空匹配零影响 + persistent journal replay + World-Field 出口。
//! - `field-selftest --arm <illegal_enum|tampered_replay|nonempty_filter_impact>`
//!   M122 自证红臂(门 --selftest 消费)。

mod m121;
mod m122;
mod util;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: g9-physics-gates <particle-view|field|field-selftest> ...");
        std::process::exit(2);
    }
    let result = match args[1].as_str() {
        "particle-view" => m121::run(&args[2..]),
        "field" => m122::run(&args[2..]),
        "field-selftest" => m122::run_selftest_arm(&args[2..]),
        other => Err(format!("unknown subcommand {other}")),
    };
    match result {
        Ok(out) => println!("{out}"),
        Err(e) => {
            println!("{{\"ok\":false,\"error\":\"{}\"}}", util::json_escape(&e));
            std::process::exit(1);
        }
    }
}
