use evm_logs_types::{Event, Filter};

// Function to check if particular event matches specific filter
pub fn event_matches_filter(event: &Event, subscribers_filter: &Filter) -> bool {
    if subscribers_filter.address != event.log_entry.address {
        return false;
    }

    if let Some(filter_topics) = &subscribers_filter.topics {
        let event_topics = &event.log_entry.topics;

        if event_topics.len() < filter_topics.len() {
            return false;
        }

        return filter_topics.iter().enumerate().all(|(i, filter_topic_set)| {
            event_topics.get(i).is_some_and(|event_topic| filter_topic_set.contains(event_topic))
        });
    }

    true
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use candid::Nat;
    use evm_rpc_types::{Hex20, Hex32, Hex, LogEntry};
    use evm_logs_types::TopicsPosition;
    
    fn create_event(address: &str, topics: Option<Vec<&str>>) -> Event {
        Event {
            id: Nat::from(1u8),
            timestamp: 0,
            chain_id: 1,
            // data: Value::Text("test".to_string()),
            // address: address.to_string(),
            // topics: topics.map(|t| t.into_iter().map(|s| s.to_string()).collect()),
            // tx_hash: "".to_string(),
            log_entry: LogEntry {
                address: Hex20::from_str(address).unwrap(),
                topics: topics
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|s| Hex32::from_str(s).ok())
                    .collect(),
                data: Hex::from(vec![]),
                block_number: None,
                transaction_hash: None,
                transaction_index: None,
                block_hash: None,
                log_index: None,
                removed: false,
            },
        }
    }

    fn create_filter(address: &str, topics: Option<Vec<Vec<&str>>>) -> Filter {
        Filter {
            address: Hex20::from_str(address).unwrap(),
            topics: topics.map(|ts| {
                ts.into_iter()
                    .map(|topic_set| topic_set.into_iter()
                        .filter_map(|s| Hex32::from_str(s).ok())
                        .collect::<TopicsPosition>())
                    .collect()
            }),
        }
    }

    #[test]
    fn test_event_matches_filter_address_only_match() {
        let event = create_event("0xabc", None);
        let filter = create_filter("0xABC", None);

        assert!(event_matches_filter(&event, &filter));
    }

    #[test]
    fn test_event_matches_filter_address_only_no_match() {
        let event = create_event("0xdef", None);
        let filter = create_filter("0xABC", None);

        assert!(!event_matches_filter(&event, &filter));
    }

    #[test]
    fn test_event_matches_filter_topics_match() {
        let event = create_event("0xabc", Some(vec!["topic1", "topic2"]));
        let filter = create_filter("0xABC", Some(vec![vec!["topic1"], vec!["topic2"]]));

        assert!(event_matches_filter(&event, &filter));
    }

    #[test]
    fn test_event_matches_filter_topics_no_match() {
        let event = create_event("0xabc", Some(vec!["topic1", "topic3"]));
        let filter = create_filter("0xABC", Some(vec![vec!["topic1"], vec!["topic2"]]));

        assert!(!event_matches_filter(&event, &filter));
    }

    #[test]
    fn test_event_matches_filter_topics_partial_match() {
        let event = create_event("0xabc", Some(vec!["topic1"]));
        let filter = create_filter("0xABC", Some(vec![vec!["topic1", "topic2"]]));

        assert!(event_matches_filter(&event, &filter));
    }

    #[test]
    fn test_event_matches_filter_too_few_event_topics() {
        let event = create_event("0xabc", Some(vec!["topic1"]));
        let filter = create_filter("0xABC", Some(vec![vec!["topic1"], vec!["topic2"]]));

        assert!(!event_matches_filter(&event, &filter));
    }

    #[test]
    fn test_event_matches_filter_no_filter_topics() {
        let event = create_event("0xabc", Some(vec!["topic1", "topic2"]));
        let filter = create_filter("0xABC", None);

        assert!(event_matches_filter(&event, &filter));
    }

    #[test]
    fn test_event_matches_filter_no_event_topics() {
        let event = create_event("0xabc", None);
        let filter = create_filter("0xABC", Some(vec![vec!["topic1"], vec!["topic2"]]));

        assert!(!event_matches_filter(&event, &filter));
    }
}
