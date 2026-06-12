//! AST → HIR lowering(07 §1 / D-202;消费 [`crate::resolve::Resolutions`])。
//!
//! - **item/body 分离**:函数体/const 初始化器降级为独立 [`hir::Body`],
//!   item 仅持 [`hir::BodyId`](RXS-0032 实现要求,增量依赖边界);
//! - 路径节点替换为已解析 [`hir::Res`](span 键查 `Resolutions::path_res`);
//! - struct 字面量简写字段在此 desugar 为显式 `Res` 表达式;
//! - `for` / `?` 保留为 HIR 一等节点(desugar 推迟 M2.2,见 hir.rs 模块注释);
//! - extern 块在 HIR 中展平为其成员 fn(无独立节点形态,MVP 取舍)。

use crate::ast;
use crate::hir::{self, BodyId, DefId, LocalId, Res};
use crate::resolve::Resolutions;

/// lowering 入口:产出 HIR crate(解析诊断已由 resolve 阶段报出,本阶段无诊断)。
pub fn lower(file: &ast::SourceFile, res: &Resolutions) -> hir::Crate {
    let mut lw = Lowerer {
        res,
        krate: hir::Crate::default(),
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
    let root: Vec<DefId> = file.items.iter().flat_map(|it| lw.lower_item(it)).collect();
    lw.krate.root_items = root;
    lw.krate
}

struct Lowerer<'a> {
    res: &'a Resolutions,
    krate: hir::Crate,
}

impl Lowerer<'_> {
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
                let fields = self.lower_variant_fields(&s.body);
                self.set_item(id, hir::ItemKind::Struct { fields });
                vec![id]
            }
            ast::ItemKind::Enum(e) => {
                let Some(id) = self.def_of(item.span) else {
                    return Vec::new();
                };
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
                let items = self.lower_assoc_items(&im.items);
                let id = self.push_synthetic(
                    "<impl>",
                    hir::ItemKind::Impl { self_res, items },
                    item.span,
                );
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
        let params: Vec<hir::Param> = f
            .params
            .iter()
            .map(|p| match &p.kind {
                ast::ParamKind::SelfParam { .. } => hir::Param {
                    pat: hir::Pat {
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
            let locals = self
                .res
                .body_locals
                .get(&block.span)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|l| hir::LocalDecl {
                    name: l.name,
                    mutable: l.mutable,
                    span: l.span,
                })
                .collect();
            let value = hir::Expr {
                span: block.span,
                kind: hir::ExprKind::Block(self.lower_block(block)),
            };
            self.alloc_body(hir::Body {
                owner,
                locals,
                params: params.iter().map(|p| self.clone_pat(&p.pat)).collect(),
                value,
            })
        });
        hir::FnDecl {
            color: f.color,
            generic_params,
            params,
            ret,
            body,
        }
    }

    /// const/static/关联 const 的初始化器 body。
    fn lower_value_body(&mut self, owner: DefId, init: &ast::Expr) -> BodyId {
        let locals = self
            .res
            .body_locals
            .get(&init.span)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|l| hir::LocalDecl {
                name: l.name,
                mutable: l.mutable,
                span: l.span,
            })
            .collect();
        let value = self.lower_expr(init);
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
            ast::TyKind::Array { elem, .. } => hir::TyKind::Array {
                elem: Box::new(self.lower_ty(elem)),
            },
            ast::TyKind::Slice(inner) => hir::TyKind::Slice(Box::new(self.lower_ty(inner))),
            ast::TyKind::FnPtr { params, ret } => hir::TyKind::FnPtr {
                params: params.iter().map(|t| self.lower_ty(t)).collect(),
                ret: ret.as_ref().map(|t| Box::new(self.lower_ty(t))),
            },
            ast::TyKind::Infer => hir::TyKind::Infer,
            // 类型位置 const 实参与错误占位:M2.2 类型系统接管
            ast::TyKind::ConstArg(_) | ast::TyKind::Err => hir::TyKind::Err,
        };
        hir::Ty {
            kind,
            span: ty.span,
        }
    }

    fn lower_pat(&mut self, pat: &ast::Pat) -> hir::Pat {
        let kind = match &pat.kind {
            ast::PatKind::Wild => hir::PatKind::Wild,
            ast::PatKind::Binding { name, .. } => hir::PatKind::Binding {
                local: self.binding_local(name.span),
            },
            ast::PatKind::Lit { .. } => hir::PatKind::Lit,
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
            kind,
            span: pat.span,
        }
    }

    fn clone_pat(&self, p: &hir::Pat) -> hir::Pat {
        // params 在 FnDecl 与 Body 各存一份(MVP 复制;intern 化随 M2.2 评估)
        hir::Pat {
            kind: match &p.kind {
                hir::PatKind::Wild => hir::PatKind::Wild,
                hir::PatKind::Binding { local } => hir::PatKind::Binding { local: *local },
                hir::PatKind::Lit => hir::PatKind::Lit,
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
            ast::ExprKind::Try(e) => hir::ExprKind::Try(Box::new(self.lower_expr(e))),
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
            ast::ExprKind::For { pat, iter, body } => hir::ExprKind::For {
                pat: self.lower_pat(pat),
                iter: Box::new(self.lower_expr(iter)),
                body: self.lower_block(body),
            },
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
            kind,
            span: expr.span,
        }
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
            | hir::ExprKind::Try(expr)
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
            hir::ExprKind::For { iter, body, .. } => {
                walk_expr(iter, f);
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
