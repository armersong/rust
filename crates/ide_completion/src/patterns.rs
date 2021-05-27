//! Patterns telling us certain facts about current syntax element, they are used in completion context

use syntax::{
    algo::non_trivia_sibling,
    ast::{self, LoopBodyOwner},
    match_ast, AstNode, Direction, NodeOrToken, SyntaxElement,
    SyntaxKind::{self, *},
    SyntaxNode, SyntaxToken, T,
};

#[cfg(test)]
use crate::test_utils::{check_pattern_is_applicable, check_pattern_is_not_applicable};

pub(crate) fn has_trait_parent(element: SyntaxElement) -> bool {
    not_same_range_ancestor(element)
        .filter(|it| it.kind() == ASSOC_ITEM_LIST)
        .and_then(|it| it.parent())
        .filter(|it| it.kind() == TRAIT)
        .is_some()
}
#[test]
fn test_has_trait_parent() {
    check_pattern_is_applicable(r"trait A { f$0 }", has_trait_parent);
}

pub(crate) fn has_impl_parent(element: SyntaxElement) -> bool {
    not_same_range_ancestor(element)
        .filter(|it| it.kind() == ASSOC_ITEM_LIST)
        .and_then(|it| it.parent())
        .filter(|it| it.kind() == IMPL)
        .is_some()
}
#[test]
fn test_has_impl_parent() {
    check_pattern_is_applicable(r"impl A { f$0 }", has_impl_parent);
}

pub(crate) fn inside_impl_trait_block(element: SyntaxElement) -> bool {
    // Here we search `impl` keyword up through the all ancestors, unlike in `has_impl_parent`,
    // where we only check the first parent with different text range.
    element
        .ancestors()
        .find(|it| it.kind() == IMPL)
        .map(|it| ast::Impl::cast(it).unwrap())
        .map(|it| it.trait_().is_some())
        .unwrap_or(false)
}
#[test]
fn test_inside_impl_trait_block() {
    check_pattern_is_applicable(r"impl Foo for Bar { f$0 }", inside_impl_trait_block);
    check_pattern_is_applicable(r"impl Foo for Bar { fn f$0 }", inside_impl_trait_block);
    check_pattern_is_not_applicable(r"impl A { f$0 }", inside_impl_trait_block);
    check_pattern_is_not_applicable(r"impl A { fn f$0 }", inside_impl_trait_block);
}

pub(crate) fn has_field_list_parent(element: SyntaxElement) -> bool {
    not_same_range_ancestor(element).filter(|it| it.kind() == RECORD_FIELD_LIST).is_some()
}
#[test]
fn test_has_field_list_parent() {
    check_pattern_is_applicable(r"struct Foo { f$0 }", has_field_list_parent);
    check_pattern_is_applicable(r"struct Foo { f$0 pub f: i32}", has_field_list_parent);
}

pub(crate) fn has_block_expr_parent(element: SyntaxElement) -> bool {
    not_same_range_ancestor(element).filter(|it| it.kind() == BLOCK_EXPR).is_some()
}
#[test]
fn test_has_block_expr_parent() {
    check_pattern_is_applicable(r"fn my_fn() { let a = 2; f$0 }", has_block_expr_parent);
}

pub(crate) fn has_bind_pat_parent(element: SyntaxElement) -> bool {
    element.ancestors().any(|it| it.kind() == IDENT_PAT)
}

#[test]
fn test_has_bind_pat_parent() {
    check_pattern_is_applicable(r"fn my_fn(m$0) {}", has_bind_pat_parent);
    check_pattern_is_applicable(r"fn my_fn() { let m$0 }", has_bind_pat_parent);
}

pub(crate) fn has_ref_parent(element: SyntaxElement) -> bool {
    not_same_range_ancestor(element)
        .filter(|it| it.kind() == REF_PAT || it.kind() == REF_EXPR)
        .is_some()
}
#[test]
fn test_has_ref_parent() {
    check_pattern_is_applicable(r"fn my_fn(&m$0) {}", has_ref_parent);
    check_pattern_is_applicable(r"fn my() { let &m$0 }", has_ref_parent);
}

pub(crate) fn has_item_list_or_source_file_parent(element: SyntaxElement) -> bool {
    match not_same_range_ancestor(element) {
        Some(it) => it.kind() == SOURCE_FILE || it.kind() == ITEM_LIST,
        None => true,
    }
}
#[test]
fn test_has_item_list_or_source_file_parent() {
    check_pattern_is_applicable(r"i$0", has_item_list_or_source_file_parent);
    check_pattern_is_applicable(r"mod foo { f$0 }", has_item_list_or_source_file_parent);
}

pub(crate) fn is_match_arm(element: SyntaxElement) -> bool {
    not_same_range_ancestor(element.clone()).filter(|it| it.kind() == MATCH_ARM).is_some()
        && previous_sibling_or_ancestor_sibling(element)
            .and_then(|it| it.into_token())
            .filter(|it| it.kind() == FAT_ARROW)
            .is_some()
}
#[test]
fn test_is_match_arm() {
    check_pattern_is_applicable(r"fn my_fn() { match () { () => m$0 } }", is_match_arm);
}

pub(crate) fn previous_token(element: SyntaxElement) -> Option<SyntaxToken> {
    element.into_token().and_then(|it| previous_non_trivia_token(it))
}

/// Check if the token previous to the previous one is `for`.
/// For example, `for _ i$0` => true.
pub(crate) fn for_is_prev2(element: SyntaxElement) -> bool {
    element
        .into_token()
        .and_then(|it| previous_non_trivia_token(it))
        .and_then(|it| previous_non_trivia_token(it))
        .filter(|it| it.kind() == T![for])
        .is_some()
}
#[test]
fn test_for_is_prev2() {
    check_pattern_is_applicable(r"for i i$0", for_is_prev2);
}

pub(crate) fn has_prev_sibling(element: SyntaxElement, kind: SyntaxKind) -> bool {
    previous_sibling_or_ancestor_sibling(element).filter(|it| it.kind() == kind).is_some()
}
#[test]
fn test_has_impl_as_prev_sibling() {
    check_pattern_is_applicable(r"impl A w$0 {}", |it| has_prev_sibling(it, IMPL));
}

pub(crate) fn is_in_loop_body(element: SyntaxElement) -> bool {
    element
        .ancestors()
        .take_while(|it| it.kind() != FN && it.kind() != CLOSURE_EXPR)
        .find_map(|it| {
            let loop_body = match_ast! {
                match it {
                    ast::ForExpr(it) => it.loop_body(),
                    ast::WhileExpr(it) => it.loop_body(),
                    ast::LoopExpr(it) => it.loop_body(),
                    _ => None,
                }
            };
            loop_body.filter(|it| it.syntax().text_range().contains_range(element.text_range()))
        })
        .is_some()
}

fn not_same_range_ancestor(element: SyntaxElement) -> Option<SyntaxNode> {
    element
        .ancestors()
        .take_while(|it| it.text_range() == element.text_range())
        .last()
        .and_then(|it| it.parent())
}

fn previous_non_trivia_token(token: SyntaxToken) -> Option<SyntaxToken> {
    let mut token = token.prev_token();
    while let Some(inner) = token.clone() {
        if !inner.kind().is_trivia() {
            return Some(inner);
        } else {
            token = inner.prev_token();
        }
    }
    None
}

fn previous_sibling_or_ancestor_sibling(element: SyntaxElement) -> Option<SyntaxElement> {
    let token_sibling = non_trivia_sibling(element.clone(), Direction::Prev);
    if let Some(sibling) = token_sibling {
        Some(sibling)
    } else {
        // if not trying to find first ancestor which has such a sibling
        let range = element.text_range();
        let top_node = element.ancestors().take_while(|it| it.text_range() == range).last()?;
        let prev_sibling_node = top_node.ancestors().find(|it| {
            non_trivia_sibling(NodeOrToken::Node(it.to_owned()), Direction::Prev).is_some()
        })?;
        non_trivia_sibling(NodeOrToken::Node(prev_sibling_node), Direction::Prev)
    }
}
