use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok};

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
