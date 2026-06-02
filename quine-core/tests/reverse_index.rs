use quine_core::common::{RowIndex, Value};
use quine_core::related_egraph::RelatedEGraph;
use quine_core::table::Row;
use quine_core::types::{BaseType, TableDef, Type};

// ============================================================================
// AC-1: Single insert populates reverse_index
// ============================================================================

/// Insert one row into an Id-typed table populates reverse_index.
///
/// Given: a RelatedEGraph with an Id-typed table
/// When:  a single row is inserted
/// Then:  reverse_index maps the row's canonical value to a set
///        containing (table_id, row_index)
#[test]
fn ac1_single_insert_populates_reverse_index() {
    let mut eg = RelatedEGraph::default();
    eg.add_table(TableDef(
        "t".into(),
        Box::new([Type::Base(BaseType::Id)]),
        None,
    ));
    let v = eg.fresh_id();
    eg.insert(0, Row(smallvec::smallvec![]), v);

    let enodes = eg.eclass_enodes(v);
    assert_eq!(enodes.len(), 1);
    assert!(enodes.contains(&(0, RowIndex(0))));
}

// ============================================================================
// AC-2: Multiple rows with same eclass aggregate in reverse_index
// ============================================================================

/// Multiple rows with the same eclass value aggregate in reverse_index.
///
/// Given: a table where multiple rows share the same value (same eclass)
/// When:  those rows are inserted
/// Then:  reverse_index[canonical] contains all (table_id, row_index)
///        references for that eclass
#[test]
fn ac2_multiple_rows_same_eclass_aggregate() {
    let mut eg = RelatedEGraph::default();
    eg.add_table(TableDef(
        "t".into(),
        Box::new([Type::Base(BaseType::Id), Type::Base(BaseType::Id)]),
        None,
    ));
    let k1 = eg.fresh_id();
    let k2 = eg.fresh_id();
    let v = eg.fresh_id();
    eg.insert(0, Row(smallvec::smallvec![k1]), v);
    eg.insert(0, Row(smallvec::smallvec![k2]), v);

    let enodes = eg.eclass_enodes(v);
    assert_eq!(enodes.len(), 2);
    assert!(enodes.contains(&(0, RowIndex(0))));
    assert!(enodes.contains(&(0, RowIndex(1))));
}

// ============================================================================
// AC-3: Union merges reverse_index entries
// ============================================================================

/// Union merges reverse_index entries from both eclasses.
///
/// Given: two tables each with rows belonging to different eclasses
/// When:  the two eclasses are unioned and rebuild runs
/// Then:  reverse_index[merged_canonical] contains all enode references
///        from both original eclasses
#[test]
fn ac3_union_merges_reverse_index_entries() {
    let mut eg = RelatedEGraph::default();
    eg.add_table(TableDef(
        "t1".into(),
        Box::new([Type::Base(BaseType::Id)]),
        None,
    ));
    eg.add_table(TableDef(
        "t2".into(),
        Box::new([Type::Base(BaseType::Id)]),
        None,
    ));
    let v1 = eg.fresh_id();
    let v2 = eg.fresh_id();
    eg.insert(0, Row(smallvec::smallvec![]), v1);
    eg.insert(1, Row(smallvec::smallvec![]), v2);

    eg.union(v1, v2);
    eg.rebuild();

    let canonical = eg.find(v1);
    let enodes = eg.eclass_enodes(canonical);
    assert_eq!(enodes.len(), 2);
    assert!(enodes.contains(&(0, RowIndex(0))));
    assert!(enodes.contains(&(1, RowIndex(0))));
}

// ============================================================================
// AC-4: Rebuild removes merged rows from reverse_index
// ============================================================================

/// Rebuild removes absorbed rows from reverse_index.
///
/// Given: a table without merge fn, with two rows whose keys become
///        identical after a union (key canonicalization)
/// When:  rebuild processes the pending unions
/// Then:  reverse_index only contains the surviving row's reference,
///        not the absorbed row's
#[test]
fn ac4_rebuild_removes_merged_rows() {
    let mut eg = RelatedEGraph::default();
    eg.add_table(TableDef(
        "t".into(),
        Box::new([Type::Base(BaseType::Id), Type::Base(BaseType::Id)]),
        None,
    ));
    let k1 = eg.fresh_id();
    let k2 = eg.fresh_id();
    let v = eg.fresh_id();
    eg.insert(0, Row(smallvec::smallvec![k1]), v);
    eg.insert(0, Row(smallvec::smallvec![k2]), v);

    // Union the two keys so they canonicalize to the same value.
    // rebuild_row will then detect a duplicate key and absorb one row.
    eg.union(k1, k2);
    eg.rebuild();

    let canonical = eg.find(v);
    let enodes = eg.eclass_enodes(canonical);
    assert_eq!(
        enodes.len(),
        1,
        "only the surviving row should remain in reverse_index"
    );
}

// ============================================================================
// AC-5: Literal-typed tables are not tracked
// ============================================================================

/// Tables whose value column is a literal type are not tracked in reverse_index.
///
/// Given: a table whose value column is a literal type such as I64
/// When:  a row is inserted
/// Then:  reverse_index does not contain entries for that value
#[test]
fn ac5_literal_typed_tables_not_tracked() {
    let mut eg = RelatedEGraph::default();
    // Prime union_find so that Value(0) is registered and find() won't panic.
    let _ = eg.fresh_id(); // registers Value(0)

    // Table with I64 value column type (literal, not eclass)
    eg.add_table(TableDef(
        "t".into(),
        Box::new([Type::Base(BaseType::I64)]),
        None,
    ));

    // Insert with Value(0) — find(Value(0)) works because it is registered.
    // The reverse_index type check sees I64 in the table definition and skips.
    eg.insert(0, Row(smallvec::smallvec![]), Value(0));

    // eclass_enodes should return empty: I64 values are not tracked
    assert!(
        eg.eclass_enodes(Value(0)).is_empty(),
        "I64-typed value columns should not be tracked in reverse_index"
    );
}

// ============================================================================
// AC-6: Canonical changes update reverse_index keys
// ============================================================================

/// Canonical changes update reverse_index keys.
///
/// Given: a row inserted with an eclass value that later gets unioned
///        to a smaller canonical
/// When:  rebuild completes
/// Then:  reverse_index has the entry under the new canonical key
///        and the old key has no entries
#[test]
fn ac6_canonical_changes_update_reverse_index_keys() {
    let mut eg = RelatedEGraph::default();
    eg.add_table(TableDef(
        "t".into(),
        Box::new([Type::Base(BaseType::Id)]),
        None,
    ));
    let v1 = eg.fresh_id();
    let v2 = eg.fresh_id();
    eg.insert(0, Row(smallvec::smallvec![]), v1);

    // v1 < v2 because v1 was created first (Value(0) < Value(1))
    // After union(v1, v2): v1 is parent (smaller), v2 is child
    eg.union(v1, v2);
    eg.rebuild();

    // The reverse_index entry should be under the new canonical (v1)
    let enodes = eg.eclass_enodes(v1);
    assert_eq!(enodes.len(), 1);
    assert!(enodes.contains(&(0, RowIndex(0))));

    // eclass_enodes canonicalizes input, so v2 also resolves to the same result
    assert_eq!(
        eg.eclass_enodes(v2).len(),
        1,
        "eclass_enodes canonicalizes input, so v2 resolves to v1"
    );
}

// ============================================================================
// AC-7: eclass_enodes returns all enodes for an eclass
// ============================================================================

/// eclass_enodes returns all enode references for an eclass.
///
/// Given: reverse_index has multiple (table_id, row_index) entries for
///        an eclass
/// When:  eclass_enodes(eclass) is called
/// Then:  all references are returned
#[test]
fn ac7_eclass_enodes_returns_all_enodes() {
    let mut eg = RelatedEGraph::default();
    eg.add_table(TableDef(
        "t".into(),
        Box::new([Type::Base(BaseType::Id), Type::Base(BaseType::Id)]),
        None,
    ));
    let v = eg.fresh_id();
    let k1 = eg.fresh_id();
    let k2 = eg.fresh_id();
    let k3 = eg.fresh_id();
    eg.insert(0, Row(smallvec::smallvec![k1]), v);
    eg.insert(0, Row(smallvec::smallvec![k2]), v);
    eg.insert(0, Row(smallvec::smallvec![k3]), v);

    let enodes = eg.eclass_enodes(v);
    assert_eq!(enodes.len(), 3);
    assert!(enodes.contains(&(0, RowIndex(0))));
    assert!(enodes.contains(&(0, RowIndex(1))));
    assert!(enodes.contains(&(0, RowIndex(2))));
}

// ============================================================================
// AC-8: eclass_enodes canonicalizes input
// ============================================================================

/// eclass_enodes canonicalizes the input before querying.
///
/// Given: an eclass value that has been unioned under a smaller canonical
/// When:  eclass_enodes(non_canonical_value) is called
/// Then:  the result matches eclass_enodes(canonical_value)
#[test]
fn ac8_eclass_enodes_canonicalizes_input() {
    let mut eg = RelatedEGraph::default();
    eg.add_table(TableDef(
        "t".into(),
        Box::new([Type::Base(BaseType::Id)]),
        None,
    ));
    let v1 = eg.fresh_id(); // canonical (smaller)
    let v2 = eg.fresh_id(); // non-canonical (larger)
    eg.insert(0, Row(smallvec::smallvec![]), v1);

    eg.union(v1, v2);
    eg.rebuild();

    // Query with non-canonical v2 should return same as canonical v1
    let by_canonical = eg.eclass_enodes(v1);
    let by_non_canonical = eg.eclass_enodes(v2);
    assert_eq!(by_canonical, by_non_canonical);
    assert_eq!(by_canonical.len(), 1);
}

// ============================================================================
// AC-9: eclass_enodes returns empty for unknown eclass
// ============================================================================

/// eclass_enodes returns an empty set for an unknown eclass.
///
/// Given: an empty RelatedEGraph
/// When:  eclass_enodes(any_value) is called
/// Then:  an empty set is returned
#[test]
fn ac9_eclass_enodes_unknown_eclass_returns_empty() {
    let mut eg = RelatedEGraph::default();
    let v = eg.fresh_id();
    let enodes = eg.eclass_enodes(v);
    assert!(enodes.is_empty());
}

// ============================================================================
// AC-10: eclass_enodes works cross-table
// ============================================================================

/// eclass_enodes returns references from all tables.
///
/// Given: multiple tables with rows belonging to the same eclass
/// When:  eclass_enodes(eclass) is called
/// Then:  references from all tables are returned with correct table_ids
#[test]
fn ac10_eclass_enodes_cross_table() {
    let mut eg = RelatedEGraph::default();
    eg.add_table(TableDef(
        "t1".into(),
        Box::new([Type::Base(BaseType::Id)]),
        None,
    ));
    eg.add_table(TableDef(
        "t2".into(),
        Box::new([Type::Base(BaseType::Id)]),
        None,
    ));
    eg.add_table(TableDef(
        "t3".into(),
        Box::new([Type::Base(BaseType::Id)]),
        None,
    ));
    let v = eg.fresh_id();
    eg.insert(0, Row(smallvec::smallvec![]), v);
    eg.insert(1, Row(smallvec::smallvec![]), v);
    eg.insert(2, Row(smallvec::smallvec![]), v);

    let enodes = eg.eclass_enodes(v);
    assert_eq!(enodes.len(), 3);
    assert!(enodes.contains(&(0, RowIndex(0))));
    assert!(enodes.contains(&(1, RowIndex(0))));
    assert!(enodes.contains(&(2, RowIndex(0))));
}
