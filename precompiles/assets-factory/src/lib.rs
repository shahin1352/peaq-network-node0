// This file is part of Peaq.

// Copyright (C) 2019-2023 Peaq Technologies Pte.Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(test, feature(assert_matches))]

use fp_evm::PrecompileHandle;
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	sp_runtime::traits::StaticLookup,
	traits::{ConstU32, OriginTrait},
};

use pallet_evm::AddressMapping;
use peaq_primitives_xcm::EVMAddressToAssetId;
use precompile_utils::{
	prelude::{
		Address, BoundedBytes, InjectBacktrace, PrecompileHandleExt, RevertReason, RuntimeHelper,
		SYSTEM_ACCOUNT_SIZE,
	},
	solidity, EvmResult,
};
use sp_runtime::traits::Bounded;

use peaq_primitives_xcm::AssetIdExt;
use sp_core::{H160, U256};
use sp_std::{
	convert::{TryFrom, TryInto},
	marker::PhantomData,
	vec::Vec,
};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

type GetBytesLimit = ConstU32<{ 2u32.pow(16) }>;

/// Alias for the Balance type for the provided Runtime and Instance.
pub type BalanceOf<Runtime, Instance = ()> = <Runtime as pallet_assets::Config<Instance>>::Balance;

/// Alias for the Asset Id type for the provided Runtime and Instance.
pub type AssetIdOf<Runtime, Instance = ()> = <Runtime as pallet_assets::Config<Instance>>::AssetId;

/// Alias for the Asset Id Parametertype for the provided Runtime and Instance.
pub type AssetIdParameterOf<Runtime, Instance = ()> =
	<Runtime as pallet_assets::Config<Instance>>::AssetIdParameter;

pub struct AssetsFactoryPrecompile<Runtime, Instance: 'static = ()>(
	PhantomData<(Runtime, Instance)>,
);

#[precompile_utils::precompile]
impl<Runtime, Instance> AssetsFactoryPrecompile<Runtime, Instance>
where
	Instance: 'static,
	Runtime: pallet_assets::Config<Instance> + pallet_evm::Config + frame_system::Config,
	Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	Runtime::RuntimeCall: From<pallet_assets::Call<Runtime, Instance>>,
	<Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<Runtime::AccountId>>,
	BalanceOf<Runtime, Instance>: TryFrom<U256> + Into<U256> + solidity::Codec,
	AssetIdOf<Runtime, Instance>: TryFrom<u64> + AssetIdExt,
	AssetIdParameterOf<Runtime, Instance>: TryFrom<u64> + AssetIdExt,
	Runtime: EVMAddressToAssetId<AssetIdOf<Runtime, Instance>>,
	<<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin: OriginTrait,
{
	#[precompile::public("convertAssetIdToAddress(uint64)")]
	#[precompile::view]
	fn convert_asset_id_to_address(
		_handle: &mut impl PrecompileHandle,
		id: u64,
	) -> EvmResult<Address> {
		let asset_id = id
			.try_into()
			.map_err(|_| RevertReason::value_is_too_large("asset id").in_field("id"))?;

		Ok(Runtime::asset_id_to_address(asset_id).into())
	}

	#[precompile::public("create(uint64,address,uint128)")]
	fn create(
		handle: &mut impl PrecompileHandle,
		id: u64,
		admin: Address,
		min_balance: u128,
	) -> EvmResult {
		handle.record_log_costs_manual(3, 32)?;

		let admin: H160 = admin.into();
		let asset_id: AssetIdParameterOf<Runtime, Instance> = id
			.try_into()
			.map_err(|_| RevertReason::value_is_too_large("asset id").in_field("id"))?;

		if !asset_id.is_allow_to_create() {
			return Err(RevertReason::Custom("Invalid asset id".into()).into())
		}

		let min_balance: BalanceOf<Runtime, Instance> =
			min_balance.try_into().unwrap_or_else(|_| Bounded::max_value());

		// Build call with origin.
		{
			let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
			let admin = Runtime::AddressMapping::into_account_id(admin);

			// Dispatch call (if enough gas).
			RuntimeHelper::<Runtime>::try_dispatch(
				handle,
				Some(origin).into(),
				pallet_assets::Call::<Runtime, Instance>::create {
					id: asset_id,
					admin: Runtime::Lookup::unlookup(admin),
					min_balance,
				},
				SYSTEM_ACCOUNT_SIZE,
			)?;
		}

		Ok(())
	}

	#[precompile::public("setMetadata(uint64,bytes,bytes,uint8)")]
	fn set_metadata(
		handle: &mut impl PrecompileHandle,
		id: u64,
		name: BoundedBytes<GetBytesLimit>,
		symbol: BoundedBytes<GetBytesLimit>,
		decimals: u8,
	) -> EvmResult {
		handle.record_log_costs_manual(3, 32)?;

		let asset_id = id
			.try_into()
			.map_err(|_| RevertReason::value_is_too_large("asset id").in_field("id"))?;
		let name: Vec<_> = name.into();
		let symbol: Vec<_> = symbol.into();

		// Build call with origin.
		{
			let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);

			// Dispatch call (if enough gas).
			RuntimeHelper::<Runtime>::try_dispatch(
				handle,
				Some(origin).into(),
				pallet_assets::Call::<Runtime, Instance>::set_metadata {
					id: asset_id,
					name,
					symbol,
					decimals,
				},
				SYSTEM_ACCOUNT_SIZE,
			)?;
		}

		Ok(())
	}

	#[precompile::public("setMinBalance(uint64,uint128)")]
	fn set_min_balance(
		handle: &mut impl PrecompileHandle,
		id: u64,
		min_balance: u128,
	) -> EvmResult {
		handle.record_log_costs_manual(3, 32)?;

		let asset_id = id
			.try_into()
			.map_err(|_| RevertReason::value_is_too_large("asset id").in_field("id"))?;

		let min_balance: BalanceOf<Runtime, Instance> =
			min_balance.try_into().unwrap_or_else(|_| Bounded::max_value());

		// Build call with origin.
		{
			let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);

			// Dispatch call (if enough gas).
			RuntimeHelper::<Runtime>::try_dispatch(
				handle,
				Some(origin).into(),
				pallet_assets::Call::<Runtime, Instance>::set_min_balance {
					id: asset_id,
					min_balance,
				},
				SYSTEM_ACCOUNT_SIZE,
			)?;
		}

		Ok(())
	}

	#[precompile::public("setTeam(uint64,address,address,address)")]
	fn set_team(
		handle: &mut impl PrecompileHandle,
		id: u64,
		issuer: Address,
		admin: Address,
		freezer: Address,
	) -> EvmResult {
		handle.record_log_costs_manual(3, 32)?;

		let asset_id = id
			.try_into()
			.map_err(|_| RevertReason::value_is_too_large("asset id").in_field("id"))?;
		let issuer: H160 = issuer.into();
		let admin: H160 = admin.into();
		let freezer: H160 = freezer.into();

		// Build call with origin.
		{
			let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
			let issuer = Runtime::AddressMapping::into_account_id(issuer);
			let admin = Runtime::AddressMapping::into_account_id(admin);
			let freezer = Runtime::AddressMapping::into_account_id(freezer);

			// Dispatch call (if enough gas).
			RuntimeHelper::<Runtime>::try_dispatch(
				handle,
				Some(origin).into(),
				pallet_assets::Call::<Runtime, Instance>::set_team {
					id: asset_id,
					issuer: Runtime::Lookup::unlookup(issuer),
					admin: Runtime::Lookup::unlookup(admin),
					freezer: Runtime::Lookup::unlookup(freezer),
				},
				SYSTEM_ACCOUNT_SIZE,
			)?;
		}

		Ok(())
	}

	#[precompile::public("transferOwnership(uint64,address)")]
	fn transfer_ownership(
		handle: &mut impl PrecompileHandle,
		id: u64,
		owner: Address,
	) -> EvmResult {
		handle.record_log_costs_manual(3, 32)?;

		let asset_id = id
			.try_into()
			.map_err(|_| RevertReason::value_is_too_large("asset id").in_field("id"))?;
		let owner: H160 = owner.into();

		// Build call with origin.
		{
			let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
			let owner = Runtime::AddressMapping::into_account_id(owner);

			// Dispatch call (if enough gas).
			RuntimeHelper::<Runtime>::try_dispatch(
				handle,
				Some(origin).into(),
				pallet_assets::Call::<Runtime, Instance>::transfer_ownership {
					id: asset_id,
					owner: Runtime::Lookup::unlookup(owner),
				},
				SYSTEM_ACCOUNT_SIZE,
			)?;
		}

		Ok(())
	}
	#[precompile::public("startDestroy(uint64)")]
	fn start_destroy(handle: &mut impl PrecompileHandle, id: u64) -> EvmResult {
		handle.record_log_costs_manual(3, 32)?;

		let asset_id = id
			.try_into()
			.map_err(|_| RevertReason::value_is_too_large("asset id").in_field("id"))?;

		// Build call with origin.
		{
			let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);

			// Dispatch call (if enough gas).
			RuntimeHelper::<Runtime>::try_dispatch(
				handle,
				Some(origin).into(),
				pallet_assets::Call::<Runtime, Instance>::start_destroy { id: asset_id },
				SYSTEM_ACCOUNT_SIZE,
			)?;
		}

		Ok(())
	}

	#[precompile::public("finishDestroy(uint64)")]
	fn finish_destroy(handle: &mut impl PrecompileHandle, id: u64) -> EvmResult {
		handle.record_log_costs_manual(3, 32)?;

		let asset_id = id
			.try_into()
			.map_err(|_| RevertReason::value_is_too_large("asset id").in_field("id"))?;

		// Build call with origin.
		{
			let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);

			// Dispatch call (if enough gas).
			RuntimeHelper::<Runtime>::try_dispatch(
				handle,
				Some(origin).into(),
				pallet_assets::Call::<Runtime, Instance>::finish_destroy { id: asset_id },
				SYSTEM_ACCOUNT_SIZE,
			)?;
		}

		Ok(())
	}
}