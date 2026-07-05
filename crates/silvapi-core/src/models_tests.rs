use super::fuzzy_match;

#[test]
fn subsequence_matches() {
    assert!(fuzzy_match("berry-firmness", "brfrm"));
}

#[test]
fn out_of_order_fails() {
    assert!(!fuzzy_match("abc", "acb"));
}

#[test]
fn empty_needle_matches() {
    assert!(fuzzy_match("anything", ""));
}

#[test]
fn case_insensitive() {
    assert!(fuzzy_match("HelloWorld", "hw"));
}
