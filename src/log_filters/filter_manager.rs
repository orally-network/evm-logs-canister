use candid::Nat;
use std::collections::HashMap;
use evm_logs_types::Filter;

/// Data structure for a specific chain (chain_id).
/// It stores:
///   - addresses: a map of "address -> counter",
///   - first_position_topics: a map of "topic -> counter" (only from the first position),
#[derive(Debug, Default)]
struct PerChainData {
    addresses: HashMap<String, Nat>,
    first_position_topics: HashMap<String, Nat>,
}

impl PerChainData {
    fn new() -> Self {
        Self {
            addresses: HashMap::new(),
            first_position_topics: HashMap::new(),
        }
    }
}

/// A main FilterManager that stores PerChainData for each `chain_id`.
pub struct FilterManager {
    chain_data: HashMap<u32, PerChainData>,
}

impl FilterManager {
    /// Creates a new, empty manager.
    pub fn new() -> Self {
        Self {
            chain_data: HashMap::new(),
        }
    }

    /// Helper: get (or create if missing) a mutable reference to PerChainData for a given chain.
    fn get_chain_data_mut(&mut self, chain_id: u32) -> &mut PerChainData {
        self.chain_data
            .entry(chain_id)
            .or_insert_with(PerChainData::new)
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
            .entry(filter.address.clone())
            .or_insert_with(|| Nat::from(0u32)) += Nat::from(1u32);

        // If filter.topics exists and is not empty, take the first position only
        if let Some(all_positions) = &filter.topics {
            if !all_positions.is_empty() {
                let first_position = &all_positions[0];
                // Increment counters for each topic in the first position
                for topic in first_position {
                    *chain_data
                        .first_position_topics
                        .entry(topic.clone())
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
            if let Some(addr_count) = chain_data.addresses.get_mut(&filter.address) {
                if *addr_count > 0u32 {
                    *addr_count -= Nat::from(1u32);
                    if *addr_count == 0u32 {
                        chain_data.addresses.remove(&filter.address);
                    }
                }
            }

            // Decrement topic counters (from the first position) if they exist
            if let Some(all_positions) = &filter.topics {
                if !all_positions.is_empty() {
                    let first_position = &all_positions[0];
                    for topic in first_position {
                        if let Some(topic_count) = chain_data.first_position_topics.get_mut(topic) {
                            if *topic_count > 0u32 {
                                *topic_count -= Nat::from(1u32);
                                if *topic_count == 0u32 {
                                    chain_data.first_position_topics.remove(topic);
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
    ///             otherwise => Some([ list_of_topics ])
    pub fn get_active_addresses_and_topics(
        &self,
        chain_id: u32,
    ) -> (Vec<String>, Option<Vec<Vec<String>>>) {
        if let Some(chain_data) = self.get_chain_data(chain_id) {
            // Gather addresses
            let addresses = chain_data
                .addresses
                .keys()
                .cloned()
                .collect::<Vec<_>>();

            // Gather topics from the first position
            let topics_collected = chain_data
                .first_position_topics
                .keys()
                .cloned()
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
    use super::*;
    use evm_logs_types::Filter;

    /// Helper function to create a Filter with a given address and optional topics.
    /// We'll keep it simple: `topics` can be a Vec of Vec of &str, which we convert to String.
    fn make_filter(address: &str, topics: Option<Vec<Vec<&str>>>) -> Filter {
        Filter {
            address: address.to_string(),
            topics: topics.map(|positions| {
                positions
                    .into_iter()
                    .map(|pos| pos.into_iter().map(|s| s.to_string()).collect())
                    .collect()
            }),
        }
    }

    #[test]
    fn test_add_single_filter_with_first_position_topics() {
        let mut manager = FilterManager::new();

        // Create a filter with one address and some topics in the first position
        let filter = make_filter(
            "0xAddress1",
            Some(vec![
                vec!["TopicA", "TopicB"], // first position
                vec!["TopicC"],           // second position (ignored)
            ]),
        );

        // Add the filter
        manager.add_filter(1, &filter);

        // Now retrieve addresses and topics for chain_id=1
        let (addresses, topics) = manager.get_active_addresses_and_topics(1);

        // We expect to see "0xAddress1" in addresses
        assert_eq!(addresses, vec!["0xAddress1".to_string()]);
        // The second part is Some(...) because we do have first-position topics
        // specifically "TopicA" and "TopicB".
        let unwrapped = topics.expect("Should have some topics");
        assert_eq!(unwrapped.len(), 1, "We only store first position in one vector");
        // Inside that vector, we expect "TopicA" and "TopicB"
        // The order in a HashMap-based scenario is not guaranteed, so let's just check they exist
        let first_pos = &unwrapped[0];
        assert!(first_pos.contains(&"TopicA".to_string()));
        assert!(first_pos.contains(&"TopicB".to_string()));
    }

    #[test]
    fn test_remove_filter_clears_data() {
        let mut manager = FilterManager::new();

        // Create a filter with a single address and a single topic in the first position
        let filter = make_filter("0xAddress2", Some(vec![vec!["TopicX"]]));

        // Add the filter
        manager.add_filter(1, &filter);
        // Check that addresses and topics are present
        let (addresses, topics) = manager.get_active_addresses_and_topics(1);
        assert_eq!(addresses, vec!["0xAddress2".to_string()]);
        assert!(topics.is_some());

        // Now remove the filter
        manager.remove_filter(1, &filter);

        // After removing, we expect the manager to have no addresses or topics for chain 1
        let (addresses_after, topics_after) = manager.get_active_addresses_and_topics(1);
        assert!(addresses_after.is_empty());
        assert!(topics_after.is_none());
    }

    #[test]
    fn test_add_multiple_filters_different_addresses() {
        let mut manager = FilterManager::new();

        let filter1 = make_filter("0xAddrA", Some(vec![vec!["T1", "T2"]]));
        let filter2 = make_filter("0xAddrB", Some(vec![vec!["T2", "T3"]]));
        // Filter with no topics => it won't contribute to first_position_topics
        let filter3 = make_filter("0xAddrC", None);

        // Add them for chain_id=1
        manager.add_filter(1, &filter1);
        manager.add_filter(1, &filter2);
        manager.add_filter(1, &filter3);

        // Now gather
        let (addresses, topics) = manager.get_active_addresses_and_topics(1);

        // We expect addresses: 0xAddrA, 0xAddrB, 0xAddrC
        assert_eq!(addresses.len(), 3);
        assert!(addresses.contains(&"0xAddrA".to_string()));
        assert!(addresses.contains(&"0xAddrB".to_string()));
        assert!(addresses.contains(&"0xAddrC".to_string()));

        // For topics, from the first position:
        // - filter1 contributed T1, T2
        // - filter2 contributed T2, T3
        // - filter3 contributed nothing
        // So we expect T1, T2, T3
        let some_topics = topics.expect("We should have topics from filter1 & filter2");
        assert_eq!(some_topics.len(), 1); // one vector
        let tvec = &some_topics[0];
        assert!(tvec.contains(&"T1".to_string()));
        assert!(tvec.contains(&"T2".to_string()));
        assert!(tvec.contains(&"T3".to_string()));
    }

    #[test]
    fn test_add_and_remove_interleaved() {
        let mut manager = FilterManager::new();

        let filter1 = make_filter("0xAddrA", Some(vec![vec!["X"]]));
        let filter2 = make_filter("0xAddrB", Some(vec![vec!["Y"]]));
        let filter3 = make_filter("0xAddrA", Some(vec![vec!["Z"]]));

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
        assert_eq!(addr_2, vec!["0xAddrB".to_string()]);
        let unwrapped_2 = topics_2.unwrap();
        assert_eq!(unwrapped_2[0], vec!["Y".to_string()]);

        // Add filter3, which is also 0xAddrA but with topic Z
        manager.add_filter(5, &filter3);

        // addresses => [0xAddrB, 0xAddrA], topics => Y, Z
        let (addr_3, topics_3) = manager.get_active_addresses_and_topics(5);
        assert_eq!(addr_3.len(), 2);
        assert!(addr_3.contains(&"0xAddrB".to_string()));
        assert!(addr_3.contains(&"0xAddrA".to_string()));
        let unwrapped_3 = topics_3.unwrap();
        // We expect Y, Z in some order
        let t3 = &unwrapped_3[0];
        assert_eq!(t3.len(), 2);
        assert!(t3.contains(&"Y".to_string()));
        assert!(t3.contains(&"Z".to_string()));
    }

    #[test]
    fn test_no_filters_for_chain() {
        let manager = FilterManager::new();
        // chain_id=99 has no filters
        let (addresses, topics) = manager.get_active_addresses_and_topics(99);
        assert!(addresses.is_empty());
        assert!(topics.is_none());
    }
}

