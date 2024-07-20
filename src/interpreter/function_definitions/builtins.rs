use std::collections::HashMap;

use lazy_static::lazy_static;

use crate::interpreter::{
    function_definitions::{
        abi::{ABI_DECODE, ABI_DECODE_CALLDATA, ABI_ENCODE, ABI_ENCODE_PACKED},
        address::ADDRESS_BALANCE,
        block::{BLOCK_BASE_FEE, BLOCK_CHAIN_ID, BLOCK_NUMBER, BLOCK_TIMESTAMP},
        concat::{CONCAT_ARRAY, CONCAT_STRING},
        console::CONSOLE_LOG,
        format::{FORMAT_FUNCTION, NON_NUM_FORMAT, NUM_FORMAT},
        iterable::{ITER_LENGTH, ITER_MAP},
        misc::{GET_TYPE, KECCAK256, MAPPING_KEYS},
        numeric::{NUM_DIV, NUM_MUL, TYPE_MAX, TYPE_MIN},
        receipt::TX_GET_RECEIPT,
        repl::{
            REPL_ACCOUNT, REPL_DEBUG, REPL_EXEC, REPL_FETCH_ABI, REPL_IS_CONNECTED,
            REPL_LIST_LEDGER_WALLETS, REPL_LIST_TYPES, REPL_LIST_VARS, REPL_LOAD_ABI,
            REPL_LOAD_LEDGER, REPL_LOAD_PRIVATE_KEY, REPL_RPC,
        },
        FunctionDefinition,
    },
    types::NonParametricType,
};

lazy_static! {
    pub static ref FUNCTIONS: HashMap<String, FunctionDefinition> = {
        let mut m = HashMap::new();
        m.insert("format".to_string(), FORMAT_FUNCTION.clone());
        m.insert("keccak256".to_string(), KECCAK256.clone());
        m.insert("type".to_string(), GET_TYPE.clone());
        m
    };
    pub static ref INSTANCE_METHODS: HashMap<NonParametricType, HashMap<String, FunctionDefinition>> = {
        let mut m = HashMap::new();

        let mut string_methods = HashMap::new();
        string_methods.insert("concat".to_string(), CONCAT_STRING.clone());
        string_methods.insert("length".to_string(), ITER_LENGTH.clone());
        string_methods.insert("format".to_string(), NON_NUM_FORMAT.clone());
        m.insert(NonParametricType::String, string_methods);

        let mut array_methods = HashMap::new();
        array_methods.insert("length".to_string(), ITER_LENGTH.clone());
        array_methods.insert("map".to_string(), ITER_MAP.clone());
        array_methods.insert("concat".to_string(), CONCAT_ARRAY.clone());
        array_methods.insert("format".to_string(), NON_NUM_FORMAT.clone());
        m.insert(NonParametricType::Array, array_methods);

        let mut bytes_methods = HashMap::new();
        bytes_methods.insert("length".to_string(), ITER_LENGTH.clone());
        bytes_methods.insert("map".to_string(), ITER_MAP.clone());
        bytes_methods.insert("concat".to_string(), CONCAT_ARRAY.clone());
        bytes_methods.insert("format".to_string(), NON_NUM_FORMAT.clone());
        m.insert(NonParametricType::Bytes, bytes_methods);

        let mut fix_bytes_methods = HashMap::new();
        fix_bytes_methods.insert("format".to_string(), NON_NUM_FORMAT.clone());
        m.insert(NonParametricType::FixBytes, fix_bytes_methods);

        let mut num_methods = HashMap::new();
        num_methods.insert("format".to_string(), NUM_FORMAT.clone());
        num_methods.insert("mul".to_string(), NUM_MUL.clone());
        num_methods.insert("div".to_string(), NUM_DIV.clone());
        for types in [NonParametricType::Int, NonParametricType::Uint] {
            m.insert(types.clone(), num_methods.clone());
        }

        let mut addr_methods = HashMap::new();
        addr_methods.insert("format".to_string(), NON_NUM_FORMAT.clone());
        addr_methods.insert("balance".to_string(), ADDRESS_BALANCE.clone());
        m.insert(NonParametricType::Address, addr_methods);

        let mut transaction_methods = HashMap::new();
        transaction_methods.insert("format".to_string(), NON_NUM_FORMAT.clone());
        transaction_methods.insert("getReceipt".to_string(), TX_GET_RECEIPT.clone());
        m.insert(NonParametricType::Transaction, transaction_methods);

        let mut mapping_methods = HashMap::new();
        mapping_methods.insert("format".to_string(), NON_NUM_FORMAT.clone());
        mapping_methods.insert("keys".to_string(), MAPPING_KEYS.clone());
        m.insert(NonParametricType::Mapping, mapping_methods);

        m
    };
    pub static ref STATIC_METHODS: HashMap<NonParametricType, HashMap<String, FunctionDefinition>> = {
        let mut m = HashMap::new();

        let mut num_methods = HashMap::new();
        num_methods.insert("max".to_string(), TYPE_MAX.clone());
        num_methods.insert("min".to_string(), TYPE_MIN.clone());
        for type_ in [NonParametricType::Int, NonParametricType::Uint] {
            m.insert(type_.clone(), num_methods.clone());
        }

        let mut contract_methods = HashMap::new();
        contract_methods.insert("decode".to_string(), ABI_DECODE_CALLDATA.clone());
        m.insert(NonParametricType::Contract, contract_methods);

        let mut abi_methods = HashMap::new();
        abi_methods.insert("encode".to_string(), ABI_ENCODE.clone());
        abi_methods.insert("encodePacked".to_string(), ABI_ENCODE_PACKED.clone());
        abi_methods.insert("decode".to_string(), ABI_DECODE.clone());
        m.insert(NonParametricType::Abi, abi_methods);

        let mut block_methods = HashMap::new();
        block_methods.insert("chainid".to_string(), BLOCK_CHAIN_ID.clone());
        block_methods.insert("basefee".to_string(), BLOCK_BASE_FEE.clone());
        block_methods.insert("number".to_string(), BLOCK_NUMBER.clone());
        block_methods.insert("timestamp".to_string(), BLOCK_TIMESTAMP.clone());
        m.insert(NonParametricType::Block, block_methods);

        let mut console_methods = HashMap::new();
        console_methods.insert("log".to_string(), CONSOLE_LOG.clone());
        m.insert(NonParametricType::Console, console_methods);

        let mut repl_methods = HashMap::new();
        repl_methods.insert("vars".to_string(), REPL_LIST_VARS.clone());
        repl_methods.insert("types".to_string(), REPL_LIST_TYPES.clone());
        repl_methods.insert("connected".to_string(), REPL_IS_CONNECTED.clone());
        repl_methods.insert("rpc".to_string(), REPL_RPC.clone());
        repl_methods.insert("debug".to_string(), REPL_DEBUG.clone());
        repl_methods.insert("exec".to_string(), REPL_EXEC.clone());
        repl_methods.insert("loadAbi".to_string(), REPL_LOAD_ABI.clone());
        repl_methods.insert("fetchAbi".to_string(), REPL_FETCH_ABI.clone());
        repl_methods.insert("account".to_string(), REPL_ACCOUNT.clone());
        repl_methods.insert("loadPrivateKey".to_string(), REPL_LOAD_PRIVATE_KEY.clone());
        repl_methods.insert(
            "listLedgerWallets".to_string(),
            REPL_LIST_LEDGER_WALLETS.clone(),
        );
        repl_methods.insert("loadLedger".to_string(), REPL_LOAD_LEDGER.clone());
        m.insert(NonParametricType::Repl, repl_methods);

        m
    };
}
