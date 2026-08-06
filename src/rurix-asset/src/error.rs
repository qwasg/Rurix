//! 资产管线库层 typed 错误(RFC-0020 §5:不为每个状态预造 RX)。

use std::fmt;

/// 导入/schema 失败类别(smoke 与 CLI 机器可读)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    /// JSON 严格性/语法失败(重复 key、非法 UTF-8、控制字符、深度等)。
    JsonStrict,
    /// GLB 容器布局/magic/version/chunk 失败。
    GlbContainer,
    /// `extensionsRequired` 越出 allowlist。
    ExtensionNotAllowed,
    /// accessor/bufferView/sparse 越界。
    AccessorOutOfBounds,
    /// 索引值 ≥ 顶点数。
    IndexOutOfBounds,
    /// node 图有环。
    NodeCycle,
    /// 引用索引不存在。
    DanglingReference,
    /// 缺失必需 buffer 字节。
    MissingBuffer,
    /// schema/logical_uri/未知字段等。
    SchemaInvalid,
    /// AP-CANON 非确定性/非子集编码。
    CanonInvalid,
    /// AP-GRAPH 环/未注册工具/未声明 env 等。
    GraphInvalid,
    /// M79 双构建/mutation 校验失败。
    VerifyFailed,
    /// 其它合法性失败。
    Invalid,
    /// IO。
    Io,
}

impl ErrorKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorKind::JsonStrict => "json_strict",
            ErrorKind::GlbContainer => "glb_container",
            ErrorKind::ExtensionNotAllowed => "extension_not_allowed",
            ErrorKind::AccessorOutOfBounds => "accessor_oob",
            ErrorKind::IndexOutOfBounds => "index_oob",
            ErrorKind::NodeCycle => "node_cycle",
            ErrorKind::DanglingReference => "dangling_reference",
            ErrorKind::MissingBuffer => "missing_buffer",
            ErrorKind::SchemaInvalid => "schema_invalid",
            ErrorKind::CanonInvalid => "canon_invalid",
            ErrorKind::GraphInvalid => "graph_invalid",
            ErrorKind::VerifyFailed => "verify_failed",
            ErrorKind::Invalid => "invalid",
            ErrorKind::Io => "io",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetError {
    pub kind: ErrorKind,
    pub message: String,
}

impl AssetError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for AssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind.as_str(), self.message)
    }
}

impl std::error::Error for AssetError {}

impl From<std::io::Error> for AssetError {
    fn from(e: std::io::Error) -> Self {
        AssetError::new(ErrorKind::Io, e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AssetError>;
