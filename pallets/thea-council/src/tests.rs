use crate::{
	mock::*, ActiveCouncilMembers, Error, PendingCouncilMembers, Proposal, Proposals, Voted,
};
use frame_support::{assert_noop, assert_ok};
use sp_core::{bounded::BoundedVec, ConstU32};
use sp_runtime::SaturatedConversion;

#[test]
fn test_add_member_returns_ok() {
	new_test_ext().execute_with(|| {
		setup_council_members();
		let (first_council_member, second_council_member, _) = get_council_members();
		let new_member = 4;
		assert_ok!(TheaCouncil::add_member(
			RuntimeOrigin::signed(first_council_member),
			new_member
		));
		// Check total Votes
		let proposal = Proposal::AddNewMember(new_member);
		let expected_votes: BoundedVec<Voted<u64>, ConstU32<100>> =
			BoundedVec::try_from(vec![Voted(first_council_member)]).unwrap();
		assert_eq!(<Proposals<Test>>::get(proposal), expected_votes);
		//Second vote
		assert_ok!(TheaCouncil::add_member(
			RuntimeOrigin::signed(second_council_member),
			new_member
		));
		let pending_set = <PendingCouncilMembers<Test>>::get();
		assert!(pending_set.contains(&new_member));
		<Proposals<Test>>::remove(proposal.clone());
		assert!(!<Proposals<Test>>::contains_key(proposal));
	})
}

#[test]
fn test_add_member_returns_sender_not_council_member() {
	new_test_ext().execute_with(|| {
		let wrong_council_member = 1;
		let new_member = 4;
		assert_noop!(
			TheaCouncil::add_member(RuntimeOrigin::signed(wrong_council_member), new_member),
			Error::<Test>::SenderNotCouncilMember
		);
	})
}

#[test]
fn test_add_member_sender_already_voted() {
	new_test_ext().execute_with(|| {
		setup_council_members();
		let (first_council_member, _, _) = get_council_members();
		let new_member = 4;
		assert_ok!(TheaCouncil::add_member(
			RuntimeOrigin::signed(first_council_member),
			new_member
		));
		assert_noop!(
			TheaCouncil::add_member(RuntimeOrigin::signed(first_council_member), new_member),
			Error::<Test>::SenderAlreadyVoted
		);
	})
}

#[test]
fn test_remove_member_returns_ok() {
	new_test_ext().execute_with(|| {
		setup_council_members();
		let (first_council_member, second_council_member, member_to_be_removed) =
			get_council_members();
		assert_ok!(TheaCouncil::remove_member(
			RuntimeOrigin::signed(first_council_member),
			member_to_be_removed
		));
		assert_ok!(TheaCouncil::remove_member(
			RuntimeOrigin::signed(second_council_member),
			member_to_be_removed
		));
		let active_set = <ActiveCouncilMembers<Test>>::get();
		assert!(!active_set.contains(&member_to_be_removed));
	})
}

#[test]
fn test_claim_membership_returns_ok() {
	new_test_ext().execute_with(|| {
		setup_council_members();
		let (first_council_member, second_council_member, _) = get_council_members();
		let new_member = 4;
		assert_ok!(TheaCouncil::add_member(
			RuntimeOrigin::signed(first_council_member),
			new_member
		));
		assert_ok!(TheaCouncil::add_member(
			RuntimeOrigin::signed(second_council_member),
			new_member
		));
		assert_ok!(TheaCouncil::claim_membership(RuntimeOrigin::signed(new_member)));
		let active_set = <ActiveCouncilMembers<Test>>::get();
		assert!(active_set.contains(&new_member));
	})
}

#[test]
fn test_claim_membership_with_unregistered_pending_member_returns_not_pending_member() {
	new_test_ext().execute_with(|| {
		let not_a_pending_member = 1;
		assert_noop!(
			TheaCouncil::claim_membership(RuntimeOrigin::signed(not_a_pending_member)),
			Error::<Test>::NotPendingMember
		);
	})
}

#[test]
fn get_expected_votes_test() {
	new_test_ext().execute_with(|| {
		// at most 10 council members allowed
		for i in 2..11 {
			// we start with 1 and it can go up to 10
			let members_vec: Vec<u64> =
				(1u64..=i).into_iter().enumerate().map(|(n, _)| n as u64 + 1).collect();
			let members = BoundedVec::try_from(members_vec).unwrap();
			<ActiveCouncilMembers<Test>>::set(members.clone());
			// we check if we have more than half of actual council members always
			let expected: u64 =
				TheaCouncil::get_expected_votes().saturated_into::<u64>().saturating_mul(2);
			println!("members {members:?}\ni: {i}\nexpected {expected}");
			assert!(expected > i);
		}
	})
}

fn setup_council_members() {
	let (first_council_member, second_council_member, third_council_member) = get_council_members();
	let council = BoundedVec::try_from(vec![
		first_council_member,
		second_council_member,
		third_council_member,
	])
	.unwrap();
	<ActiveCouncilMembers<Test>>::put(council);
}

fn get_council_members() -> (u64, u64, u64) {
	let first_council_member = 1;
	let second_council_member = 2;
	let third_council_member = 3;
	(first_council_member, second_council_member, third_council_member)
}
