use evm_logs_types::{Event, Filter};

// Function to check if particular event matches specific filter
pub fn event_matches_filter(event: &Event, subscribers_filter: &Filter) -> bool {
    let event_address = event.address.trim().to_lowercase();

    // Check if event address matches any subscriber address
    let filter_address = subscribers_filter.address.trim().to_lowercase();
    if filter_address != event_address {
        return false;
    }

    // If filter doesn't have topics, we match on address alone
    if subscribers_filter.topics.is_none() {
        return true;
    }

    // If no topics in the event but filter has topics, it's not a match
    if event.topics.is_none() {
        return false;
    }

    // Both filter and event have topics, so we need to match them
    if let (Some(event_topics), Some(filter_topics)) = (&event.topics, &subscribers_filter.topics) {
        // Ensure there are enough topics in the event to match the filter
        if event_topics.len() < filter_topics.len() {
            return false;
        }

        for (i, filter_topic_set) in filter_topics.iter().enumerate() {
            if let Some(event_topic) = event_topics.get(i) {
                let event_topic_trimmed = event_topic.trim().to_lowercase();
                if !filter_topic_set
                    .iter()
                    .any(|filter_topic| filter_topic.trim().to_lowercase() == event_topic_trimmed)
                {
                    return false;
                }
            } else {
                // If the event doesn't have enough topics, it doesn't match
                return false;
            }
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use evm_logs_types::Value;
    use candid::Nat;

    fn create_event(address: &str, topics: Option<Vec<&str>>) -> Event {
        Event {
            id: Nat::from(1u8),
            prev_id: None,
            timestamp: 0,
            chain_id: 1,
            data: Value::Text("test".to_string()),
            headers: None,
            address: address.to_string(),
            topics: topics.map(|t| t.into_iter().map(|s| s.to_string()).collect()),
            tx_hash: "".to_string(),
        }
    }

    fn create_filter(addresses: Vec<&str>, topics: Option<Vec<Vec<&str>>>) -> Filter {
        Filter {
            address: addresses.into_iter().map(|s| s.to_string()).collect(),
            topics: topics.map(|ts| {
                ts.into_iter()
                    .map(|topic_set| topic_set.into_iter().map(|s| s.to_string()).collect())
                    .collect()
            }),
        }
    }

    #[test]
    fn test_event_matches_filter_address_only_match() {
        let event = create_event("0xabc", None);
        let filter = create_filter(vec!["0xABC"], None);

        assert!(event_matches_filter(&event, &filter));
    }

    #[test]
    fn test_event_matches_filter_address_only_no_match() {
        let event = create_event("0xdef", None);
        let filter = create_filter(vec!["0xABC"], None);

        assert!(!event_matches_filter(&event, &filter));
    }

    #[test]
    fn test_event_matches_filter_topics_match() {
        let event = create_event("0xabc", Some(vec!["topic1", "topic2"]));
        let filter = create_filter(vec!["0xABC"], Some(vec![vec!["topic1"], vec!["topic2"]]));

        assert!(event_matches_filter(&event, &filter));
    }

    #[test]
    fn test_event_matches_filter_topics_no_match() {
        let event = create_event("0xabc", Some(vec!["topic1", "topic3"]));
        let filter = create_filter(vec!["0xABC"], Some(vec![vec!["topic1"], vec!["topic2"]]));

        assert!(!event_matches_filter(&event, &filter));
    }

    #[test]
    fn test_event_matches_filter_topics_partial_match() {
        let event = create_event("0xabc", Some(vec!["topic1"]));
        let filter = create_filter(vec!["0xABC"], Some(vec![vec!["topic1", "topic2"]]));

        assert!(event_matches_filter(&event, &filter));
    }

    #[test]
    fn test_event_matches_filter_too_few_event_topics() {
        let event = create_event("0xabc", Some(vec!["topic1"]));
        let filter = create_filter(vec!["0xABC"], Some(vec![vec!["topic1"], vec!["topic2"]]));

        assert!(!event_matches_filter(&event, &filter));
    }

    #[test]
    fn test_event_matches_filter_no_filter_topics() {
        let event = create_event("0xabc", Some(vec!["topic1", "topic2"]));
        let filter = create_filter(vec!["0xABC"], None);

        assert!(event_matches_filter(&event, &filter));
    }

    #[test]
    fn test_event_matches_filter_no_event_topics() {
        let event = create_event("0xabc", None);
        let filter = create_filter(vec!["0xABC"], Some(vec![vec!["topic1"], vec!["topic2"]]));

        assert!(!event_matches_filter(&event, &filter));
    }
}
