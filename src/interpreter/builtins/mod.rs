use std::collections::HashMap;

use lazy_static::lazy_static;
mod types;

mod abi;
mod address;
mod block;
mod concat;
mod console;
mod format;
mod iterable;
mod misc;
mod numeric;
mod receipt;
mod repl;

pub use types::FunctionDefinition;
pub use types::FunctionParam;

use crate::interpreter::functions::FunctionCall;
use crate::interpreter::types::NonParametricType;
use crate::interpreter::Type;
use crate::interpreter::Value;

lazy_static! {
    pub static ref VALUES: HashMap<String, Value> = {
        let mut m = HashMap::new();

        m.insert("repl".to_string(), Value::TypeObject(Type::Repl));
        m.insert("console".to_string(), Value::TypeObject(Type::Console));
        m.insert("block".to_string(), Value::TypeObject(Type::Block));
        m.insert(
            "Transaction".to_string(),
            Value::TypeObject(Type::Transaction),
        );
        m.insert("abi".to_string(), Value::TypeObject(Type::Abi));

        let funcs: Vec<(&str, &FunctionDefinition)> = vec![
            ("format", &format::FORMAT_FUNCTION),
            ("keccak256", &misc::KECCAK256),
            ("type", &misc::GET_TYPE),
        ];
        for (name, func) in funcs {
            m.insert(name.to_string(), FunctionCall::function(func).into());
        }

        m
    };
    pub static ref INSTANCE_METHODS: HashMap<NonParametricType, HashMap<String, FunctionDefinition>> = {
        let mut m = HashMap::new();

        let mut string_methods = HashMap::new();
        string_methods.insert("concat".to_string(), concat::CONCAT_STRING.clone());
        string_methods.insert("length".to_string(), iterable::ITER_LENGTH.clone());
        string_methods.insert("format".to_string(), format::NON_NUM_FORMAT.clone());
        m.insert(NonParametricType::String, string_methods);

        let mut array_methods = HashMap::new();
        array_methods.insert("length".to_string(), iterable::ITER_LENGTH.clone());
        array_methods.insert("map".to_string(), iterable::ITER_MAP.clone());
        array_methods.insert("concat".to_string(), concat::CONCAT_ARRAY.clone());
        array_methods.insert("format".to_string(), format::NON_NUM_FORMAT.clone());
        m.insert(NonParametricType::Array, array_methods);

        let mut bytes_methods = HashMap::new();
        bytes_methods.insert("length".to_string(), iterable::ITER_LENGTH.clone());
        bytes_methods.insert("map".to_string(), iterable::ITER_MAP.clone());
        bytes_methods.insert("concat".to_string(), concat::CONCAT_ARRAY.clone());
        bytes_methods.insert("format".to_string(), format::NON_NUM_FORMAT.clone());
        m.insert(NonParametricType::Bytes, bytes_methods);

        let mut fix_bytes_methods = HashMap::new();
        fix_bytes_methods.insert("format".to_string(), format::NON_NUM_FORMAT.clone());
        m.insert(NonParametricType::FixBytes, fix_bytes_methods);

        let mut num_methods = HashMap::new();
        num_methods.insert("format".to_string(), format::NUM_FORMAT.clone());
        num_methods.insert("mul".to_string(), numeric::NUM_MUL.clone());
        num_methods.insert("div".to_string(), numeric::NUM_DIV.clone());
        for types in [NonParametricType::Int, NonParametricType::Uint] {
            m.insert(types.clone(), num_methods.clone());
        }

        let mut addr_methods = HashMap::new();
        addr_methods.insert("format".to_string(), format::NON_NUM_FORMAT.clone());
        addr_methods.insert("balance".to_string(), address::ADDRESS_BALANCE.clone());
        m.insert(NonParametricType::Address, addr_methods);

        let mut transaction_methods = HashMap::new();
        transaction_methods.insert("format".to_string(), format::NON_NUM_FORMAT.clone());
        transaction_methods.insert("getReceipt".to_string(), receipt::TX_GET_RECEIPT.clone());
        m.insert(NonParametricType::Transaction, transaction_methods);

        let mut mapping_methods = HashMap::new();
        mapping_methods.insert("format".to_string(), format::NON_NUM_FORMAT.clone());
        mapping_methods.insert("keys".to_string(), misc::MAPPING_KEYS.clone());
        m.insert(NonParametricType::Mapping, mapping_methods);

        m
    };
    pub static ref STATIC_METHODS: HashMap<NonParametricType, HashMap<String, FunctionDefinition>> = {
        let mut m = HashMap::new();

        let mut num_methods = HashMap::new();
        num_methods.insert("max".to_string(), numeric::TYPE_MAX.clone());
        num_methods.insert("min".to_string(), numeric::TYPE_MIN.clone());
        for type_ in [NonParametricType::Int, NonParametricType::Uint] {
            m.insert(type_.clone(), num_methods.clone());
        }

        let mut contract_methods = HashMap::new();
        contract_methods.insert("decode".to_string(), abi::ABI_DECODE_CALLDATA.clone());
        m.insert(NonParametricType::Contract, contract_methods);

        let mut abi_methods = HashMap::new();
        abi_methods.insert("encode".to_string(), abi::ABI_ENCODE.clone());
        abi_methods.insert("encodePacked".to_string(), abi::ABI_ENCODE_PACKED.clone());
        abi_methods.insert("decode".to_string(), abi::ABI_DECODE.clone());
        m.insert(NonParametricType::Abi, abi_methods);

        let mut block_methods = HashMap::new();
        block_methods.insert("chainid".to_string(), block::BLOCK_CHAIN_ID.clone());
        block_methods.insert("basefee".to_string(), block::BLOCK_BASE_FEE.clone());
        block_methods.insert("number".to_string(), block::BLOCK_NUMBER.clone());
        block_methods.insert("timestamp".to_string(), block::BLOCK_TIMESTAMP.clone());
        m.insert(NonParametricType::Block, block_methods);

        let mut console_methods = HashMap::new();
        console_methods.insert("log".to_string(), console::CONSOLE_LOG.clone());
        m.insert(NonParametricType::Console, console_methods);

        let mut repl_methods = HashMap::new();
        repl_methods.insert("vars".to_string(), repl::REPL_LIST_VARS.clone());
        repl_methods.insert("types".to_string(), repl::REPL_LIST_TYPES.clone());
        repl_methods.insert("connected".to_string(), repl::REPL_IS_CONNECTED.clone());
        repl_methods.insert("rpc".to_string(), repl::REPL_RPC.clone());
        repl_methods.insert("debug".to_string(), repl::REPL_DEBUG.clone());
        repl_methods.insert("exec".to_string(), repl::REPL_EXEC.clone());
        repl_methods.insert("loadAbi".to_string(), repl::REPL_LOAD_ABI.clone());
        repl_methods.insert("fetchAbi".to_string(), repl::REPL_FETCH_ABI.clone());
        repl_methods.insert("account".to_string(), repl::REPL_ACCOUNT.clone());
        repl_methods.insert(
            "loadPrivateKey".to_string(),
            repl::REPL_LOAD_PRIVATE_KEY.clone(),
        );
        repl_methods.insert(
            "listLedgerWallets".to_string(),
            repl::REPL_LIST_LEDGER_WALLETS.clone(),
        );
        repl_methods.insert("loadLedger".to_string(), repl::REPL_LOAD_LEDGER.clone());
        m.insert(NonParametricType::Repl, repl_methods);

        m
    };
}
