use core::option::OptionTrait;
use core::traits::TryInto;
#[starknet::contract(account)]

// A dummy account contract with faulty validations.

mod Account {
    use array::{ArrayTrait, SpanTrait};
    use box::BoxTrait;
    use traits::TryInto;
    use option::{Option, OptionTrait};

    use starknet::{ContractAddress, call_contract_syscall, contract_address_try_from_felt252,
        get_execution_info, get_tx_info, info::SyscallResultTrait, send_message_to_l1_syscall,
        syscalls::get_block_hash_syscall, TxInfo};

    // Validate Scenarios.

    // Run the validate method with no issues.
    const VALID: felt252 = 0;
    // Logic failure.
    const INVALID: felt252 = 1;
    // Make a contract call.
    const CALL_CONTRACT: felt252 = 2;
    // Use get_block_hash syscall.
    const GET_BLOCK_HASH: felt252 = 3;
    // Use get_execution_info syscall.
    const GET_EXECUTION_INFO: felt252 = 4;
    // Write to the storage.
    const STORAGE_WRITE: felt252 = 8;

    // get_selector_from_name('foo').
    const FOO_ENTRY_POINT_SELECTOR: felt252 = (
        0x1b1a0649752af1b28b3dc29a1556eee781e4a4c3a1f7f53f90fa834de098c4d
    );

    const STORAGE_WRITE_KEY: felt252 = 15;

    #[storage]
    struct Storage {
    }

    #[external(v0)]
    fn __validate_declare__(self: @ContractState, class_hash: felt252) -> felt252 {
        faulty_validate()
    }

    #[external(v0)]
    fn __validate_deploy__(
        self: @ContractState,
        class_hash: felt252,
        contract_address_salt: felt252,
        validate_constructor: bool
    ) -> felt252 {

        if (validate_constructor == false) {
            return faulty_validate();
        }

        starknet::VALIDATED
    }

    #[external(v0)]
    fn __validate__(
        self: @ContractState,
        contract_address: ContractAddress,
        selector: felt252,
        calldata: Array<felt252>
    ) -> felt252 {
        let to_address = 0;
        // By calling the `send_message_to_l1` function in validation and execution, tests can now verify
        // the functionality of entry point counters.
        send_message_to_l1_syscall(
            to_address: to_address,
            payload: calldata.span()
        ).unwrap_syscall();
        faulty_validate()
    }

    #[external(v0)]
    fn __execute__(
        self: @ContractState,
        contract_address: ContractAddress,
        selector: felt252,
        calldata: Array<felt252>
    ) -> felt252 {
        let tx_info = starknet::get_tx_info().unbox();
        let signature = tx_info.signature;
        let scenario = *signature[0_u32];

        if (scenario == STORAGE_WRITE) {
            let key = STORAGE_WRITE_KEY.try_into().unwrap();
            let value: felt252 = *signature[2_u32];
            starknet::syscalls::storage_write_syscall(0, key, value).unwrap_syscall();

            return starknet::VALIDATED;
        }

        let to_address = 0;

        send_message_to_l1_syscall(
            to_address: to_address,
            payload: calldata.span()
        ).unwrap_syscall();

        starknet::VALIDATED
    }

    #[constructor]
    fn constructor(ref self: ContractState, validate_constructor: bool) {
        if (validate_constructor == true) {
            faulty_validate();
        }
    }

    #[external(v0)]
    fn foo(self: @ContractState) {}

    fn faulty_validate() -> felt252 {
        let tx_info = starknet::get_tx_info().unbox();
        let signature = tx_info.signature;
        let scenario = *signature[0_u32];

        if (scenario == VALID) {
            return starknet::VALIDATED;
        }
        if (scenario == INVALID) {
            assert (0 == 1, 'Invalid scenario');
            return 'INVALID';
        }
        if (scenario == CALL_CONTRACT) {
            let contract_address: felt252 = *signature[1_u32];
            let mut calldata: Array<felt252> = Default::default();
            call_contract_syscall(
                address: contract_address_try_from_felt252(contract_address).unwrap(),
                entry_point_selector: FOO_ENTRY_POINT_SELECTOR,
                calldata: calldata.span()
            )
                .unwrap_syscall();
            return starknet::VALIDATED;
        }
        if (scenario == GET_BLOCK_HASH){
            let block_number: u64 = 1992;
            get_block_hash_syscall(block_number).unwrap_syscall();
            return starknet::VALIDATED;
        }

        if (scenario == STORAGE_WRITE) {
            let key = STORAGE_WRITE_KEY.try_into().unwrap();
            let value: felt252 = *signature[1_u32];
            starknet::syscalls::storage_write_syscall(0, key, value).unwrap_syscall();
            return starknet::VALIDATED;
        }

        assert (scenario == GET_EXECUTION_INFO, 'Unknown scenario');

        let block_number: felt252 = *signature[1_u32];
        let block_timestamp: felt252 = *signature[2_u32];
        let sequencer_address: felt252 = *signature[3_u32];

        let execution_info = starknet::get_execution_info().unbox();
        let block_info = execution_info.block_info.unbox();
        assert(block_info.block_number.into() == block_number, 'BLOCK_NUMBER_MISMATCH');
        assert(block_info.block_timestamp.into() == block_timestamp, 'BLOCK_TIMESTAMP_MISMATCH');
        assert(block_info.sequencer_address.into() == sequencer_address, 'SEQUENCER_MISMATCH');

        starknet::VALIDATED
    }
}
