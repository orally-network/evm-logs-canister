use candid::Nat;
use evm_logs_types::Filter;
use std::collections::HashMap;

const MAX_TOPICS: usize = 4;

type TopicsPosition = Vec<String>;

pub struct FilterManager {
    topic_counts: Vec<HashMap<String, Nat>>,
    addresses: HashMap<String, Nat>,
    accept_any_topic_at_position: Vec<Nat>,
    total_subscriptions: Nat,
}

impl FilterManager {
    pub fn new() -> Self {
        FilterManager {
            topic_counts: vec![HashMap::new(); MAX_TOPICS],
            addresses: HashMap::new(),
            accept_any_topic_at_position: vec![Nat::from(0u32); MAX_TOPICS],
            total_subscriptions: Nat::from(0u32),
        }
    }

    /// Adds a new filter (subscription) to the manager, updating counts for addresses and topics.
    pub fn add_filter(&mut self, filter: &Filter) {
        self.total_subscriptions += Nat::from(1u32);

        *self
            .addresses
            .entry(filter.address.clone())
            .or_insert(Nat::from(0u32)) += Nat::from(1u32);

        if let Some(filter_topics) = &filter.topics {
            for (i, topics_at_pos) in filter_topics.iter().enumerate() {
                if topics_at_pos.is_empty() {
                    // Position will accept any topic if empty
                    self.accept_any_topic_at_position[i] += Nat::from(1u32);
                } else {
                    for topic in topics_at_pos {
                        *self.topic_counts[i]
                            .entry(topic.clone())
                            .or_insert(Nat::from(0u32)) += Nat::from(1u32);
                    }
                }
            }

            // if filter has less than 4 topics than the rest of positions will accept any topic
            for i in filter_topics.len()..MAX_TOPICS {
                self.accept_any_topic_at_position[i] += Nat::from(1u32);
            }
        } else {
            // No topics specified => all positions accept any topic.
            for i in 0..MAX_TOPICS {
                self.accept_any_topic_at_position[i] += Nat::from(1u32);
            }
        }
    }

    /// Removes a filter (subscription) from the manager, decrementing counts accordingly.
    pub fn remove_filter(&mut self, filter: &Filter) {
        if self.total_subscriptions > 0u32 {
            self.total_subscriptions -= Nat::from(1u32);
        }

        if let Some(count) = self.addresses.get_mut(&filter.address) {
            *count -= Nat::from(1u32);
            if *count == 0u32 {
                self.addresses.remove(&filter.address);
            }
        }

        if let Some(filter_topics) = &filter.topics {
            for (i, topics_at_pos) in filter_topics.iter().enumerate() {
                if topics_at_pos.is_empty() {
                    if self.accept_any_topic_at_position.len() > i
                        && self.accept_any_topic_at_position[i] > 0u32
                    {
                        self.accept_any_topic_at_position[i] -= Nat::from(1u32);
                    }
                } else {
                    for topic in topics_at_pos {
                        if let Some(count) = self.topic_counts[i].get_mut(topic) {
                            *count -= Nat::from(1u32);
                            if *count == 0u32 {
                                self.topic_counts[i].remove(topic);
                            }
                        }
                    }
                }
            }

            // if filter has less than 4 topics than the rest of positions will accept any topic,
            //  so we need to decrement these positions since we remove this filter
            for i in filter_topics.len()..MAX_TOPICS {
                self.accept_any_topic_at_position[i] -= Nat::from(1u32);
            }
        } else {
            // No topics => decrement "accept any" count for all positions
            for i in 0..MAX_TOPICS {
                if self.accept_any_topic_at_position[i] > 0u32 {
                    // always should happen
                    self.accept_any_topic_at_position[i] -= Nat::from(1u32);
                }
            }
        }
    }

    /// Computes a combined list of topics.
    pub fn get_combined_topics(&self) -> Option<Vec<TopicsPosition>> {
        let mut combined_topics = Vec::with_capacity(MAX_TOPICS);

        for i in 0..MAX_TOPICS {
            if self.accept_any_topic_at_position[i] > 0u32 {
                // This position accepts any topic; we cannot determine a specific set.
                // Return an empty vector for this position (accept any topic)
                combined_topics.push(Vec::new());
            } else {
                // If we have a set of topics, return them; otherwise, return an empty vector.
                if !self.topic_counts[i].is_empty() {
                    let position_topics: Vec<String> =
                        self.topic_counts[i].keys().cloned().collect();
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

    /// Returns a tuple of all active addresses and the combined topics, if any.
    pub fn get_active_addresses_and_topics(&self) -> (Vec<String>, Option<Vec<TopicsPosition>>) {
        let addresses: Vec<String> = self.addresses.keys().cloned().collect();

        let topics = self.get_combined_topics();

        (addresses, topics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use evm_logs_types::Filter;

    fn make_filter(addresses: Vec<&str>, topics: Option<Vec<Vec<&str>>>) -> Filter {
        Filter {
            address: addresses.into_iter().map(|s| s.to_string()).collect(),
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
            vec!["address1", "address2"],
            Some(vec![vec!["topic1"], vec!["topic2"], vec![], vec!["topic4"]]),
        );

        manager.add_filter(&filter);

        // Check total subscription count.
        assert_eq!(manager.total_subscriptions, Nat::from(1u32));

        // Check addresses.
        assert_eq!(manager.addresses.get("address1"), Some(&Nat::from(1u32)));
        assert_eq!(manager.addresses.get("address2"), Some(&Nat::from(1u32)));

        // Check topic counts.
        assert_eq!(
            manager.topic_counts[0].get("topic1"),
            Some(&Nat::from(1u32))
        );
        assert_eq!(
            manager.topic_counts[1].get("topic2"),
            Some(&Nat::from(1u32))
        );
        assert_eq!(
            manager.topic_counts[3].get("topic4"),
            Some(&Nat::from(1u32))
        );

        // The third position is an empty vector => accept_any_topic_at_position should increase.
        assert_eq!(manager.accept_any_topic_at_position[2], Nat::from(1u32));

        assert_eq!(manager.accept_any_topic_at_position[0], Nat::from(0u32));
        assert_eq!(manager.accept_any_topic_at_position[1], Nat::from(0u32));
        assert_eq!(manager.accept_any_topic_at_position[3], Nat::from(0u32));
    }

    #[test]
    fn test_remove_filter_with_simple_topics() {
        let mut manager = FilterManager::new();
        let filter = make_filter(
            vec!["address1"],
            Some(vec![vec!["topic1"], vec![], vec!["topic3"], vec!["topic4"]]),
        );

        manager.add_filter(&filter);
        assert_eq!(manager.total_subscriptions, Nat::from(1u32));

        // Remove the same filter
        manager.remove_filter(&filter);
        assert_eq!(manager.total_subscriptions, Nat::from(0u32));

        assert!(manager.addresses.get("address1").is_none());

        for i in 0..MAX_TOPICS {
            assert!(manager.topic_counts[i].is_empty());
        }

        for i in 0..MAX_TOPICS {
            assert_eq!(manager.accept_any_topic_at_position[i], Nat::from(0u32));
        }
    }

    #[test]
    fn test_add_multiple_filters_with_overlap() {
        let mut manager = FilterManager::new();

        let filter1 = make_filter(
            vec!["address1"],
            Some(vec![vec!["topic1"], vec!["topic2"], vec!["topic3"], vec![]]),
        );
        let filter2 = make_filter(
            vec!["address2"],
            Some(vec![vec!["topic1"], vec!["topic4"], vec![], vec![]]),
        );
        let filter3 = make_filter(vec!["address3"], None); // No topics => all accept any

        manager.add_filter(&filter1);
        manager.add_filter(&filter2);
        manager.add_filter(&filter3);

        assert_eq!(manager.total_subscriptions, Nat::from(3u32));

        // Check addresses
        assert_eq!(manager.addresses.get("address1"), Some(&Nat::from(1u32)));
        assert_eq!(manager.addresses.get("address2"), Some(&Nat::from(1u32)));
        assert_eq!(manager.addresses.get("address3"), Some(&Nat::from(1u32)));

        // Check topics for position 0: filter1 and filter2 add "A"
        assert_eq!(
            manager.topic_counts[0].get("topic1"),
            Some(&Nat::from(2u32))
        ); // Two filters use A
           // Position 1: "B" from filter1, "X" from filter2
        assert_eq!(
            manager.topic_counts[1].get("topic2"),
            Some(&Nat::from(1u32))
        );
        assert_eq!(
            manager.topic_counts[1].get("topic4"),
            Some(&Nat::from(1u32))
        );
        // Position 2: "C" from filter1, filter2 had empty => accept_any
        assert_eq!(
            manager.topic_counts[2].get("topic3"),
            Some(&Nat::from(1u32))
        );

        // Check accept_any_topic_at_position
        // filter1: last position (3) empty => accept_any_topic_at_position[3] += 1
        // filter2: positions 2 and 3 empty => accept_any_topic_at_position[2] += 1, accept_any_topic_at_position[3] += 1
        // filter3: all empty => accept_any_topic_at_position[0..4] += 1
        //
        // Position 0:
        // filter1 added "A" (no accept any)
        // filter2 added "A" (no accept any here)
        // filter3 accept any => +1
        assert_eq!(manager.accept_any_topic_at_position[0], Nat::from(1u32));

        // Position 1:
        // filter1 added "B", filter2 added "X", filter3 accept any => +1
        assert_eq!(manager.accept_any_topic_at_position[1], Nat::from(1u32));

        // Position 2:
        // filter1: "C"
        // filter2: empty => +1
        // filter3: accept any => +1
        // Total 2
        assert_eq!(manager.accept_any_topic_at_position[2], Nat::from(2u32));

        // Position 3:
        // filter1: empty => +1
        // filter2: empty => +1
        // filter3: accept any => +1
        // Total 3
        assert_eq!(manager.accept_any_topic_at_position[3], Nat::from(3u32));
    }

    #[test]
    fn test_remove_some_filters_with_overlap() {
        let mut manager = FilterManager::new();

        let filter1 = make_filter(
            vec!["address1"],
            Some(vec![vec!["topic1"], vec!["topic2"], vec!["topic3"], vec![]]),
        );
        let filter2 = make_filter(
            vec!["address2"],
            Some(vec![vec!["topic1"], vec!["topic4"], vec![], vec![]]),
        );
        let filter3 = make_filter(vec!["address3"], None); // No topics => all accept any

        manager.add_filter(&filter1);
        manager.add_filter(&filter2);
        manager.add_filter(&filter3);

        // Now remove filter2 and filter3
        manager.remove_filter(&filter2);
        manager.remove_filter(&filter3);

        // Only filter1 should remain
        assert_eq!(manager.total_subscriptions, Nat::from(1u32));

        // Addresses: address1 remains, address2 and address3 should be removed
        assert!(manager.addresses.get("address2").is_none());
        assert!(manager.addresses.get("address3").is_none());
        assert_eq!(manager.addresses.get("address1"), Some(&Nat::from(1u32)));

        // Topics:
        // After removing filter2 and filter3:
        //   filter2 and filter3 influenced accept_any and duplicated "A".
        // Only filter1 remains:
        //   topics: [ ["A"], ["B"], ["C"], [] ]
        // So for position 0 remains "A" with count 1
        assert_eq!(
            manager.topic_counts[0].get("topic1"),
            Some(&Nat::from(1u32))
        );
        // Position 1: "B" with count 1
        assert_eq!(
            manager.topic_counts[1].get("topic2"),
            Some(&Nat::from(1u32))
        );
        // Position 2: "C" with count 1
        assert_eq!(
            manager.topic_counts[2].get("topic3"),
            Some(&Nat::from(1u32))
        );
        // Position 3: empty => accept_any_topic_at_position[3] should be 1 (from filter1 only)
        // Since filter1 had that position empty, it sets accept_any = 1
        assert_eq!(manager.accept_any_topic_at_position[3], Nat::from(1u32));

        // Check that the other accept_any counts decreased correctly after removing filter2 and filter3
        // Position 0:
        // - Was +1 from filter3, now removed => 0
        assert_eq!(manager.accept_any_topic_at_position[0], Nat::from(0u32));
        // Position 1:
        // - Was +1 from filter3, now removed => 0
        assert_eq!(manager.accept_any_topic_at_position[1], Nat::from(0u32));
        // Position 2:
        // - Was 2, one from filter2, one from filter3, both removed => 0
        assert_eq!(manager.accept_any_topic_at_position[2], Nat::from(0u32));
    }

    #[test]
    fn test_get_combined_topics() {
        let mut manager = FilterManager::new();

        let filter_no_topics = make_filter(vec!["address1"], None);
        manager.add_filter(&filter_no_topics);

        // get_combined_topics should return None because all positions are accept any
        assert!(manager.get_combined_topics().is_none());

        let filter_with_topics = make_filter(
            vec!["address2"],
            Some(vec![
                vec!["topic1"],
                vec!["topic2"],
                vec!["topic3"],
                vec!["topic4"],
            ]),
        );
        manager.add_filter(&filter_with_topics);

        // Now part of it accepts any and part has specific topics.
        // According to logic, if any position accept any, we cannot produce a deterministic set for that position.
        // We should get None.
        let combined = manager.get_combined_topics();
        // First filter sets everything to accept any, second tries to set specific topics, but we still have accept any from the first.
        assert!(combined.is_none());

        // Remove the no-topic filter to remove accept any
        manager.remove_filter(&filter_no_topics);

        // Now only the filter with specific topics remains.
        // All positions have specific topics and no accept any.
        let combined = manager.get_combined_topics().unwrap();
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
            vec!["address1"],
            Some(vec![vec!["topicA"], vec![], vec!["topicC"], vec![]]),
        );
        let filter2 = make_filter(vec!["address2"], None); // Accept any for all

        manager.add_filter(&filter1);
        manager.add_filter(&filter2);

        let (addresses, topics) = manager.get_active_addresses_and_topics();

        // Check addresses
        assert!(addresses.contains(&"address1".to_string()));
        assert!(addresses.contains(&"address2".to_string()));

        // Since one filter is accept any for all, get_combined_topics returns None
        assert!(topics.is_none());

        // Remove the second filter so that only specific topics remain
        manager.remove_filter(&filter2);
        let (addresses, topics) = manager.get_active_addresses_and_topics();

        assert_eq!(addresses, vec!["address1".to_string()]);
        let topics = topics.unwrap();
        // First position: A
        assert_eq!(topics[0], vec!["topicA".to_string()]);
    }

    #[test]
    fn test_get_active_addresses_and_topics2() {
        let mut manager = FilterManager::new();

        // Scenario: add multiple filters with different combinations
        // 1. Filter with all specific topics
        let filter_full_topics = make_filter(
            vec!["address1"],
            Some(vec![
                vec!["topicA1"],
                vec!["topicA2"],
                vec!["topicA3"],
                vec!["topicA4"],
            ]),
        );

        // 2. Filter with partially defined topics and partially empty (accept any)
        let filter_partial = make_filter(
            vec!["address2"],
            Some(vec![
                vec!["topicB1"], // position 0: specific topic
                vec![],          // position 1: empty => accept any
                vec!["topicB3"], // position 2: specific topic
                vec![],          // position 3: empty => accept any
            ]),
        );

        // 3. Filter with no topics (all positions accept any)
        let filter_none = make_filter(vec!["address3"], None);

        // Add all filters
        manager.add_filter(&filter_full_topics);
        manager.add_filter(&filter_partial);
        manager.add_filter(&filter_none);

        // Check active addresses
        let (addresses, topics) = manager.get_active_addresses_and_topics();
        assert!(addresses.contains(&"address1".to_string()));
        assert!(addresses.contains(&"address2".to_string()));
        assert!(addresses.contains(&"address3".to_string()));

        // Since there is at least one filter (filter_none) that accepts any topic for all positions,
        // we expect topics == None
        assert!(topics.is_none());

        // Remove the no-topics filter to remove the universal accept any
        manager.remove_filter(&filter_none);

        // Now we have filters with partial and full topics only
        let (addresses, topics) = manager.get_active_addresses_and_topics();
        assert!(addresses.contains(&"address1".to_string()));
        assert!(addresses.contains(&"address2".to_string()));
        assert!(!addresses.contains(&"address3".to_string()));

        assert!(topics.is_some());

        // Remove the partial filter as well, leaving only the full topic filter
        manager.remove_filter(&filter_partial);

        let (addresses, topics) = manager.get_active_addresses_and_topics();
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
            vec!["address1"],
            Some(vec![
                vec!["topicA1"], // position 0: specific topic
                vec!["topicA2"], // position 1: specific topic
                vec![],          // position 2: empty => accept any
                vec![],          // position 3: empty => accept any
            ]),
        );

        // 2. A filter with multiple addresses and full topics
        let filter2 = make_filter(
            vec!["address2", "address3"],
            Some(vec![
                vec!["topicB1"],
                vec!["topicB2"],
                vec!["topicB3"],
                vec!["topicB4"],
            ]),
        );

        // 3. A filter with no topics
        let filter3 = make_filter(vec!["address4"], None);

        // 4. A filter where two positions are full and two are empty
        let filter4 = make_filter(
            vec!["address5"],
            Some(vec![
                vec!["topicC1"],
                vec![], // empty => accept any
                vec!["topicC3"],
                vec![], // empty => accept any
            ]),
        );

        // 5. A filter with one address and only one position filled (others empty)
        let filter5 = make_filter(
            vec!["address6"],
            Some(vec![
                vec!["topicD1"],
                vec![], // accept any
                vec![], // accept any
                vec![], // accept any
            ]),
        );

        // Add all these filters
        manager.add_filter(&filter1);
        manager.add_filter(&filter2);
        manager.add_filter(&filter3);
        manager.add_filter(&filter4);
        manager.add_filter(&filter5);

        // Check that all addresses are present
        let (addresses, topics) = manager.get_active_addresses_and_topics();
        for addr in &[
            "address1", "address2", "address3", "address4", "address5", "address6",
        ] {
            assert!(addresses.contains(&addr.to_string()));
        }

        // Currently, we have many accept any (from filter1, filter3, filter4, filter5).
        // Expect topics == None.
        assert!(topics.is_none());

        // Remove the no-topics filter (address4) to reduce accept any sources
        manager.remove_filter(&filter3);

        // Get active addresses and topics again
        let (addresses, topics) = manager.get_active_addresses_and_topics();
        // Check that address4 is removed
        assert!(!addresses.contains(&"address4".to_string()));

        assert!(topics.is_some());

        // Try removing all filters with accept any, leaving only the full topic filter (filter2)
        manager.remove_filter(&filter1);
        manager.remove_filter(&filter4);
        manager.remove_filter(&filter5);

        // Only filter2 remains, which has a full set of topics and no accept any
        let (addresses, topics) = manager.get_active_addresses_and_topics();
        // Only address2 and address3 should remain
        assert_eq!(addresses.len(), 2);
        assert!(addresses.contains(&"address2".to_string()));
        assert!(addresses.contains(&"address3".to_string()));

        // Now, topics should not be None, since no accept any remains
        let topics = topics.unwrap();
        assert_eq!(topics.len(), MAX_TOPICS);
        assert_eq!(topics[0], vec!["topicB1".to_string()]);
        assert_eq!(topics[1], vec!["topicB2".to_string()]);
        assert_eq!(topics[2], vec!["topicB3".to_string()]);
        assert_eq!(topics[3], vec!["topicB4".to_string()]);
    }
}
