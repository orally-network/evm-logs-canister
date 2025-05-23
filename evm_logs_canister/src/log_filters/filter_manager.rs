use std::{
  collections::HashMap,
  hash::{Hash, Hasher},
};

use candid::{CandidType, Nat};
use evm_logs_types::Filter;
use evm_rpc_types::{Hex20, Hex32};
use serde::{Deserialize, Serialize};

use crate::internals::hex_types::{WrappedHex20, WrappedHex32};

/// Data structure for a specific chain (chain_id).
/// It stores:
///   - addresses: a map of "address -> counter",
///   - first_position_topics: a map of "topic -> counter" (only from the first position),
#[derive(Debug, Default, Deserialize, CandidType, Clone)]
struct PerChainData {
  addresses: HashMap<WrappedHex20, Nat>,
  first_position_topics: HashMap<WrappedHex32, Nat>,
}

/// A main FilterManager that stores PerChainData for each `chain_id`.
#[derive(Debug, Default, Deserialize, CandidType, Clone)]
pub struct FilterManager {
  chain_data: HashMap<u32, PerChainData>,
}

impl FilterManager {
  /// Helper: get (or create if missing) a mutable reference to PerChainData for a given chain.
  fn get_chain_data_mut(&mut self, chain_id: u32) -> &mut PerChainData {
    self.chain_data.entry(chain_id).or_default()
  }

  /// Helper: get an immutable reference to PerChainData for a given chain, if it exists.
  fn get_chain_data(&self, chain_id: u32) -> Option<&PerChainData> {
    self.chain_data.get(&chain_id)
  }

  /// Adds a new filter (subscription) to the manager for a specific `chain_id`.
  /// Only the address and topics from the first position are stored.
  pub fn add_filter(&mut self, chain_id: u32, filter: &Filter) {
    let chain_data = self.get_chain_data_mut(chain_id);

    // Increment the counter for the address
    *chain_data
      .addresses
      .entry(WrappedHex20::from(&filter.address))
      .or_insert_with(|| Nat::from(0u32)) += Nat::from(1u32);

    // If filter.topics exists and is not empty, take the first position only
    if let Some(all_positions) = &filter.topics {
      if !all_positions.is_empty() {
        let first_position = &all_positions[0];
        // Increment counters for each topic in the first position
        for topic in first_position {
          *chain_data
            .first_position_topics
            .entry(WrappedHex32::from(topic))
            .or_insert_with(|| Nat::from(0u32)) += Nat::from(1u32);
        }
      }
    }
  }

  /// Removes a filter (subscription) from the manager for a specific `chain_id`.
  /// Decrements counters for the address and topics in the first position.
  pub fn remove_filter(&mut self, chain_id: u32, filter: &Filter) {
    if let Some(chain_data) = self.chain_data.get_mut(&chain_id) {
      // Decrement address counter
      if let Some(addr_count) = chain_data.addresses.get_mut(&WrappedHex20::from(&filter.address)) {
        if *addr_count > 0u32 {
          *addr_count -= 1u32;
          if *addr_count == 0u32 {
            chain_data.addresses.remove(&WrappedHex20::from(&filter.address));
          }
        }
      }

      // Decrement topic counters (from the first position) if they exist
      if let Some(all_positions) = &filter.topics {
        if !all_positions.is_empty() {
          let first_position = &all_positions[0];
          for topic in first_position {
            if let Some(topic_count) = chain_data.first_position_topics.get_mut(&WrappedHex32::from(topic)) {
              if *topic_count > 0u32 {
                *topic_count -= 1u32;
                if *topic_count == 0u32 {
                  chain_data.first_position_topics.remove(&WrappedHex32::from(topic));
                }
              }
            }
          }
        }
      }
    }
  }

  /// Returns a tuple of `(addresses, topics)`.
  /// - `addresses`: all active addresses
  /// - `topics`: if the first_position_topics is empty => None
  ///   otherwise => Some([ list_of_topics ])
  pub fn get_active_addresses_and_topics(&self, chain_id: u32) -> (Vec<Hex20>, Option<Vec<Vec<Hex32>>>) {
    if let Some(chain_data) = self.get_chain_data(chain_id) {
      // Gather addresses
      let addresses = chain_data.addresses.keys().cloned().map(|x| x.0).collect::<Vec<_>>();

      // Gather topics from the first position
      let topics_collected = chain_data
        .first_position_topics
        .keys()
        .cloned()
        .map(|x| x.0)
        .collect::<Vec<_>>();

      // If we have no topics, return None; otherwise wrap them in a single Vec
      let topics = if topics_collected.is_empty() {
        None
      } else {
        Some(vec![topics_collected])
      };

      (addresses, topics)
    } else {
      (Vec::new(), None)
    }
  }
}

#[cfg(test)]
mod tests {
  use std::str::FromStr;

  use anyhow::anyhow;
  use evm_logs_types::{Filter, TopicsPosition};
  use evm_rpc_types::{Hex20, Hex32};

  use super::*;

  /// Helper function to create a Filter with a given address and optional topics.
  /// We'll keep it simple: `topics` can be a Vec<Vec<&str>>, which we convert to String.
  fn create_filter(address: &str, topics: Option<Vec<Vec<&str>>>) -> Filter {
    Filter {
      address: Hex20::from_str(address).unwrap(),
      topics: topics.map(|ts| {
        ts.into_iter()
          .map(|topic_set| {
            topic_set
              .into_iter()
              .filter_map(|s| Hex32::from_str(s).ok())
              .collect::<TopicsPosition>()
          })
          .collect()
      }),
    }
  }

  const ADDR1_HEX20: &str = "0xd42AcA6E135D1dae6317e776F7EB96Eb91b8eb91";
  const ADDR2_HEX20: &str = "0xDA2efffa45cf5D960209aA0921Cf42a4a2a085cf";
  const ADDR3_HEX20: &str = "0x4838B106FCe9647Bdf1E7877BF73cE8B0BAD5f97";
  const ADDR4_HEX20: &str = "0xa2DA709980Effc0fA9413efEc7f5F86a849eCb93";
  const TOPIC1_HEX32: &str = "0x895950522ad88866c40789c503f088b8d88beb8de46cbb2c2329b3e968460492";
  const TOPIC2_HEX32: &str = "0xa213165f23ed89a7627d7a82973d251d654614d33104f7aabd1ed4d40ecebbea";
  const TOPIC3_HEX32: &str = "0xf8bbdcc71146cb4ca500980f5a60a18d7ce1860f1d22f08a2d25f3dbb202e42e";
  const TOPIC4_HEX32: &str = "0x13b2fce0c601a939e04b82c993f795d4df1335747782cacdaff2d3b71b576002";

  fn hex20_from_str(value: &str) -> anyhow::Result<Hex20> {
    Hex20::from_str(value).map_err(|e| anyhow!("Failed to convert Hex20 from string: {e}"))
  }

  fn hex32_from_str(value: &str) -> anyhow::Result<Hex32> {
    Hex32::from_str(value).map_err(|e| anyhow!("Failed to convert Hex32 from string: {e}"))
  }

  #[test]
  fn test_add_single_filter_with_first_position_topics() -> anyhow::Result<()> {
    let mut manager = FilterManager::default();

    // Create a filter with one address and some topics in the first position
    let filter = create_filter(
      ADDR1_HEX20,
      Some(vec![
        vec![TOPIC1_HEX32, TOPIC2_HEX32], // first position
        vec![TOPIC3_HEX32],               // second position (ignored)
      ]),
    );

    // Add the filter
    manager.add_filter(1, &filter);

    // Now retrieve addresses and topics for chain_id=1
    let (addresses, topics) = manager.get_active_addresses_and_topics(1);

    // We expect to see "0xAddress1" in addresses
    assert_eq!(addresses, vec![hex20_from_str(ADDR1_HEX20)?]);
    // The second part is Some(...) because we do have first-position topics
    // specifically "TopicA" and "TopicB".
    let unwrapped = topics.expect("Should have some topics");
    assert_eq!(unwrapped.len(), 1, "We only store first position in one vector");
    // Inside that vector, we expect "TopicA" and "TopicB"
    // The order in a HashMap-based scenario is not guaranteed, so let's just check they exist
    let first_pos = &unwrapped[0];
    assert!(first_pos.contains(&hex32_from_str(TOPIC1_HEX32)?));
    assert!(first_pos.contains(&hex32_from_str(TOPIC2_HEX32)?));
    Ok(())
  }

  #[test]
  fn test_remove_filter_clears_data() -> anyhow::Result<()> {
    let mut manager = FilterManager::default();

    // Create a filter with a single address and a single topic in the first position
    let filter = create_filter(ADDR2_HEX20, Some(vec![vec![TOPIC1_HEX32]]));

    // Add the filter
    manager.add_filter(1, &filter);
    // Check that addresses and topics are present
    let (addresses, topics) = manager.get_active_addresses_and_topics(1);
    assert_eq!(addresses, vec![hex20_from_str(ADDR2_HEX20)?]);
    assert!(topics.is_some());

    // Now remove the filter
    manager.remove_filter(1, &filter);

    // After removing, we expect the manager to have no addresses or topics for chain 1
    let (addresses_after, topics_after) = manager.get_active_addresses_and_topics(1);
    assert!(addresses_after.is_empty());
    assert!(topics_after.is_none());
    Ok(())
  }

  #[test]
  fn test_add_multiple_filters_different_addresses() -> anyhow::Result<()> {
    let mut manager = FilterManager::default();

    let filter1 = create_filter(ADDR1_HEX20, Some(vec![vec![TOPIC1_HEX32, TOPIC2_HEX32]]));
    let filter2 = create_filter(ADDR2_HEX20, Some(vec![vec![TOPIC2_HEX32, TOPIC3_HEX32]]));
    // Filter with no topics => it won't contribute to first_position_topics
    let filter3 = create_filter(ADDR3_HEX20, None);

    // Add them for chain_id=1
    manager.add_filter(1, &filter1);
    manager.add_filter(1, &filter2);
    manager.add_filter(1, &filter3);

    // Now gather
    let (addresses, topics) = manager.get_active_addresses_and_topics(1);

    // We expect addresses: 0xAddrA, 0xAddrB, 0xAddrC
    assert_eq!(addresses.len(), 3);
    assert!(addresses.contains(&hex20_from_str(ADDR1_HEX20)?));
    assert!(addresses.contains(&hex20_from_str(ADDR2_HEX20)?));
    assert!(addresses.contains(&hex20_from_str(ADDR3_HEX20)?));

    // For topics, from the first position:
    // - filter1 contributed T1, T2
    // - filter2 contributed T2, T3
    // - filter3 contributed nothing
    // So we expect T1, T2, T3
    let some_topics = topics.expect("We should have topics from filter1 & filter2");
    assert_eq!(some_topics.len(), 1); // one vector
    let tvec = &some_topics[0];
    assert!(tvec.contains(&hex32_from_str(TOPIC1_HEX32)?));
    assert!(tvec.contains(&hex32_from_str(TOPIC2_HEX32)?));
    assert!(tvec.contains(&hex32_from_str(TOPIC3_HEX32)?));
    Ok(())
  }

  #[test]
  fn test_add_and_remove_interleaved() -> anyhow::Result<()> {
    let mut manager = FilterManager::default();

    let filter1 = create_filter(ADDR1_HEX20, Some(vec![vec![TOPIC1_HEX32]]));
    let filter2 = create_filter(ADDR2_HEX20, Some(vec![vec![TOPIC2_HEX32]]));
    let filter3 = create_filter(ADDR1_HEX20, Some(vec![vec![TOPIC3_HEX32]]));

    // Add filter1 and filter2 on chain 5
    manager.add_filter(5, &filter1);
    manager.add_filter(5, &filter2);

    // addresses => [0xAddrA, 0xAddrB], topics => X, Y
    let (addr_1, topics_1) = manager.get_active_addresses_and_topics(5);
    assert_eq!(addr_1.len(), 2);
    let unwrapped_1 = topics_1.unwrap();
    assert_eq!(unwrapped_1[0].len(), 2);

    // Now remove filter1
    manager.remove_filter(5, &filter1);

    // addresses => [0xAddrB], topics => Y only
    let (addr_2, topics_2) = manager.get_active_addresses_and_topics(5);
    assert_eq!(addr_2, vec![hex20_from_str(ADDR2_HEX20)?]);
    let unwrapped_2 = topics_2.unwrap();
    assert_eq!(unwrapped_2[0], vec![hex32_from_str(TOPIC2_HEX32)?]);

    // Add filter3, which is also 0xAddrA but with topic Z
    manager.add_filter(5, &filter3);

    // addresses => [0xAddrB, 0xAddrA], topics => Y, Z
    let (addr_3, topics_3) = manager.get_active_addresses_and_topics(5);
    assert_eq!(addr_3.len(), 2);
    assert!(addr_3.contains(&hex20_from_str(ADDR2_HEX20)?));
    assert!(addr_3.contains(&hex20_from_str(ADDR1_HEX20)?));
    let unwrapped_3 = topics_3.unwrap();
    // We expect Y, Z in some order
    let t3 = &unwrapped_3[0];
    assert_eq!(t3.len(), 2);
    assert!(t3.contains(&hex32_from_str(TOPIC2_HEX32)?));
    assert!(t3.contains(&hex32_from_str(TOPIC3_HEX32)?));
    Ok(())
  }

  #[test]
  fn test_no_filters_for_chain() {
    let manager = FilterManager::default();
    // chain_id=99 has no filters
    let (addresses, topics) = manager.get_active_addresses_and_topics(99);
    assert!(addresses.is_empty());
    assert!(topics.is_none());
  }
}
