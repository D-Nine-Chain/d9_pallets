// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Treasury pallet benchmarking.

#![cfg(feature = "runtime-benchmarks")]

use super::{ Pallet as Treasury, * };

use frame_benchmarking::v1::{ account, benchmarks_instance_pallet, BenchmarkError };
use frame_support::{
	dispatch::UnfilteredDispatchable,
	ensure,
	traits::{ EnsureOrigin, OnInitialize },
};
use frame_system::RawOrigin;

benchmarks! {
    new_treasurer {
        let caller: T::AccountId = whitelisted_caller();
        let new_treasurer: T::AccountId = account("new_treasurer", 0, 0);
        Treasurer::<T>::put(caller.clone());
    }: _(RawOrigin::Signed(caller), new_treasurer)
    verify {
        ensure!(Treasurer::<T>::get() == Some(new_treasurer), "new_treasurer not set correctly");
    }
}
