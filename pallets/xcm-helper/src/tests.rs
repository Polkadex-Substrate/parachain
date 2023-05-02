use crate as xcm_helper;
use crate::{mock::*, Error};
use frame_support::{
	assert_noop, assert_ok,
	traits::{Currency, Hooks},
};
use sp_core::{bounded::BoundedVec, ConstU32};
use sp_runtime::SaturatedConversion;
use xcm::latest::{AssetId, MultiLocation};

#[test]
fn test_create_parachain_asset_returns_ok() {
	new_test_ext().execute_with(|| {
		let asset_location = MultiLocation::parent();
		let asset_id = AssetId::Concrete(asset_location);
		let asset_id = sp_std::boxed::Box::new(asset_id);
		assert_ok!(XcmHelper::create_parachain_asset(RuntimeOrigin::root(), asset_id));
	});
}

#[test]
fn test_whitelist_token_returns_ok() {
	new_test_ext().execute_with(|| {
		let token = 100;
		assert_ok!(XcmHelper::whitelist_token(RuntimeOrigin::root(), token));
	});
}

#[test]
fn test_whitelist_token_returns_token_is_already_whitelisted() {
	new_test_ext().execute_with(|| {
		let token = 100;
		assert_ok!(XcmHelper::whitelist_token(RuntimeOrigin::root(), token));
		assert_noop!(
			XcmHelper::whitelist_token(RuntimeOrigin::root(), token),
			Error::<Test>::TokenIsAlreadyWhitelisted
		);
	});
}

// #[test]
// fn test_transfer_fee_returns_ok() {
//     new_test_ext().execute_with(|| {
//         let recipient = 1;
//         let pallet_id = xcm_helper::pallet::Config::<Test>//::AssetHandlerPalletId::get().into_account_truncating();
//         Balances::deposit_creating(&pallet_id, 2_000_000_000_000_000u128.saturated_into());
//         //assert_ok!(XcmHelper::transfer_fee(RuntimeOrigin::root(), token));
//     });
// }
