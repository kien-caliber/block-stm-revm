use alloy_consensus::{Eip658Value, Receipt};
use alloy_primitives::{Address, Bytes, Log, LogData, B256, U256};
use pevm::{AccountBasic, EvmAccount, EvmCode, PevmTxExecutionResult};
use revm::primitives::{AccountInfo, AccountStatus, Bytecode, EvmStorageSlot};
use std::{
    collections::BTreeSet,
    fmt::{Debug, Display},
    hash::Hash,
    path::PathBuf,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Reason {
    path: PathBuf,
    left: String,
    right: String,
}

impl Reason {
    fn new(path: PathBuf, left: impl Debug, right: impl Debug) -> Self {
        Reason {
            path,
            left: format!("{:?}", left),
            right: format!("{:?}", right),
        }
    }
}

impl Display for Reason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let path_components: Vec<&str> = self.path.iter().map(|s| s.to_str().unwrap()).collect();

        write!(
            f,
            "{}\n   left = {}\n  right = {}",
            path_components.join("."),
            self.left,
            self.right
        )
    }
}

fn diff_simple<T: PartialEq + Debug>(path: PathBuf, left: &T, right: &T) -> Vec<Reason> {
    if left != right {
        Vec::from(&[Reason::new(path, left, right)])
    } else {
        Vec::new()
    }
}

pub trait Diffable {
    fn diff(path: PathBuf, left: &Self, right: &Self) -> Vec<Reason>;
}

macro_rules! impl_diffable_simple {
    ($t:ty) => {
        impl Diffable for $t {
            fn diff(path: PathBuf, left: &Self, right: &Self) -> Vec<Reason> {
                diff_simple(path, left, right)
            }
        }
    };
}

impl_diffable_simple!(AccountStatus);
impl_diffable_simple!(Address);
impl_diffable_simple!(B256);
impl_diffable_simple!(Bytecode);
impl_diffable_simple!(Bytes);
impl_diffable_simple!(Eip658Value);
impl_diffable_simple!(EvmCode);
impl_diffable_simple!(EvmStorageSlot);
impl_diffable_simple!(LogData);
impl_diffable_simple!(u128);
impl_diffable_simple!(U256);
impl_diffable_simple!(u64);
impl_diffable_simple!(usize);

impl<T: Diffable> Diffable for Option<T> {
    fn diff(path: PathBuf, left: &Self, right: &Self) -> Vec<Reason> {
        match (left, right) {
            (None, None) => Vec::new(),
            (None, Some(_)) => Vec::from(&[Reason::new(path, "None", "Some(_)")]),
            (Some(_), None) => Vec::from(&[Reason::new(path, "Some(_)", "None")]),
            (Some(left_inner), Some(right_inner)) => {
                Diffable::diff(path.join("unwrap()"), left_inner, right_inner)
            }
        }
    }
}

impl<T: Diffable, E: Debug + PartialEq> Diffable for Result<T, E> {
    fn diff(path: PathBuf, left: &Self, right: &Self) -> Vec<Reason> {
        match (left, right) {
            (Ok(left_inner), Ok(right_inner)) => {
                Diffable::diff(path.join("unwrap()"), left_inner, right_inner)
            }
            (Ok(_), Err(_)) => Vec::from(&[Reason::new(path, "Ok(_)", "Err(_)")]),
            (Err(_), Ok(_)) => Vec::from(&[Reason::new(path, "Err(_)", "Ok(_)")]),
            (Err(left_error), Err(right_error)) => {
                diff_simple(path.join("unwrap_err()"), left_error, right_error)
            }
        }
    }
}

impl<T: Diffable> Diffable for [T] {
    fn diff(path: PathBuf, left: &Self, right: &Self) -> Vec<Reason> {
        let mut reasons = diff_simple(path.join("len()"), &left.len(), &right.len());
        for (index, (left_item, right_item)) in Iterator::zip(left.iter(), right.iter()).enumerate()
        {
            reasons.extend(Diffable::diff(
                path.join(index.to_string()),
                left_item,
                right_item,
            ));
        }
        reasons
    }
}

impl<T: Diffable> Diffable for Vec<T> {
    fn diff(path: PathBuf, left: &Self, right: &Self) -> Vec<Reason> {
        Diffable::diff(path, &left[..], &right[..])
    }
}

impl<K: Copy + Ord + Debug + Hash, V: Diffable + Clone> Diffable
    for std::collections::HashMap<K, V>
{
    fn diff(path: PathBuf, left: &Self, right: &Self) -> Vec<Reason> {
        let mut reasons = Vec::new();
        let all_addresses: BTreeSet<K> = Iterator::chain(left.keys(), right.keys())
            .copied()
            .collect();
        for key in all_addresses {
            reasons.extend(Diffable::diff(
                path.join(format!("get(\"{:?}\")", key)),
                &left.get(&key).cloned(),
                &right.get(&key).cloned(),
            ));
        }
        reasons
    }
}

impl<K: Copy + Ord + Debug + Hash, V: Diffable + Clone> Diffable for ahash::AHashMap<K, V> {
    fn diff(path: PathBuf, left: &Self, right: &Self) -> Vec<Reason> {
        let mut reasons = Vec::new();
        let all_addresses: BTreeSet<K> = Iterator::chain(left.keys(), right.keys())
            .copied()
            .collect();
        for key in all_addresses {
            reasons.extend(Diffable::diff(
                path.join(format!("get(\"{:?}\")", key)),
                &left.get(&key).cloned(),
                &right.get(&key).cloned(),
            ));
        }
        reasons
    }
}

macro_rules! impl_diffable_complex {
    ($t:ty; $( $x:ident ),*) => {
        impl Diffable for $t {
            fn diff(path: PathBuf, left: &Self, right: &Self) -> Vec<Reason> {
                let mut reasons = Vec::new();
                $(
                    reasons.extend(Diffable::diff(
                        path.join(stringify!($x)),
                        &left.$x,
                        &right.$x,
                    ));
                )*
                reasons
            }
        }
    };
}

impl_diffable_complex!(Log; address, data);
impl_diffable_complex!(AccountInfo; balance, nonce, code_hash);
impl_diffable_complex!(AccountBasic; balance, nonce);
impl_diffable_complex!(EvmAccount; basic, storage, code_hash, code);
impl_diffable_complex!(Receipt; status, cumulative_gas_used, logs);
impl_diffable_complex!(PevmTxExecutionResult; receipt, state);
