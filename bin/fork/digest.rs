use ethers::utils::hex;

use strum_macros::{Display, EnumString};

use super::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ForkConfig {
    pub output_path: Option<String>, //TODO: Provide default storage locations based on name of config/block number
    pub output_filename: String,
    pub provider: String,
    pub block_number: u64,
    pub contracts: HashMap<String, ContractData>,
}

impl ForkConfig {
    pub fn new(fork_config: &str) -> Result<Self, ConfigError> {
        let s = Config::builder()
            .add_source(config::File::with_name(fork_config))
            .build()?;
        s.try_deserialize()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ContractData {
    pub address: Address,
    pub artifacts_path: String,
    pub mappings: HashMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Artifacts {
    #[serde(rename = "storageLayout")]
    pub storage_layout: StorageLayout,
    // TODO: Add more here if we need them.
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StorageLayout {
    pub storage: Vec<StorageItem>,
    pub types: HashMap<String, StorageType>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StorageItem {
    #[serde(rename = "astId")]
    pub ast_id: usize,
    pub contract: String,
    pub label: String,
    pub offset: usize,
    pub slot: String,
    #[serde(rename = "type")]
    pub type_: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StorageType {
    // mapping has to come first so we attempt to deserialize that way first.
    Mapping {
        encoding: String,
        key: String,
        value: String,
        label: Option<String>,
        #[serde(rename = "numberOfBytes")]
        number_of_bytes: Option<String>,
    },
    Simple {
        encoding: String,
        label: String,
        #[serde(rename = "numberOfBytes")]
        number_of_bytes: String,
    },
}

/// Digests the config file and takes in an `EthersDB` so that the data can be fetched from the
/// blockchain.
/// Once all the `AccountInfo` for the contracts are fetched, we digest the contract artifacts to
/// get the storage layout.
pub fn digest_config(
    fork_config: &ForkConfig,
    ethers_db: &mut EthersDB<Provider<Http>>,
) -> Result<CacheDB<EmptyDB>, ConfigurationError> {
    let mut db = CacheDB::new(EmptyDB::default());
    for contract_data in fork_config.contracts.values() {
        let address = contract_data.address;
        let info = ethers_db.basic(address.into()).unwrap().unwrap();
        db.insert_account_info(address.into(), info);

        let artifacts = digest_artifacts(contract_data.artifacts_path.as_str()).unwrap();
        let storage_layout = artifacts.storage_layout;

        create_storage_layout(contract_data, storage_layout, &mut db, ethers_db).unwrap();
    }
    Ok(db)
}

pub fn digest_artifacts(path: &str) -> Result<Artifacts, ConfigurationError> {
    // Read the file to a string
    let data = fs::read_to_string(path)?;
    let json_data = serde_json::from_str(&data).unwrap();
    println!("json_data: {:#?}\n", json_data);

    Ok(json_data)
}

pub fn create_storage_layout(
    contract_data: &ContractData,
    storage_layout: StorageLayout,
    db: &mut CacheDB<EmptyDB>,
    ethers_db: &mut EthersDB<Provider<Http>>,
) -> Result<(), ConfigurationError> {
    println!("storage_layout.types: {:?}\n", storage_layout.types);
    for storage_item in storage_layout.storage {
        let label = storage_item.label;
        let slot = storage_item.slot;
        let slot_bytes =
            revm::primitives::U256::from_limbs(U256::from_str_radix(slot.as_str(), 10).unwrap().0);
        let storage = ethers_db
            .storage(contract_data.address.into(), slot_bytes)
            .unwrap();
        db.insert_account_storage(contract_data.address.into(), slot_bytes, storage)
            .unwrap();
        match storage_layout.types.get(&storage_item.type_).unwrap() {
            StorageType::Simple {
                encoding: _,
                label: _,
                number_of_bytes: _,
            } => {
                // Already got the storage slot above, so continue.
                continue;
            }
            StorageType::Mapping {
                encoding: _encoding,
                key,
                value,
                label: _label,
                number_of_bytes: _number_of_bytes,
            } => {
                // Catch the case where the value of a map is a map. We don't handle that yet.
                if let StorageType::Mapping { .. } =
                    storage_layout.types.get(&value.to_string()).unwrap()
                {
                    println!(
                        "Only handling one map deep for now. A map of a map was found and ignored."
                    );
                    continue;
                }
                // We got a one-deep mapping, so we need to get the keys and values from the config and properly pad everything to get the storage slot.
                let key_bytes = match storage_layout.types.get(&key.to_string()).unwrap() {
                    StorageType::Simple {
                        encoding: _,
                        label: _,
                        number_of_bytes,
                    } => number_of_bytes.parse::<usize>().unwrap(),
                    StorageType::Mapping { .. } => {
                        println!(
                            "Only handling one map deep for now. A map of a map was found and ignored."
                        );
                        continue;
                    }
                };
                if let StorageType::Mapping { .. } =
                    storage_layout.types.get(&value.to_string()).unwrap()
                {
                    println!(
                        "Only handling one map deep for now. A map of a map was found and ignored."
                    );
                    continue;
                }

                if let Some(keys) = contract_data.mappings.get(&label) {
                    for key in keys {
                        println!("looping through keys");
                        let mut padded_key_bytes = vec![0; 32 - key_bytes];
                        let key_bytes = hex::decode(key).unwrap();
                        padded_key_bytes.extend(key_bytes.clone());
                        let to_hash: Vec<u8> = padded_key_bytes
                            .into_iter()
                            .chain(slot_bytes.to_be_bytes_vec())
                            .collect();
                        let slot_to_get = keccak256(to_hash);
                        let storage = ethers_db
                            .storage(
                                contract_data.address.into(),
                                revm::primitives::U256::from_be_bytes(slot_to_get),
                            )
                            .unwrap();
                        println!("storage: {:?}", storage);
                        db.insert_account_storage(
                            contract_data.address.into(),
                            revm::primitives::U256::from_be_bytes(slot_to_get),
                            storage,
                        )
                        .unwrap();
                    }
                }
            }
        }
    }
    Ok(())
}

#[derive(Debug, Display, EnumString, Serialize, Deserialize)]
pub enum BasicType {
    #[strum(serialize = "t_address")]
    Address {
        encoding: String,
        label: String,
        #[serde(rename = "numberOfBytes")]
        number_of_bytes: String,
    },
    #[strum(serialize = "t_uint256")]
    UInt256 {
        encoding: String,
        label: String,
        #[serde(rename = "numberOfBytes")]
        number_of_bytes: String,
    },
    #[strum(serialize = "t_string_storage")]
    String {
        encoding: String,
        label: String,
        #[serde(rename = "numberOfBytes")]
        number_of_bytes: String,
    },
    Mapping {
        encoding: String,
        key: String,
        value: String,
        label: Option<String>,
        #[serde(rename = "numberOfBytes")]
        number_of_bytes: Option<String>,
    },
}
