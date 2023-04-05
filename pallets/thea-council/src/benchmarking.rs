use super::*;

#[allow(unused)]
use crate::Pallet as TheaCouncil;
use frame_benchmarking::{account, benchmarks, whitelisted_caller};
use frame_support::{
    sp_runtime::{traits::Hash, SaturatedConversion},
    traits::{fungibles::Mutate, Currency},
};
use xcm_helper::PendingWithdrawal;
use frame_support::BoundedVec;
use frame_system::RawOrigin;
use std::time::{Duration, Instant};
const SEED: u32 = 0;

benchmarks! {
    add_member {
        // Add sender to council member
        let b in 1 .. 1000;
        let council_member: T::AccountId = account("mem1", b, SEED);
        let mut active_council_member = <ActiveCouncilMembers<T>>::get();
        active_council_member.try_push(council_member.clone()).unwrap();
        <ActiveCouncilMembers<T>>::put(active_council_member);
        let new_member: T::AccountId = account("mem2", b, SEED);
    }: _(RawOrigin::Signed(council_member), new_member)
    verify {

    }

    remove_member {
        let b in 1 .. 1000;
        let first_council_member: T::AccountId = account("mem1", b, SEED);
        let sec_council_member: T::AccountId = account("mem2", b, SEED);
        let third_council_member: T::AccountId = account("mem3", b, SEED);
        let mut active_council_member = <ActiveCouncilMembers<T>>::get();
        active_council_member.try_push(first_council_member.clone()).unwrap();
        active_council_member.try_push(sec_council_member.clone()).unwrap();
        active_council_member.try_push(third_council_member.clone()).unwrap();
        <ActiveCouncilMembers<T>>::put(active_council_member);
        let proposal = Proposal::RemoveExistingMember(third_council_member.clone());
        let votes = BoundedVec::try_from(vec![Voted(first_council_member)]).unwrap();
        <Proposals<T>>::insert(proposal, votes);
    }: _(RawOrigin::Signed(sec_council_member), third_council_member)

    claim_membership {
        let b in 1 .. 1000;
        let pending_council_member: T::AccountId = account("mem1", b, SEED);
        let mut pending_council_members = <PendingCouncilMembers<T>>::get();
        pending_council_members.try_push(pending_council_member.clone()).unwrap();
        <PendingCouncilMembers<T>>::put(pending_council_members);
    }: _(RawOrigin::Signed(pending_council_member))

    delete_transaction {
        let b in 1 .. 1000;
        let council_member: T::AccountId = account("mem1", b, SEED);
        let mut active_council_member = <ActiveCouncilMembers<T>>::get();
        active_council_member.try_push(council_member.clone()).unwrap();
        <ActiveCouncilMembers<T>>::put(active_council_member);
        // Add Pending Withdrawal
        let block_no: T::BlockNumber = 100u64.saturated_into();
        let pending_withdrawal = PendingWithdrawal::default();
        xcm_helper::Pallet::<T>::insert_pending_withdrawal(block_no, pending_withdrawal);
    }: _(RawOrigin::Signed(council_member), block_no, 0u32)
}