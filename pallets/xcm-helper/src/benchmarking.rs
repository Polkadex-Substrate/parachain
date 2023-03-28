use super::*;

#[allow(unused)]
use crate::Pallet as XcmHelper;
use frame_benchmarking::{account, benchmarks, whitelisted_caller};
use frame_support::{
    sp_runtime::{traits::Hash, SaturatedConversion},
    traits::{fungibles::Mutate, Currency},
};
use xcm_helper::PendingWithdrawal;
use frame_support::BoundedVec;
use frame_system::RawOrigin;
const SEED: u32 = 0;

benchmarks! {
    change_thea_key {

    }
}