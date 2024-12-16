use candid::Nat;
use evm_logs_types::Filter;
use std::collections::HashMap;
use evm_logs_types::ChainName;

const MAX_TOPICS: usize = 4;

type TopicsPosition = Vec<String>;

#[derive(Debug)]
struct PerChainData {
    topic_counts: Vec<HashMap<String, Nat>>,
    addresses: HashMap<String, Nat>,
    accept_any_topic_at_position: Vec<Nat>,
    total_subscriptions: Nat,
}

impl PerChainData {
    fn new() -> Self {
        Self {
            topic_counts: vec![HashMap::new(); MAX_TOPICS],
            addresses: HashMap::new(),
            accept_any_topic_at_position: vec![Nat::from(0u32); MAX_TOPICS],
            total_subscriptions: Nat::from(0u32),
        }
    }
}

pub struct FilterManager {
    // For each chain, we hold a separate set of data.
    chain_data: HashMap<ChainName, PerChainData>,
}

impl FilterManager {
    pub fn new() -> Self {
        FilterManager {
            chain_data: HashMap::new(),
        }
    }

    fn get_chain_data_mut(&mut self, chain: &ChainName) -> &mut PerChainData {
        // If data for this chain does not exist, initialize it.
        if !self.chain_data.contains_key(chain) {
            self.chain_data.insert(chain.clone(), PerChainData::new());
        }
        self.chain_data.get_mut(chain).unwrap()
    }

    fn get_chain_data(&self, chain: &ChainName) -> Option<&PerChainData> {
        self.chain_data.get(chain)
    }

    /// Adds a new filter (subscription) to the manager for a specific chain.
    pub fn add_filter(&mut self, chain: ChainName, filter: &Filter) {
        let chain_data = self.get_chain_data_mut(&chain);

        chain_data.total_subscriptions += Nat::from(1u32);

        let addr = &filter.address;
        *chain_data
            .addresses
            .entry(addr.clone())
            .or_insert(Nat::from(0u32)) += Nat::from(1u32);

        if let Some(filter_topics) = &filter.topics {
            for (i, topics_at_pos) in filter_topics.iter().enumerate() {
                if topics_at_pos.is_empty() {
                    // Position will accept any topic if empty
                    chain_data.accept_any_topic_at_position[i] += Nat::from(1u32);
                } else {
                    for topic in topics_at_pos {
                        *chain_data.topic_counts[i]
                            .entry(topic.clone())
                            .or_insert(Nat::from(0u32)) += Nat::from(1u32);
                    }
                }
            }

            // if filter has less than 4 topics than the rest of positions will accept any topic
            for i in filter_topics.len()..MAX_TOPICS {
                chain_data.accept_any_topic_at_position[i] += Nat::from(1u32);
            }
        } else {
            // No topics specified => all positions accept any topic.
            for i in 0..MAX_TOPICS {
                chain_data.accept_any_topic_at_position[i] += Nat::from(1u32);
            }
        }
    }

    /// Removes a filter (subscription) from the manager for a specific chain.
    pub fn remove_filter(&mut self, chain: ChainName, filter: &Filter) {
        let chain_data = match self.get_chain_data_mut(&chain) {
            data => data,
        };

        if chain_data.total_subscriptions > 0u32 {
            chain_data.total_subscriptions -= Nat::from(1u32);
        }

        let addr = &filter.address;
        if let Some(count) = chain_data.addresses.get_mut(addr) {
            *count -= Nat::from(1u32);
            if *count == 0u32 {
                chain_data.addresses.remove(addr);
            }
        }

        if let Some(filter_topics) = &filter.topics {
            for (i, topics_at_pos) in filter_topics.iter().enumerate() {
                if topics_at_pos.is_empty() {
                    if chain_data.accept_any_topic_at_position.len() > i
                        && chain_data.accept_any_topic_at_position[i] > 0u32
                    {
                        chain_data.accept_any_topic_at_position[i] -= Nat::from(1u32);
                    }
                } else {
                    for topic in topics_at_pos {
                        if let Some(count) = chain_data.topic_counts[i].get_mut(topic) {
                            *count -= Nat::from(1u32);
                            if *count == 0u32 {
                                chain_data.topic_counts[i].remove(topic);
                            }
                        }
                    }
                }
            }

            // if filter has less than 4 topics than the rest of positions accepted any topic,
            // decrement these positions since we remove this filter
            for i in filter_topics.len()..MAX_TOPICS {
                if chain_data.accept_any_topic_at_position[i] > 0u32 {
                    chain_data.accept_any_topic_at_position[i] -= Nat::from(1u32);
                }
            }
        } else {
            // No topics => decrement "accept any" count for all positions
            for i in 0..MAX_TOPICS {
                if chain_data.accept_any_topic_at_position[i] > 0u32 {
                    chain_data.accept_any_topic_at_position[i] -= Nat::from(1u32);
                }
            }
        }
    }

    /// Computes a combined list of topics for a given chain.
    pub fn get_combined_topics(&self, chain: ChainName) -> Option<Vec<TopicsPosition>> {
        let chain_data = self.get_chain_data(&chain)?;
        let mut combined_topics = Vec::with_capacity(MAX_TOPICS);

        for i in 0..MAX_TOPICS {
            if chain_data.accept_any_topic_at_position[i] > 0u32 {
                // This position accepts any topic; we cannot determine a specific set.
                // Return an empty vector for this position (accept any topic)
                combined_topics.push(Vec::new());
            } else {
                // If we have a set of topics, return them; otherwise, return an empty vector.
                if !chain_data.topic_counts[i].is_empty() {
                    let position_topics: Vec<String> =
                        chain_data.topic_counts[i].keys().cloned().collect();
                    combined_topics.push(position_topics);
                } else {
                    // No "accept any" and no topics => empty vector
                    combined_topics.push(Vec::new());
                }
            }
        }

        // If all positions are empty, return None
        let all_empty = combined_topics.iter().all(|topics| topics.is_empty());
        if all_empty {
            None
        } else {
            Some(combined_topics)
        }
    }

    /// Returns a tuple of all active addresses and the combined topics for a given chain, if any.
    pub fn get_active_addresses_and_topics(
        &self,
        chain: ChainName,
    ) -> (Vec<String>, Option<Vec<TopicsPosition>>) {
        let chain_data = self.get_chain_data(&chain);

        if chain_data.is_none() {
            return (Vec::new(), None);
        }

        let chain_data = chain_data.unwrap();
        let addresses: Vec<String> = chain_data.addresses.keys().cloned().collect();
        let topics = self.get_combined_topics(chain);

        (addresses, topics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use evm_logs_types::Filter;

    /// Helper function to create a Filter with a single address.
    fn make_filter(address: &str, topics: Option<Vec<Vec<&str>>>) -> Filter {
        Filter {
            address: address.to_string(),
            topics: topics.map(|outer| {
                outer
                    .into_iter()
                    .map(|inner| inner.into_iter().map(|s| s.to_string()).collect())
                    .collect()
            }),
        }
    }

    #[test]
    fn test_add_filter_with_simple_topics() {
        let mut manager = FilterManager::new();
        let filter = make_filter(
            "address1",
            Some(vec![vec!["topic1"], vec!["topic2"], vec![], vec!["topic4"]]),
        );

        // Add filter to the Ethereum chain
        manager.add_filter(ChainName::Ethereum, &filter);

        // Retrieve the data for Ethereum chain
        let chain_data = manager.chain_data.get(&ChainName::Ethereum).unwrap();

        // Check total subscription count.
        assert_eq!(chain_data.total_subscriptions, Nat::from(1u32));

        // Check addresses.
        assert_eq!(chain_data.addresses.get("address1"), Some(&Nat::from(1u32)));

        // Check topic counts.
        assert_eq!(
            chain_data.topic_counts[0].get("topic1"),
            Some(&Nat::from(1u32))
        );
        assert_eq!(
            chain_data.topic_counts[1].get("topic2"),
            Some(&Nat::from(1u32))
        );
        assert_eq!(
            chain_data.topic_counts[3].get("topic4"),
            Some(&Nat::from(1u32))
        );

        // The third position is an empty vector => accept_any_topic_at_position should increase.
        assert_eq!(chain_data.accept_any_topic_at_position[2], Nat::from(1u32));

        // Ensure other positions are unaffected.
        assert_eq!(chain_data.accept_any_topic_at_position[0], Nat::from(0u32));
        assert_eq!(chain_data.accept_any_topic_at_position[1], Nat::from(0u32));
        assert_eq!(chain_data.accept_any_topic_at_position[3], Nat::from(0u32));
    }

    #[test]
    fn test_remove_filter_with_simple_topics() {
        let mut manager = FilterManager::new();
        let filter = make_filter(
            "address1",
            Some(vec![vec!["topic1"], vec![], vec!["topic3"], vec!["topic4"]]),
        );

        // Add the filter to the Ethereum chain
        manager.add_filter(ChainName::Ethereum, &filter);
        assert_eq!(manager.chain_data.get(&ChainName::Ethereum).unwrap().total_subscriptions, Nat::from(1u32));

        // Remove the same filter
        manager.remove_filter(ChainName::Ethereum, &filter);

        // After removal, verify that chain_data for Ethereum exists but is empty
        let chain_data_after_removal = manager.chain_data.get(&ChainName::Ethereum).unwrap();
        
        assert_eq!(chain_data_after_removal.total_subscriptions, Nat::from(0u32));
        
        assert!(chain_data_after_removal.addresses.is_empty());

        // Check that all topic_counts are empty
        for i in 0..MAX_TOPICS {
            assert!(chain_data_after_removal.topic_counts[i].is_empty(), 
                "topic_counts at position {} should be empty", i);
        }

        // Check that accept_any_topic_at_position are all zero
        for i in 0..MAX_TOPICS {
            assert_eq!(
                chain_data_after_removal.accept_any_topic_at_position[i],
                Nat::from(0u32),
                "accept_any_topic_at_position at position {} should be 0",
                i
            );
        }
    }

    #[test]
    fn test_add_multiple_filters_with_overlap() {
        let mut manager = FilterManager::new();

        let filter1 = make_filter(
            "address1",
            Some(vec![vec!["topic1"], vec!["topic2"], vec!["topic3"], vec![]]),
        );
        let filter2 = make_filter(
            "address2",
            Some(vec![vec!["topic1"], vec!["topic4"], vec![], vec![]]),
        );
        let filter3 = make_filter("address3", None); // No topics => all accept any

        // Add filters to the Ethereum chain
        manager.add_filter(ChainName::Ethereum, &filter1);
        manager.add_filter(ChainName::Ethereum, &filter2);
        manager.add_filter(ChainName::Ethereum, &filter3);

        assert_eq!(manager.chain_data.get(&ChainName::Ethereum).unwrap().total_subscriptions, Nat::from(3u32));

        // Check addresses
        let chain_data = manager.chain_data.get(&ChainName::Ethereum).unwrap();
        assert_eq!(chain_data.addresses.get("address1"), Some(&Nat::from(1u32)));
        assert_eq!(chain_data.addresses.get("address2"), Some(&Nat::from(1u32)));
        assert_eq!(chain_data.addresses.get("address3"), Some(&Nat::from(1u32)));

        // Check topics for position 0: filter1 and filter2 add "topic1"
        assert_eq!(
            chain_data.topic_counts[0].get("topic1"),
            Some(&Nat::from(2u32))
        );

        // Position 1: "topic2" from filter1, "topic4" from filter2
        assert_eq!(
            chain_data.topic_counts[1].get("topic2"),
            Some(&Nat::from(1u32))
        );
        assert_eq!(
            chain_data.topic_counts[1].get("topic4"),
            Some(&Nat::from(1u32))
        );

        // Position 2: "topic3" from filter1
        assert_eq!(
            chain_data.topic_counts[2].get("topic3"),
            Some(&Nat::from(1u32))
        );

        // Check accept_any_topic_at_position
        // filter1: position 3 empty => accept_any_topic_at_position[3] += 1
        // filter2: positions 2 and 3 empty => accept_any_topic_at_position[2] += 1, accept_any_topic_at_position[3] += 1
        // filter3: all positions accept any => accept_any_topic_at_position[0..4] += 1

        // Position 0:
        // filter1 and filter2 have specific topics, filter3 accepts any => +1
        assert_eq!(chain_data.accept_any_topic_at_position[0], Nat::from(1u32));

        // Position 1:
        // filter1 and filter2 have specific topics, filter3 accepts any => +1
        assert_eq!(chain_data.accept_any_topic_at_position[1], Nat::from(1u32));

        // Position 2:
        // filter1 has specific topic, filter2 and filter3 accept any => +2
        assert_eq!(chain_data.accept_any_topic_at_position[2], Nat::from(2u32));

        // Position 3:
        // filter1, filter2, and filter3 accept any => +3
        assert_eq!(chain_data.accept_any_topic_at_position[3], Nat::from(3u32));
    }

    #[test]
    fn test_remove_some_filters_with_overlap() {
        let mut manager = FilterManager::new();

        let filter1 = make_filter(
            "address1",
            Some(vec![vec!["topic1"], vec!["topic2"], vec!["topic3"], vec![]]),
        );
        let filter2 = make_filter(
            "address2",
            Some(vec![vec!["topic1"], vec!["topic4"], vec![], vec![]]),
        );
        let filter3 = make_filter("address3", None); // No topics => all accept any

        // Add filters to the Ethereum chain
        manager.add_filter(ChainName::Ethereum, &filter1);
        manager.add_filter(ChainName::Ethereum, &filter2);
        manager.add_filter(ChainName::Ethereum, &filter3);

        // Now remove filter2 and filter3
        manager.remove_filter(ChainName::Ethereum, &filter2);
        manager.remove_filter(ChainName::Ethereum, &filter3);

        // Only filter1 should remain
        let chain_data = manager.chain_data.get(&ChainName::Ethereum).unwrap();
        assert_eq!(chain_data.total_subscriptions, Nat::from(1u32));

        // Addresses: address1 remains, address2 and address3 should be removed
        assert!(chain_data.addresses.get("address2").is_none());
        assert!(chain_data.addresses.get("address3").is_none());
        assert_eq!(chain_data.addresses.get("address1"), Some(&Nat::from(1u32)));

        // Topics:
        // After removing filter2 and filter3:
        // Only filter1 remains:
        // topics: [ ["topic1"], ["topic2"], ["topic3"], [] ]
        // So for position 0 remains "topic1" with count 1
        assert_eq!(
            chain_data.topic_counts[0].get("topic1"),
            Some(&Nat::from(1u32))
        );

        // Position 1: "topic2" with count 1
        assert_eq!(
            chain_data.topic_counts[1].get("topic2"),
            Some(&Nat::from(1u32))
        );

        // Position 2: "topic3" with count 1
        assert_eq!(
            chain_data.topic_counts[2].get("topic3"),
            Some(&Nat::from(1u32))
        );

        // Position 3: empty => accept_any_topic_at_position[3] should be 1 (from filter1 only)
        assert_eq!(chain_data.accept_any_topic_at_position[3], Nat::from(1u32));

        // Check that the other accept_any counts decreased correctly after removing filter2 and filter3
        // Position 0:
        // - Was +1 from filter3, now removed => 0
        assert_eq!(chain_data.accept_any_topic_at_position[0], Nat::from(0u32));

        // Position 1:
        // - Was +1 from filter3, now removed => 0
        assert_eq!(chain_data.accept_any_topic_at_position[1], Nat::from(0u32));

        // Position 2:
        // - Was 2, one from filter2, one from filter3, both removed => 0
        assert_eq!(chain_data.accept_any_topic_at_position[2], Nat::from(0u32));
    }

    #[test]
    fn test_get_combined_topics() {
        let mut manager = FilterManager::new();

        let filter_no_topics = make_filter("address1", None);
        manager.add_filter(ChainName::Ethereum, &filter_no_topics);

        // get_combined_topics should return None because all positions are accept any
        assert!(manager.get_combined_topics(ChainName::Ethereum).is_none());

        let filter_with_topics = make_filter(
            "address2",
            Some(vec![
                vec!["topic1"],
                vec!["topic2"],
                vec!["topic3"],
                vec!["topic4"],
            ]),
        );
        manager.add_filter(ChainName::Ethereum, &filter_with_topics);

        // Now part of it accepts any and part has specific topics.
        // According to logic, if any position accept any, we cannot produce a deterministic set for that position.
        // We should get None.
        let combined = manager.get_combined_topics(ChainName::Ethereum);
        // First filter sets everything to accept any, second tries to set specific topics, but we still have accept any from the first.
        assert!(combined.is_none());

        // Remove the no-topic filter to remove accept any
        manager.remove_filter(ChainName::Ethereum, &filter_no_topics);

        // Now only the filter with specific topics remains.
        // All positions have specific topics and no accept any.
        let combined = manager.get_combined_topics(ChainName::Ethereum).unwrap();
        assert_eq!(combined.len(), MAX_TOPICS);
        assert_eq!(combined[0], vec!["topic1".to_string()]);
        assert_eq!(combined[1], vec!["topic2".to_string()]);
        assert_eq!(combined[2], vec!["topic3".to_string()]);
        assert_eq!(combined[3], vec!["topic4".to_string()]);
    }

    #[test]
    fn test_get_active_addresses_and_topics1() {
        let mut manager = FilterManager::new();
        let filter1 = make_filter(
            "address1",
            Some(vec![vec!["topicA"], vec![], vec!["topicC"], vec![]]),
        );
        let filter2 = make_filter("address2", None); // Accept any for all

        // Add filters to the Ethereum chain
        manager.add_filter(ChainName::Ethereum, &filter1);
        manager.add_filter(ChainName::Ethereum, &filter2);

        let (addresses, topics) = manager.get_active_addresses_and_topics(ChainName::Ethereum);

        // Check addresses
        assert!(addresses.contains(&"address1".to_string()));
        assert!(addresses.contains(&"address2".to_string()));

        // Since one filter is accept any for all, get_combined_topics returns None
        assert!(topics.is_none());

        // Remove the second filter so that only specific topics remain
        manager.remove_filter(ChainName::Ethereum, &filter2);
        let (addresses, topics) = manager.get_active_addresses_and_topics(ChainName::Ethereum);

        assert_eq!(addresses, vec!["address1".to_string()]);
        let topics = topics.unwrap();
        // First position: ["topicA"]
        assert_eq!(topics[0], vec!["topicA".to_string()]);

        // Second position: accept any was previously allowed, but since filter2 was removed,
        // and filter1 has empty topics at position 1, which was already accounted for in FilterManager.
        // Therefore, accept_any_topic_at_position[1] should have been decreased.
        // However, since filter1 has topics at position 1, we need to verify.

        // From filter1: topics at position 1 are empty => accept_any_topic_at_position[1] +=1
        // Since filter2 was removed, which also affects accept_any_topic_at_position[1], but in filter1 it's already counted.

        // Thus, after removing filter2, if filter1 still has topics as empty at position 1, accept_any_topic_at_position[1] should be 1
        // Therefore, combined topics should have empty vector at position 1
        // As per FilterManager::get_combined_topics, if accept_any_topic_at_position[i] >0, push empty vector

        // But in this test, topics.unwrap() should have empty vector for position 1
        assert_eq!(topics[1], Vec::<String>::new());

        // Third position: ["topicC"]
        assert_eq!(topics[2], vec!["topicC".to_string()]);

        // Fourth position: accept any was allowed by filter1
        assert_eq!(topics[3], Vec::<String>::new());
    }

    #[test]
    fn test_get_active_addresses_and_topics2() {
        let mut manager = FilterManager::new();

        // Scenario: add multiple filters with different combinations
        // 1. Filter with all specific topics
        let filter_full_topics = make_filter(
            "address1",
            Some(vec![
                vec!["topicA1"],
                vec!["topicA2"],
                vec!["topicA3"],
                vec!["topicA4"],
            ]),
        );

        // 2. Filter with partially defined topics and partially empty (accept any)
        let filter_partial = make_filter(
            "address2",
            Some(vec![
                vec!["topicB1"], // position 0: specific topic
                vec![],          // position 1: empty => accept any
                vec!["topicB3"], // position 2: specific topic
                vec![],          // position 3: empty => accept any
            ]),
        );

        // 3. Filter with no topics (all positions accept any)
        let filter_none = make_filter("address3", None);

        // Add all filters to the Ethereum chain
        manager.add_filter(ChainName::Ethereum, &filter_full_topics);
        manager.add_filter(ChainName::Ethereum, &filter_partial);
        manager.add_filter(ChainName::Ethereum, &filter_none);

        // Check active addresses
        let (addresses, topics) = manager.get_active_addresses_and_topics(ChainName::Ethereum);
        assert!(addresses.contains(&"address1".to_string()));
        assert!(addresses.contains(&"address2".to_string()));
        assert!(addresses.contains(&"address3".to_string()));

        // Since there is at least one filter (filter_none) that accepts any topic for all positions,
        // we expect topics == None
        assert!(topics.is_none());

        // Remove the no-topics filter to remove the universal accept any
        manager.remove_filter(ChainName::Ethereum, &filter_none);

        // Now we have filters with partial and full topics only
        let (addresses, topics) = manager.get_active_addresses_and_topics(ChainName::Ethereum);
        assert!(addresses.contains(&"address1".to_string()));
        assert!(addresses.contains(&"address2".to_string()));
        assert!(!addresses.contains(&"address3".to_string()));

        assert!(topics.is_some());

        // Remove the partial filter as well, leaving only the full topic filter
        manager.remove_filter(ChainName::Ethereum, &filter_partial);

        let (addresses, topics) = manager.get_active_addresses_and_topics(ChainName::Ethereum);
        assert_eq!(addresses, vec!["address1".to_string()]);
        let topics = topics.unwrap();
        assert_eq!(topics.len(), MAX_TOPICS);
        assert_eq!(topics[0], vec!["topicA1".to_string()]);
        assert_eq!(topics[1], vec!["topicA2".to_string()]);
        assert_eq!(topics[2], vec!["topicA3".to_string()]);
        assert_eq!(topics[3], vec!["topicA4".to_string()]);
    }

    #[test]
    fn test_get_active_addresses_and_topics3() {
        let mut manager = FilterManager::new();

        // Create a complex scenario with many filters:

        // 1. A filter with one address and two specific topics, two positions empty
        let filter1 = make_filter(
            "address1",
            Some(vec![
                vec!["topicA1"], // position 0: specific topic
                vec!["topicA2"], // position 1: specific topic
                vec![],          // position 2: empty => accept any
                vec![],          // position 3: empty => accept any
            ]),
        );

        // 2. A filter with multiple addresses and full topics
        let filter2 = make_filter(
            "address2",
            Some(vec![
                vec!["topicB1"],
                vec!["topicB2"],
                vec!["topicB3"],
                vec!["topicB4"],
            ]),
        );

        // 3. A filter with no topics
        let filter3 = make_filter("address3", None);

        // 4. A filter where two positions are full and two are empty
        let filter4 = make_filter(
            "address4",
            Some(vec![
                vec!["topicC1"],
                vec![],          // empty => accept any
                vec!["topicC3"], // position 2: specific topic
                vec![],          // empty => accept any
            ]),
        );

        // 5. A filter with one address and only one position filled (others empty)
        let filter5 = make_filter(
            "address5",
            Some(vec![
                vec!["topicD1"],
                vec![], // accept any
                vec![], // accept any
                vec![], // accept any
            ]),
        );

        // Add all filters to the Ethereum chain
        manager.add_filter(ChainName::Ethereum, &filter1);
        manager.add_filter(ChainName::Ethereum, &filter2);
        manager.add_filter(ChainName::Ethereum, &filter3);
        manager.add_filter(ChainName::Ethereum, &filter4);
        manager.add_filter(ChainName::Ethereum, &filter5);

        // Check that all addresses are present
        let (addresses, topics) = manager.get_active_addresses_and_topics(ChainName::Ethereum);
        for addr in &[
            "address1", "address2", "address3", "address4", "address5",
        ] {
            assert!(addresses.contains(&addr.to_string()));
        }

        // Currently, we have many accept any (from filter1, filter3, filter4, filter5).
        // Expect topics == None.
        assert!(topics.is_none());

        // Remove the no-topics filter (address3) to reduce accept any sources
        manager.remove_filter(ChainName::Ethereum, &filter3);

        // Get active addresses and topics again
        let (addresses, topics) = manager.get_active_addresses_and_topics(ChainName::Ethereum);
        // Check that address3 is removed
        assert!(!addresses.contains(&"address3".to_string()));

        assert!(topics.is_some());

        // Try removing all filters with accept any, leaving only the full topic filter (filter2)
        manager.remove_filter(ChainName::Ethereum, &filter1);
        manager.remove_filter(ChainName::Ethereum, &filter4);
        manager.remove_filter(ChainName::Ethereum, &filter5);

        // Only filter2 remains, which has a full set of topics and no accept any
        let (addresses, topics) = manager.get_active_addresses_and_topics(ChainName::Ethereum);
        // Only address2 should remain
        assert_eq!(addresses, vec!["address2".to_string()]);

        // Now, topics should not be None, since no accept any remains
        let topics = topics.unwrap();
        assert_eq!(topics.len(), MAX_TOPICS);
        assert_eq!(topics[0], vec!["topicB1".to_string()]);
        assert_eq!(topics[1], vec!["topicB2".to_string()]);
        assert_eq!(topics[2], vec!["topicB3".to_string()]);
        assert_eq!(topics[3], vec!["topicB4".to_string()]);
    }
}

