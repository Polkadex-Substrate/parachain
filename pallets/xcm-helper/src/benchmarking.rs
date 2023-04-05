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
use sp_core::{ecdsa::{self, Signature}, Pair};
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
				ConstU32<10>> = BoundedVec::try_from(vec![(boxed_asset_id, boxed_multilocation)]).unwrap();
        let withdraw_nonce = 0u32;
        let key_pair = Pair::generate();
	    let public_key = key_pair.public();
        let signature: sp_core::ecdsa::Signature = key_pair.sign(boundec_vec);
    }: _(RawOrigin::Signed(2), boundec_vec, withdraw_nonce, signature)
}