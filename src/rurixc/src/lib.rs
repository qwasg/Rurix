//! rurixc — Rurix 编译器(D-201,07 号文档总体管线)。
//!
//! M1.1 范围:诊断地基(契约 D-M1-1)——`Span`/`SourceMap`/`DiagCtxt` 与
//! message-key 骨架,先于 lexer 落地(r1 顺序,07 §5)。

pub mod diag;
pub mod lexer;
pub mod messages;
pub mod source_map;
pub mod span;
