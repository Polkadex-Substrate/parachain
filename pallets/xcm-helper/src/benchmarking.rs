use super::*;

#[allow(unused)]
use crate::Pallet as XcmHelper;
use frame_benchmarking::{account, benchmarks, whitelisted_caller, Box};
use frame_support::{
    sp_runtime::{traits::Hash, SaturatedConversion},
    traits::{fungibles::Mutate, Currency, EnsureOrigin},
    BoundedVec,
};
use frame_system::RawOrigin;
use parity_scale_codec::Encode;
use sp_core::{
    ecdsa::{self, Signature},
    ConstU32,
};
use sp_std::vec;
use xcm::{
    latest::{
        Error as XcmError, Fungibility, Junction, Junctions, MultiAsset, MultiAssets,
        MultiLocation, NetworkId,
    },
    v1::AssetId,
    v2::WeightLimit,
    VersionedMultiAssets, VersionedMultiLocation,
};

const SEED: u32 = 0;

benchmarks! {
    create_parachain_asset {
		let b in 1 .. 1000;
		let asset_location = MultiLocation::new(1, Junctions::X1(Junction::Parachain(2011)));
		let asset_id = AssetId::Concrete(asset_location);
	}: _(RawOrigin::Root, Box::new(asset_id))

    whitelist_token {
		let b in 1 .. 1000;
		let token = b as u128;
	}: _(RawOrigin::Root, token)

    transfer_fee {

    }

}

#[cfg(test)]
use frame_benchmarking::impl_benchmark_test_suite;

#[cfg(test)]
impl_benchmark_test_suite!(XcmHelper, crate::mock::new_test_ext(), crate::mock::Test);