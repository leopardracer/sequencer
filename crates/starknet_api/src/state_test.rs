use std::collections::HashMap;

use indexmap::{indexmap, IndexMap};
use serde_json::json;

use crate::class_hash;
use crate::core::{ClassHash, CompiledClassHash, Nonce};
use crate::deprecated_contract_class::EntryPointOffset;
use crate::state::{SierraContractClass, ThinStateDiff};
use crate::test_utils::read_json_file;

#[test]
fn entry_point_offset_from_json_str() {
    let data = r#"
        {
            "offset_1":  2 ,
            "offset_2": "0x7b"
        }"#;
    let offsets: HashMap<String, EntryPointOffset> = serde_json::from_str(data).unwrap();

    assert_eq!(EntryPointOffset(2), offsets["offset_1"]);
    assert_eq!(EntryPointOffset(123), offsets["offset_2"]);
}

#[test]
fn entry_point_offset_into_json_str() {
    let offset = EntryPointOffset(123);
    assert_eq!(json!(offset), json!(format!("{:#x}", offset.0)));
}

#[test]
fn thin_state_diff_len() {
    let state_diff = ThinStateDiff {
        deployed_contracts: indexmap! {
            0u64.into() => ClassHash(4u64.into()),
        },
        storage_diffs: indexmap! {
            0u64.into() => indexmap! {
                0u64.into() => 0u64.into(),
                1u64.into() => 1u64.into(),
            },
            1u64.into() => indexmap! {
                0u64.into() => 0u64.into(),
            },
        },
        declared_classes: indexmap! {
            ClassHash(4u64.into()) => CompiledClassHash(9u64.into()),
            ClassHash(5u64.into()) => CompiledClassHash(10u64.into()),
        },
        deprecated_declared_classes: vec![
            ClassHash(6u64.into()),
            ClassHash(7u64.into()),
            ClassHash(8u64.into()),
        ],
        nonces: indexmap! {
            0u64.into() => Nonce(1u64.into()),
            1u64.into() => Nonce(1u64.into()),
        },
    };
    assert_eq!(state_diff.len(), 11);
}

#[test]
fn thin_state_diff_is_empty() {
    assert!(ThinStateDiff::default().is_empty());
    assert!(
        ThinStateDiff {
            storage_diffs: indexmap! { Default::default() => IndexMap::new() },
            ..Default::default()
        }
        .is_empty()
    );

    assert!(
        !ThinStateDiff {
            deployed_contracts: indexmap! { Default::default() => Default::default() },
            ..Default::default()
        }
        .is_empty()
    );
    assert!(
        !ThinStateDiff {
            storage_diffs: indexmap! { Default::default() => indexmap! { Default::default() => Default::default() } },
            ..Default::default()
        }
        .is_empty()
    );
    assert!(
        !ThinStateDiff {
            declared_classes: indexmap! { Default::default() => Default::default() },
            ..Default::default()
        }
        .is_empty()
    );
    assert!(
        !ThinStateDiff {
            deprecated_declared_classes: vec![Default::default()],
            ..Default::default()
        }
        .is_empty()
    );
    assert!(
        !ThinStateDiff {
            nonces: indexmap! { Default::default() => Default::default() },
            ..Default::default()
        }
        .is_empty()
    );
}

#[test]
fn calc_class_hash() {
    let class: SierraContractClass = read_json_file("class.json");
    let expected_class_hash =
        class_hash!("0x29927c8af6bccf3f6fda035981e765a7bdbf18a2dc0d630494f8758aa908e2b");
    let calculated_class_hash = class.calculate_class_hash();
    assert_eq!(calculated_class_hash, expected_class_hash);
}
