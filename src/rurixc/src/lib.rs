//! rurixc — Rurix 编译器(D-201,07 号文档总体管线)。
//!
//! M1.1 范围:诊断地基(契约 D-M1-1)——`Span`/`SourceMap`/`DiagCtxt` 与
//! message-key 骨架,先于 lexer 落地(r1 顺序,07 §5)。
//! M1.2 范围:lexer + 词法条款(契约 D-M1-2,RXS-0001 ~ RXS-0010)。
//! M1.3 范围:parser/AST/feature gate(契约 D-M1-3,RXS-0011 ~ RXS-0031)。
//! M1.4 范围:诊断渲染/UI golden 通道/rx fmt 雏形(契约 D-M1-4 / D-M1-5)。

pub mod ast;
pub mod diag;
pub mod feature_gate;
pub mod lexer;
pub mod messages;
pub mod parser;
pub mod render;
pub mod source_map;
pub mod span;
