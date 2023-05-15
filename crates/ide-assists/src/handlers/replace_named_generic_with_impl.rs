use syntax::{
    ast::{self, make::impl_trait_type, HasGenericParams, HasName, HasTypeBounds},
    ted, AstNode,
};

use crate::{AssistContext, AssistId, AssistKind, Assists};

// Assist: replace_named_generic_with_impl
//
// Replaces named generic with an `impl Trait` in function argument.
//
// ```
// fn new<P$0: AsRef<Path>>(location: P) -> Self {}
// ```
// ->
// ```
// fn new(location: impl AsRef<Path>) -> Self {}
// ```
pub(crate) fn replace_named_generic_with_impl(
    acc: &mut Assists,
    ctx: &AssistContext<'_>,
) -> Option<()> {
    // finds `<P: AsRef<Path>>`
    let type_param = ctx.find_node_at_offset::<ast::TypeParam>()?;

    // The list of type bounds / traits: `AsRef<Path>`
    let type_bound_list = type_param.type_bound_list()?;

    // returns `P`
    let type_param_name = type_param.name()?;

    let fn_ = type_param.syntax().ancestors().find_map(ast::Fn::cast)?;
    let params = fn_
        .param_list()?
        .params()
        .filter_map(|param| {
            // function parameter type needs to match generic type name
            if let ast::Type::PathType(path_type) = param.ty()? {
                let left = path_type.path()?.segment()?.name_ref()?.ident_token()?.to_string();
                let right = type_param_name.to_string();
                if left == right {
                    Some(param)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    if params.is_empty() {
        return None;
    }

    let target = type_param.syntax().text_range();

    acc.add(
        AssistId("replace_named_generic_with_impl", AssistKind::RefactorRewrite),
        "Replace named generic with impl",
        target,
        |edit| {
            let type_param = edit.make_mut(type_param);
            let fn_ = edit.make_mut(fn_);

            // get all params
            let param_types = params
                .iter()
                .filter_map(|param| match param.ty() {
                    Some(ast::Type::PathType(param_type)) => Some(edit.make_mut(param_type)),
                    _ => None,
                })
                .collect::<Vec<_>>();

            if let Some(generic_params) = fn_.generic_param_list() {
                generic_params.remove_generic_param(ast::GenericParam::TypeParam(type_param));
                if generic_params.generic_params().count() == 0 {
                    ted::remove(generic_params.syntax());
                }
            }

            // get type bounds in signature type: `P` -> `impl AsRef<Path>`
            let new_bounds = impl_trait_type(type_bound_list);
            for param_type in param_types.iter().rev() {
                ted::replace(param_type.syntax(), new_bounds.clone_for_update().syntax());
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::check_assist;

    #[test]
    fn replace_generic_moves_into_function() {
        check_assist(
            replace_named_generic_with_impl,
            r#"fn new<T$0: ToString>(input: T) -> Self {}"#,
            r#"fn new(input: impl ToString) -> Self {}"#,
        );
    }

    #[test]
    fn replace_generic_with_inner_associated_type() {
        check_assist(
            replace_named_generic_with_impl,
            r#"fn new<P$0: AsRef<Path>>(input: P) -> Self {}"#,
            r#"fn new(input: impl AsRef<Path>) -> Self {}"#,
        );
    }

    #[test]
    fn replace_generic_trait_applies_to_all_matching_params() {
        check_assist(
            replace_named_generic_with_impl,
            r#"fn new<T$0: ToString>(a: T, b: T) -> Self {}"#,
            r#"fn new(a: impl ToString, b: impl ToString) -> Self {}"#,
        );
    }

    #[test]
    fn replace_generic_with_multiple_generic_names() {
        check_assist(
            replace_named_generic_with_impl,
            r#"fn new<P: AsRef<Path>, T$0: ToString>(t: T, p: P) -> Self {}"#,
            r#"fn new<P: AsRef<Path>>(t: impl ToString, p: P) -> Self {}"#,
        );
    }

    #[test]
    fn replace_generic_with_multiple_trait_bounds() {
        check_assist(
            replace_named_generic_with_impl,
            r#"fn new<P$0: Send + Sync>(p: P) -> Self {}"#,
            r#"fn new(p: impl Send + Sync) -> Self {}"#,
        );
    }
}
