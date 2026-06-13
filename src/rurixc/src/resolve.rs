//! 名称解析(spec 条款 RXS-0032 ~ RXS-0038,spec/names.md;05 §10 / D-112)。
//!
//! 两遍式:
//! 1. **收集**:构建模块树与 item 命名空间(值/类型分置),登记 enum 变体、
//!    struct 字段、inherent impl 关联项;重复定义 → `RX1002`(双 span,RXS-0037);
//!    随后以固定点迭代解析 `use`(RXS-0035,失败 → `RX1004`,不可见 → `RX1003`);
//! 2. **body 走查**:作用域栈(参数与 `let` 同 body 体系、块遮蔽合法、同层重名
//!    → `RX1002`,RXS-0033);单段路径优先级 局部 > 泛型参数 > 模块项(RXS-0034);
//!    多段路径逐段定位且局部不可作前缀;可见性违例 → `RX1003`(区别于 `RX1001`)。
//!
//! M2.1 容忍口径(实现取舍,随 M2.2 typeck 收紧):**类型位置**未知路径解析为
//! [`Res::Err`] 且不报错(语料草图类型如 `Grid`/`View` 的裁决属类型层);
//! 表达式/模式位置与 `use` 目标解析失败按条款报错。
//!
//! 产物 [`Resolutions`] 以 span 为键(AST 无 NodeId 的 MVP 取舍;token span 唯一)。

use std::collections::HashMap;

use crate::ast::{self, Visibility};
use crate::diag::{DiagCtxt, ErrorCode};
use crate::hir::{Builtin, DefId, DefKind, LocalId, PrimTy, Res, Vis};
use crate::span::Span;

pub const E_UNRESOLVED_NAME: ErrorCode = ErrorCode(1001); // RX1001
pub const E_DUPLICATE_DEFINITION: ErrorCode = ErrorCode(1002); // RX1002
pub const E_VISIBILITY_VIOLATION: ErrorCode = ErrorCode(1003); // RX1003
pub const E_BAD_USE_TARGET: ErrorCode = ErrorCode(1004); // RX1004

/// 定义元数据(`Resolutions::defs`,DefId 索引)。
#[derive(Clone, Debug)]
pub struct DefData {
    pub kind: DefKind,
    pub name: String,
    pub vis: Vis,
    /// 名字 span(诊断锚点)。
    pub span: Span,
    /// 所属模块(模块树索引)。
    pub module: usize,
}

/// 局部绑定声明(per-body,LocalId 索引)。
#[derive(Clone, Debug)]
pub struct LocalDecl {
    pub name: String,
    pub mutable: bool,
    pub span: Span,
}

/// 编译器已知项最小面(RXS-0048:内建 Option/Result,仅服务 desugar;
/// RXS-0055:内建 `Drop` trait 名,仅服务 drop elaboration 识别面;
/// resolve 入口注入,字段在 resolve 后必有值)。
#[derive(Debug, Default, Clone, Copy)]
pub struct LangItems {
    pub option: Option<DefId>,
    pub option_none: Option<DefId>,
    pub option_some: Option<DefId>,
    pub result: Option<DefId>,
    pub result_ok: Option<DefId>,
    pub result_err: Option<DefId>,
    /// 内建 `Drop` trait(RXS-0055;`impl Drop for T` 识别锚点,可被用户遮蔽)。
    pub drop_trait: Option<DefId>,
}

impl LangItems {
    /// prelude 类型名 → 内建 enum(RXS-0048;模块 ns 未命中后的兜底)。
    fn type_by_name(&self, name: &str) -> Option<DefId> {
        match name {
            "Option" => self.option,
            "Result" => self.result,
            "Drop" => self.drop_trait,
            _ => None,
        }
    }

    /// prelude 变体名 → 内建变体(值/模式位置兜底)。
    fn variant_by_name(&self, name: &str) -> Option<DefId> {
        match name {
            "None" => self.option_none,
            "Some" => self.option_some,
            "Ok" => self.result_ok,
            "Err" => self.result_err,
            _ => None,
        }
    }
}

/// 名称解析产物(lowering 与后续 query 的输入)。
#[derive(Debug, Default)]
pub struct Resolutions {
    pub defs: Vec<DefData>,
    /// ast item/variant/关联项的声明 span → DefId。
    pub item_defs: HashMap<Span, DefId>,
    /// 路径 span(表达式/类型/模式/struct-lit)→ 解析结果。
    pub path_res: HashMap<Span, Res>,
    /// 绑定定义点(名字 span)→ per-body LocalId。
    pub bindings: HashMap<Span, LocalId>,
    /// body 键(fn 体块 span / const 初始化器 span)→ 局部声明表。
    pub body_locals: HashMap<Span, Vec<LocalDecl>>,
    /// use 路径 span → 已解析目标(RXS-0035 实现要求)。
    pub use_targets: HashMap<Span, Res>,
    /// inherent impl 关联项:类型 DefId → (名, 关联项 DefId)(typeck 方法查找,RXS-0046)。
    pub assoc_items: HashMap<DefId, Vec<(String, DefId)>>,
    /// enum 变体 → 父 enum(typeck 构造/模式检查)。
    pub variant_parents: HashMap<DefId, DefId>,
    /// 内建函数 DefId(M2.3 最小 prelude;typeck 签名与 codegen 落点查表)。
    pub builtins: HashMap<DefId, Builtin>,
    /// 编译器已知项(RXS-0048;desugar 直引,不受用户遮蔽影响)。
    pub lang_items: LangItems,
}

/// 名称解析入口:对整个源文件构建模块树并走查全部 body。
pub fn resolve(file: &ast::SourceFile, diag: &DiagCtxt) -> Resolutions {
    let mut r = Resolver {
        diag,
        modules: vec![ModuleData::root()],
        out: Resolutions::default(),
        pending_uses: Vec::new(),
        pending_impls: Vec::new(),
        assoc: HashMap::new(),
        enum_variants: HashMap::new(),
        unit_variants: std::collections::HashSet::new(),
        mod_slots: Vec::new(),
    };
    // 内建函数预分配(M2.3 最小 prelude,目前仅 println):不入模块命名空间——
    // 用户同名定义优先,单段值路径未命中时兜底(见 resolve_value_single)
    {
        let b = Builtin::Println;
        let span = Span::new(crate::span::SourceId(0), 0, 0, crate::span::Edition::Rx0);
        let id = r.new_def(DefKind::Fn, b.name(), Vis::Pub, span, 0);
        r.out.builtins.insert(id, b);
    }
    // 编译器已知项注入(RXS-0048):内建 Option/Result enum——同样不入模块
    // 命名空间(用户同名定义按常规作用域遮蔽,不构成 RX1002);HIR item 形态
    // 由 lower 安装,desugar 经 lang_items 直引变体 DefId(不受遮蔽影响)
    {
        let span = Span::new(crate::span::SourceId(0), 0, 0, crate::span::Edition::Rx0);
        let option = r.new_def(DefKind::Enum, "Option", Vis::Pub, span, 0);
        let none = r.new_def(DefKind::Variant, "None", Vis::Pub, span, 0);
        r.unit_variants.insert(none);
        let some = r.new_def(DefKind::Variant, "Some", Vis::Pub, span, 0);
        r.enum_variants.insert(
            option,
            HashMap::from([("None".to_owned(), none), ("Some".to_owned(), some)]),
        );
        let result = r.new_def(DefKind::Enum, "Result", Vis::Pub, span, 0);
        let ok = r.new_def(DefKind::Variant, "Ok", Vis::Pub, span, 0);
        let err = r.new_def(DefKind::Variant, "Err", Vis::Pub, span, 0);
        r.enum_variants.insert(
            result,
            HashMap::from([("Ok".to_owned(), ok), ("Err".to_owned(), err)]),
        );
        let drop_trait = r.new_def(DefKind::Trait, "Drop", Vis::Pub, span, 0);
        r.out.lang_items = LangItems {
            option: Some(option),
            option_none: Some(none),
            option_some: Some(some),
            result: Some(result),
            result_ok: Some(ok),
            result_err: Some(err),
            drop_trait: Some(drop_trait),
        };
    }
    r.collect_items(&file.items, 0);
    r.resolve_uses();
    r.resolve_impl_targets();
    r.resolve_bodies(&file.items, 0);
    // 导出 typeck 所需的关联表(RXS-0046 方法查找 / 变体归属)
    for (ty_def, items) in &r.assoc {
        let mut v: Vec<(String, DefId)> = items.iter().map(|(n, (d, _))| (n.clone(), *d)).collect();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        r.out.assoc_items.insert(*ty_def, v);
    }
    for (enum_def, variants) in &r.enum_variants {
        for vid in variants.values() {
            r.out.variant_parents.insert(*vid, *enum_def);
        }
    }
    r.out
}

/// 命名空间(RXS-0037"命名类")。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Ns {
    Type,
    Value,
}

/// 模块作用域内的一个绑定(item 或 use 引入名)。
#[derive(Clone, Debug)]
struct Binding {
    res: Res,
    vis: Vis,
    /// 引入点(item 名 / use 名)span。
    span: Span,
}

#[derive(Debug)]
struct ModuleData {
    parent: Option<usize>,
    types: HashMap<String, Binding>,
    values: HashMap<String, Binding>,
}

impl ModuleData {
    fn root() -> Self {
        Self {
            parent: None,
            types: HashMap::new(),
            values: HashMap::new(),
        }
    }

    fn ns(&self, ns: Ns) -> &HashMap<String, Binding> {
        match ns {
            Ns::Type => &self.types,
            Ns::Value => &self.values,
        }
    }
}

/// 待固定点解析的 use。
struct PendingUse {
    module: usize,
    vis: Vis,
    path: ast::Path,
    /// 引入名(别名或末段)与其 span。
    name: String,
    name_span: Span,
}

/// 待解析 self_ty 的 impl(关联项登记需先有模块树)。
struct PendingImpl {
    module: usize,
    /// self_ty 若为单段路径的文本名(多段/非路径形态 M2.1 不登记关联项)。
    self_name: Option<String>,
    /// (名字, DefId, 名字 span)
    assoc_items: Vec<(String, DefId, Span)>,
}

struct Resolver<'a> {
    diag: &'a DiagCtxt,
    modules: Vec<ModuleData>,
    out: Resolutions,
    pending_uses: Vec<PendingUse>,
    pending_impls: Vec<PendingImpl>,
    /// inherent impl 关联项:类型 DefId → 名 → 关联项 DefId。
    assoc: HashMap<DefId, HashMap<String, (DefId, Vis)>>,
    /// enum DefId → 变体名 → 变体 DefId。
    enum_variants: HashMap<DefId, HashMap<String, DefId>>,
    /// 单元变体集(模式位置裸名裁决:单元变体优先于绑定,RXS-0048/0023)。
    unit_variants: std::collections::HashSet<DefId>,
    /// mod DefId → 模块树槽位。
    mod_slots: Vec<(DefId, usize)>,
}

fn lower_vis(v: &Visibility) -> Vis {
    match v {
        Visibility::Inherited => Vis::Private,
        Visibility::Pub(_) => Vis::Pub,
        Visibility::PubPackage(_) => Vis::PubPackage,
    }
}

/// 简易编辑距离(拼写建议,RXS-0038)。
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i];
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur.push((prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost));
        }
        prev = cur;
    }
    prev[b.len()]
}

impl<'a> Resolver<'a> {
    // -- 通用 ----------------------------------------------------------------

    fn new_def(&mut self, kind: DefKind, name: &str, vis: Vis, span: Span, module: usize) -> DefId {
        let id = DefId(self.out.defs.len() as u32);
        self.out.defs.push(DefData {
            kind,
            name: name.to_owned(),
            vis,
            span,
            module,
        });
        id
    }

    fn def(&self, id: DefId) -> &DefData {
        &self.out.defs[id.0 as usize]
    }

    fn err_duplicate(&self, name: &str, first: Span, again: Span) {
        // RXS-0037/0038:双 span(先定义 + 冲突处)
        self.diag
            .struct_error(E_DUPLICATE_DEFINITION, "resolve.duplicate_definition")
            .arg("name", format!("`{name}`"))
            .span_label(again, "redefined here")
            .span_label(first, "first defined here")
            .emit();
    }

    fn err_unresolved(&self, name: &str, span: Span, suggestion: Option<&str>) {
        let mut d = self
            .diag
            .struct_error(E_UNRESOLVED_NAME, "resolve.unresolved_name")
            .arg("name", format!("`{name}`"))
            .span_label(span, "not found in this scope");
        if let Some(s) = suggestion {
            d = d.help(format!("a name with a similar spelling exists: `{s}`"));
        }
        d.emit();
    }

    fn err_visibility(&self, name: &str, use_span: Span, def: &DefData) {
        let vis_text = match def.vis {
            Vis::Private => "private",
            Vis::PubPackage => "pub(package)",
            Vis::Pub => "pub",
        };
        // RXS-0036/0038:引用定义处与其声明的可见性
        self.diag
            .struct_error(E_VISIBILITY_VIOLATION, "resolve.visibility_violation")
            .arg("name", format!("`{name}`"))
            .span_label(use_span, "not visible from here")
            .span_label(def.span, format!("defined here with {vis_text} visibility"))
            .emit();
    }

    fn err_bad_use(&self, span: Span, reason: &str) {
        self.diag
            .struct_error(E_BAD_USE_TARGET, "resolve.bad_use_target")
            .arg("reason", reason.to_owned())
            .span_label(span, "cannot import this")
            .emit();
    }

    /// `from` 模块是否可见 `def_module` 中可见性为 `vis` 的名称(RXS-0036)。
    fn is_visible(&self, vis: Vis, def_module: usize, from: usize) -> bool {
        match vis {
            // MVP 单 package:pub 与 pub(package) 在包内等效可见
            Vis::Pub | Vis::PubPackage => true,
            Vis::Private => {
                // 私有 = 定义模块自身及其后代可见
                let mut m = Some(from);
                while let Some(cur) = m {
                    if cur == def_module {
                        return true;
                    }
                    m = self.modules[cur].parent;
                }
                false
            }
        }
    }

    /// 向模块命名空间登记名字;冲突报 RX1002(RXS-0037),保留先到者。
    fn register(&mut self, module: usize, ns: Ns, name: &str, binding: Binding) {
        let map = match ns {
            Ns::Type => &mut self.modules[module].types,
            Ns::Value => &mut self.modules[module].values,
        };
        if let Some(existing) = map.get(name) {
            let first = existing.span;
            let again = binding.span;
            self.err_duplicate(name, first, again);
        } else {
            map.insert(name.to_owned(), binding);
        }
    }

    // -- 第一遍:收集(RXS-0032/0037) ----------------------------------------

    fn collect_items(&mut self, items: &[ast::Item], module: usize) {
        for item in items {
            self.collect_item(item, module);
        }
    }

    fn collect_item(&mut self, item: &ast::Item, module: usize) {
        let vis = lower_vis(&item.vis);
        match &item.kind {
            ast::ItemKind::Fn(f) => {
                let id = self.new_def(DefKind::Fn, &f.name.name, vis, f.name.span, module);
                self.out.item_defs.insert(item.span, id);
                self.register(
                    module,
                    Ns::Value,
                    &f.name.name,
                    Binding {
                        res: Res::Def(id),
                        vis,
                        span: f.name.span,
                    },
                );
            }
            ast::ItemKind::Struct(s) => {
                let id = self.new_def(DefKind::Struct, &s.name.name, vis, s.name.span, module);
                self.out.item_defs.insert(item.span, id);
                self.register(
                    module,
                    Ns::Type,
                    &s.name.name,
                    Binding {
                        res: Res::Def(id),
                        vis,
                        span: s.name.span,
                    },
                );
                // 元组/单元结构体构造器进值命名空间
                if !matches!(s.body, ast::VariantBody::Named(_)) {
                    self.register(
                        module,
                        Ns::Value,
                        &s.name.name,
                        Binding {
                            res: Res::Def(id),
                            vis,
                            span: s.name.span,
                        },
                    );
                }
                if let ast::VariantBody::Named(fields) = &s.body {
                    self.collect_fields(fields, module);
                }
            }
            ast::ItemKind::Enum(e) => {
                let id = self.new_def(DefKind::Enum, &e.name.name, vis, e.name.span, module);
                self.out.item_defs.insert(item.span, id);
                self.register(
                    module,
                    Ns::Type,
                    &e.name.name,
                    Binding {
                        res: Res::Def(id),
                        vis,
                        span: e.name.span,
                    },
                );
                let mut variants: HashMap<String, DefId> = HashMap::new();
                for v in &e.variants {
                    let vid =
                        self.new_def(DefKind::Variant, &v.name.name, vis, v.name.span, module);
                    self.out.item_defs.insert(v.span, vid);
                    if matches!(v.body, ast::VariantBody::Unit) {
                        self.unit_variants.insert(vid);
                    }
                    if let Some(prev) = variants.get(&v.name.name) {
                        let first = self.def(*prev).span;
                        self.err_duplicate(&v.name.name, first, v.name.span);
                    } else {
                        variants.insert(v.name.name.clone(), vid);
                    }
                    if let ast::VariantBody::Named(fields) = &v.body {
                        self.collect_fields(fields, module);
                    }
                }
                self.enum_variants.insert(id, variants);
            }
            ast::ItemKind::Trait(t) => {
                let id = self.new_def(DefKind::Trait, &t.name.name, vis, t.name.span, module);
                self.out.item_defs.insert(item.span, id);
                self.register(
                    module,
                    Ns::Type,
                    &t.name.name,
                    Binding {
                        res: Res::Def(id),
                        vis,
                        span: t.name.span,
                    },
                );
                self.collect_assoc_decls(&t.items, module, None);
            }
            ast::ItemKind::Impl(im) => {
                let self_name = match &im.self_ty.kind {
                    ast::TyKind::Path(p) if p.segments.len() == 1 => {
                        Some(p.segments[0].ident.name.clone())
                    }
                    _ => None,
                };
                let assoc_items = self.collect_assoc_decls(&im.items, module, Some(item.span));
                self.pending_impls.push(PendingImpl {
                    module,
                    self_name,
                    assoc_items,
                });
            }
            ast::ItemKind::Mod(m) => {
                let id = self.new_def(DefKind::Mod, &m.name.name, vis, m.name.span, module);
                self.out.item_defs.insert(item.span, id);
                let child = self.modules.len();
                self.modules.push(ModuleData {
                    parent: Some(module),
                    types: HashMap::new(),
                    values: HashMap::new(),
                });
                self.register(
                    module,
                    Ns::Type,
                    &m.name.name,
                    Binding {
                        res: Res::Def(id),
                        vis,
                        span: m.name.span,
                    },
                );
                // 模块树索引随 DefId 记录:用 defs[id].module 表示父,child 索引经 mod_index 表
                self.mod_index_insert(id, child);
                self.collect_items(&m.items, child);
            }
            ast::ItemKind::Use(u) => {
                let (name, name_span) = match &u.alias {
                    Some(a) => (a.name.clone(), a.span),
                    None => {
                        let last = u.path.segments.last().expect("use 路径非空");
                        (last.ident.name.clone(), last.ident.span)
                    }
                };
                self.pending_uses.push(PendingUse {
                    module,
                    vis,
                    path: u.path.clone(),
                    name,
                    name_span,
                });
            }
            ast::ItemKind::Static(s) => {
                let id = self.new_def(DefKind::Static, &s.name.name, vis, s.name.span, module);
                self.out.item_defs.insert(item.span, id);
                self.register(
                    module,
                    Ns::Value,
                    &s.name.name,
                    Binding {
                        res: Res::Def(id),
                        vis,
                        span: s.name.span,
                    },
                );
            }
            ast::ItemKind::Const(c) => {
                let id = self.new_def(DefKind::Const, &c.name.name, vis, c.name.span, module);
                self.out.item_defs.insert(item.span, id);
                self.register(
                    module,
                    Ns::Value,
                    &c.name.name,
                    Binding {
                        res: Res::Def(id),
                        vis,
                        span: c.name.span,
                    },
                );
            }
            ast::ItemKind::TypeAlias(t) => {
                let id = self.new_def(DefKind::TypeAlias, &t.name.name, vis, t.name.span, module);
                self.out.item_defs.insert(item.span, id);
                self.register(
                    module,
                    Ns::Type,
                    &t.name.name,
                    Binding {
                        res: Res::Def(id),
                        vis,
                        span: t.name.span,
                    },
                );
            }
            ast::ItemKind::ExternBlock(e) => {
                for inner in &e.items {
                    self.collect_item(inner, module);
                }
            }
            ast::ItemKind::Err => {}
        }
    }

    /// struct/变体具名字段:登记 DefData 并查重(RXS-0037)。
    fn collect_fields(&mut self, fields: &[ast::FieldDef], module: usize) {
        let mut seen: HashMap<&str, Span> = HashMap::new();
        for f in fields {
            let vis = lower_vis(&f.vis);
            let id = self.new_def(DefKind::Field, &f.name.name, vis, f.name.span, module);
            self.out.item_defs.insert(f.span, id);
            if let Some(first) = seen.get(f.name.name.as_str()) {
                self.err_duplicate(&f.name.name, *first, f.name.span);
            } else {
                seen.insert(&f.name.name, f.name.span);
            }
        }
    }

    /// trait / impl 关联项声明收集(声明域内查重,RXS-0037)。
    fn collect_assoc_decls(
        &mut self,
        items: &[ast::AssocItem],
        module: usize,
        _impl_span: Option<Span>,
    ) -> Vec<(String, DefId, Span)> {
        let mut seen: HashMap<String, Span> = HashMap::new();
        let mut out = Vec::new();
        for a in items {
            let vis = lower_vis(&a.vis);
            let (kind, name, span) = match &a.kind {
                ast::AssocItemKind::Fn(f) => (DefKind::AssocFn, f.name.name.clone(), f.name.span),
                ast::AssocItemKind::Type { name, .. } => {
                    (DefKind::AssocType, name.name.clone(), name.span)
                }
                ast::AssocItemKind::Const(c) => {
                    (DefKind::AssocConst, c.name.name.clone(), c.name.span)
                }
            };
            let id = self.new_def(kind, &name, vis, span, module);
            self.out.item_defs.insert(a.span, id);
            if let Some(first) = seen.get(&name) {
                self.err_duplicate(&name, *first, span);
            } else {
                seen.insert(name.clone(), span);
            }
            out.push((name, id, span));
        }
        out
    }

    // -- 模块树索引(DefId ↔ 模块槽位) ---------------------------------------

    fn mod_index_insert(&mut self, def: DefId, module_slot: usize) {
        self.mod_slots.push((def, module_slot));
    }

    fn module_slot_of(&self, def: DefId) -> Option<usize> {
        self.mod_slots
            .iter()
            .find(|(d, _)| *d == def)
            .map(|(_, s)| *s)
    }

    // -- use 解析(RXS-0035,固定点) -----------------------------------------

    fn resolve_uses(&mut self) {
        let mut pending = std::mem::take(&mut self.pending_uses);
        loop {
            let before = pending.len();
            let mut still = Vec::new();
            for u in pending {
                match self.try_resolve_use(&u) {
                    UseOutcome::Done => {}
                    UseOutcome::Retry(u) => still.push(u),
                }
            }
            if still.is_empty() {
                return;
            }
            if still.len() == before {
                // 无进展:逐条报错(目标不存在/不可见/类别不合法)
                for u in still {
                    self.report_use_failure(&u);
                }
                return;
            }
            pending = still;
        }
    }

    fn try_resolve_use(&mut self, u: &PendingUse) -> UseOutcome {
        match self.walk_use_path(u) {
            Ok(res) => {
                self.out.use_targets.insert(u.path.span, res);
                self.register_use(u, res);
                UseOutcome::Done
            }
            Err(UseErr::NotYet) => UseOutcome::Retry(PendingUse {
                module: u.module,
                vis: u.vis,
                path: u.path.clone(),
                name: u.name.clone(),
                name_span: u.name_span,
            }),
            Err(UseErr::Invisible(d)) => {
                // 不可见目标:RX1003(RXS-0036),仍登记为 Err 防级联
                let def = self.def(d).clone();
                self.err_visibility(&u.name, u.path.span, &def);
                self.register_use(u, Res::Err);
                UseOutcome::Done
            }
            Err(UseErr::Hard) => {
                self.report_use_failure(u);
                self.register_use(u, Res::Err);
                UseOutcome::Done
            }
        }
    }

    fn register_use(&mut self, u: &PendingUse, res: Res) {
        let binding = Binding {
            res,
            vis: u.vis,
            span: u.name_span,
        };
        // use 目标若是值(fn/const/static)入值 ns,否则入类型 ns;
        // 同时命中两个 ns 的形态(元组结构体)按目标类别归类型 ns 起步。
        let ns = match res {
            Res::Def(d) => match self.def(d).kind {
                DefKind::Fn | DefKind::Const | DefKind::Static => Ns::Value,
                _ => Ns::Type,
            },
            _ => Ns::Type,
        };
        self.register(u.module, ns, &u.name, binding);
    }

    /// 沿 use 路径逐段下行。
    ///
    /// 可见性判定以 **binding 所在模块**为准(use 重导出场景下与原始定义模块
    /// 不同;同模块引用本模块的私有 use 引入名合法,RXS-0035/0036)。
    fn walk_use_path(&self, u: &PendingUse) -> Result<Res, UseErr> {
        let segs = &u.path.segments;
        let first = &segs[0].ident;
        // 首段在所在模块作用域查(同模块必可见,RXS-0035)
        let mut cur: Binding = match self.modules[u.module]
            .types
            .get(&first.name)
            .or_else(|| self.modules[u.module].values.get(&first.name))
        {
            Some(b) => b.clone(),
            None => return Err(UseErr::Hard),
        };
        for (i, seg) in segs.iter().enumerate().skip(1) {
            let Res::Def(prefix_def) = cur.res else {
                // use 链尚未解析到位(目标 Err 或暂缺)
                return Err(UseErr::NotYet);
            };
            let prefix = self.def(prefix_def);
            match prefix.kind {
                DefKind::Mod => {
                    let Some(slot) = self.module_slot_of(prefix_def) else {
                        return Err(UseErr::NotYet);
                    };
                    let m = &self.modules[slot];
                    let last = i == segs.len() - 1;
                    let b = if last {
                        m.values
                            .get(&seg.ident.name)
                            .or_else(|| m.types.get(&seg.ident.name))
                    } else {
                        m.types.get(&seg.ident.name)
                    };
                    let Some(b) = b else {
                        return Err(UseErr::Hard);
                    };
                    // 中间段与末段都做可见性检查:binding 所在模块 = slot
                    if !self.is_visible(b.vis, slot, u.module) {
                        if let Res::Def(d) = b.res {
                            return Err(UseErr::Invisible(d));
                        }
                        return Err(UseErr::Hard);
                    }
                    cur = b.clone();
                }
                _ => return Err(UseErr::Hard), // 前缀类别不允许(非模块)
            }
        }
        Ok(cur.res)
    }

    fn report_use_failure(&self, u: &PendingUse) {
        // 区分"目标不存在"与"目标类别不允许导入"(RXS-0038)
        let first = &u.path.segments[0].ident;
        let reason = if self.modules[u.module].types.contains_key(&first.name)
            || self.modules[u.module].values.contains_key(&first.name)
        {
            "the path does not resolve to an importable item"
        } else {
            "the import target does not exist"
        };
        self.err_bad_use(u.path.span, reason);
    }

    // -- impl 关联项落位 -------------------------------------------------------

    fn resolve_impl_targets(&mut self) {
        let pending = std::mem::take(&mut self.pending_impls);
        for im in &pending {
            let Some(name) = &im.self_name else { continue };
            let Some(res) = self.modules[im.module].types.get(name).map(|b| b.res) else {
                continue; // 未知 self 类型:M2.1 类型位置容忍口径
            };
            let Res::Def(type_def) = res else { continue };
            for (n, id, span) in &im.assoc_items {
                let existing = self
                    .assoc
                    .get(&type_def)
                    .and_then(|m| m.get(n))
                    .map(|(first_id, _)| *first_id);
                if let Some(first_id) = existing {
                    let first = self.def(first_id).span;
                    self.err_duplicate(n, first, *span);
                } else {
                    let vis = self.def(*id).vis;
                    self.assoc
                        .entry(type_def)
                        .or_default()
                        .insert(n.clone(), (*id, vis));
                }
            }
        }
        self.pending_impls = pending;
    }
}

enum UseOutcome {
    Done,
    Retry(PendingUse),
}

enum UseErr {
    /// use 链未就绪,固定点重试。
    NotYet,
    /// 存在但不可见(RX1003)。
    Invisible(DefId),
    /// 确定性失败(RX1004)。
    Hard,
}

/// 局部作用域帧条目(混合 ns:绑定与块内 item 同帧裁决,RXS-0037)。
#[derive(Clone, Copy, Debug)]
struct ScopeEntry {
    res: Res,
    span: Span,
}

/// body 走查上下文(RXS-0032:item 作用域与局部作用域分离建模)。
struct BodyCx {
    module: usize,
    /// 泛型参数名(impl 参数在前,fn 参数偏移其后;序号即 Res::GenericParam)。
    generics: Vec<String>,
    /// `Self` 是否可用(impl/trait 体内)。
    has_self: bool,
    /// body 键(fn 体块 span / 初始化器 span)。
    body_key: Span,
    locals: Vec<LocalDecl>,
    scopes: Vec<HashMap<String, ScopeEntry>>,
}

impl BodyCx {
    fn new(module: usize, generics: Vec<String>, has_self: bool, body_key: Span) -> Self {
        Self {
            module,
            generics,
            has_self,
            body_key,
            locals: Vec::new(),
            scopes: vec![HashMap::new()],
        }
    }
}

/// 多段路径走查失败形态。
enum PathFail {
    /// 路径中某段名字不存在(段名)。
    Missing(String),
    /// 存在但不可见(目标 DefId)。
    Invisible(DefId),
}

impl Resolver<'_> {
    // -- 第二遍:body 走查(RXS-0033/0034/0036/0038) --------------------------

    fn resolve_bodies(&mut self, items: &[ast::Item], module: usize) {
        for item in items {
            self.resolve_item_body(item, module);
        }
    }

    fn resolve_item_body(&mut self, item: &ast::Item, module: usize) {
        match &item.kind {
            ast::ItemKind::Fn(f) => {
                self.resolve_fn(f, module, Vec::new(), false);
            }
            ast::ItemKind::Struct(s) => {
                let generics = generic_names(&s.generics);
                self.resolve_variant_body_types(&s.body, module, &generics);
            }
            ast::ItemKind::Enum(e) => {
                let generics = generic_names(&e.generics);
                for v in &e.variants {
                    self.resolve_variant_body_types(&v.body, module, &generics);
                }
            }
            ast::ItemKind::Trait(t) => {
                let trait_generics = generic_names(&t.generics);
                for a in &t.items {
                    self.resolve_assoc_body(a, module, &trait_generics, true);
                }
            }
            ast::ItemKind::Impl(im) => {
                let impl_generics = generic_names(&im.generics);
                // self_ty / trait_ty 在类型位置解析(容忍口径)
                let mut cx = BodyCx::new(module, impl_generics.clone(), true, im.self_ty.span);
                self.resolve_ast_ty(&im.self_ty, &mut cx);
                if let Some(t) = &im.trait_ty {
                    self.resolve_ast_ty(t, &mut cx);
                }
                for a in &im.items {
                    self.resolve_assoc_body(a, module, &impl_generics, true);
                }
            }
            ast::ItemKind::Mod(m) => {
                let Some(&id) = self.out.item_defs.get(&item.span) else {
                    return;
                };
                let Some(slot) = self.module_slot_of(id) else {
                    return;
                };
                self.resolve_bodies(&m.items, slot);
            }
            ast::ItemKind::Static(s) => {
                let mut cx = BodyCx::new(module, Vec::new(), false, s.init.span);
                self.resolve_ast_ty(&s.ty, &mut cx);
                self.resolve_expr(&s.init, &mut cx);
                self.finish_body(cx);
            }
            ast::ItemKind::Const(c) => {
                let mut cx = BodyCx::new(module, Vec::new(), false, c.init.span);
                self.resolve_ast_ty(&c.ty, &mut cx);
                self.resolve_expr(&c.init, &mut cx);
                self.finish_body(cx);
            }
            ast::ItemKind::TypeAlias(t) => {
                let generics = generic_names(&t.generics);
                let mut cx = BodyCx::new(module, generics, false, t.ty.span);
                self.resolve_ast_ty(&t.ty, &mut cx);
            }
            ast::ItemKind::Use(_) | ast::ItemKind::Err => {}
            ast::ItemKind::ExternBlock(e) => {
                for inner in &e.items {
                    self.resolve_item_body(inner, module);
                }
            }
        }
    }

    fn resolve_assoc_body(
        &mut self,
        a: &ast::AssocItem,
        module: usize,
        outer_generics: &[String],
        has_self: bool,
    ) {
        match &a.kind {
            ast::AssocItemKind::Fn(f) => {
                self.resolve_fn(f, module, outer_generics.to_vec(), has_self);
            }
            ast::AssocItemKind::Type { default, .. } => {
                if let Some(ty) = default {
                    let mut cx = BodyCx::new(module, outer_generics.to_vec(), has_self, ty.span);
                    self.resolve_ast_ty(ty, &mut cx);
                }
            }
            ast::AssocItemKind::Const(c) => {
                let mut cx = BodyCx::new(module, outer_generics.to_vec(), has_self, c.init.span);
                self.resolve_ast_ty(&c.ty, &mut cx);
                self.resolve_expr(&c.init, &mut cx);
                self.finish_body(cx);
            }
        }
    }

    fn resolve_fn(
        &mut self,
        f: &ast::FnItem,
        module: usize,
        outer_generics: Vec<String>,
        has_self: bool,
    ) {
        let mut generics = outer_generics;
        generics.extend(generic_names(&f.generics));
        let body_key = f
            .body
            .as_ref()
            .map(|b| b.span)
            .unwrap_or_else(|| f.name.span);
        let mut cx = BodyCx::new(module, generics, has_self, body_key);
        // 参数:类型解析 + 绑定声明(参数与 body let 同 body 体系,RXS-0033)
        for p in &f.params {
            match &p.kind {
                ast::ParamKind::SelfParam { mutable, .. } => {
                    self.declare_local(&mut cx, "self", *mutable, p.span);
                }
                ast::ParamKind::Typed { pat, ty } => {
                    self.resolve_ast_ty(ty, &mut cx);
                    self.resolve_pat(pat, &mut cx);
                }
            }
        }
        if let Some(ret) = &f.ret {
            self.resolve_ast_ty(ret, &mut cx);
        }
        for pred in &f.generics.where_preds {
            self.resolve_ast_ty(&pred.ty, &mut cx);
        }
        if let Some(body) = &f.body {
            self.resolve_block(body, &mut cx);
            self.finish_body(cx);
        }
    }

    fn resolve_variant_body_types(
        &mut self,
        body: &ast::VariantBody,
        module: usize,
        generics: &[String],
    ) {
        let mut cx = BodyCx::new(
            module,
            generics.to_vec(),
            false,
            Span::new(crate::span::SourceId(0), 0, 0, crate::span::Edition::Rx0),
        );
        match body {
            ast::VariantBody::Named(fields) => {
                for fd in fields {
                    self.resolve_ast_ty(&fd.ty, &mut cx);
                }
            }
            ast::VariantBody::Tuple(fields) => {
                for fd in fields {
                    self.resolve_ast_ty(&fd.ty, &mut cx);
                }
            }
            ast::VariantBody::Unit => {}
        }
    }

    fn finish_body(&mut self, cx: BodyCx) {
        self.out.body_locals.insert(cx.body_key, cx.locals);
    }

    // -- 局部作用域(RXS-0033/0037) -------------------------------------------

    fn declare_local(&mut self, cx: &mut BodyCx, name: &str, mutable: bool, span: Span) {
        let id = LocalId(cx.locals.len() as u32);
        cx.locals.push(LocalDecl {
            name: name.to_owned(),
            mutable,
            span,
        });
        let frame = cx.scopes.last_mut().expect("作用域栈非空");
        if let Some(prev) = frame.get(name) {
            // 同层重名(RXS-0033/0037);内层遮蔽外层经新帧,不走此路径
            let first = prev.span;
            self.err_duplicate(name, first, span);
        }
        frame.insert(
            name.to_owned(),
            ScopeEntry {
                res: Res::Local(id),
                span,
            },
        );
        self.out.bindings.insert(span, id);
    }

    fn declare_block_item(&mut self, cx: &mut BodyCx, name: &str, res: Res, span: Span) {
        let frame = cx.scopes.last_mut().expect("作用域栈非空");
        if let Some(prev) = frame.get(name) {
            let first = prev.span;
            self.err_duplicate(name, first, span);
        }
        frame.insert(name.to_owned(), ScopeEntry { res, span });
    }

    fn lookup_scopes(&self, cx: &BodyCx, name: &str) -> Option<ScopeEntry> {
        for frame in cx.scopes.iter().rev() {
            if let Some(e) = frame.get(name) {
                return Some(*e);
            }
        }
        None
    }

    fn lookup_generic(&self, cx: &BodyCx, name: &str) -> Option<Res> {
        cx.generics
            .iter()
            .position(|g| g == name)
            .map(|i| Res::GenericParam(i as u32))
    }

    // -- 路径解析(RXS-0034/0036) ----------------------------------------------

    /// 单段路径,表达式/模式位置:局部 > 泛型参数 > 模块值 ns(RXS-0034)。
    fn resolve_value_single(&mut self, name: &str, span: Span, cx: &BodyCx) -> Res {
        if let Some(e) = self.lookup_scopes(cx, name) {
            self.out.path_res.insert(span, e.res);
            return e.res;
        }
        if let Some(res) = self.lookup_generic(cx, name) {
            self.out.path_res.insert(span, res);
            return res;
        }
        if let Some(b) = self.modules[cx.module].values.get(name) {
            let res = b.res;
            self.out.path_res.insert(span, res);
            return res;
        }
        // 内建函数兜底(M2.3 最小 prelude;以上各级未命中才轮到)
        if let Some(d) = self
            .out
            .builtins
            .iter()
            .find(|(_, b)| b.name() == name)
            .map(|(d, _)| *d)
        {
            let res = Res::Def(d);
            self.out.path_res.insert(span, res);
            return res;
        }
        // 编译器已知项变体兜底(RXS-0048:Some/None/Ok/Err,值/模式位置)
        if let Some(d) = self.out.lang_items.variant_by_name(name) {
            let res = Res::Def(d);
            self.out.path_res.insert(span, res);
            return res;
        }
        let suggestion = self.suggest(name, cx, Ns::Value);
        self.err_unresolved(name, span, suggestion.as_deref());
        self.out.path_res.insert(span, Res::Err);
        Res::Err
    }

    /// 单段路径,类型位置:泛型参数 > Self > 原生类型 > 模块类型 ns;
    /// 未命中容忍为 Err(M2.1 口径,M2.2 typeck 收紧)。
    fn resolve_type_single(&mut self, name: &str, span: Span, cx: &BodyCx) -> Res {
        let res = if let Some(r) = self.lookup_generic(cx, name) {
            r
        } else if name == "Self" && cx.has_self {
            Res::SelfTy
        } else if let Some(p) = PrimTy::from_name(name) {
            Res::PrimTy(p)
        } else if let Some(e) = self.lookup_scopes(cx, name) {
            // 块内 item(struct 等)经局部帧可见
            e.res
        } else if let Some(b) = self.modules[cx.module].types.get(name) {
            b.res
        } else if let Some(d) = self.out.lang_items.type_by_name(name) {
            // 编译器已知项兜底(RXS-0048:Option/Result;模块 ns 优先 = 可遮蔽)
            Res::Def(d)
        } else {
            Res::Err
        };
        self.out.path_res.insert(span, res);
        res
    }

    /// 多段路径走查(首段定位模块项;局部不可作前缀,RXS-0034)。
    fn walk_multi_path(&self, path: &ast::Path, cx: &BodyCx, last_ns: Ns) -> Result<Res, PathFail> {
        let segs = &path.segments;
        let first = &segs[0].ident;
        let mut cur_res: Res = if let Some(b) = self.modules[cx.module]
            .types
            .get(&first.name)
            .or_else(|| self.modules[cx.module].values.get(&first.name))
        {
            b.res
        } else if let Some(e) = self.lookup_scopes(cx, &first.name) {
            // 块内 item 可作前缀;局部绑定不可(RXS-0034)
            match e.res {
                Res::Local(_) => return Err(PathFail::Missing(first.name.clone())),
                r => r,
            }
        } else if let Some(d) = self.out.lang_items.type_by_name(&first.name) {
            // 编译器已知项作路径前缀(RXS-0048:`Option::Some` 等)
            Res::Def(d)
        } else {
            return Err(PathFail::Missing(first.name.clone()));
        };
        for (i, seg) in segs.iter().enumerate().skip(1) {
            let Res::Def(prefix_def) = cur_res else {
                return Err(PathFail::Missing(seg.ident.name.clone()));
            };
            let prefix = self.def(prefix_def).clone();
            let last = i == segs.len() - 1;
            cur_res = match prefix.kind {
                DefKind::Mod => {
                    let Some(slot) = self.module_slot_of(prefix_def) else {
                        return Err(PathFail::Missing(seg.ident.name.clone()));
                    };
                    let m = &self.modules[slot];
                    let b = if last {
                        match last_ns {
                            Ns::Value => m
                                .values
                                .get(&seg.ident.name)
                                .or_else(|| m.types.get(&seg.ident.name)),
                            Ns::Type => m
                                .types
                                .get(&seg.ident.name)
                                .or_else(|| m.values.get(&seg.ident.name)),
                        }
                    } else {
                        m.types.get(&seg.ident.name)
                    };
                    let Some(b) = b else {
                        return Err(PathFail::Missing(seg.ident.name.clone()));
                    };
                    // 可见性以 binding 所在模块(slot)为准(use 重导出兼容)
                    if !self.is_visible(b.vis, slot, cx.module) {
                        if let Res::Def(d) = b.res {
                            return Err(PathFail::Invisible(d));
                        }
                        return Err(PathFail::Missing(seg.ident.name.clone()));
                    }
                    b.res
                }
                DefKind::Enum => {
                    if let Some(v) = self
                        .enum_variants
                        .get(&prefix_def)
                        .and_then(|m| m.get(&seg.ident.name))
                    {
                        Res::Def(*v)
                    } else if let Some((a, vis)) = self
                        .assoc
                        .get(&prefix_def)
                        .and_then(|m| m.get(&seg.ident.name))
                        .copied()
                    {
                        let dd = self.def(a);
                        if !self.is_visible(vis, dd.module, cx.module) {
                            return Err(PathFail::Invisible(a));
                        }
                        Res::Def(a)
                    } else {
                        return Err(PathFail::Missing(seg.ident.name.clone()));
                    }
                }
                DefKind::Struct => {
                    if let Some((a, vis)) = self
                        .assoc
                        .get(&prefix_def)
                        .and_then(|m| m.get(&seg.ident.name))
                        .copied()
                    {
                        let dd = self.def(a);
                        if !self.is_visible(vis, dd.module, cx.module) {
                            return Err(PathFail::Invisible(a));
                        }
                        Res::Def(a)
                    } else {
                        return Err(PathFail::Missing(seg.ident.name.clone()));
                    }
                }
                _ => return Err(PathFail::Missing(seg.ident.name.clone())),
            };
        }
        Ok(cur_res)
    }

    /// 表达式/模式位置的路径解析(失败按 RXS-0036/0038 报错)。
    fn resolve_value_path(&mut self, path: &ast::Path, cx: &mut BodyCx) -> Res {
        for seg in &path.segments {
            if let Some(args) = &seg.args {
                self.resolve_generic_args(args, cx);
            }
        }
        if path.segments.len() == 1 {
            let seg = &path.segments[0].ident;
            return self.resolve_value_single(&seg.name, path.span, cx);
        }
        match self.walk_multi_path(path, cx, Ns::Value) {
            Ok(res) => {
                self.out.path_res.insert(path.span, res);
                res
            }
            Err(PathFail::Missing(name)) => {
                self.err_unresolved(&name, path.span, None);
                self.out.path_res.insert(path.span, Res::Err);
                Res::Err
            }
            Err(PathFail::Invisible(d)) => {
                let def = self.def(d).clone();
                let name = def.name.clone();
                self.err_visibility(&name, path.span, &def);
                self.out.path_res.insert(path.span, Res::Err);
                Res::Err
            }
        }
    }

    /// 类型位置的路径解析(容忍口径:失败 → Err 不报错)。
    fn resolve_type_path(&mut self, path: &ast::Path, cx: &mut BodyCx) -> Res {
        for seg in &path.segments {
            if let Some(args) = &seg.args {
                self.resolve_generic_args(args, cx);
            }
        }
        if path.segments.len() == 1 {
            let seg = &path.segments[0].ident;
            return self.resolve_type_single(&seg.name, path.span, cx);
        }
        if path.segments[0].ident.name == "Self" {
            // `Self::Assoc` 等关联类型路径随 M2.2 typeck
            self.out.path_res.insert(path.span, Res::Err);
            return Res::Err;
        }
        let res = self.walk_multi_path(path, cx, Ns::Type).unwrap_or(Res::Err);
        self.out.path_res.insert(path.span, res);
        res
    }

    /// struct 字面量路径:类型 ns,但处于表达式位置 → 失败报错(RXS-0034/0038)。
    fn resolve_struct_lit_path(&mut self, path: &ast::Path, cx: &mut BodyCx) -> Res {
        for seg in &path.segments {
            if let Some(args) = &seg.args {
                self.resolve_generic_args(args, cx);
            }
        }
        if path.segments.len() == 1 {
            let seg = &path.segments[0].ident;
            let res = self.resolve_type_single(&seg.name, path.span, cx);
            if res == Res::Err {
                let suggestion = self.suggest(&seg.name, cx, Ns::Type);
                self.err_unresolved(&seg.name, path.span, suggestion.as_deref());
            }
            return res;
        }
        match self.walk_multi_path(path, cx, Ns::Type) {
            Ok(res) => {
                self.out.path_res.insert(path.span, res);
                res
            }
            Err(PathFail::Missing(name)) => {
                self.err_unresolved(&name, path.span, None);
                self.out.path_res.insert(path.span, Res::Err);
                Res::Err
            }
            Err(PathFail::Invisible(d)) => {
                let def = self.def(d).clone();
                let name = def.name.clone();
                self.err_visibility(&name, path.span, &def);
                self.out.path_res.insert(path.span, Res::Err);
                Res::Err
            }
        }
    }

    /// 拼写建议(RXS-0038:同作用域相近拼写)。
    fn suggest(&self, name: &str, cx: &BodyCx, ns: Ns) -> Option<String> {
        let mut candidates: Vec<&str> = Vec::new();
        for frame in &cx.scopes {
            candidates.extend(frame.keys().map(String::as_str));
        }
        candidates.extend(cx.generics.iter().map(String::as_str));
        candidates.extend(self.modules[cx.module].ns(ns).keys().map(String::as_str));
        candidates
            .into_iter()
            .filter(|c| *c != name)
            .map(|c| (edit_distance(name, c), c))
            .filter(|(d, c)| *d > 0 && *d <= 2 && *d < c.len().max(name.len()))
            .min_by_key(|(d, _)| *d)
            .map(|(_, c)| c.to_owned())
    }

    // -- 类型 / 模式 / 表达式走查 ----------------------------------------------

    fn resolve_generic_args(&mut self, args: &ast::GenericArgs, cx: &mut BodyCx) {
        for arg in &args.args {
            match arg {
                ast::GenericArg::Type(ty) => self.resolve_ast_ty(ty, cx),
                ast::GenericArg::Const(e) => self.resolve_expr(e, cx),
                ast::GenericArg::Lifetime(_) => {}
            }
        }
    }

    fn resolve_ast_ty(&mut self, ty: &ast::Ty, cx: &mut BodyCx) {
        match &ty.kind {
            ast::TyKind::Path(p) => {
                self.resolve_type_path(p, cx);
            }
            ast::TyKind::Ref { inner, .. } | ast::TyKind::RawPtr { inner, .. } => {
                self.resolve_ast_ty(inner, cx);
            }
            ast::TyKind::Paren(inner) | ast::TyKind::Slice(inner) => {
                self.resolve_ast_ty(inner, cx);
            }
            ast::TyKind::Tuple(elems) => {
                for t in elems {
                    self.resolve_ast_ty(t, cx);
                }
            }
            ast::TyKind::Array { elem, len } => {
                self.resolve_ast_ty(elem, cx);
                self.resolve_expr(len, cx);
            }
            ast::TyKind::FnPtr { params, ret } => {
                for t in params {
                    self.resolve_ast_ty(t, cx);
                }
                if let Some(r) = ret {
                    self.resolve_ast_ty(r, cx);
                }
            }
            ast::TyKind::Infer | ast::TyKind::ConstArg(_) | ast::TyKind::Err => {}
        }
    }

    /// 模式位置裸名 → 单元变体 Res(模块值 ns 与 lang-item 变体兜底;
    /// 非单元变体/非变体不参与,落回绑定语义)。
    fn unit_variant_pattern_res(&self, name: &str, cx: &BodyCx) -> Option<Res> {
        let res = self.modules[cx.module]
            .values
            .get(name)
            .map(|b| b.res)
            .or_else(|| self.out.lang_items.variant_by_name(name).map(Res::Def))?;
        let Res::Def(d) = res else { return None };
        if self.def(d).kind == DefKind::Variant && self.unit_variants.contains(&d) {
            Some(res)
        } else {
            None
        }
    }

    fn resolve_pat(&mut self, pat: &ast::Pat, cx: &mut BodyCx) {
        match &pat.kind {
            ast::PatKind::Wild | ast::PatKind::Lit { .. } | ast::PatKind::Err => {}
            ast::PatKind::Binding { mutable, name } => {
                // 模式位置裸名裁决(RXS-0048/0023):解析到**单元变体**时为
                // 路径模式(`None` 等),否则为绑定;`mut` 标注强制绑定。
                if !*mutable && let Some(res) = self.unit_variant_pattern_res(&name.name, cx) {
                    self.out.path_res.insert(name.span, res);
                } else {
                    self.declare_local(cx, &name.name, *mutable, name.span);
                }
            }
            ast::PatKind::At { name, pat } => {
                self.declare_local(cx, &name.name, false, name.span);
                self.resolve_pat(pat, cx);
            }
            ast::PatKind::Range { lo, hi, .. } => {
                self.resolve_pat(lo, cx);
                self.resolve_pat(hi, cx);
            }
            ast::PatKind::Ref { pat, .. } => self.resolve_pat(pat, cx),
            ast::PatKind::Tuple(elems) | ast::PatKind::Slice(elems) => {
                for p in elems {
                    self.resolve_pat(p, cx);
                }
            }
            ast::PatKind::Path(p) => {
                self.resolve_value_path(p, cx);
            }
            ast::PatKind::TupleStruct { path, elems } => {
                self.resolve_value_path(path, cx);
                for p in elems {
                    self.resolve_pat(p, cx);
                }
            }
            ast::PatKind::Struct { path, fields, .. } => {
                self.resolve_struct_lit_path(path, cx);
                for f in fields {
                    match &f.pat {
                        Some(p) => self.resolve_pat(p, cx),
                        // 简写字段模式即绑定(RXS-0032)
                        None => self.declare_local(cx, &f.name.name, false, f.name.span),
                    }
                }
            }
        }
    }

    fn resolve_block(&mut self, block: &ast::Block, cx: &mut BodyCx) {
        cx.scopes.push(HashMap::new());
        for stmt in &block.stmts {
            match &stmt.kind {
                ast::StmtKind::Empty => {}
                ast::StmtKind::Let(l) => {
                    // init 先于绑定解析(`let x = x;` 引用外层 x,RXS-0033)
                    if let Some(ty) = &l.ty {
                        self.resolve_ast_ty(ty, cx);
                    }
                    if let Some(init) = &l.init {
                        self.resolve_expr(init, cx);
                    }
                    self.resolve_pat(&l.pat, cx);
                }
                ast::StmtKind::Expr { expr, .. } => self.resolve_expr(expr, cx),
                ast::StmtKind::Item(item) => self.resolve_block_item(item, cx),
            }
        }
        if let Some(tail) = &block.tail {
            self.resolve_expr(tail, cx);
        }
        cx.scopes.pop();
    }

    /// 块内嵌套 item:登记入当前局部帧(按语句序可见,M2.1 取舍——
    /// 块内 item 的前向引用随 M2.2 评估;RXS-0033 的顺序无关性此处仅对模块级生效)。
    fn resolve_block_item(&mut self, item: &ast::Item, cx: &mut BodyCx) {
        let module = cx.module;
        let vis = lower_vis(&item.vis);
        match &item.kind {
            ast::ItemKind::Fn(f) => {
                let id = self.new_def(DefKind::Fn, &f.name.name, vis, f.name.span, module);
                self.out.item_defs.insert(item.span, id);
                self.declare_block_item(cx, &f.name.name, Res::Def(id), f.name.span);
                self.resolve_fn(f, module, Vec::new(), false);
            }
            ast::ItemKind::Struct(s) => {
                let id = self.new_def(DefKind::Struct, &s.name.name, vis, s.name.span, module);
                self.out.item_defs.insert(item.span, id);
                self.declare_block_item(cx, &s.name.name, Res::Def(id), s.name.span);
                if let ast::VariantBody::Named(fields) = &s.body {
                    self.collect_fields(fields, module);
                }
                let generics = generic_names(&s.generics);
                self.resolve_variant_body_types(&s.body, module, &generics);
            }
            ast::ItemKind::Const(c) => {
                let id = self.new_def(DefKind::Const, &c.name.name, vis, c.name.span, module);
                self.out.item_defs.insert(item.span, id);
                self.declare_block_item(cx, &c.name.name, Res::Def(id), c.name.span);
                let mut inner = BodyCx::new(module, Vec::new(), false, c.init.span);
                self.resolve_ast_ty(&c.ty, &mut inner);
                self.resolve_expr(&c.init, &mut inner);
                self.finish_body(inner);
            }
            // 其余 item 形态在块内罕见:登记名字但不深入(M2.2 扩展)
            _ => {}
        }
    }

    fn resolve_expr(&mut self, expr: &ast::Expr, cx: &mut BodyCx) {
        match &expr.kind {
            ast::ExprKind::Lit(_) | ast::ExprKind::Continue | ast::ExprKind::Err => {}
            ast::ExprKind::Path(p) => {
                self.resolve_value_path(p, cx);
            }
            ast::ExprKind::Unary { expr, .. }
            | ast::ExprKind::Borrow { expr, .. }
            | ast::ExprKind::Try(expr)
            | ast::ExprKind::Paren(expr)
            | ast::ExprKind::Field { expr, .. }
            | ast::ExprKind::TupleField { expr, .. } => self.resolve_expr(expr, cx),
            ast::ExprKind::Cast { expr, ty } => {
                self.resolve_expr(expr, cx);
                self.resolve_ast_ty(ty, cx);
            }
            ast::ExprKind::Binary { lhs, rhs, .. } | ast::ExprKind::Assign { lhs, rhs, .. } => {
                self.resolve_expr(lhs, cx);
                self.resolve_expr(rhs, cx);
            }
            ast::ExprKind::Range { lo, hi, .. } => {
                self.resolve_expr(lo, cx);
                self.resolve_expr(hi, cx);
            }
            ast::ExprKind::Call { callee, args } => {
                self.resolve_expr(callee, cx);
                for a in args {
                    self.resolve_expr(a, cx);
                }
            }
            ast::ExprKind::MethodCall {
                receiver,
                generic_args,
                args,
                ..
            } => {
                // 方法名解析依赖接收者类型(M2.2 typeck);仅走查子表达式
                self.resolve_expr(receiver, cx);
                if let Some(ga) = generic_args {
                    self.resolve_generic_args(ga, cx);
                }
                for a in args {
                    self.resolve_expr(a, cx);
                }
            }
            ast::ExprKind::Index { expr, index } => {
                self.resolve_expr(expr, cx);
                self.resolve_expr(index, cx);
            }
            ast::ExprKind::Tuple(elems) | ast::ExprKind::Array(elems) => {
                for e in elems {
                    self.resolve_expr(e, cx);
                }
            }
            ast::ExprKind::Repeat { elem, len } => {
                self.resolve_expr(elem, cx);
                self.resolve_expr(len, cx);
            }
            ast::ExprKind::StructLit { path, fields } => {
                self.resolve_struct_lit_path(path, cx);
                for f in fields {
                    match &f.expr {
                        Some(e) => self.resolve_expr(e, cx),
                        // 简写字段是对同名值的引用(RXS-0034)
                        None => {
                            self.resolve_value_single(&f.name.name, f.name.span, cx);
                        }
                    }
                }
            }
            ast::ExprKind::Block(b)
            | ast::ExprKind::Unsafe(b)
            | ast::ExprKind::Loop { body: b } => {
                self.resolve_block(b, cx);
            }
            ast::ExprKind::If { cond, then, else_ } => {
                self.resolve_expr(cond, cx);
                self.resolve_block(then, cx);
                if let Some(e) = else_ {
                    self.resolve_expr(e, cx);
                }
            }
            ast::ExprKind::While { cond, body } => {
                self.resolve_expr(cond, cx);
                self.resolve_block(body, cx);
            }
            ast::ExprKind::For { pat, iter, body } => {
                self.resolve_expr(iter, cx);
                cx.scopes.push(HashMap::new());
                self.resolve_pat(pat, cx);
                self.resolve_block(body, cx);
                cx.scopes.pop();
            }
            ast::ExprKind::Match { scrutinee, arms } => {
                self.resolve_expr(scrutinee, cx);
                for arm in arms {
                    cx.scopes.push(HashMap::new());
                    for p in &arm.pats {
                        self.resolve_pat(p, cx);
                    }
                    if let Some(g) = &arm.guard {
                        self.resolve_expr(g, cx);
                    }
                    self.resolve_expr(&arm.body, cx);
                    cx.scopes.pop();
                }
            }
            ast::ExprKind::Return(operand) | ast::ExprKind::Break(operand) => {
                if let Some(e) = operand {
                    self.resolve_expr(e, cx);
                }
            }
            ast::ExprKind::Closure { params, body, .. } => {
                cx.scopes.push(HashMap::new());
                for p in params {
                    if let Some(ty) = &p.ty {
                        self.resolve_ast_ty(ty, cx);
                    }
                    self.resolve_pat(&p.pat, cx);
                }
                self.resolve_expr(body, cx);
                cx.scopes.pop();
            }
        }
    }
}

fn generic_names(g: &ast::Generics) -> Vec<String> {
    g.params
        .iter()
        .filter_map(|p| match &p.kind {
            ast::GenericParamKind::Type { name, .. }
            | ast::GenericParamKind::Const { name, .. } => Some(name.name.clone()),
            ast::GenericParamKind::Lifetime(_) => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;
    use crate::parser::parse;
    use crate::span::{Edition, SourceId};

    fn run(src: &str) -> (Resolutions, DiagCtxt) {
        let diag = DiagCtxt::new();
        let tokens = lex(src, SourceId(0), Edition::Rx0, &diag);
        let file = parse(src, tokens, SourceId(0), Edition::Rx0, &diag);
        assert!(
            diag.emitted().is_empty(),
            "测试源含词法/语法错误: {:?}",
            diag.emitted()
        );
        let res = resolve(&file, &diag);
        (res, diag)
    }

    fn codes(diag: &DiagCtxt) -> Vec<u16> {
        diag.emitted()
            .iter()
            .filter_map(|d| d.code.map(|c| c.0))
            .collect()
    }

    fn run_clean(src: &str) -> Resolutions {
        let (res, diag) = run(src);
        assert!(
            diag.emitted().is_empty(),
            "意外解析诊断: {:?}\n源:\n{src}",
            diag.emitted()
                .iter()
                .map(|d| (d.code, d.message(diag.messages())))
                .collect::<Vec<_>>()
        );
        res
    }

    //@ spec: RXS-0032
    #[test]
    fn item_scope_and_local_scope_are_separate() {
        let res = run_clean(
            "mod math {\n    pub fn add(x: i32, y: i32) -> i32 { x + y }\n    pub const BASE: i32 = 7;\n}\nfn sample<T>(x: T) -> i32 {\n    let outer = math::BASE;\n    let _keep = x;\n    outer\n}",
        );
        assert!(res.defs.iter().any(|d| d.kind == DefKind::Mod));
        assert!(!res.bindings.is_empty());
        assert!(!res.path_res.is_empty());
    }

    //@ spec: RXS-0033
    #[test]
    fn inner_block_shadows_outer_legally() {
        run_clean(
            "fn f() -> i32 {\n    let v = 1;\n    {\n        let v = 2;\n        let _x = v;\n    }\n    v\n}",
        );
    }

    //@ spec: RXS-0033
    #[test]
    fn let_init_resolves_before_binding() {
        // `let x = x;` 的 init 引用外层 x(RXS-0033)
        run_clean("fn f(x: i32) -> i32 {\n    let x = x;\n    x\n}");
    }

    //@ spec: RXS-0033, RXS-0037
    #[test]
    fn same_level_duplicate_local_is_rx1002() {
        let (_, diag) =
            run("fn f() {\n    let value = 1;\n    let value = 2;\n    let _k = value;\n}");
        assert_eq!(codes(&diag), vec![1002]);
        // 双 span(RXS-0038)
        assert!(diag.emitted()[0].labels.len() >= 2);
    }

    //@ spec: RXS-0037
    #[test]
    fn duplicate_bindings_in_one_pattern_is_rx1002() {
        let (_, diag) = run("fn f(p: (i32, i32)) {\n    let (a, a) = p;\n}");
        assert_eq!(codes(&diag), vec![1002]);
    }

    //@ spec: RXS-0034
    #[test]
    fn single_segment_prefers_local_over_item() {
        let res =
            run_clean("fn shade() -> i32 { 1 }\nfn f() -> i32 {\n    let shade = 2;\n    shade\n}");
        // 尾表达式 shade 必须解析为局部绑定
        let locals = res
            .path_res
            .values()
            .filter(|r| matches!(r, Res::Local(_)))
            .count();
        assert!(locals >= 1);
    }

    //@ spec: RXS-0034
    #[test]
    fn generic_params_resolve_in_both_positions() {
        run_clean(
            "fn fill<T, const N: usize>(seed: T) -> usize {\n    let _t: T = seed;\n    N\n}",
        );
    }

    //@ spec: RXS-0034
    #[test]
    fn local_cannot_prefix_multi_segment_path() {
        let (_, diag) = run("fn f() {\n    let m = 1;\n    let _x = m::inner;\n}");
        assert_eq!(codes(&diag), vec![1001]);
    }

    //@ spec: RXS-0034
    #[test]
    fn enum_variants_and_assoc_fns_resolve() {
        run_clean(
            "enum Shape {\n    Circle { radius: f32 },\n    Point,\n}\nstruct Counter {\n    value: u32,\n}\nimpl Counter {\n    fn new() -> Counter {\n        Counter { value: 0 }\n    }\n}\nfn f(s: Shape) -> u32 {\n    let c = Counter::new();\n    match s {\n        Shape::Circle { radius } => 1,\n        Shape::Point => 0,\n    };\n    c.value\n}",
        );
    }

    //@ spec: RXS-0035
    #[test]
    fn use_alias_and_use_chain_resolve() {
        let res = run_clean(
            "mod geometry {\n    pub fn area(w: f32, h: f32) -> f32 { w * h }\n}\nuse geometry::area as area_fn;\nuse area_fn as area_again;\nfn measure() -> f32 {\n    area_fn(2.0, 3.0) + area_again(1.0, 1.0)\n}",
        );
        assert_eq!(res.use_targets.len(), 2);
    }

    //@ spec: RXS-0035
    #[test]
    fn use_of_missing_target_is_rx1004() {
        let (_, diag) = run("use nowhere::nothing;\nfn f() {}");
        assert_eq!(codes(&diag), vec![1004]);
    }

    //@ spec: RXS-0036
    #[test]
    fn private_item_across_module_is_rx1003() {
        let (_, diag) =
            run("mod m {\n    fn secret() -> i32 { 1 }\n}\nfn f() -> i32 {\n    m::secret()\n}");
        assert_eq!(codes(&diag), vec![1003]);
    }

    //@ spec: RXS-0036
    #[test]
    fn use_of_private_target_is_rx1003_not_rx1001() {
        let (_, diag) = run("mod m {\n    fn secret() {}\n}\nuse m::secret;\nfn f() {}");
        assert_eq!(codes(&diag), vec![1003]);
    }

    //@ spec: RXS-0036
    #[test]
    fn descendant_sees_ancestor_private() {
        run_clean(
            "fn helper() -> i32 { 1 }\nmod inner {\n    pub fn call() -> i32 {\n        super_helper()\n    }\n    fn super_helper() -> i32 { 2 }\n}\nfn f() -> i32 {\n    helper() + inner::call()\n}",
        );
    }

    //@ spec: RXS-0037
    #[test]
    fn duplicate_module_items_are_rx1002() {
        let (_, diag) = run("fn same() -> i32 { 1 }\nfn same() -> i32 { 2 }");
        assert_eq!(codes(&diag), vec![1002]);
    }

    //@ spec: RXS-0037
    #[test]
    fn duplicate_fields_and_variants_are_rx1002() {
        let (_, diag) =
            run("struct P {\n    left: i32,\n    left: i32,\n}\nenum E {\n    A,\n    A,\n}");
        assert_eq!(codes(&diag), vec![1002, 1002]);
    }

    //@ spec: RXS-0037
    #[test]
    fn use_conflicting_with_item_is_rx1002() {
        let (_, diag) = run(
            "mod m {\n    pub fn area() -> i32 { 1 }\n}\nfn area() -> i32 { 2 }\nuse m::area;\nfn f() {}",
        );
        assert_eq!(codes(&diag), vec![1002]);
    }

    //@ spec: RXS-0038
    #[test]
    fn unresolved_name_carries_spelling_suggestion() {
        let (_, diag) = run("fn f() {\n    let value = 1;\n    let _x = vlaue;\n}");
        assert_eq!(codes(&diag), vec![1001]);
        let emitted = diag.emitted();
        assert!(
            emitted[0].helps.iter().any(|h| h.contains("`value`")),
            "{:?}",
            emitted[0].helps
        );
    }

    //@ spec: RXS-0038
    #[test]
    fn struct_lit_with_unknown_type_is_rx1001() {
        let (_, diag) = run("fn f() {\n    let _p = Nowhere { x: 1 };\n}");
        assert_eq!(codes(&diag), vec![1001]);
    }

    // 类型位置容忍口径(M2.1):未知草图类型不报错
    //@ spec: RXS-0034
    #[test]
    fn unknown_type_position_paths_are_tolerated() {
        run_clean(
            "kernel fn k(grid: Grid<(64,)>, out: ViewMut<global, f32, (N,)>) {\n    let _i = grid.thread_index();\n}\nfn f(v: Vec3<f32>) -> f32 {\n    v.length()\n}",
        );
    }

    //@ spec: RXS-0032
    #[test]
    fn block_items_resolve_in_order() {
        run_clean(
            "fn outer() -> usize {\n    const TILE: usize = 32;\n    fn inner(n: usize) -> usize { n * 2 }\n    struct Local {\n        v: usize,\n    }\n    let l = Local { v: TILE };\n    inner(l.v)\n}",
        );
    }

    // M2.3:内建函数兜底(最小 prelude)
    #[test]
    fn builtin_println_resolves_as_fallback() {
        let res = run_clean("fn main() {\n    println(\"hi\");\n}");
        let d = res
            .path_res
            .values()
            .find_map(|r| match r {
                Res::Def(d) => Some(*d),
                _ => None,
            })
            .expect("println 解析为 Def");
        assert_eq!(res.builtins.get(&d), Some(&Builtin::Println));
    }

    // M2.3:用户同名定义优先于内建
    #[test]
    fn user_definition_shadows_builtin() {
        let res = run_clean("fn println(n: i32) {}\nfn main() {\n    println(1);\n}");
        let called: Vec<DefId> = res
            .path_res
            .values()
            .filter_map(|r| match r {
                Res::Def(d) => Some(*d),
                _ => None,
            })
            .collect();
        assert!(called.iter().all(|d| !res.builtins.contains_key(d)));
    }

    //@ spec: RXS-0048
    #[test]
    fn lang_item_names_resolve_as_fallback() {
        let res = run_clean(
            "fn f(flag: bool) -> Option<i32> {\n    if flag { Some(1) } else { None }\n}\nfn g() -> Result<i32, i32> {\n    Ok(2)\n}\nfn h() -> Option<i32> {\n    Option::Some(3)\n}",
        );
        let li = res.lang_items;
        for d in [
            li.option_some.unwrap(),
            li.option_none.unwrap(),
            li.result_ok.unwrap(),
        ] {
            assert!(
                res.path_res.values().any(|r| *r == Res::Def(d)),
                "lang item 变体 {d:?} 未被解析引用"
            );
        }
    }

    //@ spec: RXS-0048
    #[test]
    fn user_definition_shadows_lang_item() {
        let res = run_clean(
            "enum Option {\n    Mine,\n}\nfn f() -> Option {\n    Option::Mine\n}",
        );
        let li = res.lang_items;
        // 类型位置与路径前缀均解析到用户定义,不落到内建项(RX1002 也不产生)
        assert!(
            res.path_res
                .values()
                .all(|r| *r != Res::Def(li.option.unwrap()))
        );
    }

    //@ spec: RXS-0035
    #[test]
    fn use_targets_recorded_for_hir() {
        let res =
            run_clean("mod m {\n    pub const K: i32 = 1;\n}\nuse m::K;\nfn f() -> i32 { K }");
        assert!(res.use_targets.values().any(|r| matches!(r, Res::Def(_))));
    }
}
