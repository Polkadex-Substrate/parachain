use super::*;

#[allow(unused)]
use crate::Pallet as XcmHelper;
use frame_benchmarking::{account, benchmarks, whitelisted_caller};
use frame_support::{
    sp_runtime::{traits::Hash, SaturatedConversion},
    traits::{fungibles::Mutate, Currency},
};
use frame_support::BoundedVec;
use frame_system::RawOrigin;
use frame_benchmarking::Box;
use xcm_builder::test_utils::{NetworkId};
use sp_core::{ecdsa::{self, Signature}};
use sp_core::ConstU32;
use xcm::{
    latest::{
        Error as XcmError, Fungibility, Junction, Junctions, MultiAsset, MultiAssets,
        MultiLocation,
    },
    v1::AssetId,
    v2::WeightLimit,
    VersionedMultiAssets, VersionedMultiLocation,
};
use parity_scale_codec::{Encode};

const SEED: u32 = 0;

benchmarks! {
    set_thea_key {
        let b in 1 .. 1000;
        let thea_key: [u8;64] = [b as u8;64];
    }: _(RawOrigin::Root, thea_key)

    create_parachain_asset {
        let b in 1 .. 1000;
        let asset_location = MultiLocation::new(1, Junctions::X1(Junction::Parachain(2011)));
        let asset_id = AssetId::Concrete(asset_location);
    }: _(RawOrigin::Root, Box::new(asset_id))

    whitelist_token {
        let b in 1 .. 1000;
        let token = b as u128;
    }: _(RawOrigin::Root, token)

    withdraw_asset {
        let b in 1 .. 1000;
        let asset_location = MultiLocation::new(1, Junctions::X1(Junction::Parachain(b)));
	    let asset_id = AssetId::Concrete(asset_location);
	    let multi_asset = MultiAsset::from((asset_id, Fungibility::Fungible(1000000000)));
	    let wrapped_multi_asset = VersionedMultiAssets::V1(MultiAssets::from(multi_asset));
	    let boxed_asset_id = Box::new(wrapped_multi_asset);
        let destination = MultiLocation::new(1, Junctions::X2(Junction::Parachain(2011), Junction::AccountId32 { network: NetworkId::Any, id: [1;32] }));
	    let boxed_multilocation = Box::new(VersionedMultiLocation::V1(destination));
        let boundec_vec:  BoundedVec<
				(
					sp_std::boxed::Box<VersionedMultiAssets>,
					sp_std::boxed::Box<VersionedMultiLocation>,
				),
				ConstU32<10>> = BoundedVec::try_from(vec![(boxed_asset_id, boxed_multilocation)]
        ).unwrap();
        let public_key = sp_core::ecdsa::Public::from_raw([3, 31, 175, 212, 203, 148, 135, 36, 88, 104, 149, 228, 133, 42, 19, 172, 28, 211, 198, 250, 241, 19, 167, 9, 117, 60, 3, 83, 155, 157, 81, 94, 70]);
        let signature = sp_core::ecdsa::Signature::from_raw([29, 25, 189, 233, 74, 130, 67, 116, 196, 119, 24, 191, 23, 3, 188, 199, 30, 10, 104, 197, 155, 24, 27, 159, 72, 86, 222, 18, 14, 56, 177, 36, 121,
            126, 243, 103, 14, 254, 111, 98, 166, 153, 18, 158, 90, 150, 45, 250, 40, 174, 129, 162, 155, 104, 28, 101, 198, 160, 90, 250, 191, 15, 162, 153, 0]);

        let council_member: T::AccountId = account("mem1", b, SEED);
    }: _(RawOrigin::Signed(council_member), boundec_vec, 0, signature)
}

#[cfg(test)]
use frame_benchmarking::impl_benchmark_test_suite;

#[cfg(test)]
impl_benchmark_test_suite!(XcmHelper, crate::mock::new_test_ext(), crate::mock::Test);