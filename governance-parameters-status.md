# Governance & Parameters System Status Report

## Current State of Affairs

### Parameters Pallet Issues
The current d9-parameters pallet may need to be **GUTTED** and redesigned. Key issues:
1. Contains signed integer types (i32) which should be removed - this is a strictly no_std environment with unsigned integers only
2. The parameter types currently defined are:
   - U8
   - U32
   - U64
   - U128
   - I32 (NEEDS REMOVAL)

### Governance Pallet Architecture Issues
The governance pallet currently has parameter changing logic that should NOT be there:
- `execute_parameter_change()` function should be removed
- Governance should ONLY handle voting and proposal management
- Actual execution of changes should be handled by the parameters pallet or the target pallets themselves

The governance pallet needs to be shifted to be more geared towards **contract governance** rather than runtime parameter changes.

## **ATTENTION: Core Priority - Transaction Fees**
The most critical parameter that needs governance control is **transaction fees**. 
- The Claude app already has logic for fee management that needs to be transferred here
- This should be the primary focus for the parameter system

## Survey of Current Runtime Parameters

### Single Value Parameters in Pallets (StorageValue items):

1. **d9-node-voting**
   - `PalletAdmin`: T::AccountId
   - `CurrentSessionIndex`: SessionIndex
   - `CurrentNumberOfCandidatesNodes`: u32

2. **d9-node-rewards**
   - `NodeRewardContract`: T::AccountId
   - `PalletAdmin`: T::AccountId

3. **d9-council-lock**
   - `PalletAdmin`: T::AccountId
   - `MiningPoolContract`: T::AccountId
   - `ProposalFee`: BalanceOf<T>
   - `NextProposalIndex`: ProposalIndex (u32)

4. **d9-punishment**
   - No single value parameters found

5. **d9-validator-oversight**
   - No single value parameters found

6. **d9-node-liveness**
   - `PalletAdmin`: T::AccountId

7. **d9-candidate-registry**
   - `PalletAdmin`: T::AccountId

8. **d9-governance**
   - `NextTemplateId`: u32
   - `NextProposalId`: u32
   - `GovernanceAdmin`: T::AccountId

9. **d9-parameters**
   - Uses StorageMap for parameters, not StorageValue

### Type Analysis
Most parameters are:
- **AccountId** types (for admin/contract addresses)
- **u32** for counters/indices
- **BalanceOf<T>** for fees (which is typically u128)

## TODO: Contract Parameter Analysis
Need to run Claude analysis on the smart contracts to identify:
- What parameters exist in contracts that need governance
- Current parameter types used in contracts
- How contracts currently handle parameter updates

## Recommended Actions

1. **Redesign Parameters Pallet**
   - Remove all signed integer support
   - Focus on unsigned types only (u8, u32, u64, u128)
   - Ensure strict no_std compliance

2. **Refactor Governance Pallet**
   - Remove parameter execution logic
   - Focus on proposal creation, voting, and approval
   - Let parameters pallet or individual pallets handle execution

3. **Priority: Fee Governance**
   - Transfer fee management logic from Claude app
   - Implement fee parameter control as first use case
   - Test with transaction fee adjustments

4. **Contract Integration**
   - Analyze contract parameters
   - Design contract call templates for parameter updates
   - Ensure governance can manage contract parameters effectively

## Build Status
- ✅ d9-parameters: Builds (but needs redesign)
- ✅ d9-governance: Builds (but needs refactoring)
- ✅ d9-node-voting: Builds with governance integration
- ✅ d9-node-rewards: Builds with governance integration

## Note on MaxEncodedLen Issue
The governance pallet storage items need `MaxEncodedLen` derives added to:
- `ContractCallTemplate`
- `ProposalType`
- `Proposal`

This is blocking proper storage but can be fixed by adding the derive macro.