//! AST → HIR lowering(07 §1 / D-202;消费 [`crate::resolve::Resolutions`])。
//!
//! - **item/body 分离**:函数体/const 初始化器降级为独立 [`hir::Body`],
//!   item 仅持 [`hir::BodyId`](RXS-0032 实现要求,增量依赖边界);
//! - 路径节点替换为已解析 [`hir::Res`](span 键查 `Resolutions::path_res`);
//! - struct 字面量简写字段在此 desugar 为显式 `Res` 表达式;
//! - **`for` / `?` 在此 desugar**(RXS-0049/RXS-0050 等价形式;M3.1 收口):
//!   展开引用 RXS-0048 编译器已知项(内建 Option/Result,经 `lang_items`
//!   直引 DefId,不受用户遮蔽影响);合成局部追加到所在 body 的局部表,
//!   合成推进步用 [`hir::ExprKind::SynthInt`](无源文本支撑);
//! - extern 块在 HIR 中展平为其成员 fn(无独立节点形态,MVP 取舍)。

use crate::ast::{self, BinOp};
use crate::hir::{self, BodyId, DefId, HirId, LocalId, Res};
use crate::resolve::Resolutions;
use crate::span::Span;

/// lowering 入口:产出 HIR crate(解析诊断已由 resolve 阶段报出,本阶段无诊断)。
pub fn lower(file: &ast::SourceFile, res: &Resolutions) -> hir::Crate {
    let mut lw = Lowerer {
        res,
        krate: hir::Crate::default(),
        next_hir: 0,
        cur_locals: Vec::new(),
    };
    // 槽位预填:resolver 分配过的全部 DefId 都有 Item 槽(字段等保持 Err 占位)
    for (i, d) in res.defs.iter().enumerate() {
        lw.krate.items.push(hir::Item {
            def_id: DefId(i as u32),
            name: d.name.clone(),
            kind: hir::ItemKind::Err,
            vis: d.vis,
            span: d.span,
        });
    }
    lw.install_lang_items();
    let root: Vec<DefId> = file.items.iter().flat_map(|it| lw.lower_item(it)).collect();
    lw.krate.root_items = root;
    lw.krate
}

struct Lowerer<'a> {
    res: &'a Resolutions,
    krate: hir::Crate,
    /// HirId 分配计数(crate 内全局递增;clone_pat 共享原 id,见该方法注释)。
    next_hir: u32,
    /// 当前 body 的局部声明表(resolver 产物起底;desugar 合成局部追加于此,
    /// body 收尾时整体取走——LocalId 即本表下标,追加不动摇既有编号)。
    cur_locals: Vec<hir::LocalDecl>,
}

impl Lowerer<'_> {
    fn next_hir_id(&mut self) -> HirId {
        let id = HirId(self.next_hir);
        self.next_hir += 1;
        id
    }

    fn path_res(&self, span: crate::span::Span) -> Res {
        self.res.path_res.get(&span).copied().unwrap_or(Res::Err)
    }

    fn binding_local(&self, span: crate::span::Span) -> LocalId {
        // 解析错误恢复路径上可能缺失;LocalId(u32::MAX) 为占位哨兵
        self.res
            .bindings
            .get(&span)
            .copied()
            .unwrap_or(LocalId(u32::MAX))
    }

    fn def_of(&self, span: crate::span::Span) -> Option<DefId> {
        self.res.item_defs.get(&span).copied()
    }

    fn set_item(&mut self, id: DefId, kind: hir::ItemKind) {
        self.krate.items[id.0 as usize].kind = kind;
    }

    /// 追加 resolver 未分配 DefId 的合成 item(impl 块)。
    fn push_synthetic(
        &mut self,
        name: &str,
        kind: hir::ItemKind,
        span: crate::span::Span,
    ) -> DefId {
        let id = DefId(self.krate.items.len() as u32);
        self.krate.items.push(hir::Item {
            def_id: id,
            name: name.to_owned(),
            kind,
            vis: hir::Vis::Private,
            span,
        });
        id
    }

    fn alloc_body(&mut self, body: hir::Body) -> BodyId {
        let id = BodyId(self.krate.bodies.len() as u32);
        self.krate.bodies.push(body);
        id
    }

    /// `#[derive(Copy)]` 标注登记(RXS-0053;仅 struct/enum 调用本方法,
    /// 合法性由 typeck 定义处检查裁决 RX2008)。
    fn record_copy_derive(&mut self, id: DefId, attrs: &[ast::Attr]) {
        for a in attrs {
            if a.inner {
                continue;
            }
            let m = &a.meta;
            let is_derive = m.path.segments.len() == 1 && m.path.segments[0].ident.name == "derive";
            if !is_derive {
                continue;
            }
            let ast::MetaKind::List(inner) = &m.kind else {
                continue;
            };
            for entry in inner {
                if let ast::MetaInner::Meta(mi) = entry
                    && mi.path.segments.len() == 1
                    && mi.path.segments[0].ident.name == "Copy"
                {
                    self.krate.copy_derives.insert(id, a.span);
                    return;
                }
            }
        }
    }

    /// 安装编译器已知项的 HIR 形态(RXS-0048):内建 Option/Result enum。
    /// resolver 已分配 DefId(槽位预填为 Err),此处补 Enum/Variant item;
    /// 载荷字段类型 = 泛型参数(Some/Ok → Param 0,Err → Param 1)。
    fn install_lang_items(&mut self) {
        let li = self.res.lang_items;
        let span = Span::new(crate::span::SourceId(0), 0, 0, crate::span::Edition::Rx0);
        let param_field = |i: u32| hir::FieldDef {
            name: "0".to_owned(),
            vis: hir::Vis::Pub,
            ty: hir::Ty {
                kind: hir::TyKind::Res(Res::GenericParam(i), Vec::new()),
                span,
            },
            span,
        };
        let (Some(option), Some(none), Some(some), Some(result), Some(ok), Some(err)) = (
            li.option,
            li.option_none,
            li.option_some,
            li.result,
            li.result_ok,
            li.result_err,
        ) else {
            unreachable!("lang items 在 resolve 入口注入(RXS-0048)");
        };
        self.set_item(
            option,
            hir::ItemKind::Enum {
                variants: vec![none, some],
            },
        );
        self.set_item(none, hir::ItemKind::Variant { fields: Vec::new() });
        self.set_item(
            some,
            hir::ItemKind::Variant {
                fields: vec![param_field(0)],
            },
        );
        self.set_item(
            result,
            hir::ItemKind::Enum {
                variants: vec![ok, err],
            },
        );
        self.set_item(
            ok,
            hir::ItemKind::Variant {
                fields: vec![param_field(0)],
            },
        );
        self.set_item(
            err,
            hir::ItemKind::Variant {
                fields: vec![param_field(1)],
            },
        );
        // 内建 Drop trait(RXS-0055:识别锚点,无关联项形态)
        if let Some(drop_trait) = li.drop_trait {
            self.set_item(drop_trait, hir::ItemKind::Trait { items: Vec::new() });
        }
        // 设备 View 族容器 + 地址空间标记(RXS-0067):空字段 struct 形态——
        // 地址空间作为类型实参由 lower_ty 原样携带(typeck 合一处裁决)。
        for d in li
            .view
            .into_iter()
            .chain(li.view_mut)
            .chain(li.buffer)
            .chain(li.addr_spaces.into_iter().flatten())
        {
            self.set_item(d, hir::ItemKind::Struct { fields: Vec::new() });
        }
    }

    /// 降级一个 item;返回其在所属容器中的 DefId 列表(extern 块展平为多个)。
    fn lower_item(&mut self, item: &ast::Item) -> Vec<DefId> {
        match &item.kind {
            ast::ItemKind::Fn(f) => {
                let Some(id) = self.def_of(item.span) else {
                    return Vec::new();
                };
                let decl = self.lower_fn(f, id);
                self.set_item(id, hir::ItemKind::Fn(decl));
                vec![id]
            }
            ast::ItemKind::Struct(s) => {
                let Some(id) = self.def_of(item.span) else {
                    return Vec::new();
                };
                self.record_copy_derive(id, &item.attrs);
                let fields = self.lower_variant_fields(&s.body);
                self.set_item(id, hir::ItemKind::Struct { fields });
                vec![id]
            }
            ast::ItemKind::Enum(e) => {
                let Some(id) = self.def_of(item.span) else {
                    return Vec::new();
                };
                self.record_copy_derive(id, &item.attrs);
                let mut variants = Vec::new();
                for v in &e.variants {
                    if let Some(vid) = self.def_of(v.span) {
                        let fields = self.lower_variant_fields(&v.body);
                        self.set_item(vid, hir::ItemKind::Variant { fields });
                        variants.push(vid);
                    }
                }
                self.set_item(id, hir::ItemKind::Enum { variants });
                vec![id]
            }
            ast::ItemKind::Trait(t) => {
                let Some(id) = self.def_of(item.span) else {
                    return Vec::new();
                };
                let items = self.lower_assoc_items(&t.items);
                self.set_item(id, hir::ItemKind::Trait { items });
                vec![id]
            }
            ast::ItemKind::Impl(im) => {
                let self_res = match &im.self_ty.kind {
                    ast::TyKind::Path(p) => self.path_res(p.span),
                    _ => Res::Err,
                };
                // trait impl 的 trait 路径解析(RXS-0055 Drop 识别面的输入)
                let trait_res = im.trait_ty.as_ref().map(|t| match &t.kind {
                    ast::TyKind::Path(p) => self.path_res(p.span),
                    _ => Res::Err,
                });
                let items = self.lower_assoc_items(&im.items);
                let id = self.push_synthetic(
                    "<impl>",
                    hir::ItemKind::Impl {
                        self_res,
                        trait_res,
                        items,
                    },
                    item.span,
                );
                // `impl Drop for T` 登记(RXS-0055:trait 路径绑定到内建
                // Drop 时识别;用户遮蔽的同名 trait 不参与)
                if trait_res
                    == Some(Res::Def(
                        self.res.lang_items.drop_trait.unwrap_or(DefId(u32::MAX)),
                    ))
                {
                    let adt = match self_res {
                        Res::Def(d) => Some(d),
                        _ => None,
                    };
                    self.krate.drop_impls.push(hir::DropImpl {
                        adt,
                        impl_def: id,
                        span: item.span,
                    });
                }
                vec![id]
            }
            ast::ItemKind::Mod(m) => {
                let Some(id) = self.def_of(item.span) else {
                    return Vec::new();
                };
                let children: Vec<DefId> =
                    m.items.iter().flat_map(|it| self.lower_item(it)).collect();
                self.set_item(id, hir::ItemKind::Mod { items: children });
                vec![id]
            }
            ast::ItemKind::Use(u) => {
                // use 的 DefId 未在 resolver 分配(其为别名而非定义);合成节点
                // 保留已解析目标与导出名(RXS-0035 实现要求)
                let target = self
                    .res
                    .use_targets
                    .get(&u.path.span)
                    .copied()
                    .unwrap_or(Res::Err);
                let name = u
                    .alias
                    .as_ref()
                    .map(|a| a.name.clone())
                    .unwrap_or_else(|| u.path.segments.last().unwrap().ident.name.clone());
                let id = self.push_synthetic(&name, hir::ItemKind::Use { target }, item.span);
                vec![id]
            }
            ast::ItemKind::Static(s) => {
                let Some(id) = self.def_of(item.span) else {
                    return Vec::new();
                };
                let ty = self.lower_ty(&s.ty);
                let body = self.lower_value_body(id, &s.init);
                self.set_item(
                    id,
                    hir::ItemKind::Static {
                        mutable: s.mutable,
                        ty,
                        body,
                    },
                );
                vec![id]
            }
            ast::ItemKind::Const(c) => {
                let Some(id) = self.def_of(item.span) else {
                    return Vec::new();
                };
                let ty = self.lower_ty(&c.ty);
                let body = self.lower_value_body(id, &c.init);
                self.set_item(id, hir::ItemKind::Const { ty, body });
                vec![id]
            }
            ast::ItemKind::TypeAlias(t) => {
                let Some(id) = self.def_of(item.span) else {
                    return Vec::new();
                };
                let ty = self.lower_ty(&t.ty);
                self.set_item(id, hir::ItemKind::TypeAlias { ty });
                vec![id]
            }
            ast::ItemKind::ExternBlock(e) => {
                // 展平:成员 fn 直接进入容器
                e.items.iter().flat_map(|it| self.lower_item(it)).collect()
            }
            ast::ItemKind::Err => Vec::new(),
        }
    }

    fn lower_assoc_items(&mut self, items: &[ast::AssocItem]) -> Vec<DefId> {
        let mut out = Vec::new();
        for a in items {
            let Some(id) = self.def_of(a.span) else {
                continue;
            };
            match &a.kind {
                ast::AssocItemKind::Fn(f) => {
                    let decl = self.lower_fn(f, id);
                    self.set_item(id, hir::ItemKind::Fn(decl));
                }
                ast::AssocItemKind::Type { default, .. } => {
                    let kind = match default {
                        Some(ty) => {
                            let t = self.lower_ty(ty);
                            hir::ItemKind::TypeAlias { ty: t }
                        }
                        None => hir::ItemKind::AssocType,
                    };
                    self.set_item(id, kind);
                }
                ast::AssocItemKind::Const(c) => {
                    let ty = self.lower_ty(&c.ty);
                    let body = self.lower_value_body(id, &c.init);
                    self.set_item(id, hir::ItemKind::Const { ty, body });
                }
            }
            out.push(id);
        }
        out
    }

    fn lower_fn(&mut self, f: &ast::FnItem, owner: DefId) -> hir::FnDecl {
        let generic_params = f
            .generics
            .params
            .iter()
            .filter_map(|p| match &p.kind {
                ast::GenericParamKind::Type { name, .. }
                | ast::GenericParamKind::Const { name, .. } => Some(name.name.clone()),
                ast::GenericParamKind::Lifetime(_) => None,
            })
            .collect();
        let self_kind = f.params.first().and_then(|p| match &p.kind {
            ast::ParamKind::SelfParam {
                by_ref, mutable, ..
            } => Some(hir::SelfKind {
                by_ref: *by_ref,
                mutable: *mutable,
            }),
            _ => None,
        });
        let params: Vec<hir::Param> = f
            .params
            .iter()
            .map(|p| match &p.kind {
                ast::ParamKind::SelfParam { .. } => hir::Param {
                    pat: hir::Pat {
                        hir_id: self.next_hir_id(),
                        kind: hir::PatKind::Binding {
                            local: self.binding_local(p.span),
                        },
                        span: p.span,
                    },
                    ty: None,
                    span: p.span,
                },
                ast::ParamKind::Typed { pat, ty } => hir::Param {
                    pat: self.lower_pat(pat),
                    ty: Some(self.lower_ty(ty)),
                    span: p.span,
                },
            })
            .collect();
        let ret = f.ret.as_ref().map(|t| self.lower_ty(t));
        let body = f.body.as_ref().map(|block| {
            // 当前 body 局部表起底(嵌套 item body 经保存/恢复隔离)
            let resolved = self.resolved_locals(block.span);
            let saved = std::mem::replace(&mut self.cur_locals, resolved);
            let value = hir::Expr {
                hir_id: self.next_hir_id(),
                span: block.span,
                kind: hir::ExprKind::Block(self.lower_block(block)),
            };
            let locals = std::mem::replace(&mut self.cur_locals, saved);
            self.alloc_body(hir::Body {
                owner,
                locals,
                params: params.iter().map(|p| self.clone_pat(&p.pat)).collect(),
                value,
            })
        });
        hir::FnDecl {
            color: f.color,
            stage: f.stage,
            generic_params,
            params,
            self_kind,
            ret,
            body,
        }
    }

    /// resolver 登记的 body 局部表(body 键 = fn 体块 span / 初始化器 span)。
    fn resolved_locals(&self, body_key: Span) -> Vec<hir::LocalDecl> {
        self.res
            .body_locals
            .get(&body_key)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|l| hir::LocalDecl {
                name: l.name,
                mutable: l.mutable,
                span: l.span,
            })
            .collect()
    }

    /// desugar 合成局部(RXS-0049/0050:不可被用户代码引用——不入 resolver
    /// 作用域,仅追加到当前 body 局部表)。
    fn fresh_local(&mut self, name: &str, mutable: bool, span: Span) -> LocalId {
        let id = LocalId(self.cur_locals.len() as u32);
        self.cur_locals.push(hir::LocalDecl {
            name: name.to_owned(),
            mutable,
            span,
        });
        id
    }

    /// const/static/关联 const 的初始化器 body。
    fn lower_value_body(&mut self, owner: DefId, init: &ast::Expr) -> BodyId {
        let resolved = self.resolved_locals(init.span);
        let saved = std::mem::replace(&mut self.cur_locals, resolved);
        let value = self.lower_expr(init);
        let locals = std::mem::replace(&mut self.cur_locals, saved);
        self.alloc_body(hir::Body {
            owner,
            locals,
            params: Vec::new(),
            value,
        })
    }

    fn lower_variant_fields(&mut self, body: &ast::VariantBody) -> Vec<hir::FieldDef> {
        match body {
            ast::VariantBody::Named(fields) => fields
                .iter()
                .map(|f| hir::FieldDef {
                    name: f.name.name.clone(),
                    vis: lower_vis(&f.vis),
                    ty: self.lower_ty(&f.ty),
                    span: f.span,
                })
                .collect(),
            ast::VariantBody::Tuple(fields) => fields
                .iter()
                .enumerate()
                .map(|(i, f)| hir::FieldDef {
                    name: i.to_string(),
                    vis: lower_vis(&f.vis),
                    ty: self.lower_ty(&f.ty),
                    span: f.span,
                })
                .collect(),
            ast::VariantBody::Unit => Vec::new(),
        }
    }

    // -- 类型 / 模式 / 表达式 ---------------------------------------------------

    fn lower_ty(&mut self, ty: &ast::Ty) -> hir::Ty {
        let kind = match &ty.kind {
            ast::TyKind::Path(p) => {
                // 末段 Type 实参降级(泛型 Adt 实例化数据,RXS-0045)
                let args = p
                    .segments
                    .last()
                    .and_then(|s| s.args.as_ref())
                    .map(|ga| {
                        ga.args
                            .iter()
                            .filter_map(|a| match a {
                                ast::GenericArg::Type(t) => Some(self.lower_ty(t)),
                                _ => None,
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                hir::TyKind::Res(self.path_res(p.span), args)
            }
            ast::TyKind::Ref { mutable, inner, .. } => hir::TyKind::Ref {
                mutable: *mutable,
                inner: Box::new(self.lower_ty(inner)),
            },
            ast::TyKind::RawPtr { mutable, inner } => hir::TyKind::RawPtr {
                mutable: *mutable,
                inner: Box::new(self.lower_ty(inner)),
            },
            ast::TyKind::Tuple(elems) => {
                hir::TyKind::Tuple(elems.iter().map(|t| self.lower_ty(t)).collect())
            }
            ast::TyKind::Paren(inner) => return self.lower_ty(inner),
            ast::TyKind::Array { elem, len } => hir::TyKind::Array {
                elem: Box::new(self.lower_ty(elem)),
                // 整数字面量长度的 span(M5.3,device shared/array codegen 用;取值
                // 在 MIR lowering 解析源文本);非字面量(const 泛型/表达式)→ None,
                // 落 RD-007。
                len: match &len.kind {
                    ast::ExprKind::Lit(l) if l.kind == ast::LitKind::Int => Some(l.span),
                    _ => None,
                },
            },
            ast::TyKind::Slice(inner) => hir::TyKind::Slice(Box::new(self.lower_ty(inner))),
            ast::TyKind::FnPtr { params, ret } => hir::TyKind::FnPtr {
                params: params.iter().map(|t| self.lower_ty(t)).collect(),
                ret: ret.as_ref().map(|t| Box::new(self.lower_ty(t))),
            },
            ast::TyKind::Infer => hir::TyKind::Infer,
            // 类型位置 const 实参与错误占位:const 字面量保留 span(M5.3)
            ast::TyKind::ConstArg(lit) => hir::TyKind::ConstLit { span: lit.span },
            ast::TyKind::Err => hir::TyKind::Err,
        };
        hir::Ty {
            kind,
            span: ty.span,
        }
    }

    fn lower_pat(&mut self, pat: &ast::Pat) -> hir::Pat {
        let kind = match &pat.kind {
            ast::PatKind::Wild => hir::PatKind::Wild,
            ast::PatKind::Binding { name, .. } => {
                // 裸名裁决为单元变体路径模式时,resolver 记录的是 path_res
                // 而非 binding(RXS-0048/0023;见 resolve_pat)
                if let Some(res) = self.res.path_res.get(&name.span) {
                    hir::PatKind::Res(*res)
                } else {
                    hir::PatKind::Binding {
                        local: self.binding_local(name.span),
                    }
                }
            }
            ast::PatKind::Lit { negated, lit } => hir::PatKind::Lit {
                negated: *negated,
                lit: lit.clone(),
            },
            ast::PatKind::Range { .. } => hir::PatKind::Range,
            ast::PatKind::At { name, pat } => hir::PatKind::At {
                local: self.binding_local(name.span),
                pat: Box::new(self.lower_pat(pat)),
            },
            ast::PatKind::Ref { pat, .. } => hir::PatKind::Ref {
                pat: Box::new(self.lower_pat(pat)),
            },
            ast::PatKind::Tuple(elems) => {
                hir::PatKind::Tuple(elems.iter().map(|p| self.lower_pat(p)).collect())
            }
            ast::PatKind::Slice(elems) => {
                hir::PatKind::Slice(elems.iter().map(|p| self.lower_pat(p)).collect())
            }
            ast::PatKind::Path(p) => hir::PatKind::Res(self.path_res(p.span)),
            ast::PatKind::TupleStruct { path, elems } => hir::PatKind::TupleStruct {
                res: self.path_res(path.span),
                elems: elems.iter().map(|p| self.lower_pat(p)).collect(),
            },
            ast::PatKind::Struct { path, fields, rest } => hir::PatKind::Struct {
                res: self.path_res(path.span),
                fields: fields
                    .iter()
                    .map(|f| {
                        let sub = match &f.pat {
                            Some(p) => self.lower_pat(p),
                            // 简写字段模式 = 同名绑定(RXS-0032)
                            None => hir::Pat {
                                hir_id: self.next_hir_id(),
                                kind: hir::PatKind::Binding {
                                    local: self.binding_local(f.name.span),
                                },
                                span: f.name.span,
                            },
                        };
                        (f.name.name.clone(), Some(sub))
                    })
                    .collect(),
                rest: *rest,
            },
            ast::PatKind::Err => hir::PatKind::Err,
        };
        hir::Pat {
            hir_id: self.next_hir_id(),
            kind,
            span: pat.span,
        }
    }

    fn clone_pat(&self, p: &hir::Pat) -> hir::Pat {
        // params 在 FnDecl 与 Body 各存一份(MVP 复制;intern 化随 M2.2 评估);
        // hir_id 共享原节点:typeck 只走 Body 侧,FnDecl 侧仅供签名消费
        hir::Pat {
            hir_id: p.hir_id,
            kind: match &p.kind {
                hir::PatKind::Wild => hir::PatKind::Wild,
                hir::PatKind::Binding { local } => hir::PatKind::Binding { local: *local },
                hir::PatKind::Lit { negated, lit } => hir::PatKind::Lit {
                    negated: *negated,
                    lit: lit.clone(),
                },
                hir::PatKind::Range => hir::PatKind::Range,
                hir::PatKind::At { local, pat } => hir::PatKind::At {
                    local: *local,
                    pat: Box::new(self.clone_pat(pat)),
                },
                hir::PatKind::Ref { pat } => hir::PatKind::Ref {
                    pat: Box::new(self.clone_pat(pat)),
                },
                hir::PatKind::Tuple(v) => {
                    hir::PatKind::Tuple(v.iter().map(|p| self.clone_pat(p)).collect())
                }
                hir::PatKind::Slice(v) => {
                    hir::PatKind::Slice(v.iter().map(|p| self.clone_pat(p)).collect())
                }
                hir::PatKind::Res(r) => hir::PatKind::Res(*r),
                hir::PatKind::TupleStruct { res, elems } => hir::PatKind::TupleStruct {
                    res: *res,
                    elems: elems.iter().map(|p| self.clone_pat(p)).collect(),
                },
                hir::PatKind::Struct { res, fields, rest } => hir::PatKind::Struct {
                    res: *res,
                    fields: fields
                        .iter()
                        .map(|(n, p)| (n.clone(), p.as_ref().map(|p| self.clone_pat(p))))
                        .collect(),
                    rest: *rest,
                },
                hir::PatKind::Err => hir::PatKind::Err,
            },
            span: p.span,
        }
    }

    fn lower_block(&mut self, block: &ast::Block) -> hir::Block {
        let mut stmts = Vec::new();
        for stmt in &block.stmts {
            match &stmt.kind {
                ast::StmtKind::Empty => {}
                ast::StmtKind::Let(l) => {
                    stmts.push(hir::Stmt::Let {
                        pat: self.lower_pat(&l.pat),
                        ty: l.ty.as_ref().map(|t| self.lower_ty(t)),
                        init: l.init.as_ref().map(|e| self.lower_expr(e)),
                        shared: l.shared,
                    });
                }
                ast::StmtKind::Expr { expr, .. } => {
                    stmts.push(hir::Stmt::Expr(self.lower_expr(expr)));
                }
                ast::StmtKind::Item(item) => {
                    for id in self.lower_item(item) {
                        stmts.push(hir::Stmt::Item(id));
                    }
                }
            }
        }
        hir::Block {
            stmts,
            tail: block.tail.as_ref().map(|e| Box::new(self.lower_expr(e))),
            span: block.span,
        }
    }

    fn lower_expr(&mut self, expr: &ast::Expr) -> hir::Expr {
        let kind = match &expr.kind {
            ast::ExprKind::Lit(l) => hir::ExprKind::Lit(l.clone()),
            ast::ExprKind::Path(p) => hir::ExprKind::Res(self.path_res(p.span)),
            ast::ExprKind::Unary { op, expr } => hir::ExprKind::Unary {
                op: *op,
                expr: Box::new(self.lower_expr(expr)),
            },
            ast::ExprKind::Borrow { mutable, expr } => hir::ExprKind::Borrow {
                mutable: *mutable,
                expr: Box::new(self.lower_expr(expr)),
            },
            ast::ExprKind::Binary { op, lhs, rhs } => hir::ExprKind::Binary {
                op: *op,
                lhs: Box::new(self.lower_expr(lhs)),
                rhs: Box::new(self.lower_expr(rhs)),
            },
            ast::ExprKind::Assign { op, lhs, rhs } => hir::ExprKind::Assign {
                op: *op,
                lhs: Box::new(self.lower_expr(lhs)),
                rhs: Box::new(self.lower_expr(rhs)),
            },
            ast::ExprKind::Cast { expr, ty } => hir::ExprKind::Cast {
                expr: Box::new(self.lower_expr(expr)),
                ty: self.lower_ty(ty),
            },
            ast::ExprKind::Range { lo, hi, inclusive } => hir::ExprKind::Range {
                lo: Box::new(self.lower_expr(lo)),
                hi: Box::new(self.lower_expr(hi)),
                inclusive: *inclusive,
            },
            ast::ExprKind::Call { callee, args } => hir::ExprKind::Call {
                callee: Box::new(self.lower_expr(callee)),
                args: args.iter().map(|a| self.lower_expr(a)).collect(),
            },
            ast::ExprKind::MethodCall {
                receiver,
                method,
                args,
                ..
            } => hir::ExprKind::MethodCall {
                receiver: Box::new(self.lower_expr(receiver)),
                method: method.name.clone(),
                args: args.iter().map(|a| self.lower_expr(a)).collect(),
            },
            ast::ExprKind::Field { expr, field } => hir::ExprKind::Field {
                expr: Box::new(self.lower_expr(expr)),
                field: field.name.clone(),
            },
            ast::ExprKind::TupleField { expr, index, .. } => hir::ExprKind::TupleField {
                expr: Box::new(self.lower_expr(expr)),
                index: *index,
            },
            ast::ExprKind::Index { expr, index } => hir::ExprKind::Index {
                expr: Box::new(self.lower_expr(expr)),
                index: Box::new(self.lower_expr(index)),
            },
            // `?` desugar(RXS-0050)
            ast::ExprKind::Try(e) => return self.desugar_try(expr.span, e),
            ast::ExprKind::Tuple(elems) => {
                hir::ExprKind::Tuple(elems.iter().map(|e| self.lower_expr(e)).collect())
            }
            ast::ExprKind::Array(elems) => {
                hir::ExprKind::Array(elems.iter().map(|e| self.lower_expr(e)).collect())
            }
            ast::ExprKind::Repeat { elem, len } => hir::ExprKind::Repeat {
                elem: Box::new(self.lower_expr(elem)),
                len: Box::new(self.lower_expr(len)),
            },
            ast::ExprKind::StructLit { path, fields } => hir::ExprKind::StructLit {
                res: self.path_res(path.span),
                fields: fields
                    .iter()
                    .map(|f| {
                        let value = match &f.expr {
                            Some(e) => Some(self.lower_expr(e)),
                            // 简写字段 desugar 为显式 Res 表达式
                            None => Some(hir::Expr {
                                hir_id: self.next_hir_id(),
                                kind: hir::ExprKind::Res(self.path_res(f.name.span)),
                                span: f.name.span,
                            }),
                        };
                        (f.name.name.clone(), value)
                    })
                    .collect(),
            },
            ast::ExprKind::Paren(e) => return self.lower_expr(e),
            ast::ExprKind::Block(b) => hir::ExprKind::Block(self.lower_block(b)),
            ast::ExprKind::Unsafe(b) => hir::ExprKind::Unsafe(self.lower_block(b)),
            ast::ExprKind::If { cond, then, else_ } => hir::ExprKind::If {
                cond: Box::new(self.lower_expr(cond)),
                then: self.lower_block(then),
                else_: else_.as_ref().map(|e| Box::new(self.lower_expr(e))),
            },
            ast::ExprKind::While { cond, body } => hir::ExprKind::While {
                cond: Box::new(self.lower_expr(cond)),
                body: self.lower_block(body),
            },
            // `for` desugar(RXS-0049)
            ast::ExprKind::For { pat, iter, body } => {
                return self.desugar_for(expr.span, pat, iter, body);
            }
            ast::ExprKind::Loop { body } => hir::ExprKind::Loop {
                body: self.lower_block(body),
            },
            ast::ExprKind::Match { scrutinee, arms } => hir::ExprKind::Match {
                scrutinee: Box::new(self.lower_expr(scrutinee)),
                arms: arms
                    .iter()
                    .map(|a| hir::Arm {
                        pats: a.pats.iter().map(|p| self.lower_pat(p)).collect(),
                        guard: a.guard.as_ref().map(|g| self.lower_expr(g)),
                        body: self.lower_expr(&a.body),
                    })
                    .collect(),
            },
            ast::ExprKind::Return(operand) => {
                hir::ExprKind::Return(operand.as_ref().map(|e| Box::new(self.lower_expr(e))))
            }
            ast::ExprKind::Break(operand) => {
                hir::ExprKind::Break(operand.as_ref().map(|e| Box::new(self.lower_expr(e))))
            }
            ast::ExprKind::Continue => hir::ExprKind::Continue,
            ast::ExprKind::Closure { params, body, .. } => hir::ExprKind::Closure {
                params: params.iter().map(|p| self.lower_pat(&p.pat)).collect(),
                body: Box::new(self.lower_expr(body)),
            },
            ast::ExprKind::Err => hir::ExprKind::Err,
        };
        hir::Expr {
            hir_id: self.next_hir_id(),
            kind,
            span: expr.span,
        }
    }

    // -- desugar(RXS-0049 / RXS-0050;合成节点统一携带原构造的 span) ----------

    fn mk_expr(&mut self, span: Span, kind: hir::ExprKind) -> hir::Expr {
        hir::Expr {
            hir_id: self.next_hir_id(),
            kind,
            span,
        }
    }

    fn mk_pat(&mut self, span: Span, kind: hir::PatKind) -> hir::Pat {
        hir::Pat {
            hir_id: self.next_hir_id(),
            kind,
            span,
        }
    }

    fn local_ref(&mut self, span: Span, l: LocalId) -> hir::Expr {
        self.mk_expr(span, hir::ExprKind::Res(Res::Local(l)))
    }

    fn binding(&mut self, span: Span, l: LocalId) -> hir::Pat {
        self.mk_pat(span, hir::PatKind::Binding { local: l })
    }

    fn lit_bool(&mut self, span: Span, v: bool) -> hir::Expr {
        self.mk_expr(
            span,
            hir::ExprKind::Lit(ast::Lit {
                kind: ast::LitKind::Bool(v),
                suffix: None,
                span,
            }),
        )
    }

    fn binary(&mut self, span: Span, op: BinOp, lhs: hir::Expr, rhs: hir::Expr) -> hir::Expr {
        self.mk_expr(
            span,
            hir::ExprKind::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            },
        )
    }

    /// 变体构造调用 `V(arg)`(V = 内建变体 DefId,直引不经名称解析,RXS-0048)。
    fn variant_call(&mut self, span: Span, variant: DefId, arg: hir::Expr) -> hir::Expr {
        let callee = self.mk_expr(span, hir::ExprKind::Res(Res::Def(variant)));
        self.mk_expr(
            span,
            hir::ExprKind::Call {
                callee: Box::new(callee),
                args: vec![arg],
            },
        )
    }

    /// `loop { match scrut { Some(pat) => body, None => break } }` 公共骨架
    /// (RXS-0049 三种形态共用的 loop+match 外壳)。
    fn for_loop_shell(
        &mut self,
        span: Span,
        scrutinee: hir::Expr,
        user_pat: hir::Pat,
        user_body: hir::Block,
    ) -> hir::Expr {
        let li = self.res.lang_items;
        let some = li.option_some.expect("lang items 已注入");
        let none = li.option_none.expect("lang items 已注入");
        let body_span = user_body.span;
        let some_body = self.mk_expr(body_span, hir::ExprKind::Block(user_body));
        let some_pat = self.mk_pat(
            span,
            hir::PatKind::TupleStruct {
                res: Res::Def(some),
                elems: vec![user_pat],
            },
        );
        let none_pat = self.mk_pat(span, hir::PatKind::Res(Res::Def(none)));
        let break_expr = self.mk_expr(span, hir::ExprKind::Break(None));
        let mtch = self.mk_expr(
            span,
            hir::ExprKind::Match {
                scrutinee: Box::new(scrutinee),
                arms: vec![
                    hir::Arm {
                        pats: vec![some_pat],
                        guard: None,
                        body: some_body,
                    },
                    hir::Arm {
                        pats: vec![none_pat],
                        guard: None,
                        body: break_expr,
                    },
                ],
            },
        );
        self.mk_expr(
            span,
            hir::ExprKind::Loop {
                body: hir::Block {
                    stmts: Vec::new(),
                    tail: Some(Box::new(mtch)),
                    span,
                },
            },
        )
    }

    /// `for` desugar(RXS-0049):区间形态展开为计数推进 + loop+match;
    /// 一般迭代器形态展开为 `__it.next()` 协议(RXS-0048 形状约定)。
    fn desugar_for(
        &mut self,
        span: Span,
        pat: &ast::Pat,
        iter: &ast::Expr,
        body: &ast::Block,
    ) -> hir::Expr {
        let mut iter_inner = iter;
        while let ast::ExprKind::Paren(inner) = &iter_inner.kind {
            iter_inner = inner;
        }
        if let ast::ExprKind::Range { lo, hi, inclusive } = &iter_inner.kind {
            return self.desugar_for_range(span, pat, lo, hi, *inclusive, body);
        }
        // 一般迭代器形态:
        // { let mut __it = it; loop { match __it.next() { Some(p) => body, None => break } } }
        let init = self.lower_expr(iter);
        let it = self.fresh_local("__for_it", true, span);
        let user_pat = self.lower_pat(pat);
        let user_body = self.lower_block(body);
        let recv = self.local_ref(span, it);
        let next_call = self.mk_expr(
            span,
            hir::ExprKind::MethodCall {
                receiver: Box::new(recv),
                method: "next".to_owned(),
                args: Vec::new(),
            },
        );
        let lp = self.for_loop_shell(span, next_call, user_pat, user_body);
        let it_pat = self.binding(span, it);
        self.mk_expr(
            span,
            hir::ExprKind::Block(hir::Block {
                stmts: vec![hir::Stmt::Let {
                    pat: it_pat,
                    ty: None,
                    init: Some(init),
                    shared: false,
                }],
                tail: Some(Box::new(lp)),
                span,
            }),
        )
    }

    /// 区间形态(RXS-0049):推进先于 body(continue 安全);闭区间经
    /// `__done` 旗标避免对类型最大值的越界递增。
    fn desugar_for_range(
        &mut self,
        span: Span,
        pat: &ast::Pat,
        lo: &ast::Expr,
        hi: &ast::Expr,
        inclusive: bool,
        body: &ast::Block,
    ) -> hir::Expr {
        let li = self.res.lang_items;
        let some = li.option_some.expect("lang items 已注入");
        let none = li.option_none.expect("lang items 已注入");
        let lo_e = self.lower_expr(lo);
        let hi_e = self.lower_expr(hi);
        let i = self.fresh_local("__for_i", true, span);
        let hi_l = self.fresh_local("__for_hi", false, span);
        let v = self.fresh_local("__for_v", false, span);
        let done = inclusive.then(|| self.fresh_local("__for_done", true, span));
        let user_pat = self.lower_pat(pat);
        let user_body = self.lower_block(body);

        // 推进步("next"):取 __v = __i 并推进游标,产出 Some(__v) / None
        let step_let = {
            let i_ref = self.local_ref(span, i);
            let v_pat = self.binding(span, v);
            hir::Stmt::Let {
                pat: v_pat,
                ty: None,
                init: Some(i_ref),
                shared: false,
            }
        };
        let advance = if let Some(done) = done {
            // if __i == __hi { __done = true; } else { __i = __i + 1; }
            let i_ref = self.local_ref(span, i);
            let hi_ref = self.local_ref(span, hi_l);
            let cond = self.binary(span, BinOp::Eq, i_ref, hi_ref);
            let done_lhs = self.local_ref(span, done);
            let true_lit = self.lit_bool(span, true);
            let set_done = self.mk_expr(
                span,
                hir::ExprKind::Assign {
                    op: None,
                    lhs: Box::new(done_lhs),
                    rhs: Box::new(true_lit),
                },
            );
            let inc = self.increment(span, i);
            let inc_block = self.block_of_stmt(span, inc);
            self.mk_expr(
                span,
                hir::ExprKind::If {
                    cond: Box::new(cond),
                    then: hir::Block {
                        stmts: vec![hir::Stmt::Expr(set_done)],
                        tail: None,
                        span,
                    },
                    else_: Some(Box::new(inc_block)),
                },
            )
        } else {
            self.increment(span, i)
        };
        let v_ref = self.local_ref(span, v);
        let some_call = self.variant_call(span, some, v_ref);
        let then_blk = hir::Block {
            stmts: vec![step_let, hir::Stmt::Expr(advance)],
            tail: Some(Box::new(some_call)),
            span,
        };
        // 续行条件:半开 __i < __hi;闭区间 !(__done || __i > __hi)
        let cond = {
            let i_ref = self.local_ref(span, i);
            let hi_ref = self.local_ref(span, hi_l);
            if let Some(done) = done {
                let gt = self.binary(span, BinOp::Gt, i_ref, hi_ref);
                let done_ref = self.local_ref(span, done);
                let stop = self.binary(span, BinOp::Or, done_ref, gt);
                self.mk_expr(
                    span,
                    hir::ExprKind::Unary {
                        op: crate::ast::UnOp::Not,
                        expr: Box::new(stop),
                    },
                )
            } else {
                self.binary(span, BinOp::Lt, i_ref, hi_ref)
            }
        };
        let none_ref = self.mk_expr(span, hir::ExprKind::Res(Res::Def(none)));
        let else_blk = self.mk_expr(
            span,
            hir::ExprKind::Block(hir::Block {
                stmts: Vec::new(),
                tail: Some(Box::new(none_ref)),
                span,
            }),
        );
        let scrut = self.mk_expr(
            span,
            hir::ExprKind::If {
                cond: Box::new(cond),
                then: then_blk,
                else_: Some(Box::new(else_blk)),
            },
        );
        let lp = self.for_loop_shell(span, scrut, user_pat, user_body);

        let mut stmts = Vec::new();
        let i_pat = self.binding(span, i);
        stmts.push(hir::Stmt::Let {
            pat: i_pat,
            ty: None,
            init: Some(lo_e),
            shared: false,
        });
        let hi_pat = self.binding(span, hi_l);
        stmts.push(hir::Stmt::Let {
            pat: hi_pat,
            ty: None,
            init: Some(hi_e),
            shared: false,
        });
        if let Some(done) = done {
            let done_pat = self.binding(span, done);
            let false_lit = self.lit_bool(span, false);
            stmts.push(hir::Stmt::Let {
                pat: done_pat,
                ty: None,
                init: Some(false_lit),
                shared: false,
            });
        }
        self.mk_expr(
            span,
            hir::ExprKind::Block(hir::Block {
                stmts,
                tail: Some(Box::new(lp)),
                span,
            }),
        )
    }

    /// `__i = __i + 1`(SynthInt 推进步,RXS-0049)。
    fn increment(&mut self, span: Span, i: LocalId) -> hir::Expr {
        let lhs = self.local_ref(span, i);
        let one = self.mk_expr(span, hir::ExprKind::SynthInt(1));
        self.mk_expr(
            span,
            hir::ExprKind::Assign {
                op: Some(BinOp::Add),
                lhs: Box::new(lhs),
                rhs: Box::new(one),
            },
        )
    }

    fn block_of_stmt(&mut self, span: Span, stmt_expr: hir::Expr) -> hir::Expr {
        self.mk_expr(
            span,
            hir::ExprKind::Block(hir::Block {
                stmts: vec![hir::Stmt::Expr(stmt_expr)],
                tail: None,
                span,
            }),
        )
    }

    /// `?` desugar(RXS-0050):
    /// `match e { Ok(__v) => __v, Err(__e) => return Err(__e) }`。
    fn desugar_try(&mut self, span: Span, inner: &ast::Expr) -> hir::Expr {
        let li = self.res.lang_items;
        let ok = li.result_ok.expect("lang items 已注入");
        let err = li.result_err.expect("lang items 已注入");
        let scrutinee = self.lower_expr(inner);
        let v = self.fresh_local("__try_v", false, span);
        let e = self.fresh_local("__try_e", false, span);
        let v_bind = self.binding(span, v);
        let ok_pat = self.mk_pat(
            span,
            hir::PatKind::TupleStruct {
                res: Res::Def(ok),
                elems: vec![v_bind],
            },
        );
        let ok_body = self.local_ref(span, v);
        let e_bind = self.binding(span, e);
        let err_pat = self.mk_pat(
            span,
            hir::PatKind::TupleStruct {
                res: Res::Def(err),
                elems: vec![e_bind],
            },
        );
        let e_ref = self.local_ref(span, e);
        let rethrow = self.variant_call(span, err, e_ref);
        let err_body = self.mk_expr(span, hir::ExprKind::Return(Some(Box::new(rethrow))));
        self.mk_expr(
            span,
            hir::ExprKind::Match {
                scrutinee: Box::new(scrutinee),
                arms: vec![
                    hir::Arm {
                        pats: vec![ok_pat],
                        guard: None,
                        body: ok_body,
                    },
                    hir::Arm {
                        pats: vec![err_pat],
                        guard: None,
                        body: err_body,
                    },
                ],
            },
        )
    }
}

fn lower_vis(v: &ast::Visibility) -> hir::Vis {
    match v {
        ast::Visibility::Inherited => hir::Vis::Private,
        ast::Visibility::Pub(_) => hir::Vis::Pub,
        ast::Visibility::PubPackage(_) => hir::Vis::PubPackage,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::DiagCtxt;
    use crate::lexer::lex;
    use crate::parser::parse;
    use crate::resolve::resolve;
    use crate::span::{Edition, SourceId};

    fn lower_clean(src: &str) -> hir::Crate {
        let diag = DiagCtxt::new();
        let tokens = lex(src, SourceId(0), Edition::Rx0, &diag);
        let file = parse(src, tokens, SourceId(0), Edition::Rx0, &diag);
        let res = resolve(&file, &diag);
        assert!(
            diag.emitted().is_empty(),
            "意外诊断: {:?}",
            diag.emitted()
                .iter()
                .map(|d| (d.code, d.message(diag.messages())))
                .collect::<Vec<_>>()
        );
        lower(&file, &res)
    }

    //@ spec: RXS-0032
    #[test]
    fn items_and_bodies_are_separated() {
        let krate =
            lower_clean("const K: i32 = 1;\nfn f(x: i32) -> i32 {\n    let y = x + K;\n    y\n}");
        // fn 与 const 各有独立 body;item 仅持 BodyId
        assert_eq!(krate.bodies.len(), 2);
        let f = krate
            .items
            .iter()
            .find(|i| i.name == "f")
            .expect("fn f 存在");
        let hir::ItemKind::Fn(decl) = &f.kind else {
            panic!("期待 Fn,实得 {:?}", f.kind)
        };
        let body = krate.body(decl.body.expect("有 body"));
        assert_eq!(body.owner, f.def_id);
        // 局部表:x(参数)与 y
        assert_eq!(body.locals.len(), 2);
    }

    //@ spec: RXS-0034
    #[test]
    fn paths_carry_res_in_hir() {
        let krate = lower_clean(
            "fn helper() -> i32 { 1 }\nfn f() -> i32 {\n    let a = 1;\n    a + helper()\n}",
        );
        let f = krate.items.iter().find(|i| i.name == "f").unwrap();
        let hir::ItemKind::Fn(decl) = &f.kind else {
            panic!()
        };
        let body = krate.body(decl.body.unwrap());
        let mut saw_local = false;
        let mut saw_def = false;
        collect_res(&body.value, &mut |r| match r {
            Res::Local(_) => saw_local = true,
            Res::Def(_) => saw_def = true,
            _ => {}
        });
        assert!(saw_local && saw_def, "尾表达式应含 Local 与 Def 解析");
    }

    //@ spec: RXS-0035
    #[test]
    fn use_target_retained_in_hir() {
        let krate = lower_clean(
            "mod m {\n    pub fn g() -> i32 { 1 }\n}\nuse m::g as gg;\nfn f() -> i32 { gg() }",
        );
        let use_item = krate
            .items
            .iter()
            .find(|i| matches!(i.kind, hir::ItemKind::Use { .. }))
            .expect("use 节点存在");
        assert_eq!(use_item.name, "gg");
        let hir::ItemKind::Use { target } = use_item.kind else {
            panic!()
        };
        assert!(matches!(target, Res::Def(_)));
    }

    //@ spec: RXS-0032
    #[test]
    fn struct_lit_shorthand_desugars_to_res() {
        let krate =
            lower_clean("struct P {\n    mass: f32,\n}\nfn f(mass: f32) -> P {\n    P { mass }\n}");
        let f = krate.items.iter().find(|i| i.name == "f").unwrap();
        let hir::ItemKind::Fn(decl) = &f.kind else {
            panic!()
        };
        let body = krate.body(decl.body.unwrap());
        let mut shorthand_resolved = false;
        walk_expr(&body.value, &mut |e| {
            if let hir::ExprKind::StructLit { fields, .. } = &e.kind
                && let Some((_, Some(v))) = fields.first()
                && matches!(v.kind, hir::ExprKind::Res(Res::Local(_)))
            {
                shorthand_resolved = true;
            }
        });
        assert!(shorthand_resolved, "简写字段应 desugar 为 Res 表达式");
    }

    //@ spec: RXS-0032
    #[test]
    fn block_items_become_stmt_items() {
        let krate = lower_clean("fn outer() -> usize {\n    const TILE: usize = 32;\n    TILE\n}");
        let outer = krate.items.iter().find(|i| i.name == "outer").unwrap();
        let hir::ItemKind::Fn(decl) = &outer.kind else {
            panic!()
        };
        let body = krate.body(decl.body.unwrap());
        let hir::ExprKind::Block(b) = &body.value.kind else {
            panic!()
        };
        assert!(b.stmts.iter().any(|s| matches!(s, hir::Stmt::Item(_))));
        // 嵌套 const 自身也有独立 body
        assert_eq!(krate.bodies.len(), 2);
    }

    //@ spec: RXS-0048
    #[test]
    fn lang_items_installed_as_builtin_enums() {
        let krate = lower_clean("fn main() {}");
        let opt = krate
            .items
            .iter()
            .find(|i| i.name == "Option")
            .expect("内建 Option 存在");
        let hir::ItemKind::Enum { variants } = &opt.kind else {
            panic!("期待 Enum,实得 {:?}", opt.kind)
        };
        assert_eq!(variants.len(), 2);
        let some = krate.items.iter().find(|i| i.name == "Some").unwrap();
        let hir::ItemKind::Variant { fields } = &some.kind else {
            panic!()
        };
        assert!(matches!(
            fields[0].ty.kind,
            hir::TyKind::Res(Res::GenericParam(0), _)
        ));
        let err = krate.items.iter().find(|i| i.name == "Err").unwrap();
        let hir::ItemKind::Variant { fields } = &err.kind else {
            panic!()
        };
        assert!(matches!(
            fields[0].ty.kind,
            hir::TyKind::Res(Res::GenericParam(1), _)
        ));
    }

    //@ spec: RXS-0049
    #[test]
    fn for_range_desugars_to_loop_match_with_synth_step() {
        let krate =
            lower_clean("fn f(n: i32) {\n    for i in 0..n {\n        let _x = i;\n    }\n}");
        let f = krate.items.iter().find(|i| i.name == "f").unwrap();
        let hir::ItemKind::Fn(decl) = &f.kind else {
            panic!()
        };
        let body = krate.body(decl.body.unwrap());
        // 合成局部追加在 resolver 局部之后,用户不可引用
        for name in ["__for_i", "__for_hi", "__for_v"] {
            assert!(body.locals.iter().any(|l| l.name == name), "缺 {name}");
        }
        let (mut saw_loop, mut saw_synth, mut shell_ok) = (false, false, false);
        walk_expr(&body.value, &mut |e| match &e.kind {
            hir::ExprKind::Loop { .. } => saw_loop = true,
            hir::ExprKind::SynthInt(1) => saw_synth = true,
            hir::ExprKind::Match { arms, .. } if arms.len() == 2 => {
                // Some(p) 臂 + None 臂(RXS-0049 loop+match 外壳)
                let some_arm = matches!(arms[0].pats[0].kind, hir::PatKind::TupleStruct { .. });
                let none_arm = matches!(arms[1].pats[0].kind, hir::PatKind::Res(Res::Def(_)));
                let break_body = matches!(arms[1].body.kind, hir::ExprKind::Break(None));
                shell_ok = some_arm && none_arm && break_body;
            }
            _ => {}
        });
        assert!(saw_loop && saw_synth && shell_ok);
    }

    //@ spec: RXS-0049
    #[test]
    fn for_inclusive_range_uses_done_flag() {
        let krate =
            lower_clean("fn f(n: i32) {\n    for i in 0..=n {\n        let _x = i;\n    }\n}");
        let f = krate.items.iter().find(|i| i.name == "f").unwrap();
        let hir::ItemKind::Fn(decl) = &f.kind else {
            panic!()
        };
        let body = krate.body(decl.body.unwrap());
        assert!(body.locals.iter().any(|l| l.name == "__for_done"));
    }

    //@ spec: RXS-0048, RXS-0049
    #[test]
    fn for_iterator_desugars_to_next_protocol() {
        let krate = lower_clean(
            "struct C {\n    n: i32,\n}\nfn f(c: C) {\n    for v in c {\n        let _x = v;\n    }\n}",
        );
        let f = krate.items.iter().find(|i| i.name == "f").unwrap();
        let hir::ItemKind::Fn(decl) = &f.kind else {
            panic!()
        };
        let body = krate.body(decl.body.unwrap());
        assert!(
            body.locals
                .iter()
                .any(|l| l.name == "__for_it" && l.mutable)
        );
        let mut saw_next = false;
        walk_expr(&body.value, &mut |e| {
            if let hir::ExprKind::MethodCall { method, .. } = &e.kind
                && method == "next"
            {
                saw_next = true;
            }
        });
        assert!(saw_next, "一般迭代器形态应展开为 __it.next() 协议");
    }

    //@ spec: RXS-0050
    #[test]
    fn try_desugars_to_match_with_rethrow() {
        let krate = lower_clean(
            "fn f(r: Result<i32, i32>) -> Result<i32, i32> {\n    let v = r?;\n    Ok(v)\n}",
        );
        let f = krate.items.iter().find(|i| i.name == "f").unwrap();
        let hir::ItemKind::Fn(decl) = &f.kind else {
            panic!()
        };
        let body = krate.body(decl.body.unwrap());
        for name in ["__try_v", "__try_e"] {
            assert!(body.locals.iter().any(|l| l.name == name), "缺 {name}");
        }
        let mut rethrow_ok = false;
        walk_expr(&body.value, &mut |e| {
            if let hir::ExprKind::Match { arms, .. } = &e.kind
                && arms.len() == 2
                && let hir::ExprKind::Return(Some(r)) = &arms[1].body.kind
                && matches!(r.kind, hir::ExprKind::Call { .. })
            {
                rethrow_ok = true;
            }
        });
        assert!(rethrow_ok, "Err 臂应展开为 return Err(__e)");
    }

    fn collect_res(e: &hir::Expr, f: &mut impl FnMut(Res)) {
        walk_expr(e, &mut |e| {
            if let hir::ExprKind::Res(r) = &e.kind {
                f(*r);
            }
        });
    }

    fn walk_expr(e: &hir::Expr, f: &mut impl FnMut(&hir::Expr)) {
        f(e);
        match &e.kind {
            hir::ExprKind::Unary { expr, .. }
            | hir::ExprKind::Borrow { expr, .. }
            | hir::ExprKind::Cast { expr, .. }
            | hir::ExprKind::Field { expr, .. } => walk_expr(expr, f),
            hir::ExprKind::Binary { lhs, rhs, .. }
            | hir::ExprKind::Assign { lhs, rhs, .. }
            | hir::ExprKind::Range {
                lo: lhs, hi: rhs, ..
            } => {
                walk_expr(lhs, f);
                walk_expr(rhs, f);
            }
            hir::ExprKind::Call { callee, args } => {
                walk_expr(callee, f);
                for a in args {
                    walk_expr(a, f);
                }
            }
            hir::ExprKind::MethodCall { receiver, args, .. } => {
                walk_expr(receiver, f);
                for a in args {
                    walk_expr(a, f);
                }
            }
            hir::ExprKind::Index { expr, index } => {
                walk_expr(expr, f);
                walk_expr(index, f);
            }
            hir::ExprKind::Tuple(v) | hir::ExprKind::Array(v) => {
                for e in v {
                    walk_expr(e, f);
                }
            }
            hir::ExprKind::Repeat { elem, len } => {
                walk_expr(elem, f);
                walk_expr(len, f);
            }
            hir::ExprKind::StructLit { fields, .. } => {
                for (_, v) in fields {
                    if let Some(e) = v {
                        walk_expr(e, f);
                    }
                }
            }
            hir::ExprKind::Block(b)
            | hir::ExprKind::Unsafe(b)
            | hir::ExprKind::Loop { body: b } => {
                walk_block(b, f);
            }
            hir::ExprKind::If { cond, then, else_ } => {
                walk_expr(cond, f);
                walk_block(then, f);
                if let Some(e) = else_ {
                    walk_expr(e, f);
                }
            }
            hir::ExprKind::While { cond, body } => {
                walk_expr(cond, f);
                walk_block(body, f);
            }
            hir::ExprKind::Match { scrutinee, arms } => {
                walk_expr(scrutinee, f);
                for a in arms {
                    if let Some(g) = &a.guard {
                        walk_expr(g, f);
                    }
                    walk_expr(&a.body, f);
                }
            }
            hir::ExprKind::Return(op) | hir::ExprKind::Break(op) => {
                if let Some(e) = op {
                    walk_expr(e, f);
                }
            }
            hir::ExprKind::Closure { body, .. } => walk_expr(body, f),
            hir::ExprKind::TupleField { expr, .. } => walk_expr(expr, f),
            hir::ExprKind::Lit(_)
            | hir::ExprKind::SynthInt(_)
            | hir::ExprKind::Res(_)
            | hir::ExprKind::Continue
            | hir::ExprKind::Err => {}
        }
    }

    fn walk_block(b: &hir::Block, f: &mut impl FnMut(&hir::Expr)) {
        for s in &b.stmts {
            match s {
                hir::Stmt::Let { init, .. } => {
                    if let Some(e) = init {
                        walk_expr(e, f);
                    }
                }
                hir::Stmt::Expr(e) => walk_expr(e, f),
                hir::Stmt::Item(_) => {}
            }
        }
        if let Some(t) = &b.tail {
            walk_expr(t, f);
        }
    }
}
