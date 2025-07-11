// A dummy account contract with faulty validations.

%lang starknet

from starkware.cairo.common.alloc import alloc
from starkware.cairo.common.bool import FALSE, TRUE
from starkware.cairo.common.cairo_builtins import HashBuiltin
from starkware.starknet.common.syscalls import (
    TxInfo,
    call_contract,
    get_block_number,
    get_block_timestamp,
    get_sequencer_address,
    get_tx_info,
    storage_write
)
from starkware.starknet.common.messages import send_message_to_l1

// Validate Scenarios.

// Run the validate method with no issues.
const VALID = 0;
// Logic failure.
const INVALID = 1;
// Make a contract call.
const CALL_CONTRACT = 2;
// Use get_block_number syscall.
const GET_BLOCK_NUMBER = 5;
// Use get_block_timestamp syscall.
const GET_BLOCK_TIMESTAMP = 6;
// Use get_sequencer_address syscall.
const GET_SEQUENCER_ADDRESS = 7;
// Write to the storage.
const STORAGE_WRITE = 8;

// get_selector_from_name('foo').
const FOO_ENTRY_POINT_SELECTOR = (
    0x1b1a0649752af1b28b3dc29a1556eee781e4a4c3a1f7f53f90fa834de098c4d);

const STORAGE_WRITE_KEY = 15;

@external
func __validate_declare__{syscall_ptr: felt*}(class_hash: felt) {
    faulty_validate();
    return ();
}

@external
func __validate_deploy__{syscall_ptr: felt*}(
    class_hash: felt, contract_address_salt: felt, validate_constructor: felt
) {
    if (validate_constructor == FALSE) {
        faulty_validate();
        return ();
    }

    return ();
}

@external
func __validate__{syscall_ptr: felt*}(
    contract_address: felt, selector: felt, calldata_len: felt, calldata: felt*
) {
    let to_address = 0;
    // By calling the `send_message_to_l1` function in validation and execution, tests can now verify
    // the functionality of entry point counters.
    send_message_to_l1(to_address, calldata_len, calldata);
    faulty_validate();
    return ();
}

@external
func __execute__{syscall_ptr: felt*, pedersen_ptr: HashBuiltin*, range_check_ptr}(
    contract_address: felt, selector: felt, calldata_len: felt, calldata: felt*
) {
    let (tx_info: TxInfo*) = get_tx_info();
    let scenario = tx_info.signature[0];
    if (scenario == STORAGE_WRITE) {
        let value = tx_info.signature[2];
        storage_write(address=STORAGE_WRITE_KEY, value=value);
        return ();
    }

    let to_address = 0;

    send_message_to_l1(to_address, calldata_len, calldata);
    return ();
}

@constructor
func constructor{syscall_ptr: felt*, pedersen_ptr: HashBuiltin*, range_check_ptr}(
    validate_constructor: felt
) {
    if (validate_constructor == TRUE) {
        faulty_validate();
        return ();
    }

    return ();
}

func faulty_validate{syscall_ptr: felt*}() {
    let (tx_info: TxInfo*) = get_tx_info();
    let scenario = tx_info.signature[0];

    if (scenario == VALID) {
        return ();
    }
    if (scenario == INVALID) {
        assert 0 = 1;
        return ();
    }
    if (scenario == CALL_CONTRACT) {
        let contract_address = tx_info.signature[1];
        let (calldata: felt*) = alloc();
        call_contract(
            contract_address=contract_address,
            function_selector=FOO_ENTRY_POINT_SELECTOR,
            calldata_size=0,
            calldata=calldata,
        );
        return ();
    }
    if (scenario == GET_BLOCK_NUMBER) {
        let expected_block_number = tx_info.signature[1];
        let (block_number) = get_block_number();
        assert block_number = expected_block_number;
        return ();
    }
    if (scenario == GET_BLOCK_TIMESTAMP) {
        let expected_block_timestamp = tx_info.signature[1];
        let (block_timestamp) = get_block_timestamp();
        assert block_timestamp = expected_block_timestamp;
        return ();
    }
    if (scenario == STORAGE_WRITE) {
        let value = tx_info.signature[1];
        storage_write(address=STORAGE_WRITE_KEY, value=value);
        return ();
    }

    assert scenario = GET_SEQUENCER_ADDRESS;
    let sequencer_address = get_sequencer_address();
    return ();
}

@external
func foo() {
    return ();
}
