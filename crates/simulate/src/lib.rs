pub mod execution;
pub mod price_simulation;

use bytes::Bytes;
use ethers::{
    abi::{Abi, Address, Tokenizable},
    prelude::BaseContract,
};
use revm::{
    db::{CacheDB, EmptyDB},
    primitives::{ExecutionResult, TransactTo, B160, U256},
    EVM,
};

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use bindings;
    use bytes::Bytes;
    use ethers::prelude::BaseContract;
    use revm::primitives::{ruint::Uint, ExecutionResult, Output, TransactTo, B160};

    use crate::execution::ExecutionManager;
    #[test]
    fn test_string_write() {
        // Set up the execution manager and a user address.
        let mut manager = ExecutionManager::new();
        let user_address = B160::from_str("0x0000000000000000000000000000000000000001").unwrap();

        // Get bytecode and abi for the writer contract.
        let writer_contract = BaseContract::from(bindings::writer::WRITER_ABI.clone());
        let writer_bytecode = Bytes::copy_from_slice(&bindings::writer::WRITER_BYTECODE).into();

        // Deploy the writer contract.
        manager.execute(
            user_address,
            writer_bytecode,
            TransactTo::create(),
            Uint::from(0),
        );

        let writer_contract_address = manager
            .evm
            .db()
            .unwrap()
            .clone()
            .accounts
            .into_iter()
            .nth(2)
            .unwrap()
            .0;

        // Generate calldata for the 'echoString' function
        let test_string = "Hello, world!";
        let input_arguments = test_string.to_string();
        let call_bytes = writer_contract.encode("echoString", input_arguments);
        let call_bytes = Bytes::from(hex::decode(hex::encode(call_bytes.unwrap())).unwrap());

        // Call the 'echoString' function.
        let result = manager.execute(
            user_address,
            call_bytes,
            TransactTo::Call(writer_contract_address),
            Uint::from(0),
        );

        // unpack output call enum into raw bytes
        let value = match result {
            ExecutionResult::Success { output, .. } => match output {
                Output::Call(value) => Some(value),
                Output::Create(_, Some(_)) => None,
                _ => None,
            },
            _ => None,
        };

        let response: String = writer_contract
            .decode_output("echoString", value.unwrap())
            .unwrap();

        println!("Minting Response: {response:#?}");
        assert_eq!(response, test_string);
    }
}
